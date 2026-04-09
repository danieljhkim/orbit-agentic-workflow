use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use orbit_types::{InvocationTrace, OrbitError};
use rusqlite::params;

use crate::{Store, now_string};

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
pub struct ActivityInvocationMetrics {
    pub activity_id: String,
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
struct ActivitySample {
    activity_id: String,
    input_tokens: u64,
    cache_read_tokens: u64,
    cache_create_tokens: u64,
    output_tokens: u64,
    tool_call_count: u64,
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

    pub fn list_activity_invocation_metrics(
        &self,
    ) -> Result<Vec<ActivityInvocationMetrics>, OrbitError> {
        let samples = self.load_activity_samples()?;
        let mut grouped: HashMap<String, Vec<ActivitySample>> = HashMap::new();
        for sample in samples {
            grouped
                .entry(sample.activity_id.clone())
                .or_default()
                .push(sample);
        }

        let mut rows = grouped
            .into_iter()
            .map(|(activity_id, samples)| {
                let total_input_tokens = samples.iter().map(|s| s.input_tokens).sum::<u64>();
                let total_cache_read_tokens =
                    samples.iter().map(|s| s.cache_read_tokens).sum::<u64>();
                let total_cache_create_tokens =
                    samples.iter().map(|s| s.cache_create_tokens).sum::<u64>();
                let total_output_tokens = samples.iter().map(|s| s.output_tokens).sum::<u64>();
                let total_tool_calls = samples.iter().map(|s| s.tool_call_count).sum::<u64>();
                let mut totals = samples
                    .iter()
                    .map(|sample| sample.input_tokens.saturating_add(sample.output_tokens))
                    .collect::<Vec<_>>();
                totals.sort_unstable();

                ActivityInvocationMetrics {
                    activity_id,
                    invocation_count: totals.len() as u64,
                    total_input_tokens,
                    total_cache_read_tokens,
                    total_cache_create_tokens,
                    total_output_tokens,
                    total_tokens: total_input_tokens.saturating_add(total_output_tokens),
                    avg_tokens: average(&totals),
                    p50_tokens: percentile(&totals, 50),
                    p95_tokens: percentile(&totals, 95),
                    total_tool_calls,
                }
            })
            .collect::<Vec<_>>();

        rows.sort_by(|left, right| {
            right
                .total_tokens
                .cmp(&left.total_tokens)
                .then_with(|| left.activity_id.cmp(&right.activity_id))
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

    fn load_activity_samples(&self) -> Result<Vec<ActivitySample>, OrbitError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;
        let mut stmt = conn
            .prepare(
                r#"
                SELECT activity_id, input_tokens, cache_read_tokens, cache_create_tokens,
                       output_tokens, tool_call_count
                FROM invocations
                ORDER BY activity_id ASC, id ASC
                "#,
            )
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(ActivitySample {
                    activity_id: row.get(0)?,
                    input_tokens: row.get::<_, i64>(1)? as u64,
                    cache_read_tokens: row.get::<_, i64>(2)? as u64,
                    cache_create_tokens: row.get::<_, i64>(3)? as u64,
                    output_tokens: row.get::<_, i64>(4)? as u64,
                    tool_call_count: row.get::<_, i64>(5)? as u64,
                })
            })
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| OrbitError::Store(e.to_string()))
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
