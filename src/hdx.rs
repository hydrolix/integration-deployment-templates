use reqwest::Client;
use serde_json::{json, Value};
use tokio::time::sleep;
use tokio::time::Duration;
use uuid::Uuid;

// These are static but not secret
const ORG_UUID: &str = "d867bf48-4281-4496-8432-a93aa989aae6";
const PROJ_UUID: &str = "c7605c4b-9854-41c4-a210-b861d13e8bf4";
const PROJ_NAME: &str = "sample_project";

use crate::FOR_MARKETPLACE;

use crate::{BUNDLE_TESTING_CLUSTER, BUNDLE_TESTING_PASSWORD, BUNDLE_TESTING_USERNAME};

const HTTP_TIMEOUT: u64 = 120;

use lazy_static::lazy_static;

lazy_static! {
    static ref CLIENT: Client = reqwest::Client::new();
}

pub async fn get_auth_token() -> Result<String, String> {
    let url = format!("https://{}/config/v1/login", *BUNDLE_TESTING_CLUSTER);

    let response = match CLIENT
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "username": BUNDLE_TESTING_USERNAME.to_string(),
            "password": BUNDLE_TESTING_PASSWORD.to_string(),
        }))
        .send()
        .await
    {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} url={url} error={e}",
                file!(),
                line!()
            ));
        }
    };

    let json: serde_json::Value = match response.json().await {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Could not deserialize: {e}",
                file!(),
                line!()
            ));
        }
    };

    let token = json["auth_token"]["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();

    if token.is_empty() {
        Err(format!(
            "ERROR: {}.{} Could not find token in payload",
            file!(),
            line!()
        ))
    } else {
        Ok(token)
    }
}

pub async fn create_table(bearer_token: &str, table_name: &str) -> Result<String, String> {
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
        "https://{}/config/v1/orgs/{ORG_UUID}/projects/{PROJ_UUID}/tables",
        *BUNDLE_TESTING_CLUSTER
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
        return Err(format!(
            "ERROR: {}.{} {}",
            file!(),
            line!(),
            response.status()
        ));
    }

    // Just grab the body in case we want to debug something
    let table_data = match response.text().await {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} url={url} error={e}",
                file!(),
                line!()
            ));
        }
    };

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

pub async fn add_transform_to_table(
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
        "https://{}/config/v1/orgs/{ORG_UUID}/projects/{PROJ_UUID}/tables/{table_uuid}/transforms/",
        *BUNDLE_TESTING_CLUSTER
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

pub async fn insert_into_table(
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

        // Check if this is a retryable error
        let is_retryable = match status.as_u16() {
            // Client errors that shouldn't be retried
            400..=499 if status != 408 && status != 429 => true,
            // Server errors and rate limiting - retryable
            500..=599 | 408 | 429 => true,
            // Other status codes - retryable
            _ => true,
        };

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
pub fn create_table_name() -> String {
    let ending = Uuid::new_v4()
        .to_string()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .take(10)
        .collect::<String>();

    format!("testing_{ending}")
}

pub fn create_project_name() -> String {
    PROJ_NAME.to_string()
}

#[allow(dead_code)]
pub async fn get_table_list(bearer_token: &str, debug_mode: bool) -> Result<String, String> {
    let url = format!(
        "https://{}/config/v1/orgs/{ORG_UUID}/projects/{PROJ_UUID}/tables",
        *BUNDLE_TESTING_CLUSTER
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
pub async fn delete_a_table(bearer_token: &str, uuid: &str) -> Result<(), String> {
    let url = format!(
        "https://{}/config/v1/orgs/{ORG_UUID}/projects/{PROJ_UUID}/tables/{uuid}",
        *BUNDLE_TESTING_CLUSTER
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
