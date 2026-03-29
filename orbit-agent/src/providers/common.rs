use serde_json::{Value, json};

pub(crate) fn render_prompt_with_embedded_envelope(envelope_json: &[u8]) -> Vec<u8> {
    let prompt = "You are Orbit's agent executor.\n\
Read the execution envelope JSON and perform the requested work.\n\
Return exactly one JSON object and nothing else.\n\
Required response schema:\n\
{\"schemaVersion\":1,\"status\":\"success|failed|timeout\",\"result\":{...},\"error\":null,\"durationMs\":123}\n\
Rules:\n\
- Output valid JSON only. No markdown fences. No explanatory text.\n\
- result MUST be a JSON object (never null, never omitted). Populate it with the fields from the activity's output_schema_json.\n\
- If execution cannot complete, return status=\"failed\" with non-empty error.code and error.message (result may be {}).\n\
- On success, result must contain all required fields declared in output_schema_json.";
    let envelope_text = String::from_utf8_lossy(envelope_json);
    format!("{prompt}\nExecution envelope:\n{envelope_text}\n").into_bytes()
}

/// Returns `true` if the schema is non-empty and declares at least one property,
/// meaning it describes a concrete structure (not freeform `{}`).
pub(crate) fn has_concrete_output_schema(schema: Option<&Value>) -> bool {
    let Some(schema) = schema else {
        return false;
    };
    match schema {
        Value::Object(map) if map.is_empty() => false,
        Value::Object(map) => map.contains_key("properties"),
        _ => false,
    }
}

/// Build a JSON Schema for the full agent response envelope, embedding the
/// activity's `output_schema_json` as the schema for the `result` field.
pub(crate) fn build_envelope_schema(result_schema: &Value) -> Value {
    json!({
        "type": "object",
        "required": ["schemaVersion", "status", "result", "durationMs"],
        "additionalProperties": false,
        "properties": {
            "schemaVersion": {
                "type": "integer",
                "const": 1
            },
            "status": {
                "type": "string",
                "enum": ["success", "failed", "timeout"]
            },
            "result": result_schema,
            "error": {
                "anyOf": [
                    { "type": "null" },
                    {
                        "type": "object",
                        "required": ["code", "message"],
                        "properties": {
                            "code": { "type": "string" },
                            "message": { "type": "string" },
                            "details": {}
                        }
                    }
                ]
            },
            "durationMs": {
                "type": "integer"
            }
        }
    })
}
