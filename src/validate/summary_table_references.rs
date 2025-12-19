use crate::bundle_struct::Bundle;
use regex::Regex;
use std::collections::HashSet;
use std::path::Path;
use tokio::fs;

pub async fn run(base: &str, bundle: &Bundle) -> Result<(), String> {
    println!("Validating summary table references in dashboards...");

    // Get summary table names from bundle.json
    let summary_tables = match &bundle.summary_tables {
        Some(tables) => tables,
        None => {
            println!("  No summary tables defined, skipping validation");
            return Ok(());
        }
    };

    if summary_tables.is_empty() {
        println!("  No summary tables defined, skipping validation");
        return Ok(());
    }

    let summary_table_names: Vec<String> = summary_tables
        .iter()
        .map(|st| st.name.clone())
        .collect();

    println!("  Expected summary tables: {:?}", summary_table_names);

    // Get dashboard files
    let dashboard_files = get_dashboard_files(base, bundle).await?;

    for dashboard_path in dashboard_files {
        let dashboard_json = match fs::read_to_string(&dashboard_path).await {
            Ok(content) => content,
            Err(e) => return Err(format!("Failed to read dashboard {}: {}", dashboard_path, e)),
        };

        // Check each summary table name
        for summary_name in &summary_table_names {
            let expected_ref = format!("__PROJECT_NAME__.{}", summary_name);

            // Count references to this summary table
            let count = dashboard_json.matches(&expected_ref).count();
            if count > 0 {
                println!("  ✓ Dashboard {} references {} ({} times)",
                    Path::new(&dashboard_path).file_name().unwrap().to_str().unwrap(),
                    summary_name,
                    count
                );
            }
        }

        // Extract ALL __PROJECT_NAME__.{table_name} references using regex
        let table_ref_pattern = Regex::new(r"__PROJECT_NAME__\.([a-zA-Z0-9_]+)").unwrap();
        let mut referenced_tables: HashSet<String> = HashSet::new();

        for cap in table_ref_pattern.captures_iter(&dashboard_json) {
            if let Some(table_match) = cap.get(1) {
                let table_name = table_match.as_str().to_string();

                // Skip template variables (e.g., __TABLE_NAME__, __VARIABLE__, etc.)
                if table_name.starts_with("__") && table_name.ends_with("__") {
                    continue;
                }

                referenced_tables.insert(table_name);
            }
        }

        // Check each referenced table exists in bundle
        for referenced_table in &referenced_tables {
            if !summary_table_names.contains(referenced_table) {
                return Err(format!(
                    "Dashboard {} references '__PROJECT_NAME__.{}' but no summary table with that name exists in bundle.json. Available: {:?}",
                    dashboard_path, referenced_table, summary_table_names
                ));
            }
        }
    }

    println!("  ✓ All summary table references are valid");
    Ok(())
}

async fn get_dashboard_files(base: &str, bundle: &Bundle) -> Result<Vec<String>, String> {
    let mut files = Vec::new();

    // Primary dashboard
    let path = format!("{}/{}", base, bundle.dashboard.path);
    if Path::new(&path).exists() {
        files.push(path);
    }

    // Other dashboards
    if let Some(ref other_dashboards) = bundle.other_dashboards {
        for dashboard in other_dashboards {
            let path = format!("{}/{}", base, dashboard.path);
            if Path::new(&path).exists() {
                files.push(path);
            }
        }
    }

    Ok(files)
}
