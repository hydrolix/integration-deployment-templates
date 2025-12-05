use crate::bundle_struct::Bundle;
use regex::Regex;
use tokio::fs;

pub async fn run(base: &str, bundle: &Bundle) -> Result<(), String> {
    println!("Validating datasource UID consistency...");

    // Get all dashboard files
    let dashboard_files = get_dashboard_files(base, bundle).await?;

    // Pattern to find datasource UIDs in JSON
    // Look for "uid": "something" in datasource contexts
    let uid_pattern = Regex::new(r#""datasource":\s*\{[^}]*"uid":\s*"([^"]+)""#).unwrap();

    for dashboard_path in dashboard_files {
        let dashboard_name = std::path::Path::new(&dashboard_path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();

        let dashboard_json = fs::read_to_string(&dashboard_path).await
            .map_err(|e| format!("Failed to read dashboard {}: {}", dashboard_path, e))?;

        println!("  Checking dashboard: {}", dashboard_name);

        // Find all datasource UIDs
        for cap in uid_pattern.captures_iter(&dashboard_json) {
            if let Some(uid_match) = cap.get(1) {
                let uid = uid_match.as_str();

                // Check if it's NOT using the placeholder
                if uid != "__DATASOURCE__" {
                    // Allow special Grafana datasources
                    if uid == "-- Grafana --" || uid == "-- Mixed --" || uid == "-- Dashboard --" {
                        continue;
                    }

                    return Err(format!(
                        "Dashboard {} uses hardcoded datasource UID '{}' instead of '__DATASOURCE__' placeholder. \
                        This will break deployment flexibility.",
                        dashboard_name, uid
                    ));
                }
            }
        }

        println!("    ✓ All datasources use '__DATASOURCE__' placeholder");
    }

    println!("  ✓ Datasource UID consistency validation passed");
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
