use crate::models::bundle::Bundle;
use chrono::NaiveDateTime;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

/// Approximately 6 months in seconds (183 days).
const STALENESS_THRESHOLD_SECS: u64 = 183 * 86400;

/// Go reference-time tokens translated to chrono/strptime directives.
/// Listed in priority order so a positional scan picks the longest match first.
/// Fractional-second tokens come first (longest variants ahead of shorter ones)
/// so e.g. `.999999` doesn't mis-resolve as `.999` + `999`.
/// Only padded tokens are supported; a layout using unpadded Go tokens (e.g.
/// "1" for month, "5" for seconds) will produce a wrong strptime pattern and
/// degrade to the skip-on-parse-failure branch below.
const GO_LAYOUT_TOKENS: &[(&str, &str)] = &[
    (".000000000", "%.f"),
    (".999999999", "%.f"),
    (".000000", "%.f"),
    (".999999", "%.f"),
    (".000", "%.f"),
    (".999", "%.f"),
    ("2006", "%Y"),
    ("01", "%m"),
    ("02", "%d"),
    ("15", "%H"),
    ("04", "%M"),
    ("05", "%S"),
];

/// Resolve the sample_data value for a primary timestamp column. Mirrors the
/// Python `_resolve_sample_path` algorithm: prefer the post-transform output
/// name when it has a non-null value; otherwise walk each `from_json_pointers`
/// entry (single- or multi-segment, e.g. `/httpMessage/start` for SIEM); finally
/// fall back to `from_input_field`. Returns None when nothing resolves to a
/// non-null value, so the caller can `continue` past columns whose fixture is
/// missing entirely.
fn resolve_primary_value<'a>(col: &Value, sample_data: &'a Value) -> Option<&'a Value> {
    let col_name = col.get("name").and_then(|v| v.as_str())?;

    let by_name = sample_data.get(col_name);
    if matches!(by_name, Some(v) if !v.is_null()) {
        return by_name;
    }

    let source = col.get("datatype").and_then(|d| d.get("source"));
    if let Some(ptrs) = source
        .and_then(|s| s.get("from_json_pointers"))
        .and_then(|p| p.as_array())
    {
        for ptr in ptrs {
            if let Some(p) = ptr.as_str() {
                if let Some(v) = sample_data.pointer(p) {
                    if !v.is_null() {
                        return Some(v);
                    }
                }
            }
        }
    }

    if let Some(fif) = source
        .and_then(|s| s.get("from_input_field"))
        .and_then(|v| v.as_str())
    {
        let by_input = sample_data.get(fif);
        if matches!(by_input, Some(v) if !v.is_null()) {
            return by_input;
        }
    }

    by_name
}

/// Translate a Go reference-time layout (e.g. `2006-01-02T15:04:05`) to the
/// chrono/strftime equivalent (`%Y-%m-%dT%H:%M:%S`). Unknown characters pass
/// through as literals.
fn translate_go_layout(fmt: &str) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < fmt.len() {
        let rest = &fmt[i..];
        let mut matched = false;
        for (go_token, strp_token) in GO_LAYOUT_TOKENS {
            if rest.starts_with(go_token) {
                out.push_str(strp_token);
                i += go_token.len();
                matched = true;
                break;
            }
        }
        if !matched {
            let ch = rest.chars().next().unwrap_or(' ');
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

/// Returns a list of warning messages for stale sample_data timestamps.
/// An empty list means all timestamps are fresh.
pub async fn run(base: &str, bundle: &Bundle) -> Vec<String> {
    let mut warnings = Vec::new();

    let now_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    for t in &bundle.tables {
        for tt in &t.transforms {
            let full_path = format!("{base}/{}", tt.path);

            let content = match fs::read_to_string(&full_path).await {
                Ok(v) => v,
                Err(_) => continue,
            };

            let transform_json: Value = match serde_json::from_str(&content) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let output_columns = match transform_json["settings"]["output_columns"].as_array() {
                Some(cols) => cols,
                None => continue,
            };

            // Find the primary timestamp column (epoch or datetime).
            let primary_col = match output_columns.iter().find(|col| {
                let dt = match col.get("datatype") {
                    Some(d) => d,
                    None => return false,
                };
                let type_str = dt.get("type").and_then(|v| v.as_str()).unwrap_or("");
                let is_ts = type_str == "epoch" || type_str == "datetime";
                let is_primary = dt.get("primary").and_then(|v| v.as_bool()).unwrap_or(false);
                is_ts && is_primary
            }) {
                Some(c) => c,
                None => continue,
            };

            let dt = &primary_col["datatype"];
            let primary_type = dt
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let primary_col_name = primary_col
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let default_format = if primary_type == "datetime" { "" } else { "s" };
            let primary_format = dt
                .get("format")
                .and_then(|f| f.as_str())
                .unwrap_or(default_format)
                .to_string();

            let sample_data = &transform_json["settings"]["sample_data"];
            let primary_value = match resolve_primary_value(primary_col, sample_data) {
                Some(v) => v,
                None => continue,
            };

            let primary_secs: u64 = match primary_type.as_str() {
                "datetime" => {
                    if primary_format.is_empty() {
                        continue;
                    }
                    let value = match primary_value {
                        Value::String(s) => s.clone(),
                        _ => continue,
                    };
                    let strptime_fmt = translate_go_layout(&primary_format);
                    match NaiveDateTime::parse_from_str(&value, &strptime_fmt) {
                        Ok(dt) => {
                            let ts = dt.and_utc().timestamp();
                            if ts < 0 {
                                continue;
                            }
                            ts as u64
                        }
                        Err(_) => continue,
                    }
                }
                _ => {
                    let raw_secs = match primary_value {
                        Value::Number(n) => match n.as_u64() {
                            Some(v) => v,
                            None => match n.as_f64() {
                                Some(v) => v as u64,
                                None => continue,
                            },
                        },
                        Value::String(s) => match s.trim().parse::<u64>() {
                            Ok(v) => v,
                            Err(_) => continue,
                        },
                        _ => continue,
                    };
                    let divisor: u64 = match primary_format.as_str() {
                        "ms" => 1_000,
                        "us" => 1_000_000,
                        "ns" => 1_000_000_000,
                        _ => 1,
                    };
                    raw_secs / divisor
                }
            };

            if now_epoch > primary_secs && (now_epoch - primary_secs) > STALENESS_THRESHOLD_SECS {
                let age_days = (now_epoch - primary_secs) / 86400;
                warnings.push(format!(
                    "Stale sample_data in {}: primary timestamp '{}' is ~{} days old (threshold: {} days)",
                    tt.path, primary_col_name, age_days, STALENESS_THRESHOLD_SECS / 86400
                ));
            }
        }
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tokio::fs;

    fn now_epoch() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn make_transform_json(primary_ts: u64) -> String {
        format!(
            r#"{{
                "name": "test_transform",
                "settings": {{
                    "output_columns": [
                        {{
                            "name": "timestamp",
                            "datatype": {{
                                "type": "epoch",
                                "primary": true,
                                "format": "s",
                                "resolution": "ms"
                            }}
                        }},
                        {{
                            "name": "bytes",
                            "datatype": {{
                                "type": "uint64"
                            }}
                        }}
                    ],
                    "sample_data": {{
                        "timestamp": {},
                        "bytes": 1234
                    }}
                }}
            }}"#,
            primary_ts
        )
    }

    fn make_bundle(transform_path: &str) -> Bundle {
        serde_json::from_str(&format!(
            r#"{{
                "name": "test-bundle",
                "source": "test",
                "method": "http_streaming",
                "beta": false,
                "base_url": "https://example.com/test",
                "dashboard": {{
                    "path": "dashboards/test.json",
                    "project_var": "__PROJECT_NAME__"
                }},
                "tables": [{{
                    "dashboard_var": "__TABLE_NAME__",
                    "name": "logs",
                    "transforms": [{{
                        "path": "{}"
                    }}]
                }}],
                "ui": {{
                    "primary_url": "https://example.com",
                    "method": {{ "full_title": "HTTP Streaming", "icon_url": "https://example.com/icon.png" }},
                    "source": {{ "full_title": "Test Source", "icon_url": "https://example.com/icon.png" }},
                    "data_category": "cdn"
                }},
                "metadata": {{
                    "version": "1.0.0",
                    "maintainer": "test",
                    "description": "test bundle",
                    "channel_type": "AWS"
                }}
            }}"#,
            transform_path
        ))
        .unwrap()
    }

    #[tokio::test]
    async fn test_fresh_data_passes() {
        let dir = tempfile::tempdir().unwrap();
        let transform_path = "transformations/transform.json";
        let full_path = dir.path().join(transform_path);
        fs::create_dir_all(full_path.parent().unwrap())
            .await
            .unwrap();

        // Timestamp from 1 day ago — fresh
        let ts = now_epoch() - 86400;
        fs::write(&full_path, make_transform_json(ts))
            .await
            .unwrap();

        let bundle = make_bundle(transform_path);
        let warnings = run(dir.path().to_str().unwrap(), &bundle).await;
        assert!(
            warnings.is_empty(),
            "Fresh data should produce no warnings: {:?}",
            warnings
        );
    }

    #[tokio::test]
    async fn test_stale_data_warns() {
        let dir = tempfile::tempdir().unwrap();
        let transform_path = "transformations/transform.json";
        let full_path = dir.path().join(transform_path);
        fs::create_dir_all(full_path.parent().unwrap())
            .await
            .unwrap();

        // Timestamp from 1 year ago — stale
        let ts = now_epoch() - (365 * 86400);
        fs::write(&full_path, make_transform_json(ts))
            .await
            .unwrap();

        let bundle = make_bundle(transform_path);
        let warnings = run(dir.path().to_str().unwrap(), &bundle).await;
        assert!(!warnings.is_empty(), "Stale data should produce warnings");
        assert!(
            warnings[0].contains("Stale sample_data"),
            "Warning should mention staleness: {}",
            warnings[0]
        );
    }

    fn make_datetime_transform_json(primary_ts: &str, format: &str) -> String {
        format!(
            r#"{{
                "name": "test_transform",
                "settings": {{
                    "output_columns": [
                        {{
                            "name": "timestamp",
                            "datatype": {{
                                "type": "datetime",
                                "primary": true,
                                "format": "{format}",
                                "resolution": "ms"
                            }}
                        }}
                    ],
                    "sample_data": {{
                        "timestamp": "{primary_ts}"
                    }}
                }}
            }}"#
        )
    }

    #[tokio::test]
    async fn test_fresh_datetime_data_passes() {
        let dir = tempfile::tempdir().unwrap();
        let transform_path = "transformations/transform.json";
        let full_path = dir.path().join(transform_path);
        fs::create_dir_all(full_path.parent().unwrap())
            .await
            .unwrap();

        let fresh = chrono::Utc::now() - chrono::Duration::days(30);
        let fresh_str = fresh.format("%Y-%m-%dT%H:%M:%S").to_string();
        fs::write(
            &full_path,
            make_datetime_transform_json(&fresh_str, "2006-01-02T15:04:05"),
        )
        .await
        .unwrap();

        let bundle = make_bundle(transform_path);
        let warnings = run(dir.path().to_str().unwrap(), &bundle).await;
        assert!(
            warnings.is_empty(),
            "Fresh datetime should produce no warnings: {:?}",
            warnings
        );
    }

    #[tokio::test]
    async fn test_stale_datetime_data_warns() {
        let dir = tempfile::tempdir().unwrap();
        let transform_path = "transformations/transform.json";
        let full_path = dir.path().join(transform_path);
        fs::create_dir_all(full_path.parent().unwrap())
            .await
            .unwrap();

        // Stale: well over 5 years old regardless of when the test runs
        fs::write(
            &full_path,
            make_datetime_transform_json("2020-06-15T12:34:56", "2006-01-02T15:04:05"),
        )
        .await
        .unwrap();

        let bundle = make_bundle(transform_path);
        let warnings = run(dir.path().to_str().unwrap(), &bundle).await;
        assert!(
            !warnings.is_empty(),
            "Stale datetime should produce warnings"
        );
        assert!(
            warnings[0].contains("Stale sample_data"),
            "Warning should mention staleness: {}",
            warnings[0]
        );
    }

    /// SIEM-shaped transform: primary epoch sourced from a multi-segment
    /// JSON pointer with the value at a nested location in sample_data.
    fn make_nested_pointer_transform_json(primary_ts_literal: &str) -> String {
        format!(
            r#"{{
                "name": "siem_transform",
                "settings": {{
                    "output_columns": [
                        {{
                            "name": "timestamp",
                            "datatype": {{
                                "type": "epoch",
                                "primary": true,
                                "format": "s",
                                "source": {{ "from_json_pointers": ["/httpMessage/start"] }}
                            }}
                        }}
                    ],
                    "sample_data": {{
                        "type": "akamai_siem",
                        "httpMessage": {{
                            "host": "siem.example.com",
                            "start": {primary_ts_literal}
                        }}
                    }}
                }}
            }}"#
        )
    }

    #[tokio::test]
    async fn test_stale_nested_pointer_numeric_epoch_warns() {
        // LOTC-1523: SIEM-style nested-pointer primary must produce a warning
        // when stale. Previously the validator silently skipped because it
        // looked up sample_data["timestamp"] (post-transform name) and
        // missed the actual location at /httpMessage/start.
        let dir = tempfile::tempdir().unwrap();
        let transform_path = "transformations/siem/transform.json";
        let full_path = dir.path().join(transform_path);
        fs::create_dir_all(full_path.parent().unwrap())
            .await
            .unwrap();

        let stale_ts = now_epoch() - (365 * 86400); // 1 year ago
        fs::write(
            &full_path,
            make_nested_pointer_transform_json(&stale_ts.to_string()),
        )
        .await
        .unwrap();

        let bundle = make_bundle(transform_path);
        let warnings = run(dir.path().to_str().unwrap(), &bundle).await;
        assert!(
            !warnings.is_empty(),
            "Stale nested-pointer epoch should produce warnings"
        );
        assert!(
            warnings[0].contains("Stale sample_data"),
            "Warning should mention staleness: {}",
            warnings[0]
        );
    }

    #[tokio::test]
    async fn test_fresh_nested_pointer_epoch_passes() {
        // LOTC-1523: nested-pointer primary within the freshness threshold
        // must not produce a warning.
        let dir = tempfile::tempdir().unwrap();
        let transform_path = "transformations/siem/transform.json";
        let full_path = dir.path().join(transform_path);
        fs::create_dir_all(full_path.parent().unwrap())
            .await
            .unwrap();

        let fresh_ts = now_epoch() - 86400; // 1 day ago
        fs::write(
            &full_path,
            make_nested_pointer_transform_json(&fresh_ts.to_string()),
        )
        .await
        .unwrap();

        let bundle = make_bundle(transform_path);
        let warnings = run(dir.path().to_str().unwrap(), &bundle).await;
        assert!(
            warnings.is_empty(),
            "Fresh nested-pointer epoch should produce no warnings: {:?}",
            warnings
        );
    }

    #[tokio::test]
    async fn test_stale_nested_pointer_string_epoch_warns() {
        // LOTC-1523: real SIEM shape stores epochs as JSON strings (e.g.
        // "1491303422"). Validator must accept Value::String at nested paths
        // and parse as a numeric epoch.
        let dir = tempfile::tempdir().unwrap();
        let transform_path = "transformations/siem/transform.json";
        let full_path = dir.path().join(transform_path);
        fs::create_dir_all(full_path.parent().unwrap())
            .await
            .unwrap();

        let stale_ts = now_epoch() - (365 * 86400);
        // Note the surrounding quotes: stored as a JSON string, not a number.
        let literal = format!("\"{stale_ts}\"");
        fs::write(&full_path, make_nested_pointer_transform_json(&literal))
            .await
            .unwrap();

        let bundle = make_bundle(transform_path);
        let warnings = run(dir.path().to_str().unwrap(), &bundle).await;
        assert!(
            !warnings.is_empty(),
            "Stale string-typed nested epoch should produce warnings"
        );
        assert!(
            warnings[0].contains("Stale sample_data"),
            "Warning should mention staleness: {}",
            warnings[0]
        );
    }

    /// apicontext-shaped transform: datetime primary sourced from a
    /// single-segment pointer where the column name (camelCase) differs from
    /// the raw sample_data key (snake_case). Pre-fix the validator silently
    /// skipped because `sample_data.get("startTime")` was None.
    fn make_renamed_datetime_transform_json(primary_ts: &str) -> String {
        format!(
            r#"{{
                "name": "apicontext_transform",
                "settings": {{
                    "output_columns": [
                        {{
                            "name": "startTime",
                            "datatype": {{
                                "type": "datetime",
                                "primary": true,
                                "format": "2006-01-02T15:04:05.999999Z",
                                "source": {{ "from_json_pointers": ["/start_time"] }}
                            }}
                        }}
                    ],
                    "sample_data": {{
                        "start_time": "{primary_ts}",
                        "result": "HTTP_CLIENT_ERROR"
                    }}
                }}
            }}"#
        )
    }

    #[tokio::test]
    async fn test_stale_renamed_datetime_warns() {
        // LOTC-1523 Case 2: column `startTime` renamed from raw `/start_time`.
        // Validator must walk the JSON pointer (not look up by column name) or
        // it silently passes a year-stale fixture.
        let dir = tempfile::tempdir().unwrap();
        let transform_path = "transformations/transform.json";
        let full_path = dir.path().join(transform_path);
        fs::create_dir_all(full_path.parent().unwrap())
            .await
            .unwrap();

        fs::write(
            &full_path,
            make_renamed_datetime_transform_json("2024-10-07T06:27:17.160556Z"),
        )
        .await
        .unwrap();

        let bundle = make_bundle(transform_path);
        let warnings = run(dir.path().to_str().unwrap(), &bundle).await;
        assert!(
            !warnings.is_empty(),
            "Stale renamed-column datetime should produce warnings"
        );
        assert!(
            warnings[0].contains("Stale sample_data"),
            "Warning should mention staleness: {}",
            warnings[0]
        );
        assert!(
            warnings[0].contains("startTime"),
            "Warning should name the primary column: {}",
            warnings[0]
        );
    }

    #[tokio::test]
    async fn test_fresh_renamed_datetime_passes() {
        // Companion: a fresh value at the renamed location must not warn.
        let dir = tempfile::tempdir().unwrap();
        let transform_path = "transformations/transform.json";
        let full_path = dir.path().join(transform_path);
        fs::create_dir_all(full_path.parent().unwrap())
            .await
            .unwrap();

        let fresh = chrono::Utc::now() - chrono::Duration::days(30);
        let fresh_str = fresh.format("%Y-%m-%dT%H:%M:%S.%6fZ").to_string();
        fs::write(&full_path, make_renamed_datetime_transform_json(&fresh_str))
            .await
            .unwrap();

        let bundle = make_bundle(transform_path);
        let warnings = run(dir.path().to_str().unwrap(), &bundle).await;
        assert!(
            warnings.is_empty(),
            "Fresh renamed-column datetime should produce no warnings: {:?}",
            warnings
        );
    }

    #[tokio::test]
    async fn test_no_primary_column_passes() {
        let dir = tempfile::tempdir().unwrap();
        let transform_path = "transformations/transform.json";
        let full_path = dir.path().join(transform_path);
        fs::create_dir_all(full_path.parent().unwrap())
            .await
            .unwrap();

        // Transform with no primary epoch column
        let json = r#"{
            "name": "test",
            "settings": {
                "output_columns": [
                    {
                        "name": "bytes",
                        "datatype": { "type": "uint64" }
                    }
                ],
                "sample_data": { "bytes": 1234 }
            }
        }"#;
        fs::write(&full_path, json).await.unwrap();

        let bundle = make_bundle(transform_path);
        let warnings = run(dir.path().to_str().unwrap(), &bundle).await;
        assert!(
            warnings.is_empty(),
            "No primary column should produce no warnings: {:?}",
            warnings
        );
    }
}
