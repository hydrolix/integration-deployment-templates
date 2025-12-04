use crate::bundle_struct::Bundle;
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

        // Look for references to summary tables that don't exist
        // Common patterns: "summary_min", "summary_hour", "mcdn_summary"
        let common_patterns = vec!["summary_min", "summary_hour", "mcdn_summary"];
        for pattern in common_patterns {
            if dashboard_json.contains(&format!("__PROJECT_NAME__.{}", pattern)) {
                let pattern_in_bundle = summary_table_names.iter().any(|name| name.contains(pattern));
                if !pattern_in_bundle {
                    return Err(format!(
                        "Dashboard {} references '__PROJECT_NAME__.{}' but no summary table with that name exists in bundle.json. Available: {:?}",
                        dashboard_path, pattern, summary_table_names
                    ));
                }
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
