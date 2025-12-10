// Check if required functions and dictionaries exist in Hydrolix (Production Mode)

use serde_json::Value;
use std::collections::HashSet;
use tokio::fs;

use crate::bundle_struct::Bundle;
use crate::BUNDLE_TESTING_CLUSTER;

const ORG_UUID: &str = "d867bf48-4281-4496-8432-a93aa989aae6";  // markeplace-dev
const ORG_UUID_SAND: &str = "b646d78a-5fb2-4d5f-afef-b705bf185174";  // partnersandbox
const PROJ_UUID: &str = "67e79a3c-f7d6-4b33-a207-fef4579a3152";  // markeplace-dev cdn_test_project
const PROJ_UUID_SAND: &str = "469dbd34-6f06-4dfe-8fd1-9adf82123ecf";  // partnersandbox
const PROJ_NAME: &str = "cdn_test_project";

pub async fn check_dependencies_exist(
    bearer_token: &str,
    bundle: &Bundle,
    base_dir: &str,
) -> Result<(), String> {
    let mut missing_functions: Vec<String> = vec![];
    let mut missing_dictionaries: Vec<String> = vec![];
    let mut missing_files: Vec<String> = vec![];

    // Check functions
    if let Some(deps) = &bundle.dependencies {
        if let Some(hydrolix) = &deps.hydrolix {
            if let Some(required_functions) = &hydrolix.required_functions {
                let functions_url = format!(
                    "https://{}/config/v1/orgs/{}/projects/{}/functions/",
                    *BUNDLE_TESTING_CLUSTER, ORG_UUID, PROJ_UUID
                );

                let client = reqwest::Client::new();
                match client
                    .get(&functions_url)
                    .header("Authorization", format!("Bearer {}", bearer_token))
                    .send()
                    .await
                {
                    Ok(response) if response.status().is_success() => {
                        let response_data: Value = response
                            .json()
                            .await
                            .map_err(|e| format!("Failed to parse functions response: {}", e))?;

                        let empty_vec = vec![];
                        let existing_functions = if response_data.is_array() {
                            response_data.as_array().unwrap()
                        } else if let Some(functions) = response_data.get("functions") {
                            functions.as_array().unwrap_or(&empty_vec)
                        } else if let Some(data) = response_data.get("data") {
                            data.as_array().unwrap_or(&empty_vec)
                        } else {
                            &empty_vec
                        };

                        let existing_names: HashSet<String> = existing_functions
                            .iter()
                            .filter_map(|f| f.get("name").and_then(|n| n.as_str()))
                            .map(|s| s.to_string())
                            .collect();

                        for function_name in required_functions {
                            let full_name = format!("{}_{}", PROJ_NAME, function_name);

                            if !existing_names.contains(&full_name) {
                                missing_functions.push(function_name.clone());
                            }

                            let file_path =
                                format!("{}/functions/{}.json", base_dir, function_name);
                            if fs::metadata(&file_path).await.is_err() {
                                missing_files.push(format!("functions/{}.json", function_name));
                            }
                        }
                    }
                    Ok(response) => {
                        return Err(format!(
                            "Failed to list functions: {}: {:?}",
                            response.status(),
                            response
                        ));
                    }
                    Err(e) => {
                        return Err(format!("Failed to check functions: {}", e));
                    }
                }
            }
        }
    }

    // Check dictionaries
    if let Some(deps) = &bundle.dependencies {
        if let Some(hydrolix) = &deps.hydrolix {
            if let Some(required_dictionaries) = &hydrolix.required_dictionaries {
                let dicts_url = format!(
                    "https://{}/config/v1/orgs/{}/projects/{}/dictionaries/",
                    *BUNDLE_TESTING_CLUSTER, ORG_UUID, PROJ_UUID
                );

                let client = reqwest::Client::new();
                match client
                    .get(&dicts_url)
                    .header("Authorization", format!("Bearer {}", bearer_token))
                    .send()
                    .await
                {
                    Ok(response) if response.status().is_success() => {
                        let response_data: Value = response
                            .json()
                            .await
                            .map_err(|e| format!("Failed to parse dictionaries response: {}", e))?;

                        let empty_vec = vec![];
                        let existing_dicts = if response_data.is_array() {
                            response_data.as_array().unwrap()
                        } else if let Some(dictionaries) = response_data.get("dictionaries") {
                            dictionaries.as_array().unwrap_or(&empty_vec)
                        } else if let Some(data) = response_data.get("data") {
                            data.as_array().unwrap_or(&empty_vec)
                        } else {
                            &empty_vec
                        };

                        let existing_names: HashSet<String> = existing_dicts
                            .iter()
                            .filter_map(|d| d.get("name").and_then(|n| n.as_str()))
                            .map(|s| s.to_string())
                            .collect();

                        for dictionary_name in required_dictionaries {
                            let full_name = format!("{}_{}", PROJ_NAME, dictionary_name);

                            if !existing_names.contains(&full_name) {
                                missing_dictionaries.push(dictionary_name.clone());
                            }

                            let json_path =
                                format!("{}/dictionaries/{}.json", base_dir, dictionary_name);
                            if fs::metadata(&json_path).await.is_err() {
                                missing_files
                                    .push(format!("dictionaries/{}.json", dictionary_name));
                            }

                            let possible_extensions = vec!["csv", "yaml", "yml", "tsv"];
                            let mut found_data_file = false;
                            for ext in possible_extensions {
                                let data_path = format!(
                                    "{}/dictionaries/{}.{}",
                                    base_dir, dictionary_name, ext
                                );
                                if fs::metadata(&data_path).await.is_ok() {
                                    found_data_file = true;
                                    break;
                                }
                            }
                            if !found_data_file {
                                missing_files.push(format!(
                                    "dictionaries/{}.[csv/yaml/yml/tsv]",
                                    dictionary_name
                                ));
                            }
                        }
                    }
                    Ok(response) => {
                        return Err(format!(
                            "Failed to list dictionaries: {}",
                            response.status()
                        ));
                    }
                    Err(e) => {
                        return Err(format!("Failed to check dictionaries: {}", e));
                    }
                }
            }
        }
    }

    // Report results
    let mut errors: Vec<String> = vec![];

    if !missing_functions.is_empty() {
        errors.push("\n❌ Missing functions on cluster:".to_string());
        for name in &missing_functions {
            errors.push(format!(
                "   - {} (expected as: {}_{})",
                name, PROJ_NAME, name
            ));
        }
    }

    if !missing_dictionaries.is_empty() {
        errors.push("\n❌ Missing dictionaries on cluster:".to_string());
        for name in &missing_dictionaries {
            errors.push(format!(
                "   - {} (expected as: {}_{})",
                name, PROJ_NAME, name
            ));
        }
    }

    if !missing_files.is_empty() {
        errors.push("\n⚠️  Missing local definition files:".to_string());
        for file in &missing_files {
            errors.push(format!("   - {}", file));
        }
    }

    if !errors.is_empty() {
        errors.push("\n📋 In production mode:".to_string());
        if !missing_functions.is_empty() || !missing_dictionaries.is_empty() {
            errors.push("   • Resources must exist on cluster before deployment".to_string());
            errors.push(
                "   • Either create them manually or run without --production flag first"
                    .to_string(),
            );
        }
        if !missing_files.is_empty() {
            errors.push(
                "   • Local files should be included for documentation and validation".to_string(),
            );
        }

        return Err(errors.join("\n"));
    }

    println!("✓ All required dependencies exist on cluster");
    println!("✓ All required local files present");
    Ok(())
}
