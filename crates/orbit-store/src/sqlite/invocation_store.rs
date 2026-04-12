use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use orbit_types::{InvocationTrace, OrbitError};
use rusqlite::params;

use crate::{Store, now_string};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct InvocationQuery {
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub job_run_id: Option<String>,
    pub activity_id: Option<String>,
    pub task_id: Option<String>,
    pub agent: Option<String>,
    pub model: Option<String>,
    pub tool_name: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone)]
pub struct InvocationInsertParams {
    pub job_run_id: String,
    pub activity_id: String,
    pub agent: String,
    pub model: Option<String>,
    pub task_ids: Vec<String>,
    pub trace: InvocationTrace,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InvocationToolCallRecord {
    pub invocation_id: i64,
    pub seq: u64,
    pub tool_name: String,
    pub result_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InvocationRecord {
    pub id: i64,
    pub ts: DateTime<Utc>,
    pub job_run_id: String,
    pub activity_id: String,
    pub agent: String,
    pub model: Option<String>,
    pub duration_ms: u64,
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_create_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub tool_call_count: u64,
    pub task_ids: Vec<String>,
    pub tool_calls: Vec<InvocationToolCallRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActivityInvocationMetrics {
    pub activity_id: String,
    pub agent: String,
    pub model: Option<String>,
    pub invocation_count: u64,
    pub total_input_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub total_cache_create_tokens: u64,
    pub total_output_tokens: u64,
    pub total_tokens: u64,
    pub avg_tokens: f64,
    pub p50_tokens: u64,
    pub p95_tokens: u64,
    pub total_tool_calls: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentInvocationMetrics {
    pub agent: String,
    pub model: Option<String>,
    pub invocation_count: u64,
    pub total_input_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub total_cache_create_tokens: u64,
    pub total_output_tokens: u64,
    pub total_tokens: u64,
    pub avg_tokens: f64,
    pub p50_tokens: u64,
    pub p95_tokens: u64,
    pub total_tool_calls: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskInvocationMetrics {
    pub task_id: String,
    pub invocation_count: u64,
    pub total_input_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub total_cache_create_tokens: u64,
    pub total_output_tokens: u64,
    pub total_tokens: u64,
    pub total_tool_calls: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolInvocationMetrics {
    pub activity_id: String,
    pub tool_name: String,
    pub call_count: u64,
    pub avg_result_bytes: f64,
    pub total_result_bytes: u64,
}

#[derive(Debug, Clone)]
struct InvocationSample {
    activity_id: String,
    agent: String,
    model: Option<String>,
    input_tokens: u64,
    cache_read_tokens: u64,
    cache_create_tokens: u64,
    output_tokens: u64,
    tool_call_count: u64,
}

#[derive(Debug, Clone, Default)]
struct MetricBucket {
    invocation_count: u64,
    total_input_tokens: u64,
    total_cache_read_tokens: u64,
    total_cache_create_tokens: u64,
    total_output_tokens: u64,
    total_tool_calls: u64,
    totals: Vec<u64>,
}

impl MetricBucket {
    fn add(&mut self, sample: &InvocationSample) {
        self.invocation_count += 1;
        self.total_input_tokens += sample.input_tokens;
        self.total_cache_read_tokens += sample.cache_read_tokens;
        self.total_cache_create_tokens += sample.cache_create_tokens;
        self.total_output_tokens += sample.output_tokens;
        self.total_tool_calls += sample.tool_call_count;
        self.totals
            .push(sample.input_tokens.saturating_add(sample.output_tokens));
    }
}

#[derive(Debug, Clone)]
struct TaskSample {
    task_id: String,
    input_tokens: u64,
    cache_read_tokens: u64,
    cache_create_tokens: u64,
    output_tokens: u64,
    tool_call_count: u64,
}

impl Store {
    pub fn insert_invocation_trace_record(
        &self,
        params: &InvocationInsertParams,
    ) -> Result<(), OrbitError> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;
        let tx = conn
            .transaction()
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        tx.execute(
            r#"INSERT INTO invocations(
                ts, job_run_id, activity_id, agent, model, duration_ms,
                input_tokens, cache_read_tokens, cache_create_tokens,
                output_tokens, tool_call_count
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"#,
            params![
                now_string(),
                params.job_run_id,
                params.activity_id,
                params.agent,
                params.model,
                params.trace.duration_ms as i64,
                params.trace.usage.input as i64,
                params.trace.usage.cache_read as i64,
                params.trace.usage.cache_create as i64,
                params.trace.usage.output as i64,
                params.trace.tool_calls.len() as i64,
            ],
        )
        .map_err(|e| OrbitError::Store(e.to_string()))?;

        let invocation_id = tx.last_insert_rowid();

        for task_id in &params.task_ids {
            tx.execute(
                "INSERT OR IGNORE INTO invocation_tasks(invocation_id, task_id) VALUES (?1, ?2)",
                params![invocation_id, task_id],
            )
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        }

        for tool_call in &params.trace.tool_calls {
            tx.execute(
                r#"INSERT INTO tool_calls(invocation_id, seq, tool_name, result_bytes)
                   VALUES (?1, ?2, ?3, ?4)"#,
                params![
                    invocation_id,
                    tool_call.seq as i64,
                    tool_call.tool_name,
                    tool_call.result_bytes as i64,
                ],
            )
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        }

        tx.commit().map_err(|e| OrbitError::Store(e.to_string()))
    }

    pub fn list_invocation_records(
        &self,
        filter: &InvocationQuery,
    ) -> Result<Vec<InvocationRecord>, OrbitError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;

        let mut conditions = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref since) = filter.since {
            conditions.push(format!("i.ts >= ?{}", param_values.len() + 1));
            param_values.push(Box::new(since.to_rfc3339()));
        }
        if let Some(ref until) = filter.until {
            conditions.push(format!("i.ts <= ?{}", param_values.len() + 1));
            param_values.push(Box::new(until.to_rfc3339()));
        }
        if let Some(ref job_run_id) = filter.job_run_id {
            conditions.push(format!("i.job_run_id = ?{}", param_values.len() + 1));
            param_values.push(Box::new(job_run_id.clone()));
        }
        if let Some(ref activity_id) = filter.activity_id {
            conditions.push(format!("i.activity_id = ?{}", param_values.len() + 1));
            param_values.push(Box::new(activity_id.clone()));
        }
        if let Some(ref task_id) = filter.task_id {
            conditions.push(format!(
                "EXISTS (SELECT 1 FROM invocation_tasks it WHERE it.invocation_id = i.id AND it.task_id = ?{})",
                param_values.len() + 1
            ));
            param_values.push(Box::new(task_id.clone()));
        }
        if let Some(ref agent) = filter.agent {
            conditions.push(format!("i.agent = ?{}", param_values.len() + 1));
            param_values.push(Box::new(agent.clone()));
        }
        if let Some(ref model) = filter.model {
            conditions.push(format!("i.model = ?{}", param_values.len() + 1));
            param_values.push(Box::new(model.clone()));
        }
        if let Some(ref tool_name) = filter.tool_name {
            conditions.push(format!(
                "EXISTS (SELECT 1 FROM tool_calls tc WHERE tc.invocation_id = i.id AND tc.tool_name = ?{})",
                param_values.len() + 1
            ));
            param_values.push(Box::new(tool_name.clone()));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let limit = if filter.limit == 0 { 100 } else { filter.limit };
        let sql = format!(
            "SELECT i.id, i.ts, i.job_run_id, i.activity_id, i.agent, i.model, i.duration_ms, \
             i.input_tokens, i.cache_read_tokens, i.cache_create_tokens, i.output_tokens, \
             i.tool_call_count \
             FROM invocations i {where_clause} ORDER BY i.ts DESC, i.id DESC LIMIT ?{}",
            param_values.len() + 1
        );
        param_values.push(Box::new(limit as i64));

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|value| value.as_ref()).collect();
        let rows = stmt
            .query_map(param_refs.as_slice(), |row| {
                let ts_raw: String = row.get(1)?;
                Ok(InvocationRecord {
                    id: row.get(0)?,
                    ts: DateTime::parse_from_rfc3339(&ts_raw)
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                ts_raw.len(),
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?
                        .with_timezone(&Utc),
                    job_run_id: row.get(2)?,
                    activity_id: row.get(3)?,
                    agent: row.get(4)?,
                    model: row.get(5)?,
                    duration_ms: row.get::<_, i64>(6)? as u64,
                    input_tokens: row.get::<_, i64>(7)? as u64,
                    cache_read_tokens: row.get::<_, i64>(8)? as u64,
                    cache_create_tokens: row.get::<_, i64>(9)? as u64,
                    output_tokens: row.get::<_, i64>(10)? as u64,
                    total_tokens: row.get::<_, i64>(7)? as u64 + row.get::<_, i64>(10)? as u64,
                    tool_call_count: row.get::<_, i64>(11)? as u64,
                    task_ids: Vec::new(),
                    tool_calls: Vec::new(),
                })
            })
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        let mut records = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        drop(stmt);
        drop(conn);
        if records.is_empty() {
            return Ok(records);
        }

        let invocation_ids = records.iter().map(|record| record.id).collect::<Vec<_>>();
        let task_ids = self.load_invocation_task_ids(&invocation_ids)?;
        let tool_calls = self.load_invocation_tool_calls(&invocation_ids)?;

        let mut index_by_id = HashMap::new();
        for (index, record) in records.iter().enumerate() {
            index_by_id.insert(record.id, index);
        }

        for (invocation_id, values) in task_ids {
            if let Some(index) = index_by_id.get(&invocation_id).copied() {
                records[index].task_ids = values;
            }
        }
        for (invocation_id, values) in tool_calls {
            if let Some(index) = index_by_id.get(&invocation_id).copied() {
                records[index].tool_calls = values;
            }
        }

        Ok(records)
    }

    pub fn list_activity_invocation_metrics(
        &self,
    ) -> Result<Vec<ActivityInvocationMetrics>, OrbitError> {
        let samples = self.load_invocation_samples()?;
        let grouped = self.group_invocation_metrics(samples, |sample| {
            (
                sample.activity_id.clone(),
                sample.agent.clone(),
                sample.model.clone(),
            )
        });
        let mut rows = grouped
            .into_iter()
            .map(|((activity_id, agent, model), bucket)| {
                let mut totals = bucket.totals;
                totals.sort_unstable();
                ActivityInvocationMetrics {
                    activity_id,
                    agent,
                    model,
                    invocation_count: bucket.invocation_count,
                    total_input_tokens: bucket.total_input_tokens,
                    total_cache_read_tokens: bucket.total_cache_read_tokens,
                    total_cache_create_tokens: bucket.total_cache_create_tokens,
                    total_output_tokens: bucket.total_output_tokens,
                    total_tokens: bucket
                        .total_input_tokens
                        .saturating_add(bucket.total_output_tokens),
                    avg_tokens: average(&totals),
                    p50_tokens: percentile(&totals, 50),
                    p95_tokens: percentile(&totals, 95),
                    total_tool_calls: bucket.total_tool_calls,
                }
            })
            .collect::<Vec<_>>();

        rows.sort_by(|left, right| {
            right
                .total_tokens
                .cmp(&left.total_tokens)
                .then_with(|| left.activity_id.cmp(&right.activity_id))
                .then_with(|| left.agent.cmp(&right.agent))
                .then_with(|| left.model.cmp(&right.model))
        });
        Ok(rows)
    }

    pub fn list_agent_invocation_metrics(&self) -> Result<Vec<AgentInvocationMetrics>, OrbitError> {
        let samples = self.load_invocation_samples()?;
        let grouped = self.group_invocation_metrics(samples, |sample| {
            (sample.agent.clone(), sample.model.clone())
        });
        let mut rows = grouped
            .into_iter()
            .map(|((agent, model), bucket)| {
                let mut totals = bucket.totals;
                totals.sort_unstable();
                AgentInvocationMetrics {
                    agent,
                    model,
                    invocation_count: bucket.invocation_count,
                    total_input_tokens: bucket.total_input_tokens,
                    total_cache_read_tokens: bucket.total_cache_read_tokens,
                    total_cache_create_tokens: bucket.total_cache_create_tokens,
                    total_output_tokens: bucket.total_output_tokens,
                    total_tokens: bucket
                        .total_input_tokens
                        .saturating_add(bucket.total_output_tokens),
                    avg_tokens: average(&totals),
                    p50_tokens: percentile(&totals, 50),
                    p95_tokens: percentile(&totals, 95),
                    total_tool_calls: bucket.total_tool_calls,
                }
            })
            .collect::<Vec<_>>();

        rows.sort_by(|left, right| {
            right
                .total_tokens
                .cmp(&left.total_tokens)
                .then_with(|| left.agent.cmp(&right.agent))
                .then_with(|| left.model.cmp(&right.model))
        });
        Ok(rows)
    }

    pub fn get_task_invocation_metrics(
        &self,
        task_id: &str,
    ) -> Result<TaskInvocationMetrics, OrbitError> {
        let mut rows = self.list_task_invocation_metrics(Some(task_id))?;
        Ok(rows.pop().unwrap_or(TaskInvocationMetrics {
            task_id: task_id.to_string(),
            invocation_count: 0,
            total_input_tokens: 0,
            total_cache_read_tokens: 0,
            total_cache_create_tokens: 0,
            total_output_tokens: 0,
            total_tokens: 0,
            total_tool_calls: 0,
        }))
    }

    pub fn list_top_task_invocation_metrics(
        &self,
        limit: usize,
    ) -> Result<Vec<TaskInvocationMetrics>, OrbitError> {
        let mut rows = self.list_task_invocation_metrics(None)?;
        if limit > 0 && rows.len() > limit {
            rows.truncate(limit);
        }
        Ok(rows)
    }

    pub fn list_tool_invocation_metrics(&self) -> Result<Vec<ToolInvocationMetrics>, OrbitError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;
        let mut stmt = conn
            .prepare(
                r#"
                SELECT i.activity_id, tc.tool_name, COUNT(*) AS call_count,
                       COALESCE(AVG(tc.result_bytes), 0), COALESCE(SUM(tc.result_bytes), 0)
                FROM tool_calls tc
                INNER JOIN invocations i ON i.id = tc.invocation_id
                GROUP BY i.activity_id, tc.tool_name
                ORDER BY call_count DESC, i.activity_id ASC, tc.tool_name ASC
                "#,
            )
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(ToolInvocationMetrics {
                    activity_id: row.get(0)?,
                    tool_name: row.get(1)?,
                    call_count: row.get::<_, i64>(2)? as u64,
                    avg_result_bytes: row.get(3)?,
                    total_result_bytes: row.get::<_, i64>(4)? as u64,
                })
            })
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| OrbitError::Store(e.to_string()))
    }

    fn load_invocation_task_ids(
        &self,
        invocation_ids: &[i64],
    ) -> Result<HashMap<i64, Vec<String>>, OrbitError> {
        if invocation_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;
        let placeholders = invocation_ids
            .iter()
            .enumerate()
            .map(|(index, _)| format!("?{}", index + 1))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT invocation_id, task_id FROM invocation_tasks WHERE invocation_id IN ({placeholders}) ORDER BY invocation_id ASC, task_id ASC"
        );
        let params: Vec<Box<dyn rusqlite::types::ToSql>> = invocation_ids
            .iter()
            .map(|id| Box::new(*id) as Box<dyn rusqlite::types::ToSql>)
            .collect();
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|value| value.as_ref()).collect();
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        let rows = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        let mut grouped: HashMap<i64, Vec<String>> = HashMap::new();
        for row in rows {
            let (invocation_id, task_id) = row.map_err(|e| OrbitError::Store(e.to_string()))?;
            grouped.entry(invocation_id).or_default().push(task_id);
        }
        Ok(grouped)
    }

    fn load_invocation_tool_calls(
        &self,
        invocation_ids: &[i64],
    ) -> Result<HashMap<i64, Vec<InvocationToolCallRecord>>, OrbitError> {
        if invocation_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;
        let placeholders = invocation_ids
            .iter()
            .enumerate()
            .map(|(index, _)| format!("?{}", index + 1))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT invocation_id, seq, tool_name, result_bytes FROM tool_calls WHERE invocation_id IN ({placeholders}) ORDER BY invocation_id ASC, seq ASC"
        );
        let params: Vec<Box<dyn rusqlite::types::ToSql>> = invocation_ids
            .iter()
            .map(|id| Box::new(*id) as Box<dyn rusqlite::types::ToSql>)
            .collect();
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|value| value.as_ref()).collect();
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        let rows = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok(InvocationToolCallRecord {
                    invocation_id: row.get(0)?,
                    seq: row.get::<_, i64>(1)? as u64,
                    tool_name: row.get(2)?,
                    result_bytes: row.get::<_, i64>(3)? as u64,
                })
            })
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        let mut grouped: HashMap<i64, Vec<InvocationToolCallRecord>> = HashMap::new();
        for row in rows {
            let call = row.map_err(|e| OrbitError::Store(e.to_string()))?;
            grouped.entry(call.invocation_id).or_default().push(call);
        }
        Ok(grouped)
    }

    fn load_invocation_samples(&self) -> Result<Vec<InvocationSample>, OrbitError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;
        let mut stmt = conn
            .prepare(
                r#"
                SELECT activity_id, agent, model, input_tokens, cache_read_tokens,
                       cache_create_tokens, output_tokens, tool_call_count
                FROM invocations
                ORDER BY activity_id ASC, agent ASC, model ASC, id ASC
                "#,
            )
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(InvocationSample {
                    activity_id: row.get(0)?,
                    agent: row.get(1)?,
                    model: row.get(2)?,
                    input_tokens: row.get::<_, i64>(3)? as u64,
                    cache_read_tokens: row.get::<_, i64>(4)? as u64,
                    cache_create_tokens: row.get::<_, i64>(5)? as u64,
                    output_tokens: row.get::<_, i64>(6)? as u64,
                    tool_call_count: row.get::<_, i64>(7)? as u64,
                })
            })
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| OrbitError::Store(e.to_string()))
    }

    fn group_invocation_metrics<K, F>(
        &self,
        samples: Vec<InvocationSample>,
        key_fn: F,
    ) -> BTreeMap<K, MetricBucket>
    where
        K: Ord,
        F: Fn(&InvocationSample) -> K,
    {
        let mut grouped: BTreeMap<K, MetricBucket> = BTreeMap::new();
        for sample in samples {
            let key = key_fn(&sample);
            grouped.entry(key).or_default().add(&sample);
        }
        grouped
    }

    fn list_task_invocation_metrics(
        &self,
        task_id: Option<&str>,
    ) -> Result<Vec<TaskInvocationMetrics>, OrbitError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;

        let sql = if task_id.is_some() {
            r#"
            SELECT it.task_id, i.input_tokens, i.cache_read_tokens, i.cache_create_tokens,
                   i.output_tokens, i.tool_call_count
            FROM invocation_tasks it
            INNER JOIN invocations i ON i.id = it.invocation_id
            WHERE it.task_id = ?1
            ORDER BY it.task_id ASC, i.id ASC
            "#
        } else {
            r#"
            SELECT it.task_id, i.input_tokens, i.cache_read_tokens, i.cache_create_tokens,
                   i.output_tokens, i.tool_call_count
            FROM invocation_tasks it
            INNER JOIN invocations i ON i.id = it.invocation_id
            ORDER BY it.task_id ASC, i.id ASC
            "#
        };

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        let mapper = |row: &rusqlite::Row<'_>| -> rusqlite::Result<TaskSample> {
            Ok(TaskSample {
                task_id: row.get(0)?,
                input_tokens: row.get::<_, i64>(1)? as u64,
                cache_read_tokens: row.get::<_, i64>(2)? as u64,
                cache_create_tokens: row.get::<_, i64>(3)? as u64,
                output_tokens: row.get::<_, i64>(4)? as u64,
                tool_call_count: row.get::<_, i64>(5)? as u64,
            })
        };

        let rows = if let Some(task_id) = task_id {
            stmt.query_map(params![task_id], mapper)
        } else {
            stmt.query_map([], mapper)
        }
        .map_err(|e| OrbitError::Store(e.to_string()))?;

        let mut grouped: HashMap<String, TaskInvocationMetrics> = HashMap::new();
        for row in rows {
            let row = row.map_err(|e| OrbitError::Store(e.to_string()))?;
            let entry =
                grouped
                    .entry(row.task_id.clone())
                    .or_insert_with(|| TaskInvocationMetrics {
                        task_id: row.task_id.clone(),
                        invocation_count: 0,
                        total_input_tokens: 0,
                        total_cache_read_tokens: 0,
                        total_cache_create_tokens: 0,
                        total_output_tokens: 0,
                        total_tokens: 0,
                        total_tool_calls: 0,
                    });
            entry.invocation_count += 1;
            entry.total_input_tokens += row.input_tokens;
            entry.total_cache_read_tokens += row.cache_read_tokens;
            entry.total_cache_create_tokens += row.cache_create_tokens;
            entry.total_output_tokens += row.output_tokens;
            entry.total_tokens += row.input_tokens.saturating_add(row.output_tokens);
            entry.total_tool_calls += row.tool_call_count;
        }

        let mut values = grouped.into_values().collect::<Vec<_>>();
        values.sort_by(|left, right| {
            right
                .total_tokens
                .cmp(&left.total_tokens)
                .then_with(|| left.task_id.cmp(&right.task_id))
        });
        Ok(values)
    }
}

fn average(values: &[u64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<u64>() as f64 / values.len() as f64
    }
}

fn percentile(sorted_values: &[u64], percentile: u64) -> u64 {
    if sorted_values.is_empty() {
        return 0;
    }
    let n = sorted_values.len();
    let rank = (percentile as usize * n).div_ceil(100);
    let index = rank.saturating_sub(1).min(n - 1);
    sorted_values[index]
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use orbit_types::{InvocationTrace, TokenUsage, ToolCallTrace};
    use rusqlite::params;

    use super::{InvocationInsertParams, InvocationQuery, Store};

    fn ts(raw: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(raw)
            .expect("timestamp")
            .with_timezone(&Utc)
    }

    #[allow(clippy::too_many_arguments)]
    fn seed_raw_invocation(
        store: &Store,
        id_ts: &str,
        job_run_id: &str,
        activity_id: &str,
        agent: &str,
        model: Option<&str>,
        task_ids: &[&str],
        tool_calls: &[(&str, u64, u64)],
        usage: TokenUsage,
        duration_ms: u64,
    ) {
        let conn = store.conn.lock().expect("store lock");
        conn.execute(
            r#"INSERT INTO invocations(
                ts, job_run_id, activity_id, agent, model, duration_ms,
                input_tokens, cache_read_tokens, cache_create_tokens,
                output_tokens, tool_call_count
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"#,
            params![
                id_ts,
                job_run_id,
                activity_id,
                agent,
                model,
                duration_ms as i64,
                usage.input as i64,
                usage.cache_read as i64,
                usage.cache_create as i64,
                usage.output as i64,
                tool_calls.len() as i64,
            ],
        )
        .expect("insert invocation");

        let invocation_id = conn.last_insert_rowid();
        for task_id in task_ids {
            conn.execute(
                "INSERT INTO invocation_tasks(invocation_id, task_id) VALUES (?1, ?2)",
                params![invocation_id, task_id],
            )
            .expect("insert task");
        }
        for (seq, (tool_name, result_bytes, _)) in tool_calls.iter().enumerate() {
            conn.execute(
                "INSERT INTO tool_calls(invocation_id, seq, tool_name, result_bytes) VALUES (?1, ?2, ?3, ?4)",
                params![invocation_id, seq as i64 + 1, tool_name, *result_bytes as i64],
            )
            .expect("insert tool call");
        }
    }

    #[test]
    fn list_invocation_records_loads_tasks_and_tools_with_filters() {
        let store = Store::open_in_memory().expect("store");
        seed_raw_invocation(
            &store,
            "2026-04-11T12:00:00Z",
            "run-1",
            "activity-a",
            "claude",
            Some("claude-3-7-sonnet"),
            &["task-a", "task-b"],
            &[("fs.read", 12, 0)],
            TokenUsage {
                input: 100,
                cache_read: 20,
                cache_create: 0,
                output: 10,
            },
            55,
        );
        seed_raw_invocation(
            &store,
            "2026-04-11T12:05:00Z",
            "run-2",
            "activity-b",
            "codex",
            Some("gpt-5.4"),
            &["task-c"],
            &[("exec", 99, 0)],
            TokenUsage {
                input: 200,
                cache_read: 0,
                cache_create: 5,
                output: 20,
            },
            77,
        );

        let rows = store
            .list_invocation_records(&InvocationQuery {
                since: Some(ts("2026-04-11T12:01:00Z")),
                until: Some(ts("2026-04-11T12:10:00Z")),
                tool_name: Some("exec".to_string()),
                limit: 20,
                ..Default::default()
            })
            .expect("query");

        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.job_run_id, "run-2");
        assert_eq!(row.activity_id, "activity-b");
        assert_eq!(row.agent, "codex");
        assert_eq!(row.model.as_deref(), Some("gpt-5.4"));
        assert_eq!(row.total_tokens, 220);
        assert_eq!(row.tool_call_count, 1);
        assert_eq!(row.task_ids, vec!["task-c".to_string()]);
        assert_eq!(row.tool_calls.len(), 1);
        assert_eq!(row.tool_calls[0].tool_name, "exec");
        assert_eq!(row.tool_calls[0].result_bytes, 99);
    }

    #[test]
    fn insert_invocation_trace_record_round_trips_into_raw_query() {
        let store = Store::open_in_memory().expect("store");
        store
            .insert_invocation_trace_record(&InvocationInsertParams {
                job_run_id: "run-3".to_string(),
                activity_id: "activity-c".to_string(),
                agent: "claude".to_string(),
                model: Some("claude-3-7-sonnet".to_string()),
                task_ids: vec!["task-z".to_string()],
                trace: InvocationTrace {
                    usage: TokenUsage {
                        input: 10,
                        cache_read: 2,
                        cache_create: 1,
                        output: 3,
                    },
                    tool_calls: vec![ToolCallTrace {
                        seq: 1,
                        tool_name: "fs.read".to_string(),
                        result_bytes: 42,
                        result_payload: None,
                    }],
                    duration_ms: 88,
                },
            })
            .expect("insert trace");

        let rows = store
            .list_invocation_records(&InvocationQuery {
                job_run_id: Some("run-3".to_string()),
                limit: 20,
                ..Default::default()
            })
            .expect("query");

        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.job_run_id, "run-3");
        assert_eq!(row.activity_id, "activity-c");
        assert_eq!(row.total_tokens, 13);
        assert_eq!(row.tool_call_count, 1);
        assert_eq!(row.task_ids, vec!["task-z".to_string()]);
        assert_eq!(row.tool_calls[0].tool_name, "fs.read");
        assert_eq!(row.tool_calls[0].result_bytes, 42);
    }

    #[test]
    fn list_activity_and_agent_metrics_group_by_identity() {
        let store = Store::open_in_memory().expect("store");
        seed_raw_invocation(
            &store,
            "2026-04-11T12:00:00Z",
            "run-1",
            "activity-a",
            "claude",
            Some("opus"),
            &["task-a"],
            &[("fs.read", 12, 0)],
            TokenUsage {
                input: 10,
                cache_read: 0,
                cache_create: 0,
                output: 3,
            },
            55,
        );
        seed_raw_invocation(
            &store,
            "2026-04-11T12:05:00Z",
            "run-2",
            "activity-a",
            "codex",
            Some("gpt-5.4"),
            &["task-b"],
            &[("exec", 99, 0)],
            TokenUsage {
                input: 7,
                cache_read: 1,
                cache_create: 0,
                output: 2,
            },
            77,
        );
        seed_raw_invocation(
            &store,
            "2026-04-11T12:10:00Z",
            "run-3",
            "activity-b",
            "claude",
            Some("opus"),
            &["task-c"],
            &[("fs.read", 4, 0)],
            TokenUsage {
                input: 5,
                cache_read: 0,
                cache_create: 0,
                output: 1,
            },
            31,
        );

        let activity_rows = store
            .list_activity_invocation_metrics()
            .expect("activity metrics");
        assert_eq!(activity_rows.len(), 3);
        let claude_activity = activity_rows
            .iter()
            .find(|row| {
                row.activity_id == "activity-a"
                    && row.agent == "claude"
                    && row.model.as_deref() == Some("opus")
            })
            .expect("claude activity");
        assert_eq!(claude_activity.invocation_count, 1);
        assert_eq!(claude_activity.total_tokens, 13);

        let codex_activity = activity_rows
            .iter()
            .find(|row| {
                row.activity_id == "activity-a"
                    && row.agent == "codex"
                    && row.model.as_deref() == Some("gpt-5.4")
            })
            .expect("codex activity");
        assert_eq!(codex_activity.invocation_count, 1);
        assert_eq!(codex_activity.total_tokens, 9);

        let agent_rows = store
            .list_agent_invocation_metrics()
            .expect("agent metrics");
        assert_eq!(agent_rows.len(), 2);
        let claude_agent = agent_rows
            .iter()
            .find(|row| row.agent == "claude" && row.model.as_deref() == Some("opus"))
            .expect("claude agent");
        assert_eq!(claude_agent.invocation_count, 2);
        assert_eq!(claude_agent.total_tokens, 19);

        let codex_agent = agent_rows
            .iter()
            .find(|row| row.agent == "codex" && row.model.as_deref() == Some("gpt-5.4"))
            .expect("codex agent");
        assert_eq!(codex_agent.invocation_count, 1);
        assert_eq!(codex_agent.total_tokens, 9);
    }
}
