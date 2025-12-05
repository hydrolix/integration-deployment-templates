use crate::bundle_struct::Bundle;
use regex::Regex;
use std::collections::HashSet;
use tokio::fs;

pub async fn run(base: &str, bundle: &Bundle) -> Result<(), String> {
    println!("Validating template variable consistency...");

    // Extract expected variables from bundle.json
    let mut expected_variables = HashSet::new();

    // Standard variables that should be used
    expected_variables.insert("__PROJECT_NAME__".to_string());
    expected_variables.insert("__DATASOURCE__".to_string());

    // Table variable from bundle
    if !bundle.tables.is_empty() {
        expected_variables.insert(bundle.tables[0].dashboard_var.clone());
    }

    // Summary table variables from bundle
    if let Some(summary_tables) = &bundle.summary_tables {
        for summary_table in summary_tables {
            expected_variables.insert(summary_table.dashboard_var.clone());
        }
    }

    // Dashboard UUID variable
    expected_variables.insert("__DASHBOARD_UUID__".to_string());

    println!("  Expected template variables: {:?}", expected_variables);

    // Get all dashboard files
    let dashboard_files = get_dashboard_files(base, bundle).await?;

    // Pattern to find template variables in JSON: __VARIABLE_NAME__
    let variable_pattern = Regex::new(r"__([A-Z_][A-Z0-9_]*)__").unwrap();

    for dashboard_path in dashboard_files {
        let dashboard_name = std::path::Path::new(&dashboard_path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();

        let dashboard_json = fs::read_to_string(&dashboard_path).await
            .map_err(|e| format!("Failed to read dashboard {}: {}", dashboard_path, e))?;

        println!("  Checking dashboard: {}", dashboard_name);

        // Find all variables used in this dashboard
        let mut found_variables = HashSet::new();
        for cap in variable_pattern.captures_iter(&dashboard_json) {
            if let Some(var_match) = cap.get(0) {
                found_variables.insert(var_match.as_str().to_string());
            }
        }

        // Check for unexpected variables (typos, wrong names)
        for var in &found_variables {
            if !expected_variables.contains(var) {
                // Some exceptions for special Grafana variables
                if var.starts_with("__time") || var.starts_with("__from") ||
                   var.starts_with("__to") || var == "__dashboard" ||
                   var == "__user" || var.starts_with("__interval") {
                    continue; // These are standard Grafana variables
                }

                return Err(format!(
                    "Dashboard {} uses unexpected template variable '{}'. Expected one of: {:?}",
                    dashboard_name, var, expected_variables
                ));
            }
        }

        println!("    ✓ All template variables are consistent");
    }

    println!("  ✓ Template variable consistency validation passed");
    Ok(())
}

async fn get_dashboard_files(base: &str, bundle: &Bundle) -> Result<Vec<String>, String> {
    let mut files = Vec::new();

    let path = format!("{}/{}", base, bundle.dashboard.path);
    if std::path::Path::new(&path).exists() {
        files.push(path);
    }

    if let Some(ref other_dashboards) = bundle.other_dashboards {
        for dashboard in other_dashboards {
            let path = format!("{}/{}", base, dashboard.path);
            if std::path::Path::new(&path).exists() {
                files.push(path);
            }
        }
    }

    Ok(files)
}
