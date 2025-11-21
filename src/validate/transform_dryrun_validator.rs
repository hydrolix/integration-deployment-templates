// Transform validation using Hydrolix dry-run API
// Validates transforms after they're created by sending sample data through the transform

use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::sleep;

const HTTP_TIMEOUT_SECS: u64 = 120;
const TRANSFORM_READY_DELAY_MS: u64 = 5000; // 5 seconds
const MAX_RETRIES: u32 = 5; // Back to 5 for reliability

#[derive(Debug)]
pub struct ValidationResult {
    pub success: bool,
    pub error: Option<String>,
    pub has_unknown_data: bool,
    pub unknown_values: Vec<Value>,
}

impl ValidationResult {
    fn success() -> Self {
        Self {
            success: true,
            error: None,
            has_unknown_data: false,
            unknown_values: Vec::new(),
        }
    }

    fn error(msg: String) -> Self {
        Self {
            success: false,
            error: Some(msg),
            has_unknown_data: false,
            unknown_values: Vec::new(),
        }
    }

    fn unknown_data(values: Vec<Value>) -> Self {
        Self {
            success: false,
            error: None,
            has_unknown_data: true,
            unknown_values: values,
        }
    }
}

/// Validates a transform after creation by sending sample data through it
pub async fn validate_transform_after_creation(
    cluster: &str,
    bearer_token: &str,
    project_name: &str,
    table_name: &str,
    transform_name: &str,
    sample_data: &Value,
    transform_path: &str,
    strict_mode: bool,
) -> Result<(), String> {
    println!("  🔍 Validating transform via API: {}...", transform_name);

    // Wait for transform to be fully available
    println!("    ⏳ Waiting for transform to be ready...");
    sleep(Duration::from_millis(TRANSFORM_READY_DELAY_MS)).await;

    let result = validate_via_dryrun_with_retry(
        cluster,
        bearer_token,
        project_name,
        table_name,
        transform_name,
        sample_data,
        MAX_RETRIES,
    )
    .await;

    // Check for API errors
    if !result.success && !result.has_unknown_data {
        let error_msg = format!(
            "Transform validation failed for {}:\n  Transform: {}\n  Error: {}\n\n  This usually indicates:\n    • Sample data doesn't match input schema\n    • SQL transform has syntax errors\n    • Type conversion issues in the transform\n    • Missing or incorrect column definitions",
            transform_path,
            transform_name,
            result.error.as_deref().unwrap_or("Unknown error")
        );

        if strict_mode {
            return Err(error_msg);
        } else {
            println!(
                "    ⚠️  WARNING: Validation failed - {}",
                result.error.as_deref().unwrap_or("Unknown")
            );
            return Ok(());
        }
    }

    // Check for "unknown" column with data
    if result.has_unknown_data {
        let sample_values = result
            .unknown_values
            .iter()
            .take(3)
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .collect::<Vec<_>>()
            .join(", ");

        let error_msg = format!(
            "Transform validation failed for {}:\n  Transform: {}\n  Issue: 'unknown' column contains data\n  Sample values: {}\n\n  This indicates the transform is not parsing data correctly.\n  Data is being dumped into the catch-all 'unknown' column.\n\n  Common causes:\n    • Input schema doesn't match actual data structure\n    • SQL transform missing fields\n    • Field extraction/parsing logic incorrect\n    • Column mappings misaligned",
            transform_path, transform_name, sample_values
        );

        if strict_mode {
            return Err(error_msg);
        } else {
            println!("    ⚠️  WARNING: 'unknown' column has data");
            let sample = result
                .unknown_values
                .iter()
                .take(2)
                .map(|v| serde_json::to_string(v).unwrap_or_default())
                .collect::<Vec<_>>()
                .join(", ");
            println!("       Sample values: {}", sample);
            return Ok(());
        }
    }

    println!("    ✅ Valid (parses correctly, no unknown data)");
    Ok(())
}

async fn validate_via_dryrun_with_retry(
    cluster: &str,
    bearer_token: &str,
    project_name: &str,
    table_name: &str,
    transform_name: &str,
    sample_data: &Value,
    max_retries: u32,
) -> ValidationResult {
    let mut last_result: Option<ValidationResult> = None;

    for attempt in 1..=max_retries {
        let result = validate_via_dryrun(
            cluster,
            bearer_token,
            project_name,
            table_name,
            transform_name,
            sample_data,
        )
        .await;

        // If success or has unknown data (real validation issue), return immediately
        if result.success || result.has_unknown_data {
            return result;
        }

        // If error is "unknown transform", retry after delay
        if let Some(ref error) = result.error {
            if error.contains("unknown transform") {
                last_result = Some(result);
                if attempt < max_retries {
                    println!(
                        "    ⏳ Transform not ready yet, retrying ({}/{})...",
                        attempt, max_retries
                    );
                    sleep(Duration::from_millis(5000)).await; // 5 seconds between retries
                    continue;
                }
            } else {
                // Other errors, return immediately
                return result;
            }
        } else {
            return result;
        }
    }

    // All retries exhausted
    last_result.unwrap_or_else(|| ValidationResult::error("Validation failed after retries".to_string()))
}

async fn validate_via_dryrun(
    cluster: &str,
    bearer_token: &str,
    project_name: &str,
    table_name: &str,
    transform_name: &str,
    sample_data: &Value,
) -> ValidationResult {
    let full_table_name = format!("{}.{}", project_name, table_name);
    let url = format!(
        "https://{}/ingest/event?dryrun=true&output_format=json",
        cluster
    );

    // Ensure sample data is an array
    let payload = if sample_data.is_array() {
        sample_data.clone()
    } else {
        json!([sample_data])
    };

    let payload_len = payload.as_array().map(|a| a.len()).unwrap_or(0);
    println!("    🔍 DEBUG: Sending {} sample record(s) to dry-run", payload_len);
    println!("    🔍 DEBUG: URL: {}", url);
    println!(
        "    🔍 DEBUG: Headers: x-hdx-table={}, x-hdx-transform={}",
        full_table_name, transform_name
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
        .build()
        .unwrap();

    match client
        .post(&url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("x-hdx-table", &full_table_name)
        .header("x-hdx-transform", transform_name)
        .body(payload.to_string())
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            if !status.is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Could not read error text".to_string());
                return ValidationResult::error(format!(
                    "HTTP {}: {}",
                    status,
                    error_text
                ));
            }

            let response_text = match response.text().await {
                Ok(text) => text,
                Err(e) => {
                    return ValidationResult::error(format!("Could not read response: {}", e));
                }
            };

            println!("    🔍 DEBUG: Response text length: {}", response_text.len());
            println!("    🔍 DEBUG: Response preview: {}", &response_text.chars().take(500).collect::<String>());

            // Try to parse as JSON
            match serde_json::from_str::<Value>(&response_text) {
                Ok(transformed_data) => {
                    // Debug: log the structure
                    println!(
                        "    🔍 DEBUG: Response has {} rows",
                        transformed_data
                            .get("rows")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0)
                    );

                    if let Some(meta) = transformed_data.get("meta").and_then(|v| v.as_array()) {
                        let unknown_col = meta.iter().find(|c| {
                            c.get("name")
                                .and_then(|n| n.as_str())
                                .map(|n| n == "unknown")
                                .unwrap_or(false)
                        });

                        if let Some(_) = unknown_col {
                            let unknown_index = meta
                                .iter()
                                .position(|c| {
                                    c.get("name")
                                        .and_then(|n| n.as_str())
                                        .map(|n| n == "unknown")
                                        .unwrap_or(false)
                                })
                                .unwrap();

                            println!("    🔍 DEBUG: Found 'unknown' column at index {}", unknown_index);

                            if let Some(data) = transformed_data.get("data").and_then(|d| d.as_array()) {
                                if !data.is_empty() {
                                    if let Some(first_row) = data[0].as_array() {
                                        if let Some(unknown_val) = first_row.get(unknown_index) {
                                            let preview = serde_json::to_string(unknown_val)
                                                .unwrap_or_default()
                                                .chars()
                                                .take(200)
                                                .collect::<String>();
                                            println!("    🔍 DEBUG: First row unknown value: {}", preview);
                                        }
                                    }
                                }
                            }
                        } else {
                            println!("    🔍 DEBUG: No 'unknown' column in meta");
                        }
                    }

                    // Check if response contains data with "unknown" column populated
                    let unknown_check = check_for_unknown_data(&transformed_data);

                    println!(
                        "    🔍 DEBUG: unknownCheck.hasData = {}, values count = {}",
                        !unknown_check.is_empty(),
                        unknown_check.len()
                    );

                    if !unknown_check.is_empty() {
                        return ValidationResult::unknown_data(unknown_check);
                    }

                    ValidationResult::success()
                }
                Err(e) => {
                    // Response might not be JSON, which is fine
                    println!("    🔍 DEBUG: Response is not JSON: {}", e);
                    ValidationResult::success()
                }
            }
        }
        Err(e) => ValidationResult::error(format!("Validation request failed: {}", e)),
    }
}

fn check_for_unknown_data(data: &Value) -> Vec<Value> {
    let mut unknown_values: Vec<Value> = Vec::new();

    // Handle columnar format (data[] + meta[])
    if let (Some(data_array), Some(meta_array)) = (
        data.get("data").and_then(|d| d.as_array()),
        data.get("meta").and_then(|m| m.as_array()),
    ) {
        // Find the "unknown" column index
        let unknown_column_index = meta_array.iter().position(|col| {
            col.get("name")
                .and_then(|n| n.as_str())
                .map(|n| n == "unknown")
                .unwrap_or(false)
        });

        if unknown_column_index.is_none() {
            // No unknown column found - that's actually fine
            return unknown_values;
        }

        let unknown_index = unknown_column_index.unwrap();

        // Check each row for data in the unknown column
        for row in data_array {
            if let Some(row_array) = row.as_array() {
                if let Some(unknown_value) = row_array.get(unknown_index) {
                    // Check if unknown column has actual data
                    if has_actual_data(unknown_value) {
                        unknown_values.push(unknown_value.clone());
                    }
                }
            }
        }

        return unknown_values;
    }

    // Handle object format (array of objects with .unknown property)
    let mut records: Vec<&Value> = Vec::new();

    if data.is_array() {
        if let Some(arr) = data.as_array() {
            records = arr.iter().collect();
        }
    } else if let Some(records_val) = data.get("records") {
        if let Some(arr) = records_val.as_array() {
            records = arr.iter().collect();
        }
    } else if data.is_object() {
        // Single record
        records.push(data);
    }

    for record in records {
        if let Some(unknown_value) = record.get("unknown") {
            if has_actual_data(unknown_value) {
                unknown_values.push(unknown_value.clone());
            }
        }
    }

    unknown_values
}

fn has_actual_data(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(s) => !s.is_empty(),
        Value::Object(map) => !map.is_empty(),
        Value::Array(arr) => !arr.is_empty(),
        _ => true,
    }
}

/// Validate transform by querying table for catastrophic parsing failures
/// Note: Due to shared table and identical timestamps, we check ALL unknown data
/// rather than trying to isolate specific transform's records
pub async fn validate_transform_by_querying(
    cluster: &str,
    bearer_token: &str,
    project_name: &str,
    table_name: &str,
    transform_name: &str,
    transform_path: &str,
    strict_mode: bool,
) -> Result<(), String> {
    println!("  🔍 Querying table for catastrophic parsing failures...");

    // Wait for data to be available and query pool to be ready
    println!("    ⏳ Waiting 3s for data to be queryable...");
    sleep(Duration::from_millis(3000)).await;

    // Quick check if table is queryable
    let queryable = is_table_queryable(cluster, bearer_token, project_name, table_name).await;
    if !queryable {
        println!("    ⚠️  WARNING: Table is not queryable yet (query pool not initialized)");
        println!("    This might indicate:");
        println!("      - Cluster is slow/under-resourced");
        println!("      - Query pool provisioning is taking longer than expected");
        println!("      - Possible cluster configuration issue");
    }

    // Query ALL records with unknown data
    let full_table_name = format!("{}.{}", project_name, table_name);
    let sql = format!(
        "SELECT hdx_transform, unknown FROM {} WHERE mapKeys(unknown) != [] LIMIT 100 FORMAT JSON",
        full_table_name
    );
    let url = format!(
        "https://{}/query?query={}",
        cluster,
        urlencoding::encode(&sql)
    );

    println!("    🔍 DEBUG: Querying all records with unknown data");
    println!("    🔍 DEBUG: SQL: {}", sql);
    println!("    🔍 DEBUG: Cluster: {}", cluster);

    let client = reqwest::Client::new();
    match client
        .get(&url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            if !status.is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Could not read error".to_string());
                println!(
                    "    ⚠️  WARNING: Could not query inserted records: {} - {}",
                    status,
                    error_text
                );
                return Ok(());
            }

            let result: Value = match response.json().await {
                Ok(v) => v,
                Err(e) => {
                    println!("    ⚠️  WARNING: Could not parse response: {}", e);
                    return Ok(());
                }
            };

            let rows = result.get("rows").and_then(|v| v.as_u64()).unwrap_or(0);
            println!("    🔍 DEBUG: Query returned {} rows with unknown data", rows);

            if let Some(data) = result.get("data").and_then(|d| d.as_array()) {
                if data.is_empty() {
                    println!("    ✅ No records with unknown data found in table");
                    return Ok(());
                }

                println!("    🔍 DEBUG: Analyzing {} record(s) with unknown data", data.len());

                // Analyze records for catastrophic failures
                let analysis = analyze_unknown_data(data);

                if analysis.catastrophic_count > 0 {
                    println!("    ❌ CATASTROPHIC FAILURE DETECTED IN TABLE");
                    println!(
                        "       Found {} record(s) with raw unparsed data",
                        analysis.catastrophic_count
                    );
                    println!("       Broken transforms:");
                    for (transform, count) in &analysis.catastrophic_by_transform {
                        println!("         - {}: {} record(s) with catastrophic failures", transform, count);
                    }

                    if strict_mode {
                        let error_msg = format!(
                            "Transform validation WARNING for {}:\n  Transform: {}\n  Issue: CATASTROPHIC FAILURES detected in table\n  Found {} record(s) with raw unparsed data\n  Broken transforms: {}\n\n  This indicates one or more transforms are completely broken:\n    • Input format/schema is incorrect\n    • Delimiter settings are wrong\n    • Parsing logic is not executing\n    • Data is being dumped as raw string instead of parsed fields",
                            transform_path,
                            transform_name,
                            analysis.catastrophic_count,
                            analysis.catastrophic_by_transform.keys().cloned().collect::<Vec<_>>().join(", ")
                        );
                        return Err(error_msg);
                    } else {
                        return Ok(());
                    }
                }

                if analysis.minor_count > 0 {
                    println!(
                        "    ⚠️  Minor Issue: Table has {} record(s) with unmapped fields",
                        analysis.minor_count
                    );
                    println!("       This is usually acceptable - just missing field mappings in output schema");
                    return Ok(());
                }

                println!("    ✅ No catastrophic failures detected in table");
            }

            Ok(())
        }
        Err(e) => {
            println!("    ⚠️  WARNING: Failed to validate inserted records: {}", e);
            Ok(())
        }
    }
}

struct UnknownDataAnalysis {
    catastrophic_count: usize,
    minor_count: usize,
    catastrophic_by_transform: std::collections::HashMap<String, usize>,
}

fn analyze_unknown_data(data: &[Value]) -> UnknownDataAnalysis {
    const LARGE_STRING_THRESHOLD: usize = 150;

    let mut catastrophic_count = 0;
    let mut minor_count = 0;
    let mut catastrophic_by_transform: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for (i, row) in data.iter().enumerate() {
        let transform_name = row
            .get("hdx_transform")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        if let Some(unknown_value) = row.get("unknown") {
            if let Some(unknown_map) = unknown_value.as_object() {
                if unknown_map.is_empty() {
                    continue;
                }

                let mut is_catastrophic = false;

                // Check 1: Does unknown contain a "data" key?
                if unknown_map.contains_key("data") {
                    if let Some(Value::String(_)) = unknown_map.get("data") {
                        is_catastrophic = true;
                        if i < 3 {
                            println!(
                                "    🔍 DEBUG: Record {} [{}]: Found 'data' key - CATASTROPHIC",
                                i + 1,
                                transform_name
                            );
                        }
                    }
                }

                // Check 2: Does unknown contain any very large string values?
                if !is_catastrophic {
                    for (key, value) in unknown_map.iter() {
                        if let Some(s) = value.as_str() {
                            if s.len() > LARGE_STRING_THRESHOLD {
                                is_catastrophic = true;
                                if i < 3 {
                                    println!(
                                        "    🔍 DEBUG: Record {} [{}]: Found large string ({} chars) in unknown.{} - CATASTROPHIC",
                                        i + 1,
                                        transform_name,
                                        s.len(),
                                        key
                                    );
                                }
                                break;
                            }
                        }
                    }
                }

                if is_catastrophic {
                    catastrophic_count += 1;
                    *catastrophic_by_transform.entry(transform_name).or_insert(0) += 1;
                } else {
                    minor_count += 1;
                }
            }
        }
    }

    UnknownDataAnalysis {
        catastrophic_count,
        minor_count,
        catastrophic_by_transform,
    }
}

async fn is_table_queryable(
    cluster: &str,
    bearer_token: &str,
    project_name: &str,
    table_name: &str,
) -> bool {
    let full_table_name = format!("{}.{}", project_name, table_name);
    let sql = format!("SELECT 1 FROM {} LIMIT 1 FORMAT JSON", full_table_name);
    let url = format!(
        "https://{}/query?query={}",
        cluster,
        urlencoding::encode(&sql)
    );

    let client = reqwest::Client::new();
    match client
        .get(&url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                return true;
            }

            if let Ok(error_text) = response.text().await {
                return !error_text.contains("Pool name") && !error_text.contains("does not exist");
            }

            false
        }
        Err(_) => false,
    }
}
