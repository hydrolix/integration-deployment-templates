use lazy_static::lazy_static;
use reqwest::Client;
use serde_json::{json, Value};
use tokio::time::sleep;
use tokio::time::Duration;
use uuid::Uuid;

// These are static but not secret
const ORG_UUID: &str = "d867bf48-4281-4496-8432-a93aa989aae6"; // markeplace-dev
const PROJ_UUID: &str = "67e79a3c-f7d6-4b33-a207-fef4579a3152"; // markeplace-dev cdn_test_project
const PROJ_NAME: &str = "cdn_test_project";
const HTTP_TIMEOUT: u64 = 120;

// const ORG_UUID_SAND: &str = "b646d78a-5fb2-4d5f-afef-b705bf185174";  // partnersandbox
// const PROJ_UUID_SAND: &str = "469dbd34-6f06-4dfe-8fd1-9adf82123ecf";  // partnersandbox

lazy_static! {
    static ref CLIENT: Client = reqwest::Client::new();
    static ref BUNDLE_TESTING_CLUSTER: String =
        std::env::var("BUNDLE_TESTING_CLUSTER").unwrap_or_else(|_| "".to_string());
    static ref BUNDLE_TESTING_USERNAME: String =
        std::env::var("BUNDLE_TESTING_USERNAME").unwrap_or_else(|_| "".to_string());
    static ref BUNDLE_TESTING_PASSWORD: String =
        std::env::var("BUNDLE_TESTING_PASSWORD").unwrap_or_else(|_| "".to_string());
    static ref FOR_MARKETPLACE: bool = {
        let args: Vec<String> = std::env::args().collect();
        args.contains(&"--marketplace".to_string())
    };
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

    let status = response.status();
    let json: serde_json::Value = match response.json().await {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Could not deserialize response (HTTP {}): {e}",
                file!(),
                line!(),
                status
            ));
        }
    };

    // Debug: print what we got back
    if !status.is_success() {
        return Err(format!(
            "ERROR: {}.{} Authentication failed (HTTP {}): {}",
            file!(),
            line!(),
            status,
            serde_json::to_string_pretty(&json).unwrap_or_else(|_| format!("{:?}", json))
        ));
    }

    let token = json["auth_token"]["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();

    if token.is_empty() {
        Err(format!(
            "ERROR: {}.{} Could not find token in payload. Response was: {}",
            file!(),
            line!(),
            serde_json::to_string_pretty(&json).unwrap_or_else(|_| format!("{:?}", json))
        ))
    } else {
        Ok(token)
    }
}

#[allow(dead_code)]
/// Ensure a zip file is extracted to the .extracted/ directory
/// Uses unzip -j flag to flatten directory structure
pub async fn ensure_zip_extracted(
    base_dir: &str,
    zip_file_name: &str,
    target_folder: &str,
) -> Result<(), String> {
    use tokio::fs;
    use tokio::process::Command;

    let zip_path = format!("{}/{}/{}", base_dir, target_folder, zip_file_name);
    let extract_dir = format!("{}/{}/.extracted", base_dir, target_folder);

    // Check if zip exists
    if fs::metadata(&zip_path).await.is_err() {
        // No zip file - that's okay, might have local files
        return Ok(());
    }

    // Check if already extracted
    if fs::metadata(&extract_dir).await.is_ok() {
        println!("  ✓ {} already extracted", zip_file_name);
        return Ok(());
    }

    println!("  Extracting {}...", zip_file_name);

    // Create extraction directory
    fs::create_dir_all(&extract_dir)
        .await
        .map_err(|e| format!("Failed to create extraction directory: {}", e))?;

    // Use -j flag to flatten directory structure (strip paths)
    let output = Command::new("unzip")
        .args(["-j", "-q", "-o", &zip_path, "-d", &extract_dir])
        .output()
        .await
        .map_err(|e| format!("Failed to run unzip command: {}", e))?;

    if !output.status.success() {
        let error_text = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Unzip failed: {}", error_text));
    }

    println!("  ✓ Extracted {} to .extracted/", zip_file_name);
    Ok(())
}

#[allow(dead_code)]
/// Discover dictionary files in the bundle directory
/// Scans dictionaries/ and dictionaries/.extracted/ for .json files with matching data files
pub async fn discover_dictionaries(base_dir: &str) -> Result<Vec<String>, String> {
    use tokio::fs;

    let mut discovered: Vec<String> = Vec::new();

    // Check .extracted/ (flattened) and root dictionaries/
    let possible_dirs = vec![
        format!("{}/dictionaries/.extracted", base_dir),
        format!("{}/dictionaries", base_dir),
    ];

    for dir in possible_dirs {
        // Check if directory exists
        if fs::metadata(&dir).await.is_err() {
            continue; // Directory doesn't exist, skip
        }

        let dir_display = dir
            .split('/')
            .rev()
            .take(2)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("/");
        println!("  Scanning for dictionaries in {}...", dir_display);

        let mut entries = match fs::read_dir(&dir).await {
            Ok(e) => e,
            Err(_) => continue,
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let file_name = entry.file_name().to_string_lossy().to_string();

            if !file_name.ends_with(".json") {
                continue;
            }

            let base_name = file_name.trim_end_matches(".json");

            // Skip if already found (avoid duplicates)
            if discovered.contains(&base_name.to_string()) {
                continue;
            }

            // Check if matching data file exists
            let possible_extensions = vec!["csv", "yaml", "yml", "tsv"];
            for ext in possible_extensions {
                let data_file = format!("{}/{}.{}", dir, base_name, ext);
                if fs::metadata(&data_file).await.is_ok() {
                    discovered.push(base_name.to_string());
                    println!("    Found: {} (.json + .{})", base_name, ext);
                    break;
                }
            }
        }
    }

    Ok(discovered)
}

#[allow(dead_code)]
/// Discover function files in the bundle directory
/// Scans functions/ and functions/.extracted/ for .json files
pub async fn discover_functions(base_dir: &str) -> Result<Vec<String>, String> {
    use tokio::fs;

    let mut discovered: Vec<String> = Vec::new();

    // Check .extracted/ (flattened) and root functions/
    let possible_dirs = vec![
        format!("{}/functions/.extracted", base_dir),
        format!("{}/functions", base_dir),
    ];

    for dir in possible_dirs {
        // Check if directory exists
        if fs::metadata(&dir).await.is_err() {
            continue; // Directory doesn't exist, skip
        }

        let dir_display = dir
            .split('/')
            .rev()
            .take(2)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("/");
        println!("  Scanning for functions in {}...", dir_display);

        let mut entries = match fs::read_dir(&dir).await {
            Ok(e) => e,
            Err(_) => continue,
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let file_name = entry.file_name().to_string_lossy().to_string();

            if !file_name.ends_with(".json") {
                continue;
            }

            let base_name = file_name.trim_end_matches(".json");

            // Skip if already found (avoid duplicates)
            if !discovered.contains(&base_name.to_string()) {
                discovered.push(base_name.to_string());
                println!("    Found: {}", base_name);
            }
        }
    }

    Ok(discovered)
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

pub async fn verify_table_exists(bearer_token: &str, table_name: &str) -> Result<(), String> {
    let url = format!(
        "https://{}/config/v1/orgs/{ORG_UUID}/projects/{PROJ_UUID}/tables",
        *BUNDLE_TESTING_CLUSTER
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

pub async fn create_summary_table(
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
            400..=499 if status != 408 && status != 429 => false,
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

// ============================================================================
// BUNDLE-SPECIFIC FUNCTIONS (for sample_project)
// ============================================================================

pub async fn check_and_create_function(
    bearer_token: &str,
    function_name: &str,
    base_dir: &str,
) -> Result<(), String> {
    println!("Checking function: {}...", function_name);

    let list_url = format!(
        "https://{}/config/v1/orgs/{}/projects/{}/functions/",
        *BUNDLE_TESTING_CLUSTER, ORG_UUID, PROJ_UUID
    );
    let expected_name = format!("{}_{}", PROJ_NAME, function_name);

    let client = reqwest::Client::new();
    match client
        .get(&list_url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .send()
        .await
    {
        Ok(list_response) if list_response.status().is_success() => {
            if let Ok(response_data) = list_response.json::<Value>().await {
                let empty_vec = vec![];
                let existing = if response_data.is_array() {
                    response_data.as_array().unwrap()
                } else if let Some(results) = response_data.get("results") {
                    results.as_array().unwrap_or(&empty_vec)
                } else if let Some(functions) = response_data.get("functions") {
                    functions.as_array().unwrap_or(&empty_vec)
                } else if let Some(data) = response_data.get("data") {
                    data.as_array().unwrap_or(&empty_vec)
                } else {
                    &empty_vec
                };

                for func in existing {
                    if let Some(name) = func.get("name").and_then(|n| n.as_str()) {
                        if name == function_name || name == expected_name {
                            println!(
                                "  ✓ Function {} already exists (as {})",
                                function_name, expected_name
                            );
                            return Ok(());
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("  ⚠️  Could not check for existing function: {}", e);
        }
        _ => {}
    }

    // Look for function file
    let function_file_path = format!("{}/functions/{}.json", base_dir, function_name);

    if tokio::fs::metadata(&function_file_path).await.is_err() {
        return Err(format!(
            "Bundle-specific function '{}' declared but file not found.\n  Expected: {}\n  Actions:\n    1. Add {}.json to functions/ folder, OR\n    2. Remove '{}' from required_functions in bundle.json if not needed",
            function_name, function_file_path, function_name, function_name
        ));
    }

    let content = tokio::fs::read_to_string(&function_file_path)
        .await
        .map_err(|e| format!("Failed to read function file {}: {}", function_file_path, e))?;

    let mut function_def: Value = serde_json::from_str(&content).map_err(|e| {
        format!(
            "Failed to parse function JSON {}: {}",
            function_file_path, e
        )
    })?;

    // Replace __PROJECT_NAME__ in function SQL
    if let Some(sql) = function_def.get_mut("sql") {
        if let Some(sql_str) = sql.as_str() {
            let replaced = sql_str.replace("__PROJECT_NAME__", PROJ_NAME);
            *sql = Value::String(replaced);
        }
    }

    let create_url = format!(
        "https://{}/config/v1/orgs/{}/projects/{}/functions/",
        *BUNDLE_TESTING_CLUSTER, ORG_UUID, PROJ_UUID
    );

    println!(
        "  Creating function {} (will become {})...",
        function_name, expected_name
    );

    // Merge function_name into the definition
    if let Some(obj) = function_def.as_object_mut() {
        obj.insert("name".to_string(), Value::String(function_name.to_string()));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let response = client
        .post(&create_url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&function_def)
        .send()
        .await
        .map_err(|e| format!("Failed to create function: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("HTTP {}: {}", status, error_text));
    }

    println!("  ✓ Created function {}", function_name);
    Ok(())
}

// ============================================================================
// BUNDLE-SPECIFIC DICTIONARIES (for sample_project)
// ============================================================================

async fn find_dictionary_files(
    base_dir: &str,
    dictionary_name: &str,
) -> Result<(String, String), String> {
    let search_paths = vec![
        format!("{}/dictionaries", base_dir),
        format!("{}/dictionaries/.extracted", base_dir),
    ];

    for dir in search_paths {
        let json_path = format!("{}/{}.json", dir, dictionary_name);

        if tokio::fs::metadata(&json_path).await.is_ok() {
            let possible_extensions = vec!["csv", "yaml", "yml", "tsv"];
            for ext in possible_extensions {
                let data_path = format!("{}/{}.{}", dir, dictionary_name, ext);
                if tokio::fs::metadata(&data_path).await.is_ok() {
                    return Ok((json_path, data_path));
                }
            }

            return Err(format!("Found {} but no matching data file", json_path));
        }
    }

    Err(format!(
        "Bundle-specific dictionary '{}' declared but files not found.\n  Expected:\n    - {}/dictionaries/{}.json (definition)\n    - {}/dictionaries/{}.[csv/yaml/yml/tsv] (data)\n  Actions:\n    1. Add {}.json + data file to dictionaries/ folder, OR\n    2. Check if files exist in dictionaries.zip, OR\n    3. Remove '{}' from required_dictionaries in bundle.json if not needed",
        dictionary_name, base_dir, dictionary_name, base_dir, dictionary_name, dictionary_name, dictionary_name
    ))
}

pub async fn check_and_create_dictionary(
    bearer_token: &str,
    dictionary_name: &str,
    base_dir: &str,
) -> Result<(), String> {
    println!("Checking dictionary: {}...", dictionary_name);

    let list_url = format!(
        "https://{}/config/v1/orgs/{}/projects/{}/dictionaries/",
        *BUNDLE_TESTING_CLUSTER, ORG_UUID, PROJ_UUID
    );
    let expected_name = format!("{}_{}", PROJ_NAME, dictionary_name);

    let client = reqwest::Client::new();
    match client
        .get(&list_url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .send()
        .await
    {
        Ok(list_response) if list_response.status().is_success() => {
            if let Ok(response_data) = list_response.json::<Value>().await {
                let empty_vec = vec![];
                let existing = if response_data.is_array() {
                    response_data.as_array().unwrap()
                } else if let Some(results) = response_data.get("results") {
                    results.as_array().unwrap_or(&empty_vec)
                } else if let Some(dictionaries) = response_data.get("dictionaries") {
                    dictionaries.as_array().unwrap_or(&empty_vec)
                } else if let Some(data) = response_data.get("data") {
                    data.as_array().unwrap_or(&empty_vec)
                } else {
                    &empty_vec
                };

                for dict in existing {
                    if let Some(name) = dict.get("name").and_then(|n| n.as_str()) {
                        if name == dictionary_name || name == expected_name {
                            println!(
                                "  ✓ Dictionary {} already exists (as {})",
                                dictionary_name, expected_name
                            );
                            return Ok(());
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("  ⚠️  Could not check for existing dictionary: {}", e);
        }
        _ => {}
    }

    // Find dictionary files
    let files = find_dictionary_files(base_dir, dictionary_name).await?;

    println!("  Found files: {} + {}", files.0, files.1);

    let content = tokio::fs::read_to_string(&files.0)
        .await
        .map_err(|e| format!("Failed to read dictionary definition {}: {}", files.0, e))?;

    let dict_def: Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse dictionary JSON {}: {}", files.0, e))?;

    let data_file_content = tokio::fs::read_to_string(&files.1)
        .await
        .map_err(|e| format!("Failed to read dictionary data file {}: {}", files.1, e))?;

    let file_name = files
        .1
        .split('/')
        .next_back()
        .ok_or("Invalid data file path")?;

    upload_dictionary_file(bearer_token, file_name, &data_file_content).await?;
    create_dictionary_definition(bearer_token, dictionary_name, dict_def).await?;

    println!("  ✓ Created dictionary {}", dictionary_name);
    Ok(())
}

async fn upload_dictionary_file(
    bearer_token: &str,
    file_name: &str,
    file_content: &str,
) -> Result<(), String> {
    let files_url = format!(
        "https://{}/config/v1/orgs/{}/projects/{}/dictionaries/files/",
        *BUNDLE_TESTING_CLUSTER, ORG_UUID, PROJ_UUID
    );

    let base_file_name = file_name
        .trim_end_matches(".csv")
        .trim_end_matches(".yaml")
        .trim_end_matches(".yml")
        .trim_end_matches(".tsv");

    let client = reqwest::Client::new();

    // Check if file already exists
    if let Ok(files_list_response) = client
        .get(&files_url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .send()
        .await
    {
        if files_list_response.status().is_success() {
            if let Ok(existing_files) = files_list_response.json::<Value>().await {
                if let Some(arr) = existing_files.as_array() {
                    for file in arr {
                        let name = if file.is_string() {
                            file.as_str().unwrap_or("")
                        } else {
                            file.get("name").and_then(|n| n.as_str()).unwrap_or("")
                        };

                        if name == base_file_name || name == file_name {
                            println!("  ✓ Dictionary file already uploaded: {}", file_name);
                            return Ok(());
                        }
                    }
                }
            }
        }
    }

    let ext = file_name
        .split('.')
        .next_back()
        .unwrap_or("csv")
        .to_lowercase();
    let mime_type = if ext == "yaml" || ext == "yml" {
        "application/x-yaml"
    } else {
        "text/csv"
    };

    println!(
        "  Uploading dictionary file: {} (as {})...",
        file_name, base_file_name
    );

    let form = reqwest::multipart::Form::new()
        .text("name", base_file_name.to_string())
        .part(
            "file",
            reqwest::multipart::Part::bytes(file_content.as_bytes().to_vec())
                .file_name(file_name.to_string())
                .mime_str(mime_type)
                .map_err(|e| format!("Failed to create multipart: {}", e))?,
        );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let upload_response = client
        .post(&files_url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Failed to upload dictionary file: {}", e))?;

    if !upload_response.status().is_success() {
        let status = upload_response.status();
        let error_text = upload_response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!(
            "Failed to upload (HTTP {}): {}",
            status, error_text
        ));
    }

    println!("  ✓ Uploaded dictionary file: {}", base_file_name);
    Ok(())
}

async fn create_dictionary_definition(
    bearer_token: &str,
    dictionary_name: &str,
    dict_definition: Value,
) -> Result<(), String> {
    let dict_url = format!(
        "https://{}/config/v1/orgs/{}/projects/{}/dictionaries/",
        *BUNDLE_TESTING_CLUSTER, ORG_UUID, PROJ_UUID
    );

    let expected_name = format!("{}_{}", PROJ_NAME, dictionary_name);

    let mut payload = dict_definition.clone();
    if let Some(obj) = payload.as_object_mut() {
        obj.insert(
            "name".to_string(),
            Value::String(dictionary_name.to_string()),
        );
    }

    println!(
        "  Creating dictionary definition: {} (will become {})...",
        dictionary_name, expected_name
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let dict_response = client
        .post(&dict_url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to create dictionary definition: {}", e))?;

    if !dict_response.status().is_success() {
        let status = dict_response.status();
        let error_text = dict_response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("HTTP {}: {}", status, error_text));
    }

    println!("  ✓ Created dictionary definition");
    Ok(())
}
