use chrono::{DateTime, Utc};
use orbit_common::types::{
    OrbitError, optional_csv_or_string_list_alias, optional_string, required_string,
};
use orbit_store::friction_store::{
    FrictionAddParams, FrictionListFilter, StoredFrictionRecord, add_friction, friction_stats,
    list_frictions, show_friction,
};
use serde_json::{Value, json};

use crate::OrbitRuntime;

pub(super) fn add(
    runtime: &OrbitRuntime,
    input: Value,
    model: Option<String>,
) -> Result<Value, OrbitError> {
    let body = required_string(&input, &["body", "description"], "body")?;
    let tags = optional_csv_or_string_list_alias(&input, &["tags", "tag"])?.unwrap_or_default();
    let during_task = optional_string(&input, "during_task")?
        .or_else(|| optional_string(&input, "task_id").ok().flatten());
    let model = model
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            OrbitError::InvalidInput("orbit.friction.add requires `model`".to_string())
        })?;
    let stored = add_friction(
        &runtime.data_root().join("frictions"),
        FrictionAddParams {
            model,
            body,
            tags,
            during_task,
            created_at: Utc::now(),
        },
    )?;
    record_to_json(stored)
}

pub(super) fn list(runtime: &OrbitRuntime, input: Value) -> Result<Value, OrbitError> {
    let filter = FrictionListFilter {
        model: optional_string(&input, "model")?,
        tag: optional_string(&input, "tag")?,
        from: optional_string(&input, "from")?
            .map(|raw| parse_timestamp("from", &raw))
            .transpose()?,
        to: optional_string(&input, "to")?
            .map(|raw| parse_timestamp("to", &raw))
            .transpose()?,
    };
    let records = list_frictions(&runtime.data_root().join("frictions"), &filter)?;
    Ok(Value::Array(
        records
            .into_iter()
            .map(record_to_json)
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

pub(super) fn show(runtime: &OrbitRuntime, input: Value) -> Result<Value, OrbitError> {
    let id = required_string(&input, &["id"], "id")?;
    let Some(stored) = show_friction(&runtime.data_root().join("frictions"), &id)? else {
        return Err(OrbitError::InvalidInput(format!(
            "friction record not found: {id}"
        )));
    };
    record_to_json(stored)
}

pub(super) fn stats(runtime: &OrbitRuntime) -> Result<Value, OrbitError> {
    let tasks = runtime.list_tasks()?;
    friction_stats(&runtime.data_root().join("frictions"), &tasks)
}

fn parse_timestamp(field: &str, raw: &str) -> Result<DateTime<Utc>, OrbitError> {
    DateTime::parse_from_rfc3339(raw)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| OrbitError::InvalidInput(format!("`{field}` must be RFC3339: {error}")))
}

fn record_to_json(stored: StoredFrictionRecord) -> Result<Value, OrbitError> {
    let mut value = serde_json::to_value(&stored.record)
        .map_err(|error| OrbitError::Store(format!("serialize friction record: {error}")))?;
    if let Some(object) = value.as_object_mut() {
        object.insert("path".to_string(), json!(stored.path.to_string_lossy()));
    }
    Ok(value)
}
