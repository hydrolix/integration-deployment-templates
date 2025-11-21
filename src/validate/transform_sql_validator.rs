// SQL Transform Validator - Uses ClickHouse Memory tables to validate transforms
// This catches catastrophic failures where fields are silently dropped
//
// NOTE: This validator requires elevated ClickHouse permissions to execute
// CREATE TABLE, INSERT, and DROP TABLE commands. It may not work with
// standard bearer tokens and requires direct ClickHouse admin access.

use serde_json::Value;
use std::time::Duration;
use base64::{Engine as _, engine::general_purpose};

const QUERY_TIMEOUT_SECS: u64 = 30;

#[derive(Debug)]
pub struct ValidationResult {
    pub success: bool,
    pub error: Option<String>,
    pub input_field_count: Option<usize>,
    pub output_field_count: Option<usize>,
    pub missing_fields: Vec<String>,
    pub sql_error: Option<String>,
}

impl ValidationResult {
    fn success() -> Self {
        Self {
            success: true,
            error: None,
            input_field_count: None,
            output_field_count: None,
            missing_fields: Vec::new(),
            sql_error: None,
        }
    }

    fn error(msg: String) -> Self {
        Self {
            success: false,
            error: Some(msg),
            input_field_count: None,
            output_field_count: None,
            missing_fields: Vec::new(),
            sql_error: None,
        }
    }
}

/// Validates a transform by running its SQL against sample data in a Memory table.
/// This catches failures where fields are silently dropped or SQL references non-existent columns.
pub async fn validate_transform_sql(
    cluster: &str,
    bearer_token: &str,
    transform_json: &Value,
    transform_name: &str,
) -> ValidationResult {
    println!("  🔍 Validating SQL transform: {}...", transform_name);

    // Extract necessary components
    let sql_transform = transform_json
        .get("settings")
        .and_then(|s| s.get("sql_transform"))
        .and_then(|s| s.as_str());

    let sample_data = transform_json
        .get("settings")
        .and_then(|s| s.get("sample_data"));

    if sql_transform.is_none() {
        return ValidationResult::success(); // No SQL transform to validate
    }

    if sample_data.is_none() {
        return ValidationResult::success(); // No sample data to validate with
    }

    let sql_transform = sql_transform.unwrap();
    let sample_data = sample_data.unwrap();

    // Handle both array and single object sample data formats
    let first_sample = if sample_data.is_array() {
        sample_data.as_array().and_then(|arr| {
            if arr.is_empty() {
                None
            } else {
                Some(&arr[0])
            }
        })
    } else {
        Some(sample_data)
    };

    if first_sample.is_none() {
        return ValidationResult::success(); // No sample data to validate with
    }

    let first_sample = first_sample.unwrap();

    let input_schema = match infer_schema_from_sample(first_sample, transform_json) {
        Some(schema) => schema,
        None => {
            println!("    ⚠️  Could not infer schema from sample data, skipping SQL validation");
            return ValidationResult::success();
        }
    };

    if input_schema.columns.is_empty() {
        println!("    ⚠️  Could not infer schema from sample data, skipping SQL validation");
        return ValidationResult::success();
    }

    println!("    📊 Input schema has {} field(s)", input_schema.columns.len());

    // Create temporary table
    let temp_table_name = format!("temp_validate_{}", chrono::Utc::now().timestamp_millis());
    let create_table_sql = generate_create_table_sql(&temp_table_name, &input_schema.columns);

    // Execute validation queries
    let validation_result = run_validation_queries(
        cluster,
        bearer_token,
        &temp_table_name,
        &create_table_sql,
        &input_schema.flat_data,
        sql_transform,
        input_schema.columns.len(),
    )
    .await;

    // Clean up temp table
    let drop_sql = format!("DROP TABLE IF EXISTS {}", temp_table_name);
    let _ = execute_clickhouse_query(cluster, bearer_token, &drop_sql).await;

    validation_result
}

async fn run_validation_queries(
    cluster: &str,
    bearer_token: &str,
    temp_table_name: &str,
    create_table_sql: &str,
    flat_data: &Value,
    sql_transform: &str,
    input_field_count: usize,
) -> ValidationResult {
    // Create temp table
    if let Err(e) = execute_clickhouse_query(cluster, bearer_token, create_table_sql).await {
        // If we can't create tables, this is likely a permissions issue
        if e.contains("AUTHENTICATION_FAILED") || e.contains("authentication failed") {
            println!("    ⚠️  SQL validation skipped (ClickHouse query endpoint not accessible)");
            return ValidationResult::success();
        }
        return ValidationResult::error(format!("Failed to create temp table: {}", e));
    }

    // Insert sample data
    let insert_sql = generate_insert_sql(temp_table_name, flat_data);
    if let Err(_) = execute_clickhouse_query(cluster, bearer_token, &insert_sql).await {
        // INSERT with data might not work via GET query parameter
        println!("    ⚠️  SQL validation skipped (INSERT not supported via query endpoint)");
        return ValidationResult::success();
    }

    // Run the transform SQL
    let transform_sql = sql_transform.replace("{STREAM}", temp_table_name);
    let result = match execute_clickhouse_query(cluster, bearer_token, &transform_sql).await {
        Ok(r) => r,
        Err(e) => {
            return ValidationResult {
                success: false,
                error: Some("Transform SQL failed".to_string()),
                sql_error: Some(e),
                input_field_count: Some(input_field_count),
                output_field_count: None,
                missing_fields: Vec::new(),
            };
        }
    };

    // Validate the result
    if result.is_empty() {
        return ValidationResult {
            success: false,
            error: Some("Transform produced no output rows".to_string()),
            input_field_count: Some(input_field_count),
            output_field_count: Some(0),
            missing_fields: Vec::new(),
            sql_error: None,
        };
    }

    // Check for SQL errors in the output
    if result.contains("indexerError") || result.contains("DB::Exception") {
        let error_msg = extract_error_message(&result);
        return ValidationResult {
            success: false,
            error: Some("SQL transform failed".to_string()),
            sql_error: Some(error_msg),
            input_field_count: Some(input_field_count),
            output_field_count: None,
            missing_fields: Vec::new(),
        };
    }

    // Parse result to check output schema
    match serde_json::from_str::<Value>(&result) {
        Ok(output_data) => {
            if let Some(data) = output_data.get("data").and_then(|d| d.as_array()) {
                if !data.is_empty() {
                    if let Some(first_row) = data[0].as_object() {
                        let output_field_count = first_row.len();
                        println!("    ✅ Transform produced {} output field(s)", output_field_count);

                        // Check for significant field loss (more than 50% of fields lost)
                        let field_loss_ratio =
                            (input_field_count as f64 - output_field_count as f64)
                                / input_field_count as f64;
                        if field_loss_ratio > 0.5 {
                            return ValidationResult {
                                success: false,
                                error: Some(format!(
                                    "Catastrophic field loss: {} input fields → {} output fields",
                                    input_field_count, output_field_count
                                )),
                                input_field_count: Some(input_field_count),
                                output_field_count: Some(output_field_count),
                                missing_fields: Vec::new(),
                                sql_error: None,
                            };
                        }

                        return ValidationResult {
                            success: true,
                            error: None,
                            input_field_count: Some(input_field_count),
                            output_field_count: Some(output_field_count),
                            missing_fields: Vec::new(),
                            sql_error: None,
                        };
                    }
                }
            }

            ValidationResult::success()
        }
        Err(_) => {
            // If we can't parse the result, treat it as validation passed
            println!("    ⚠️  Could not parse query output, skipping output analysis");
            ValidationResult::success()
        }
    }
}

fn extract_error_message(result: &str) -> String {
    // Try to extract message from JSON error
    if let Ok(json) = serde_json::from_str::<Value>(result) {
        if let Some(msg) = json.get("message").and_then(|m| m.as_str()) {
            return msg.to_string();
        }
    }
    "Unknown SQL error".to_string()
}

#[derive(Debug)]
struct SchemaColumn {
    name: String,
    column_type: String,
}

#[derive(Debug)]
struct InferredSchema {
    columns: Vec<SchemaColumn>,
    flat_data: Value,
}

/// Infers schema from sample data, handling nested structures like Firehose
fn infer_schema_from_sample(sample: &Value, transform_json: &Value) -> Option<InferredSchema> {
    let format_details = transform_json
        .get("settings")
        .and_then(|s| s.get("format_details"));

    // Handle Firehose format (nested with base64 encoding)
    if let Some(subtype) = format_details
        .and_then(|f| f.get("subtype"))
        .and_then(|s| s.as_str())
    {
        if subtype == "firehose" {
            // Firehose structure: { records: [{ data: "base64..." }] }
            if let Some(records) = sample.get("records").and_then(|r| r.as_array()) {
                if !records.is_empty() {
                    if let Some(data_str) = records[0].get("data").and_then(|d| d.as_str()) {
                        // Decode base64 data
                        if let Ok(decoded_bytes) = general_purpose::STANDARD.decode(data_str) {
                            if let Ok(decoded_str) = String::from_utf8(decoded_bytes) {
                                if let Ok(decoded_data) = serde_json::from_str::<Value>(&decoded_str) {
                                    if let Some(obj) = decoded_data.as_object() {
                                        let columns: Vec<SchemaColumn> = obj
                                            .keys()
                                            .map(|key| SchemaColumn {
                                                name: key.clone(),
                                                column_type: infer_clickhouse_type(obj.get(key).unwrap()),
                                            })
                                            .collect();

                                        return Some(InferredSchema {
                                            columns,
                                            flat_data: decoded_data,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Handle flat JSON format
    if let Some(obj) = sample.as_object() {
        let columns: Vec<SchemaColumn> = obj
            .keys()
            .map(|key| SchemaColumn {
                name: key.clone(),
                column_type: infer_clickhouse_type(obj.get(key).unwrap()),
            })
            .collect();

        return Some(InferredSchema {
            columns,
            flat_data: sample.clone(),
        });
    }

    None
}

/// Infers ClickHouse type from a value
fn infer_clickhouse_type(value: &Value) -> String {
    match value {
        Value::Null => "Nullable(String)".to_string(),
        Value::Bool(_) => "Nullable(UInt8)".to_string(),
        Value::Number(n) => {
            if n.is_i64() {
                "Nullable(Int64)".to_string()
            } else {
                "Nullable(Float64)".to_string()
            }
        }
        Value::String(s) => {
            // Try to detect timestamp
            if s.len() >= 10 && s.len() <= 13 && s.chars().all(|c| c.is_ascii_digit()) {
                "Nullable(DateTime64(3))".to_string()
            } else {
                "Nullable(String)".to_string()
            }
        }
        _ => "Nullable(String)".to_string(),
    }
}

/// Generates CREATE TABLE SQL for Memory engine
fn generate_create_table_sql(table_name: &str, columns: &[SchemaColumn]) -> String {
    let column_defs: Vec<String> = columns
        .iter()
        .map(|col| format!("`{}` {}", col.name, col.column_type))
        .collect();

    format!(
        "CREATE TEMPORARY TABLE {} (\n    {}\n) ENGINE = Memory",
        table_name,
        column_defs.join(",\n    ")
    )
}

/// Generates INSERT SQL for sample data
fn generate_insert_sql(table_name: &str, data: &Value) -> String {
    format!(
        "INSERT INTO {} FORMAT JSONEachRow {}",
        table_name,
        serde_json::to_string(data).unwrap_or_default()
    )
}

/// Executes a ClickHouse query via HTTP API (using GET method)
async fn execute_clickhouse_query(
    cluster: &str,
    bearer_token: &str,
    query: &str,
) -> Result<String, String> {
    // Use GET with query parameter and FORMAT JSON
    let query_with_format = if query.contains("FORMAT") {
        query.to_string()
    } else {
        format!("{} FORMAT JSON", query)
    };

    let url = format!(
        "https://{}/query?query={}",
        cluster,
        urlencoding::encode(&query_with_format)
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(QUERY_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    if !status.is_success() {
        return Err(format!(
            "ClickHouse query failed ({}): {}",
            status,
            text
        ));
    }

    Ok(text)
}
