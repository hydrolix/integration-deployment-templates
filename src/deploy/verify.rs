use serde_json::Value;
use std::future::Future;

pub fn coerce_to_epoch_secs(value: &Value, fmt: &str) -> Option<u64> {
    let raw = value
        .as_u64()
        .or_else(|| value.as_str().and_then(|s| s.parse::<u64>().ok()))?;
    let divisor: u64 = match fmt {
        "ms" => 1_000,
        "us" => 1_000_000,
        "ns" => 1_000_000_000,
        _ => 1,
    };
    Some(raw / divisor)
}

pub async fn diagnose_zero_rows<F, Fut>(
    cluster_query: F,
    project: &str,
    table: &str,
    transform_json: &Value,
    table_settings: &Value,
    now_secs: u64,
) -> String
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = Result<String, String>>,
{
    let mut findings: Vec<String> = Vec::new();

    if let Some((col_name, ts_value, fmt)) = primary_epoch_from_transform(transform_json) {
        if let Some(ts_secs) = coerce_to_epoch_secs(&ts_value, &fmt) {
            let max_age_days = table_settings
                .get("age")
                .and_then(|a| a.get("max_age_days"))
                .and_then(|v| v.as_u64())
                .unwrap_or(1);
            if now_secs > ts_secs {
                let age_days = (now_secs - ts_secs) / 86_400;
                if age_days > max_age_days {
                    findings.push(format!(
                        "PRIMARY TIMESTAMP STALE: sample_data.{} = {} ({} days old), \
                         table max_age_days = {}. Row was expired on arrival.",
                        col_name, ts_value, age_days, max_age_days
                    ));
                }
            }
        }
    }

    let missing = missing_required_fields(transform_json);
    if !missing.is_empty() {
        findings.push(format!(
            "sample_data MISSING REQUIRED FIELDS: {:?} — transform would reject these rows.",
            missing
        ));
    }

    let parts_sql = format!(
        "SELECT count() AS total, sum(active) AS active_count, \
                min(min_time) AS oldest, max(max_time) AS newest \
         FROM system.parts WHERE database = '{}' AND table = '{}'",
        project, table
    );
    if let Ok(parts) = cluster_query(parts_sql).await {
        let trimmed = parts.trim();
        if !trimmed.is_empty() {
            findings.push(format!("system.parts: {}", trimmed));
        }
    }

    let ingest_errors_sql = format!(
        "SELECT timestamp, error FROM system.ingest_errors \
         WHERE table = '{}.{}' ORDER BY timestamp DESC LIMIT 3",
        project, table
    );
    if let Ok(errs) = cluster_query(ingest_errors_sql).await {
        let trimmed = errs.trim();
        if !trimmed.is_empty() {
            findings.push(format!("Recent ingest errors: {}", trimmed));
        }
    }

    if findings.is_empty() {
        "No obvious cause detected — manual investigation required. \
         Ephemeral --guid project is left alive for debugging."
            .to_string()
    } else {
        findings.join("\n  ")
    }
}

/// Returns names of primary output columns that are absent from sample_data.
/// Narrow by design: only the primary column's absence guarantees row rejection.
/// Non-primary columns may have defaults or be optional, so we don't flag them
/// to avoid false-positive noise in diagnose_zero_rows output.
pub fn missing_required_fields(transform: &Value) -> Vec<String> {
    let settings = match transform.get("settings") {
        Some(s) => s,
        None => return Vec::new(),
    };
    let output_columns = match settings.get("output_columns").and_then(|c| c.as_array()) {
        Some(c) => c,
        None => return Vec::new(),
    };
    let sample_data = settings.get("sample_data").and_then(|s| s.as_object());

    output_columns
        .iter()
        .filter(|col| {
            col.get("datatype")
                .and_then(|dt| dt.get("primary"))
                .and_then(|p| p.as_bool())
                == Some(true)
        })
        .filter_map(|col| col.get("name").and_then(|n| n.as_str()).map(String::from))
        .filter(|name| match sample_data {
            Some(obj) => !obj.contains_key(name),
            None => true,
        })
        .collect()
}

pub fn primary_epoch_from_transform(transform: &Value) -> Option<(String, Value, String)> {
    let settings = transform.get("settings")?;
    let output_columns = settings.get("output_columns")?.as_array()?;
    let sample_data = settings.get("sample_data")?;

    output_columns.iter().find_map(|col| {
        let dt = col.get("datatype")?;
        let type_match = dt.get("type").and_then(|t| t.as_str()) == Some("epoch");
        let is_primary = dt.get("primary").and_then(|p| p.as_bool()) == Some(true);
        if !(type_match && is_primary) {
            return None;
        }
        let name = col.get("name")?.as_str()?.to_string();
        let format = dt
            .get("format")
            .and_then(|f| f.as_str())
            .unwrap_or("s")
            .to_string();
        let value = sample_data.get(&name)?.clone();
        Some((name, value, format))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn primary_epoch_returns_name_value_and_format() {
        let transform = json!({
            "settings": {
                "output_columns": [
                    {
                        "name": "ts",
                        "datatype": { "type": "epoch", "primary": true, "format": "s" }
                    }
                ],
                "sample_data": { "ts": 1607368207 }
            }
        });

        assert_eq!(
            primary_epoch_from_transform(&transform),
            Some(("ts".to_string(), json!(1607368207), "s".to_string()))
        );
    }

    #[test]
    fn coerce_to_epoch_secs_handles_numeric_value() {
        assert_eq!(
            coerce_to_epoch_secs(&json!(1607368207_u64), "s"),
            Some(1607368207)
        );
    }

    #[test]
    fn coerce_to_epoch_secs_handles_numeric_string() {
        assert_eq!(
            coerce_to_epoch_secs(&json!("1607368207"), "s"),
            Some(1607368207)
        );
    }

    #[tokio::test]
    async fn diagnose_includes_system_parts_summary() {
        let now = 1_704_067_200_u64;
        let transform = json!({
            "settings": {
                "output_columns": [
                    {
                        "name": "ts",
                        "datatype": { "type": "epoch", "primary": true, "format": "s" }
                    }
                ],
                "sample_data": { "ts": now }
            }
        });
        let table_settings = json!({ "age": { "max_age_days": 1 } });

        let query = |sql: String| async move {
            if sql.contains("system.parts") {
                Ok::<_, String>("{total: 1, active_count: 0}".to_string())
            } else {
                Ok::<_, String>(String::new())
            }
        };

        let diagnosis =
            diagnose_zero_rows(query, "proj", "tbl", &transform, &table_settings, now).await;
        assert!(
            diagnosis.contains("system.parts:"),
            "expected parts summary, got: {diagnosis}"
        );
        assert!(
            diagnosis.contains("active_count: 0"),
            "expected raw parts payload, got: {diagnosis}"
        );
    }

    #[tokio::test]
    async fn diagnose_falls_back_when_no_finding() {
        let now = 1_704_067_200_u64;
        let transform = json!({
            "settings": {
                "output_columns": [
                    {
                        "name": "ts",
                        "datatype": { "type": "epoch", "primary": true, "format": "s" }
                    },
                    { "name": "bytes", "datatype": { "type": "uint64" } }
                ],
                "sample_data": { "ts": now, "bytes": 42 }
            }
        });
        let table_settings = json!({ "age": { "max_age_days": 1 } });
        let query = |_sql: String| async move { Ok::<_, String>(String::new()) };

        let diagnosis =
            diagnose_zero_rows(query, "proj", "tbl", &transform, &table_settings, now).await;
        assert!(
            diagnosis.contains("No obvious cause"),
            "expected fallback message, got: {diagnosis}"
        );
    }

    #[tokio::test]
    async fn diagnose_emits_missing_required_fields_when_primary_absent() {
        let now = 1_704_067_200_u64;
        let transform = json!({
            "settings": {
                "output_columns": [
                    {
                        "name": "ts",
                        "datatype": { "type": "epoch", "primary": true, "format": "s" }
                    },
                    { "name": "bytes", "datatype": { "type": "uint64" } }
                ],
                // Primary column "ts" intentionally absent from sample_data.
                "sample_data": { "bytes": 42 }
            }
        });
        let table_settings = json!({ "age": { "max_age_days": 1 } });
        let query = |_sql: String| async move { Ok::<_, String>(String::new()) };

        let diagnosis =
            diagnose_zero_rows(query, "proj", "tbl", &transform, &table_settings, now).await;
        assert!(
            diagnosis.contains("MISSING REQUIRED FIELDS"),
            "expected missing-fields finding, got: {diagnosis}"
        );
        assert!(
            diagnosis.contains("ts"),
            "expected primary column name in finding, got: {diagnosis}"
        );
    }

    #[tokio::test]
    async fn diagnose_emits_recent_ingest_errors() {
        // Fresh timestamp — don't trigger the staleness finding.
        let now = 1_704_067_200_u64;
        let transform = json!({
            "settings": {
                "output_columns": [
                    {
                        "name": "ts",
                        "datatype": { "type": "epoch", "primary": true, "format": "s" }
                    }
                ],
                "sample_data": { "ts": now }
            }
        });
        let table_settings = json!({ "age": { "max_age_days": 1 } });

        let query = |sql: String| async move {
            if sql.contains("system.ingest_errors") {
                Ok::<_, String>("2024-01-01,transform rejected column".to_string())
            } else {
                Ok::<_, String>(String::new())
            }
        };

        let diagnosis =
            diagnose_zero_rows(query, "proj", "tbl", &transform, &table_settings, now).await;
        assert!(
            diagnosis.contains("Recent ingest errors"),
            "expected ingest-error finding, got: {diagnosis}"
        );
        assert!(
            diagnosis.contains("transform rejected column"),
            "expected raw error text, got: {diagnosis}"
        );
    }

    #[tokio::test]
    async fn diagnose_emits_primary_timestamp_stale() {
        let transform = json!({
            "settings": {
                "output_columns": [
                    {
                        "name": "ts",
                        "datatype": { "type": "epoch", "primary": true, "format": "s" }
                    }
                ],
                "sample_data": { "ts": 1_607_368_207_u64 }
            }
        });
        let table_settings = json!({ "age": { "max_age_days": 1 } });
        // 2024-01-01 ~ much later than 2020-12-07 sample
        let now = 1_704_067_200_u64;
        let query = |_sql: String| async move { Ok::<_, String>(String::new()) };

        let diagnosis =
            diagnose_zero_rows(query, "proj", "tbl", &transform, &table_settings, now).await;
        assert!(
            diagnosis.contains("PRIMARY TIMESTAMP STALE"),
            "expected stale finding, got: {diagnosis}"
        );
    }

    #[test]
    fn missing_required_fields_reports_primary_when_absent() {
        let transform = json!({
            "settings": {
                "output_columns": [
                    {
                        "name": "ts",
                        "datatype": { "type": "epoch", "primary": true }
                    },
                    { "name": "bytes", "datatype": { "type": "uint64" } }
                ],
                "sample_data": { "bytes": 42 }
            }
        });

        assert_eq!(missing_required_fields(&transform), vec!["ts".to_string()]);
    }

    #[test]
    fn missing_required_fields_empty_when_primary_present() {
        let transform = json!({
            "settings": {
                "output_columns": [
                    {
                        "name": "ts",
                        "datatype": { "type": "epoch", "primary": true }
                    },
                    { "name": "bytes", "datatype": { "type": "uint64" } }
                ],
                "sample_data": { "ts": 1, "bytes": 42 }
            }
        });

        assert!(missing_required_fields(&transform).is_empty());
    }

    #[test]
    fn missing_required_fields_ignores_non_primary_absences() {
        // Non-primary columns may have defaults or be optional — don't flag them.
        let transform = json!({
            "settings": {
                "output_columns": [
                    {
                        "name": "ts",
                        "datatype": { "type": "epoch", "primary": true }
                    },
                    { "name": "bytes", "datatype": { "type": "uint64" } },
                    { "name": "host", "datatype": { "type": "string" } }
                ],
                "sample_data": { "ts": 1 }
            }
        });

        assert!(
            missing_required_fields(&transform).is_empty(),
            "non-primary absences must not be reported"
        );
    }

    #[test]
    fn coerce_to_epoch_secs_applies_sub_second_divisors() {
        assert_eq!(
            coerce_to_epoch_secs(&json!(1607368207_000_u64), "ms"),
            Some(1607368207)
        );
        assert_eq!(
            coerce_to_epoch_secs(&json!(1607368207_000_000_u64), "us"),
            Some(1607368207)
        );
        assert_eq!(
            coerce_to_epoch_secs(&json!(1607368207_000_000_000_u64), "ns"),
            Some(1607368207)
        );
    }

    #[test]
    fn primary_epoch_returns_none_when_no_primary_column() {
        let transform = json!({
            "settings": {
                "output_columns": [
                    { "name": "bytes", "datatype": { "type": "uint64" } },
                    {
                        "name": "ts",
                        "datatype": { "type": "epoch", "format": "s" }
                    }
                ],
                "sample_data": { "bytes": 42, "ts": 1607368207 }
            }
        });

        assert_eq!(primary_epoch_from_transform(&transform), None);
    }
}
