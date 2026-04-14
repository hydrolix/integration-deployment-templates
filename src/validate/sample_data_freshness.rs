use crate::models::bundle::Bundle;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

/// Approximately 6 months in seconds (183 days).
const STALENESS_THRESHOLD_SECS: u64 = 183 * 86400;

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

            // Find the primary epoch column name and format
            let primary_info = output_columns.iter().find_map(|col| {
                let dt = col.get("datatype")?;
                if dt.get("type")?.as_str()? == "epoch" && dt.get("primary")?.as_bool()? {
                    let name = col.get("name")?.as_str()?.to_string();
                    let format = dt
                        .get("format")
                        .and_then(|f| f.as_str())
                        .unwrap_or("s")
                        .to_string();
                    Some((name, format))
                } else {
                    None
                }
            });

            let (primary_col_name, primary_format) = match primary_info {
                Some(info) => info,
                None => continue,
            };

            let sample_data = &transform_json["settings"]["sample_data"];

            let primary_value = match sample_data.get(&primary_col_name) {
                Some(Value::Number(n)) => match n.as_u64() {
                    Some(v) => v,
                    None => match n.as_f64() {
                        Some(v) => v as u64,
                        None => continue,
                    },
                },
                _ => continue,
            };

            // Convert to seconds based on format
            let divisor: u64 = match primary_format.as_str() {
                "ms" => 1_000,
                "us" => 1_000_000,
                "ns" => 1_000_000_000,
                _ => 1, // "s" or unknown defaults to seconds
            };
            let primary_secs = primary_value / divisor;

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
