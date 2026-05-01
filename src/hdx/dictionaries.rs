use serde_json::Value;

use crate::hdx::{HTTP_TIMEOUT, PROJ_NAME, PROJ_UUID};

#[allow(dead_code)]
/// Discover dictionary files in the bundle directory
/// Scans dictionaries/ and dictionaries/.extracted/ for .json files with matching data files
pub async fn discover(base_dir: &str) -> Result<Vec<String>, String> {
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

#[allow(dead_code)]
pub async fn create_and_check(
    bearer_token: &str,
    dictionary_name: &str,
    base_dir: &str,
) -> Result<(), String> {
    println!("Checking dictionary: {}...", dictionary_name);

    let list_url = format!(
        "https://{}/config/v1/orgs/{}/projects/{}/dictionaries/",
        super::cluster(), super::org_uuid(), PROJ_UUID
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

    upload_file(bearer_token, file_name, &data_file_content).await?;
    create_definition(bearer_token, dictionary_name, dict_def).await?;

    println!("  ✓ Created dictionary {}", dictionary_name);
    Ok(())
}

async fn upload_file(
    bearer_token: &str,
    file_name: &str,
    file_content: &str,
) -> Result<(), String> {
    let files_url = format!(
        "https://{}/config/v1/orgs/{}/projects/{}/dictionaries/files/",
        super::cluster(), super::org_uuid(), PROJ_UUID
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

async fn create_definition(
    bearer_token: &str,
    dictionary_name: &str,
    dict_definition: Value,
) -> Result<(), String> {
    let dict_url = format!(
        "https://{}/config/v1/orgs/{}/projects/{}/dictionaries/",
        super::cluster(), super::org_uuid(), PROJ_UUID
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
