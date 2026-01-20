// Validation: Check that dashboard queries reference summary tables that exist in bundle.json

use regex::Regex;
use std::collections::HashSet;
use tokio::fs;

use crate::models::bundle::Bundle;

pub async fn run(base: &str, bundle: &Bundle) -> Result<(), String> {
    // Get summary table names from bundle.json
    let summary_tables = match &bundle.summary_tables {
        Some(tables) => tables,
        None => {
            // No summary tables defined, skip validation
            return Ok(());
        }
    };

    if summary_tables.is_empty() {
        return Ok(());
    }

    let summary_table_names: Vec<String> =
        summary_tables.iter().map(|st| st.name.clone()).collect();

    let mut dashboard_path_list: Vec<String> = vec![];

    dashboard_path_list.push(format!("{}/{}", base, bundle.dashboard.path));

    if let Some(other_dashboards) = &bundle.other_dashboards {
        for other_dash in other_dashboards {
            dashboard_path_list.push(format!("{}/{}", base, other_dash.path));
        }
    }

    // Extract ALL __PROJECT_NAME__.{table_name} references using regex
    let table_ref_pattern = Regex::new(r"__PROJECT_NAME__\.([a-zA-Z0-9_]+)").unwrap();

    for full_path in dashboard_path_list {
        let content = match fs::read_to_string(&full_path).await {
            Ok(v) => v,
            Err(e) => {
                return Err(format!(
                    "ERROR: {}.{} Failed to read dashboard full_path={full_path}: error={e}\n",
                    file!(),
                    line!()
                ));
            }
        };

        // Extract dashboard name from path
        let dashboard_name = full_path.split('/').next_back().unwrap_or("unknown");

        let mut referenced_tables: HashSet<String> = HashSet::new();

        for cap in table_ref_pattern.captures_iter(&content) {
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
                    "ERROR: {}.{} Dashboard {} references '__PROJECT_NAME__.{}' but no summary table with that name exists in bundle.json.\n  \
                     Available summary tables: {:?}\n  \
                     Make sure the summary table is defined in bundle.json or the dashboard query uses the correct table name.\n",
                    file!(),
                    line!(),
                    dashboard_name,
                    referenced_table,
                    summary_table_names
                ));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matching() {
        let dashboard_content =
            r#"{"query": "SELECT * FROM __PROJECT_NAME__.summary_min WHERE timestamp > now()"}"#;

        // Should find the pattern
        assert!(dashboard_content.contains("__PROJECT_NAME__.summary_min"));

        // Should not find non-existent pattern
        assert!(!dashboard_content.contains("__PROJECT_NAME__.nonexistent_table"));
    }

    #[test]
    fn test_summary_table_name_extraction() {
        let query = "__PROJECT_NAME__.mcdn_summary_min";

        // Extract the table name part after __PROJECT_NAME__.
        let parts: Vec<&str> = query.split("__PROJECT_NAME__.").collect();
        if parts.len() > 1 {
            let table_name = parts[1];
            assert_eq!(table_name, "mcdn_summary_min");
        }
    }
}
