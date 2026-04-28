use crate::hdx::{BUNDLE_TESTING_CLUSTER, CLIENT, FOR_MARKETPLACE, HTTP_TIMEOUT, ORG_UUID};
use serde_json::{json, Value};
use tokio::time::sleep;
use tokio::time::Duration;
use uuid::Uuid;

pub async fn exists(bearer_token: &str, table_name: &str) -> Result<(), String> {
    let url = format!(
        "https://{}/config/v1/orgs/{}/projects/{}/tables",
        *BUNDLE_TESTING_CLUSTER,
        ORG_UUID,
        super::get_project_uuid()
    );

    let max_attempts = 12; // Try for up to 60 seconds (12 * 5 seconds)

    for attempt in 1..=max_attempts {
        let response = match CLIENT
            .get(&url)
            .header("Authorization", format!("Bearer {}", bearer_token))
            .header("Accept", "application/json")
            .timeout(Duration::from_secs(HTTP_TIMEOUT))
            .send()
            .await
        {
            Ok(v) => v,
            Err(e) => {
                if attempt == max_attempts {
                    return Err(format!("Failed to list tables: {}", e));
                }
                sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        if response.status().is_success() {
            if let Ok(tables_json) = response.json::<Value>().await {
                // Handle different response formats
                let empty_vec = vec![];
                let tables_array = if tables_json.is_array() {
                    tables_json.as_array().unwrap()
                } else if let Some(results) = tables_json.get("results") {
                    results.as_array().unwrap_or(&empty_vec)
                } else if let Some(data) = tables_json.get("data") {
                    data.as_array().unwrap_or(&empty_vec)
                } else {
                    &empty_vec
                };

                for table in tables_array {
                    if let Some(name) = table.get("name").and_then(|n| n.as_str()) {
                        if name == table_name {
                            println!("  ✓ Verified table '{}' exists and is ready", table_name);
                            return Ok(());
                        }
                    }
                }
            }
        }

        if attempt < max_attempts {
            println!(
                "  Table '{}' not found yet, waiting... (attempt {}/{})",
                table_name, attempt, max_attempts
            );
            sleep(Duration::from_secs(5)).await;
        }
    }

    Err(format!(
        "Table '{}' not found after {} attempts",
        table_name, max_attempts
    ))
}

/// Create a table in a specific project (used by --guid flow and integration tests).
#[allow(dead_code)]
pub async fn create_in_project(
    bearer_token: &str,
    project_uuid: &str,
    table_name: &str,
) -> Result<String, String> {
    let payload = json!({
        "name": table_name,
        "description": "testing",
        "settings": {
            "age": { "max_age_days": 1 },
            "merge": { "enabled": false }
        }
    });

    let url = format!(
        "https://{}/config/v1/orgs/{ORG_UUID}/projects/{project_uuid}/tables",
        *BUNDLE_TESTING_CLUSTER
    );

    let response = match CLIENT
        .post(&url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/plain, */*")
        .timeout(Duration::from_secs(HTTP_TIMEOUT))
        .json(&payload)
        .send()
        .await
    {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} url={url} error={e}",
                file!(),
                line!()
            ))
        }
    };

    let status = response.status();
    let table_data = match response.text().await {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} url={url} error reading response: {e}",
                file!(),
                line!()
            ));
        }
    };

    if !status.is_success() {
        return Err(format!(
            "ERROR: {}.{} {} - Server response: {}",
            file!(),
            line!(),
            status,
            table_data
        ));
    }

    let table_json: Value = match serde_json::from_str(&table_data) {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} url={url} error={e}",
                file!(),
                line!()
            ));
        }
    };

    match table_json["uuid"].as_str() {
        Some(v) => Ok(v.to_string()),
        None => Err(format!(
            "ERROR: {}.{} table UUID not found",
            file!(),
            line!()
        )),
    }
}

pub async fn create(bearer_token: &str, table_name: &str) -> Result<String, String> {
    // Prepare the JSON payload

    let payload = if *FOR_MARKETPLACE {
        json!({
                "name": table_name,
                "description": "testing",
                "settings": {
                    "age": {
                        "max_age_days": 1
                    },
                    "merge": {
                        "enabled": false
                    },
                    "default_query_options": {
                        "hdx_query_max_timerange_sec": 2592000,
                        "hdx_query_max_result_rows": 5000000,
                        "hdx_query_max_execution_time": 180
                    },
                }
        })
    } else {
        json!({
                "name": table_name,
                "description": "testing",
                "settings": {
                    "age": {
                        "max_age_days": 1
                    },
                    "merge": {
                        "enabled": false
                    }
                }
        })
    };

    let url = format!(
        "https://{}/config/v1/orgs/{}/projects/{}/tables",
        *BUNDLE_TESTING_CLUSTER,
        ORG_UUID,
        super::get_project_uuid()
    );

    // Send the POST request
    let response = match CLIENT
        .post(&url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/plain, */*")
        .timeout(Duration::from_secs(HTTP_TIMEOUT))
        .json(&payload)
        .send()
        .await
    {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} url={url} error={e}",
                file!(),
                line!()
            ))
        }
    };

    // Check if the request was successful
    let status = response.status();

    // Get the response body
    let table_data = match response.text().await {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} url={url} error reading response: {e}",
                file!(),
                line!()
            ));
        }
    };

    if !status.is_success() {
        return Err(format!(
            "ERROR: {}.{} {} - Server response: {}",
            file!(),
            line!(),
            status,
            table_data
        ));
    }

    let table_json: Value = match serde_json::from_str(&table_data) {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} url={url} error={e}",
                file!(),
                line!()
            ));
        }
    };

    match table_json["uuid"].as_str() {
        Some(v) => Ok(v.to_string()),
        None => Err(format!(
            "ERROR: {}.{} table UUID not found",
            file!(),
            line!()
        )),
    }
}

pub async fn add_transform(
    bearer_token: &str,
    table_uuid: &str,
    transform_json: &Value,
) -> Result<String, String> {
    let transform_name = match transform_json["name"].as_str() {
        Some(v) => v.to_string(),
        None => {
            return Err(format!(
                "ERROR: {}.{} Could not find the transformation name {:?}",
                file!(),
                line!(),
                transform_json
            ));
        }
    };

    let url = format!(
        "https://{}/config/v1/orgs/{}/projects/{}/tables/{table_uuid}/transforms/",
        *BUNDLE_TESTING_CLUSTER,
        ORG_UUID,
        super::get_project_uuid()
    );

    // Exponential backoff configuration
    let max_retries = 5;
    let base_delay = Duration::from_secs(1);
    let max_delay = Duration::from_secs(30);

    let mut attempt = 0;

    loop {
        attempt += 1;

        let response = match CLIENT
            .post(&url)
            .header("Authorization", format!("Bearer {}", bearer_token))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .timeout(Duration::from_secs(HTTP_TIMEOUT))
            .json(&transform_json)
            .send()
            .await
        {
            Ok(v) => v,
            Err(e) => {
                if attempt > max_retries {
                    return Err(format!(
                        "ERROR: {}.{} Failed to add transform after {} attempts: {e} {:?}",
                        file!(),
                        line!(),
                        attempt,
                        transform_json
                    ));
                }

                // Calculate exponential backoff
                let delay = calculate_backoff(attempt, base_delay, max_delay);
                tokio::time::sleep(delay).await;
                continue;
            }
        };

        if response.status().is_success() {
            return Ok(transform_name.to_string());
        }

        // Check if we should retry on HTTP errors (5xx server errors)
        if response.status().is_server_error() && attempt <= max_retries {
            let delay = calculate_backoff(attempt, base_delay, max_delay);
            tokio::time::sleep(delay).await;
            continue;
        }

        // For client errors (4xx) or after max retries, return error
        return Err(format!(
            "ERROR: {}.{}
            Hydrolix add transform failed, status: {} url={url} (attempt {})",
            file!(),
            line!(),
            response.status(),
            attempt
        ));
    }
}

// Helper function to calculate exponential backoff without jitter
fn calculate_backoff(attempt: u32, base_delay: Duration, max_delay: Duration) -> Duration {
    let exponent = attempt - 1;
    let delay_ms = base_delay.as_millis() as u64 * 2u64.pow(exponent);
    Duration::from_millis(delay_ms).min(max_delay)
}

pub async fn insert_into(
    bearer_token: &str,
    full_table_name: &str,
    transform_name: &str,
    sample_data: &Value,
) -> Result<(), String> {
    let sample_data = json!([sample_data]);
    let url = format!("https://{}/ingest/event", *BUNDLE_TESTING_CLUSTER);

    let max_retries = 20;
    let base_delay_ms = 1000; // 1 second base delay
    let max_delay_ms = 60000; // Cap at 60 seconds
    let backoff_factor: f64 = 2.0;

    for attempt in 0..max_retries {
        // Calculate exponential backoff with jitter
        if attempt > 0 {
            let exponential_delay =
                (base_delay_ms as f64 * backoff_factor.powi(attempt - 1)) as u64;
            let final_delay = exponential_delay.min(max_delay_ms);

            println!(
                "Retry attempt {}/{}, waiting {}ms",
                attempt + 1,
                max_retries,
                final_delay
            );
            sleep(Duration::from_millis(final_delay)).await;
        }

        let response = match CLIENT
            .post(url.clone())
            .header("Authorization", format!("Bearer {}", bearer_token))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("x-hdx-table", full_table_name)
            .header("x-hdx-transform", transform_name)
            .json(&sample_data)
            .send()
            .await
        {
            Ok(v) => v,
            Err(e) => {
                eprintln!(
                    "Request error on attempt {}: {} at {}.{}",
                    attempt + 1,
                    e,
                    file!(),
                    line!()
                );
                continue;
            }
        };

        if response.status().is_success() {
            if attempt > 0 {
                println!("Successfully inserted data after {} retries", attempt + 1);
            }
            return Ok(());
        }

        let status = response.status();
        let error_body = response.text().await.unwrap_or_default();

        let is_retryable = is_insert_retryable(status.as_u16(), &error_body);

        eprintln!(
            "Hydrolix insert failed on attempt {}/{}, status: {} (retryable: {}) url={url} {}.{}",
            attempt + 1,
            max_retries,
            status,
            is_retryable,
            file!(),
            line!()
        );
        eprintln!("Error response body: {error_body}");

        // Don't retry non-retryable errors
        if !is_retryable {
            return Err(format!(
                "ERROR: {}.{} Non-retryable error {} for {full_table_name}: {error_body}",
                file!(),
                line!(),
                status
            ));
        }

        // If this is the last attempt, don't continue
        if attempt == max_retries - 1 {
            break;
        }
    }

    Err(format!(
        "ERROR: {}.{} Failed to send data to {full_table_name} after {max_retries} attempts",
        file!(),
        line!()
    ))
}

/*
pub async fn insert_csv_into_table(
    bearer_token: &str,
    full_table_name: &str,
    transform_name: &str,
    sample_data: String,
) -> Result<(), String> {
    let url = format!("https://{BUNDLE_TESTING_CLUSTER}/ingest/event");

    for _i in 0..20 {
        sleep(Duration::from_secs(1)).await;

        let response = match CLIENT
            .post(url.clone())
            .header("Authorization", format!("Bearer {}", bearer_token))
            .header("Content-Type", "text/csv")
            .header("Accept", "application/json")
            .header("x-hdx-table", full_table_name)
            .header("x-hdx-transform", transform_name)
            .body(sample_data.clone())
            .send()
            .await
        {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Error: {e} at {}.{}", file!(), line!());
                continue;
            }
        };

        if response.status().is_success() {
            return Ok(());
        }

        if !response.status().is_success() {
            eprintln!(
                "Hydrolix insert failed, status: {} url={url} {}.{}",
                response.status(),
                file!(),
                line!()
            );
            let error_body = response.text().await.unwrap_or_default();
            eprintln!("Error response body: {error_body}");
            continue;
        }
    }
    Err(format!(
        "ERROR: {}.{} Failed to send data to {full_table_name}",
        file!(),
        line!()
    ))
}
*/

// Generate valid HDX table name alphanumeric-only table name from UUID (hdx tables must start with alpha)
#[allow(dead_code)]
pub fn create_table_name() -> String {
    let ending = Uuid::new_v4()
        .to_string()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .take(10)
        .collect::<String>();

    format!("testing_{ending}")
}

#[allow(dead_code)]
pub async fn get_list(bearer_token: &str, debug_mode: bool) -> Result<String, String> {
    let url = format!(
        "https://{}/config/v1/orgs/{}/projects/{}/tables",
        *BUNDLE_TESTING_CLUSTER,
        ORG_UUID,
        super::get_project_uuid()
    );

    if debug_mode {
        println!("DEBUG: {}.{} Hdx listing tables...", file!(), line!());
    }

    // Send the POST request
    let response = match CLIENT
        .get(url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/plain, */*")
        .timeout(Duration::from_secs(HTTP_TIMEOUT))
        .send()
        .await
    {
        Ok(v) => v,
        Err(e) => return Err(format!("ERROR: {}.{} {e}", file!(), line!())),
    };

    // Check if the request was successful
    if !response.status().is_success() {
        return Err(format!(
            "ERROR: {}.{} {}",
            file!(),
            line!(),
            response.status()
        ));
    }

    // Just grab the body in case we want to debug something
    match response.text().await {
        Ok(v) => Ok(v),
        Err(e) => Err(format!("ERROR: {}.{} {e}", file!(), line!())),
    }
}

#[allow(dead_code)]
pub async fn delete(bearer_token: &str, uuid: &str) -> Result<(), String> {
    let url = format!(
        "https://{}/config/v1/orgs/{}/projects/{}/tables/{uuid}",
        *BUNDLE_TESTING_CLUSTER,
        ORG_UUID,
        super::get_project_uuid()
    );

    // Send the DELETE
    let response = match CLIENT
        .delete(url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/plain, */*")
        .timeout(Duration::from_secs(HTTP_TIMEOUT))
        .send()
        .await
    {
        Ok(v) => v,
        Err(e) => return Err(format!("ERROR: {}.{} {e}", file!(), line!())),
    };

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!(
            "ERROR: {}.{} {}",
            file!(),
            line!(),
            response.status()
        ))
    }
}

/*
{
    "name": "my.summarytable",
    "description": "Minute-by-minute summary of parent table",
    "type": "summary",
    "settings": {
        "merge": {
            "enabled": true
        },
        "summary": {
            "enabled": true,
            "sql": "SELECT toStartOfMinute(timestamp) AS minute,
sum(cost) AS sum_cost, avg(tax) AS avg_tax, quantile(0.95)(distance)
AS distance_p95 FROM project.parent_table GROUP BY minute SETTINGS
hdx_primary_key='minute'"
        }
    }
}

*/

/// Execute a SQL statement against the cluster's query endpoint.
/// Returns the raw response body (TabSeparated format).
pub async fn query_sql(bearer_token: &str, sql: &str) -> Result<String, String> {
    let url = format!("https://{}/query", *BUNDLE_TESTING_CLUSTER);
    let body = format!("{} FORMAT TabSeparated", sql);

    let response = CLIENT
        .post(&url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .header("Content-Type", "text/plain")
        .timeout(Duration::from_secs(HTTP_TIMEOUT))
        .body(body)
        .send()
        .await
        .map_err(|e| format!("query_sql request failed url={url}: {e}"))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("query_sql read body failed url={url}: {e}"))?;

    if !status.is_success() {
        return Err(format!("query_sql HTTP {} url={url}: {}", status, text));
    }
    Ok(text)
}

/// Count rows in a table via the cluster's query endpoint.
pub async fn query_count(bearer_token: &str, full_table_name: &str) -> Result<u64, String> {
    let sql = format!("SELECT count() FROM {}", full_table_name);
    let body = query_sql(bearer_token, &sql).await?;
    body.trim().parse::<u64>().map_err(|e| {
        format!(
            "query_count could not parse '{}' as u64: {}",
            body.trim(),
            e
        )
    })
}

pub async fn create_summary(
    bearer_token: &str,
    table_name: &str,
    sql: &str,
) -> Result<String, String> {
    let payload = json!({
        "name": table_name,
         "type": "summary",
        "settings": {
            "summary": {
                "enabled": true,
                "sql": sql
            }
        }
    });

    let url = format!(
        "https://{}/config/v1/orgs/{}/projects/{}/tables",
        *BUNDLE_TESTING_CLUSTER,
        ORG_UUID,
        super::get_project_uuid()
    );

    // Send the POST request
    let response = match CLIENT
        .post(&url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/plain, */*")
        .timeout(Duration::from_secs(HTTP_TIMEOUT))
        .json(&payload)
        .send()
        .await
    {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} url={url} error={e}",
                file!(),
                line!()
            ))
        }
    };

    // Check if the request was successful
    if !response.status().is_success() {
        let status = response.status();
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Could not read error response".to_string());
        return Err(format!(
            "ERROR: {}.{} url={url} {} - Server response: {}",
            file!(),
            line!(),
            status,
            error_body
        ));
    }
    Ok(table_name.to_string())
}

fn is_insert_retryable(status: u16, error_body: &str) -> bool {
    // "unknown transform" is eventual-consistency lag after transform creation,
    // not a real 4xx — retry with backoff until the ingest endpoint sees it.
    if status == 400 && error_body.contains("unknown transform") {
        return true;
    }
    match status {
        400..=499 if status != 408 && status != 429 => false,
        500..=599 | 408 | 429 => true,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_transform_400_is_retryable() {
        let body = r#"{"code":400,"message":"unknown transform 'akamai'"}"#;
        assert!(is_insert_retryable(400, body));
    }

    #[test]
    fn other_400_is_not_retryable() {
        assert!(!is_insert_retryable(
            400,
            r#"{"code":400,"message":"bad input"}"#
        ));
        assert!(!is_insert_retryable(404, "not found"));
    }

    #[test]
    fn rate_limit_and_timeout_are_retryable() {
        assert!(is_insert_retryable(408, ""));
        assert!(is_insert_retryable(429, "rate limited"));
    }

    #[test]
    fn server_errors_are_retryable() {
        assert!(is_insert_retryable(500, ""));
        assert!(is_insert_retryable(503, ""));
    }
}
