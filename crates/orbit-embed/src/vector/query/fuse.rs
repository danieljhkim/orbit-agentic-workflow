use std::collections::BTreeMap;

use super::{Bm25Hit, CosineHit};

pub const RRF_K: f32 = 60.0;

#[derive(Debug, Clone, PartialEq)]
pub struct FusedCandidate {
    pub source_kind: String,
    pub source_id: String,
    pub field: String,
    pub chunk_idx_for_snippet: Option<usize>,
    pub rowid_for_snippet: Option<i64>,
    pub score: f32,
    pub bm25_rank: Option<usize>,
    pub cosine_rank: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CandidateKey {
    source_kind: String,
    source_id: String,
    field: String,
}

pub fn reciprocal_rank_fusion(cosine: &[CosineHit], bm25: &[Bm25Hit]) -> Vec<FusedCandidate> {
    let mut by_key = BTreeMap::<CandidateKey, FusedCandidate>::new();
    for hit in cosine {
        let entry = by_key
            .entry(CandidateKey {
                source_kind: hit.source_kind.clone(),
                source_id: hit.source_id.clone(),
                field: hit.field.clone(),
            })
            .or_insert_with(|| FusedCandidate {
                source_kind: hit.source_kind.clone(),
                source_id: hit.source_id.clone(),
                field: hit.field.clone(),
                chunk_idx_for_snippet: Some(hit.chunk_idx),
                rowid_for_snippet: None,
                score: 0.0,
                bm25_rank: None,
                cosine_rank: None,
            });
        if should_replace_rank(entry.cosine_rank, hit.rank) {
            if let Some(rank) = entry.cosine_rank {
                entry.score -= rrf_contribution(rank);
            }
            entry.score += rrf_contribution(hit.rank);
            entry.cosine_rank = Some(hit.rank);
            entry.chunk_idx_for_snippet = Some(hit.chunk_idx);
        }
    }
    for hit in bm25 {
        let entry = by_key
            .entry(CandidateKey {
                source_kind: hit.source_kind.clone(),
                source_id: hit.source_id.clone(),
                field: hit.field.clone(),
            })
            .or_insert_with(|| FusedCandidate {
                source_kind: hit.source_kind.clone(),
                source_id: hit.source_id.clone(),
                field: hit.field.clone(),
                chunk_idx_for_snippet: None,
                rowid_for_snippet: Some(hit.rowid),
                score: 0.0,
                bm25_rank: None,
                cosine_rank: None,
            });
        if should_replace_rank(entry.bm25_rank, hit.rank) {
            if let Some(rank) = entry.bm25_rank {
                entry.score -= rrf_contribution(rank);
            }
            entry.score += rrf_contribution(hit.rank);
            entry.bm25_rank = Some(hit.rank);
            entry.rowid_for_snippet = Some(hit.rowid);
        }
    }
    let mut candidates = by_key.into_values().collect::<Vec<_>>();
    candidates.sort_by(compare_fused_candidates);
    candidates
}

pub(crate) fn compare_fused_candidates(
    left: &FusedCandidate,
    right: &FusedCandidate,
) -> std::cmp::Ordering {
    right
        .score
        .total_cmp(&left.score)
        .then_with(|| left.source_kind.cmp(&right.source_kind))
        .then_with(|| left.source_id.cmp(&right.source_id))
        .then_with(|| left.field.cmp(&right.field))
}

pub(crate) fn rrf_contribution(rank: usize) -> f32 {
    1.0 / (RRF_K + rank as f32)
}

fn should_replace_rank(current: Option<usize>, candidate: usize) -> bool {
    current.is_none_or(|rank| candidate < rank)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cosine(id: &str, rank: usize) -> CosineHit {
        CosineHit {
            source_kind: "task".to_string(),
            source_id: id.to_string(),
            field: "purpose".to_string(),
            chunk_idx: 0,
            score: 1.0 / rank as f32,
            rank,
        }
    }

    fn bm25(id: &str, rank: usize) -> Bm25Hit {
        Bm25Hit {
            source_kind: "task".to_string(),
            source_id: id.to_string(),
            field: "purpose".to_string(),
            rowid: rank as i64,
            rank,
        }
    }

    #[test]
    fn rrf_reproduces_hand_computed_rank_example() {
        let fused = reciprocal_rank_fusion(
            &[cosine("A", 1), cosine("B", 2), cosine("C", 3)],
            &[bm25("B", 1), bm25("A", 2), bm25("D", 3)],
        );

        let by_id = fused
            .iter()
            .map(|hit| (hit.source_id.as_str(), hit.score))
            .collect::<BTreeMap<_, _>>();
        let ab_expected = (1.0 / 61.0) + (1.0 / 62.0);
        let cd_expected = 1.0 / 63.0;
        assert!((by_id["A"] - ab_expected).abs() < 0.000001);
        assert!((by_id["B"] - ab_expected).abs() < 0.000001);
        assert!((by_id["C"] - cd_expected).abs() < 0.000001);
        assert!((by_id["D"] - cd_expected).abs() < 0.000001);
        assert_eq!(
            fused
                .iter()
                .map(|hit| hit.source_id.as_str())
                .collect::<Vec<_>>(),
            vec!["A", "B", "C", "D"]
        );
    }

    #[test]
    fn rrf_counts_one_rank_per_retriever_per_field() {
        let mut lower_chunk = cosine("A", 2);
        lower_chunk.chunk_idx = 1;
        let fused = reciprocal_rank_fusion(&[cosine("A", 1), lower_chunk], &[]);

        assert_eq!(fused.len(), 1);
        assert!((fused[0].score - (1.0 / 61.0)).abs() < 0.000001);
        assert_eq!(fused[0].cosine_rank, Some(1));
        assert_eq!(fused[0].chunk_idx_for_snippet, Some(0));
    }
}
