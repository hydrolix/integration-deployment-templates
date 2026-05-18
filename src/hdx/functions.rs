use serde_json::Value;

use crate::hdx::{HTTP_TIMEOUT, PROJ_NAME, PROJ_UUID};

#[allow(dead_code)]
/// Discover function files in the bundle directory
/// Scans functions/ and functions/.extracted/ for .json files
pub async fn discover(base_dir: &str) -> Result<Vec<String>, String> {
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

#[allow(dead_code)]
pub async fn create_and_check(
    bearer_token: &str,
    function_name: &str,
    base_dir: &str,
) -> Result<(), String> {
    println!("Checking function: {}...", function_name);

    let list_url = format!(
        "https://{}/config/v1/orgs/{}/projects/{}/functions/",
        super::cluster(), super::org_uuid(), PROJ_UUID
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
        super::cluster(), super::org_uuid(), PROJ_UUID
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
