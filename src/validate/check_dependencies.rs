// Validation: Check that functions/dictionaries have local files and are used in SQL

use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;
use tokio::fs;

use crate::bundle_struct::Bundle;

pub async fn run(base: &str, bundle: &Bundle) -> Result<(), String> {
    let mut declared_functions: HashSet<String> = HashSet::new();
    let mut declared_dictionaries: HashSet<String> = HashSet::new();

    // Collect declared dependencies
    if let Some(deps) = &bundle.dependencies {
        if let Some(hydrolix) = &deps.hydrolix {
            if let Some(required_functions) = &hydrolix.required_functions {
                for fn_name in required_functions {
                    declared_functions.insert(fn_name.clone());
                }
            }

            if let Some(required_dictionaries) = &hydrolix.required_dictionaries {
                for dict_name in required_dictionaries {
                    declared_dictionaries.insert(dict_name.clone());
                }
            }
        }
    }

    // Check that local files exist for each declared dependency
    println!("Checking local function files...");
    for fn_name in &declared_functions {
        let json_path = format!("{}/functions/{}.json", base, fn_name);
        if fs::metadata(&json_path).await.is_ok() {
            println!("  ✓ Function file exists: functions/{}.json", fn_name);
        } else {
            eprintln!(
                "  ⚠️  WARNING: Function '{}' declared but no file: functions/{}.json",
                fn_name, fn_name
            );
        }
    }

    println!("Checking local dictionary files...");
    for dict_name in &declared_dictionaries {
        let json_path = format!("{}/dictionaries/{}.json", base, dict_name);

        // Check for JSON definition
        let has_json = fs::metadata(&json_path).await.is_ok();
        if has_json {
            println!(
                "  ✓ Dictionary definition exists: dictionaries/{}.json",
                dict_name
            );
        } else {
            eprintln!(
                "  ⚠️  WARNING: Dictionary '{}' declared but no definition: dictionaries/{}.json",
                dict_name, dict_name
            );
        }

        // Check for data file (CSV, YAML, or other)
        if has_json {
            let mut found_data_file = false;
            let possible_extensions = vec!["csv", "yaml", "yml", "tsv"];

            for ext in possible_extensions {
                let data_path = format!("{}/dictionaries/{}.{}", base, dict_name, ext);
                if fs::metadata(&data_path).await.is_ok() {
                    println!(
                        "  ✓ Dictionary data file exists: dictionaries/{}.{}",
                        dict_name, ext
                    );
                    found_data_file = true;
                    break;
                }
            }

            if !found_data_file {
                eprintln!(
                    "  ⚠️  WARNING: Dictionary '{}' has definition but no data file (checked .csv, .yaml, .yml, .tsv)",
                    dict_name
                );
            }
        }
    }

    // Check each transform's SQL for function/dictionary usage
    println!("Checking SQL references in transforms...");
    let mut used_functions: HashSet<String> = HashSet::new();
    let mut used_dictionaries: HashSet<String> = HashSet::new();

    // Compile regex once outside the loop
    let dict_get_pattern =
        Regex::new(r#"dict(?:Get|GetString|GetOrDefault)\s*\(\s*['"]([^'"]+)['"]"#).unwrap();

    for table in &bundle.tables {
        for transform in &table.transforms {
            let full_path = format!("{}/{}", base, transform.path);
            let content = fs::read_to_string(&full_path)
                .await
                .map_err(|e| format!("Failed to read transform {}: {}", full_path, e))?;

            let transform_json: Value = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse transform JSON {}: {}", full_path, e))?;

            let sql = transform_json
                .get("settings")
                .and_then(|s| s.get("sql_transform"))
                .and_then(|sql_val| sql_val.as_str());

            if sql.is_none() {
                continue;
            }

            let sql_str = sql.unwrap();

            // Check if declared functions are used
            for function_name in &declared_functions {
                // Match function calls: functionName(
                let pattern = format!(r"\b{}\s*\(", regex::escape(function_name));
                if let Ok(re) = Regex::new(&pattern) {
                    if re.is_match(sql_str) {
                        used_functions.insert(function_name.clone());
                    }
                }
            }

            // Check for dictGet/dictGetString calls

            for cap in dict_get_pattern.captures_iter(sql_str) {
                if let Some(dict_name_match) = cap.get(1) {
                    let dict_name = dict_name_match.as_str();
                    used_dictionaries.insert(dict_name.to_string());

                    if !declared_dictionaries.contains(dict_name) {
                        eprintln!(
                            "  ⚠️  WARNING: Transform {} uses dictionary '{}' but it's not declared in dependencies.hydrolix.required_dictionaries",
                            transform.path, dict_name
                        );
                    }
                }
            }
        }
    }

    // Report declared but unused dependencies
    for fn_name in &declared_functions {
        if !used_functions.contains(fn_name) {
            eprintln!(
                "  ⚠️  INFO: Function '{}' is declared but not used in any transforms",
                fn_name
            );
        }
    }

    for dict_name in &declared_dictionaries {
        if !used_dictionaries.contains(dict_name) {
            eprintln!(
                "  ⚠️  INFO: Dictionary '{}' is declared but not used in any transforms",
                dict_name
            );
        }
    }

    println!("✓ Dependency validation complete");
    Ok(())
}
