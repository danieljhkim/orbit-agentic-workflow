pub(crate) fn summarize_error_message(raw: Option<&str>) -> String {
    let value = raw.unwrap_or("-").replace('\n', " ");
    if value.chars().count() <= 120 {
        return value;
    }
    let truncated = value.chars().take(120).collect::<String>();
    format!("{truncated}...")
}

pub(crate) fn format_timestamp(value: Option<chrono::DateTime<chrono::Utc>>) -> String {
    value
        .map(|v| v.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_else(|| "-".to_string())
}

pub(crate) fn format_duration(value: Option<u64>) -> String {
    value
        .map(|duration| format!("{duration}ms"))
        .unwrap_or_else(|| "-".to_string())
}
