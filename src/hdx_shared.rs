// Shared resource management for commons project
// Handles functions and dictionaries that are shared across all bundles

use lazy_static::lazy_static;
use serde_json::Value;
use std::env;
use tokio::fs;

use crate::BUNDLE_TESTING_CLUSTER;

// const ORG_UUID_SAND: &str = "b646d78a-5fb2-4d5f-afef-b705bf185174";  // partnersandbox
const ORG_UUID: &str = "d867bf48-4281-4496-8432-a93aa989aae6"; // markeplace-dev
const HTTP_TIMEOUT_SECS: u64 = 120;

lazy_static! {
    static ref SHARED_PROJECT_NAME: String =
        env::var("SHARED_PROJECT_NAME").unwrap_or_else(|_| "commons".to_string());
    static ref IS_LOCAL: bool = {
        let args: Vec<String> = env::args().collect();
        args.contains(&"--local".to_string())
            || args.contains(&"--local-dashboard-only".to_string())
    };
}

// ============================================================================
// SHARED PROJECT CONTEXT
// ============================================================================

pub struct SharedProject {
    project_uuid: String,
    pub dicts: Vec<String>,
    pub funcs: Vec<String>,
}

impl SharedProject {
    pub async fn new(
        bearer_token: &str,
        funcs: Vec<String>,
        dicts: Vec<String>,
    ) -> Result<Self, String> {
        println!("Checking for shared project: {}...", *SHARED_PROJECT_NAME);

        let list_url = format!(
            "https://{}/config/v1/orgs/{}/projects/",
            *BUNDLE_TESTING_CLUSTER, ORG_UUID
        );

        let client = reqwest::Client::new();
        eprintln!("DEBUG: Requesting projects list from: {}", list_url);
        let response = match client
            .get(&list_url)
            .header("Authorization", format!("Bearer {}", bearer_token))
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                return Err(format!(
                    "Failed to list projects from URL {}: {}",
                    list_url, e
                ))
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Could not read error response".to_string());
            return Err(format!(
                "failed to list projects: {} - URL: {} - Response: {}",
                status, list_url, error_body
            ));
        }

        let projects: Value = match response.json().await {
            Ok(p) => p,
            Err(e) => return Err(format!("failed to parse projects response: {}", e)),
        };

        // Try multiple possible response structures
        let empty_vec = vec![];
        let existing = if projects.is_array() {
            projects.as_array().unwrap()
        } else if let Some(results) = projects.get("results") {
            results.as_array().unwrap_or(&empty_vec)
        } else if let Some(projects_data) = projects.get("projects") {
            projects_data.as_array().unwrap_or(&empty_vec)
        } else if let Some(data) = projects.get("data") {
            data.as_array().unwrap_or(&empty_vec)
        } else {
            &empty_vec
        };

        // Look for existing shared project
        for project in existing {
            if let Some(name) = project.get("name").and_then(|n| n.as_str()) {
                if name == *SHARED_PROJECT_NAME {
                    if let Some(uuid) = project.get("uuid").and_then(|u| u.as_str()) {
                        let project_uuid = uuid.to_string();
                        println!("  ✓ Shared project exists (uuid: {})", project_uuid);
                        return Ok(Self {
                            project_uuid,
                            funcs,
                            dicts,
                        });
                    }
                }
            }
        }

        // Project doesn't exist - create it
        println!("  Creating shared project: {}...", *SHARED_PROJECT_NAME);
        let project_uuid = create_shared_project(bearer_token).await?;
        println!("  ✓ Created shared project (uuid: {})", project_uuid);

        Ok(Self {
            project_uuid,
            funcs,
            dicts,
        })
    }

    pub fn project_uuid(&self) -> &str {
        &self.project_uuid
    }
}

async fn create_shared_project(bearer_token: &str) -> Result<String, String> {
    let create_url = format!(
        "https://{}/config/v1/orgs/{}/projects/",
        *BUNDLE_TESTING_CLUSTER, ORG_UUID
    );

    let payload = serde_json::json!({
        "name": *SHARED_PROJECT_NAME,
        "description": "Shared resources for all bundles (functions, dictionaries)",
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let response = client
        .post(&create_url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to create shared project: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("HTTP {}: {}", status, error_text));
    }

    let result: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse create response: {}", e))?;

    result
        .get("uuid")
        .and_then(|u| u.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "No UUID in response".to_string())
}

// ============================================================================
// SHARED FUNCTIONS
// ============================================================================

pub async fn check_and_create_shared_function(
    ctx: &SharedProject,
    bearer_token: &str,
    function_name: &str,
    base_dir: &str,
) -> Result<(), String> {
    println!("Checking shared function: {}...", function_name);

    let project_uuid = ctx.project_uuid();

    let list_url = format!(
        "https://{}/config/v1/orgs/{}/projects/{}/functions/",
        *BUNDLE_TESTING_CLUSTER, ORG_UUID, project_uuid
    );

    let expected_name = format!("{}_{}", *SHARED_PROJECT_NAME, function_name);

    let client = reqwest::Client::new();
    let list_response = client
        .get(&list_url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .send()
        .await
        .map_err(|e| format!("Failed to list functions before creating: {}", e))?;

    if list_response.status().is_success() {
        let response_data: Value = list_response
            .json()
            .await
            .map_err(|e| format!("Failed to parse functions response: {}", e))?;

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
                if name == function_name {
                    println!(
                        "  ✓ Shared function {} exists (as {})",
                        function_name, expected_name
                    );
                    return Ok(());
                }
            }
        }
    } else {
        return Err(format!(
            "Failed to list functions: {} {}",
            list_response.status(),
            list_response.status().canonical_reason().unwrap_or("")
        ));
    }

    // Create the function
    let function_file_path = format!("{}/functions/{}.json", base_dir, function_name);

    if fs::metadata(&function_file_path).await.is_err() {
        return Err(format!(
            "Shared function '{}' declared but file not found.\n  Expected: {}\n  Actions:\n    1. Add {}.json to functions/ folder, OR\n    2. Remove '{}' from shared_functions in bundle.json if not needed",
            function_name, function_file_path, function_name, function_name
        ));
    }

    let content = fs::read_to_string(&function_file_path)
        .await
        .map_err(|e| format!("Failed to read shared function file: {}", e))?;

    let mut function_def: Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse shared function JSON: {}", e))?;

    // Replace template variables
    if let Some(sql) = function_def.get_mut("sql") {
        if let Some(sql_str) = sql.as_str() {
            let replaced = sql_str
                .replace("__SHARED_PROJECT__", &SHARED_PROJECT_NAME)
                .replace("__PROJECT_NAME__", &SHARED_PROJECT_NAME); // Fallback
            *sql = Value::String(replaced);
        }
    }

    let create_url = format!(
        "https://{}/config/v1/orgs/{}/projects/{}/functions/",
        *BUNDLE_TESTING_CLUSTER, ORG_UUID, project_uuid
    );

    println!(
        "  Creating shared function {} (will become {})...",
        function_name, expected_name
    );

    // Merge function_name into the definition
    if let Some(obj) = function_def.as_object_mut() {
        obj.insert("name".to_string(), Value::String(function_name.to_string()));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let response = client
        .post(&create_url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .header("Content-Type", "application/json")
        .json(&function_def)
        .send()
        .await
        .map_err(|e| format!("Failed to create shared function: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("HTTP {}: {}", status, error_text));
    }

    println!("  ✓ Created shared function {}", function_name);
    Ok(())
}

// ============================================================================
// SHARED DICTIONARIES
// ============================================================================

pub async fn check_and_create_shared_dictionary(
    ctx: &SharedProject,
    bearer_token: &str,
    dictionary_name: &str,
    base_dir: &str,
) -> Result<(), String> {
    println!("Checking shared dictionary: {}...", dictionary_name);

    let project_uuid = ctx.project_uuid();

    let list_url = format!(
        "https://{}/config/v1/orgs/{}/projects/{}/dictionaries/",
        *BUNDLE_TESTING_CLUSTER, ORG_UUID, project_uuid
    );

    let expected_name = format!("{}_{}", *SHARED_PROJECT_NAME, dictionary_name);

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
                        if name == dictionary_name {
                            println!(
                                "  ✓ Shared dictionary {} exists (as {})",
                                dictionary_name, expected_name
                            );
                            return Ok(());
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!(
                "  ⚠️  Could not check for existing shared dictionary: {}",
                e
            );
        }
        _ => {}
    }

    // Create the dictionary
    let files = find_dictionary_files(base_dir, dictionary_name).await?;

    println!("  Found files: {} + {}", files.0, files.1);

    let content = fs::read_to_string(&files.0)
        .await
        .map_err(|e| format!("Failed to read shared dictionary definition: {}", e))?;

    let dict_def: Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse shared dictionary JSON: {}", e))?;

    let data_file_content = fs::read_to_string(&files.1)
        .await
        .map_err(|e| format!("Failed to read dictionary data file: {}", e))?;

    let file_name = files
        .1
        .split('/')
        .next_back()
        .ok_or("Invalid data file path")?;

    upload_shared_dictionary_file(project_uuid, bearer_token, file_name, &data_file_content)
        .await?;
    create_shared_dictionary_definition(project_uuid, bearer_token, dictionary_name, dict_def)
        .await?;

    println!("  ✓ Created shared dictionary {}", dictionary_name);
    Ok(())
}

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

        if fs::metadata(&json_path).await.is_ok() {
            let possible_extensions = vec!["csv", "yaml", "yml", "tsv"];
            for ext in possible_extensions {
                let data_path = format!("{}/{}.{}", dir, dictionary_name, ext);
                if fs::metadata(&data_path).await.is_ok() {
                    return Ok((json_path, data_path));
                }
            }

            return Err(format!("Found {} but no matching data file", json_path));
        }
    }

    Err(format!(
        "Shared dictionary '{}' declared but files not found.\n  Expected:\n    - {}/dictionaries/{}.json (definition)\n    - {}/dictionaries/{}.[csv/yaml/yml/tsv] (data)\n  Actions:\n    1. Add {}.json + data file to dictionaries/ folder, OR\n    2. Check if files exist in dictionaries.zip, OR\n    3. Remove '{}' from shared_dictionaries in bundle.json if not needed",
        dictionary_name, base_dir, dictionary_name, base_dir, dictionary_name, dictionary_name, dictionary_name
    ))
}

async fn upload_shared_dictionary_file(
    project_uuid: &str,
    bearer_token: &str,
    file_name: &str,
    file_content: &str,
) -> Result<(), String> {
    let files_url = format!(
        "https://{}/config/v1/orgs/{}/projects/{}/dictionaries/files/",
        *BUNDLE_TESTING_CLUSTER, ORG_UUID, project_uuid
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
                            println!("  ✓ Shared dictionary file already uploaded: {}", file_name);
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
        "  Uploading shared dictionary file: {} (as {})...",
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
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let upload_response = client
        .post(&files_url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Failed to upload shared dictionary file: {}", e))?;

    if !upload_response.status().is_success() {
        let error_text = upload_response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Failed to upload: {}", error_text));
    }

    println!("  ✓ Uploaded shared dictionary file: {}", base_file_name);
    Ok(())
}

async fn create_shared_dictionary_definition(
    project_uuid: &str,
    bearer_token: &str,
    dictionary_name: &str,
    dict_definition: Value,
) -> Result<(), String> {
    let dict_url = format!(
        "https://{}/config/v1/orgs/{}/projects/{}/dictionaries/",
        *BUNDLE_TESTING_CLUSTER, ORG_UUID, project_uuid
    );

    let expected_name = format!("{}_{}", *SHARED_PROJECT_NAME, dictionary_name);

    let mut payload = dict_definition.clone();
    if let Some(obj) = payload.as_object_mut() {
        obj.insert(
            "name".to_string(),
            Value::String(dictionary_name.to_string()),
        );
    }

    println!(
        "  Creating shared dictionary definition: {} (will become {})...",
        dictionary_name, expected_name
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let dict_response = client
        .post(&dict_url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to create shared dictionary definition: {}", e))?;

    if !dict_response.status().is_success() {
        let status = dict_response.status();
        let error_text = dict_response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("HTTP {}: {}", status, error_text));
    }

    println!("  ✓ Created shared dictionary definition");
    Ok(())
}

pub fn get_shared_project_name() -> String {
    SHARED_PROJECT_NAME.clone()
}
