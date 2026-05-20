use orbit_common::types::OrbitError;
use rusqlite::params;

use crate::vector::store::VectorStore;

#[derive(Debug, Clone, PartialEq)]
pub struct Bm25Hit {
    pub source_kind: String,
    pub source_id: String,
    pub field: String,
    pub rowid: i64,
    pub rank: usize,
}

pub fn bm25_top_k(
    store: &VectorStore,
    query: &str,
    kind: Option<&str>,
    limit: usize,
) -> Result<Vec<Bm25Hit>, OrbitError> {
    if query.trim().is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    let match_query = fts_phrase_quote(query);
    let conn = store.connection();
    let conn = conn
        .lock()
        .map_err(|error| OrbitError::Store(format!("mutex poisoned: {error}")))?;
    let mut hits = Vec::new();
    if let Some(kind) = kind {
        let mut stmt = conn
            .prepare(
                r#"
                    SELECT source_kind, source_id, field, rowid, bm25(corpus_fts) AS rank
                    FROM corpus_fts
                    WHERE corpus_fts MATCH ?1 AND source_kind = ?2
                    ORDER BY rank
                    LIMIT ?3
                "#,
            )
            .map_err(|error| OrbitError::Store(error.to_string()))?;
        let mut rows = stmt
            .query(params![match_query, kind, limit as i64])
            .map_err(|error| OrbitError::Store(error.to_string()))?;
        collect_hits(&mut rows, &mut hits)?;
    } else {
        let mut stmt = conn
            .prepare(
                r#"
                    SELECT source_kind, source_id, field, rowid, bm25(corpus_fts) AS rank
                    FROM corpus_fts
                    WHERE corpus_fts MATCH ?1
                    ORDER BY rank
                    LIMIT ?2
                "#,
            )
            .map_err(|error| OrbitError::Store(error.to_string()))?;
        let mut rows = stmt
            .query(params![match_query, limit as i64])
            .map_err(|error| OrbitError::Store(error.to_string()))?;
        collect_hits(&mut rows, &mut hits)?;
    }
    Ok(hits)
}

fn collect_hits(rows: &mut rusqlite::Rows<'_>, hits: &mut Vec<Bm25Hit>) -> Result<(), OrbitError> {
    while let Some(row) = rows
        .next()
        .map_err(|error| OrbitError::Store(error.to_string()))?
    {
        hits.push(Bm25Hit {
            source_kind: row
                .get(0)
                .map_err(|error| OrbitError::Store(error.to_string()))?,
            source_id: row
                .get(1)
                .map_err(|error| OrbitError::Store(error.to_string()))?,
            field: row
                .get(2)
                .map_err(|error| OrbitError::Store(error.to_string()))?,
            rowid: row
                .get(3)
                .map_err(|error| OrbitError::Store(error.to_string()))?,
            rank: hits.len() + 1,
        });
    }
    Ok(())
}

pub fn snippet_for_hit(
    store: &VectorStore,
    source_kind: &str,
    source_id: &str,
    field: &str,
    chunk_idx: Option<usize>,
    rowid: Option<i64>,
) -> Result<Option<String>, OrbitError> {
    if let Some(rowid) = rowid {
        return snippet_by_rowid(store, rowid);
    }
    let Some(chunk_idx) = chunk_idx else {
        return Ok(None);
    };
    snippet_by_chunk_idx(store, source_kind, source_id, field, chunk_idx)
}

fn snippet_by_rowid(store: &VectorStore, rowid: i64) -> Result<Option<String>, OrbitError> {
    let conn = store.connection();
    let conn = conn
        .lock()
        .map_err(|error| OrbitError::Store(format!("mutex poisoned: {error}")))?;
    conn.query_row(
        "SELECT content FROM corpus_fts WHERE rowid = ?1",
        params![rowid],
        |row| row.get::<_, String>(0),
    )
    .map(Some)
    .or_else(|error| match error {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(OrbitError::Store(other.to_string())),
    })
}

fn snippet_by_chunk_idx(
    store: &VectorStore,
    source_kind: &str,
    source_id: &str,
    field: &str,
    chunk_idx: usize,
) -> Result<Option<String>, OrbitError> {
    let conn = store.connection();
    let conn = conn
        .lock()
        .map_err(|error| OrbitError::Store(format!("mutex poisoned: {error}")))?;
    conn.query_row(
        r#"
            SELECT content
            FROM corpus_fts
            WHERE source_kind = ?1 AND source_id = ?2 AND field = ?3
            ORDER BY rowid
            LIMIT 1 OFFSET ?4
        "#,
        params![source_kind, source_id, field, chunk_idx as i64],
        |row| row.get::<_, String>(0),
    )
    .map(Some)
    .or_else(|error| match error {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(OrbitError::Store(other.to_string())),
    })
}

fn fts_phrase_quote(query: &str) -> String {
    format!("\"{}\"", query.trim().replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NoopEmbedder;
    use crate::vector::EmbeddingField;

    #[test]
    fn bm25_top_k_ranks_lexical_matches() {
        let store = VectorStore::open_in_memory().unwrap();
        let embedder = NoopEmbedder::small();
        for (id, text) in [
            ("T1", "alpha beta"),
            ("T2", "neutrino unique token"),
            ("T3", "gamma delta"),
        ] {
            store
                .upsert_embeddings(
                    "task",
                    id,
                    &[EmbeddingField::new("purpose", text)],
                    &embedder,
                    false,
                )
                .unwrap();
        }

        let hits = bm25_top_k(&store, "neutrino", Some("task"), 3).unwrap();

        assert_eq!(hits[0].source_id, "T2");
        assert_eq!(hits[0].field, "purpose");
        assert_eq!(hits[0].rank, 1);
    }

    #[test]
    fn bm25_top_k_filters_by_source_kind() {
        let store = VectorStore::open_in_memory().unwrap();
        let embedder = NoopEmbedder::small();
        store
            .upsert_embeddings(
                "task",
                "T1",
                &[EmbeddingField::new("purpose", "neutrino task")],
                &embedder,
                false,
            )
            .unwrap();
        store
            .upsert_embeddings(
                "doc",
                "D1",
                &[EmbeddingField::new("summary", "neutrino doc")],
                &embedder,
                false,
            )
            .unwrap();

        let task_hits = bm25_top_k(&store, "neutrino", Some("task"), 10).unwrap();
        let all_hits = bm25_top_k(&store, "neutrino", None, 10).unwrap();

        assert_eq!(task_hits.len(), 1);
        assert_eq!(task_hits[0].source_kind, "task");
        assert_eq!(all_hits.len(), 2);
    }

    #[test]
    fn bm25_phrase_quotes_embedded_double_quotes() {
        assert_eq!(
            fts_phrase_quote("foo \"bar\" baz"),
            "\"foo \"\"bar\"\" baz\""
        );
    }

    #[test]
    fn snippet_lookup_preserves_chunk_order() {
        let store = VectorStore::open_in_memory().unwrap();
        let conn = store.connection();
        let conn = conn.lock().unwrap();
        conn.execute(
            "INSERT INTO corpus_fts(source_kind, source_id, field, content) VALUES (?1, ?2, ?3, ?4)",
            ("task", "T1", "purpose", "first chunk"),
        )
        .unwrap();
        conn.execute(
            "INSERT INTO corpus_fts(source_kind, source_id, field, content) VALUES (?1, ?2, ?3, ?4)",
            ("task", "T1", "purpose", "second chunk"),
        )
        .unwrap();
        drop(conn);

        let snippet = snippet_for_hit(&store, "task", "T1", "purpose", Some(1), None).unwrap();

        assert_eq!(snippet.as_deref(), Some("second chunk"));
    }
}
