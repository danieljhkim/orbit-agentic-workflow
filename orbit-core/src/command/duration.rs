use chrono::{DateTime, Utc};
use orbit_types::OrbitError;

/// Parses a duration-relative string like "1h", "90d", "30m", "2w"
/// or an RFC3339/naive timestamp into a `DateTime<Utc>`.
///
/// For bare durations, the result is `now - duration`.
pub fn parse_since(input: &str) -> Result<DateTime<Utc>, OrbitError> {
    let input = input.trim();

    // Try RFC3339 first
    if let Ok(parsed) = DateTime::parse_from_rfc3339(input) {
        return Ok(parsed.with_timezone(&Utc));
    }

    // Try naive datetime "2026-01-15 10:00:00" or "2026-01-15T10:00:00"
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(input, "%Y-%m-%dT%H:%M:%S") {
        return Ok(naive.and_utc());
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M:%S") {
        return Ok(naive.and_utc());
    }

    // Try bare duration: number + suffix
    let (num_str, suffix) = split_duration(input)?;
    let num: i64 = num_str
        .parse()
        .map_err(|_| OrbitError::InvalidInput(format!("invalid duration number: {num_str}")))?;

    if num <= 0 {
        return Err(OrbitError::InvalidInput(
            "duration must be positive".to_string(),
        ));
    }

    let seconds = match suffix {
        "s" => num,
        "m" => num * 60,
        "h" => num * 3600,
        "d" => num * 86400,
        "w" => num * 604800,
        other => {
            return Err(OrbitError::InvalidInput(format!(
                "unknown duration suffix: {other} (use s/m/h/d/w)"
            )));
        }
    };

    let duration = chrono::Duration::seconds(seconds);
    Ok(Utc::now() - duration)
}

fn split_duration(input: &str) -> Result<(&str, &str), OrbitError> {
    let pos = input
        .find(|c: char| c.is_alphabetic())
        .ok_or_else(|| OrbitError::InvalidInput(format!("invalid duration format: {input}")))?;

    let (num, suffix) = input.split_at(pos);
    if num.is_empty() {
        return Err(OrbitError::InvalidInput(format!(
            "missing number in duration: {input}"
        )));
    }

    Ok((num, suffix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hours() {
        let result = parse_since("1h").expect("parse 1h");
        let diff = Utc::now() - result;
        // Should be approximately 1 hour
        assert!((diff.num_seconds() - 3600).abs() < 5);
    }

    #[test]
    fn parse_days() {
        let result = parse_since("90d").expect("parse 90d");
        let diff = Utc::now() - result;
        assert!((diff.num_seconds() - 90 * 86400).abs() < 5);
    }

    #[test]
    fn parse_minutes() {
        let result = parse_since("30m").expect("parse 30m");
        let diff = Utc::now() - result;
        assert!((diff.num_seconds() - 1800).abs() < 5);
    }

    #[test]
    fn parse_weeks() {
        let result = parse_since("2w").expect("parse 2w");
        let diff = Utc::now() - result;
        assert!((diff.num_seconds() - 2 * 604800).abs() < 5);
    }

    #[test]
    fn parse_seconds() {
        let result = parse_since("60s").expect("parse 60s");
        let diff = Utc::now() - result;
        assert!((diff.num_seconds() - 60).abs() < 5);
    }

    #[test]
    fn parse_rfc3339() {
        let result = parse_since("2026-01-15T10:00:00+00:00").expect("parse rfc3339");
        assert!(result.to_rfc3339().starts_with("2026-01-15"));
    }

    #[test]
    fn parse_naive_timestamp() {
        let result = parse_since("2026-01-15T10:00:00").expect("parse naive");
        assert!(result.to_rfc3339().starts_with("2026-01-15"));
    }

    #[test]
    fn invalid_input_errors() {
        assert!(parse_since("abc").is_err());
        assert!(parse_since("").is_err());
        assert!(parse_since("0h").is_err());
        assert!(parse_since("-1h").is_err());
        assert!(parse_since("5x").is_err());
    }
}
