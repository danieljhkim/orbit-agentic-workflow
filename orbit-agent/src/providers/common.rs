pub(crate) fn render_prompt_with_embedded_envelope(envelope_json: &[u8]) -> Vec<u8> {
    let prompt = "You are Orbit's agent executor.\n\
Read the execution envelope JSON and perform the requested work.\n\
Return exactly one JSON object and nothing else.\n\
Required response schema:\n\
{\"schemaVersion\":1,\"status\":\"success|failed|timeout\",\"result\":{},\"error\":null,\"durationMs\":123}\n\
Rules:\n\
- Output valid JSON only.\n\
- No markdown fences.\n\
- If execution cannot complete, return status=\"failed\" with non-empty error.code and error.message.\n\
- Keep result as a JSON object.";
    let envelope_text = String::from_utf8_lossy(envelope_json);
    format!("{prompt}\nExecution envelope:\n{envelope_text}\n").into_bytes()
}
