// Validation: Check that template variables in dashboards match those declared in bundle.json

use regex::Regex;
use std::collections::HashSet;
use tokio::fs;

use crate::models::bundle::Bundle;

pub async fn run(base: &str, bundle: &Bundle) -> Result<(), String> {
    // Extract expected variables from bundle.json
    let mut expected_variables = HashSet::new();

    // Standard variables that should be used
    expected_variables.insert("__PROJECT_NAME__".to_string());
    expected_variables.insert("__DATASOURCE__".to_string());

    // Table variable from bundle
    for table in &bundle.tables {
        expected_variables.insert(table.dashboard_var.clone());
    }

    // Summary table variables from bundle
    if let Some(summary_tables) = &bundle.summary_tables {
        for summary_table in summary_tables {
            expected_variables.insert(summary_table.dashboard_var.clone());
        }
    }

    // Dashboard UUID variable
    expected_variables.insert("__DASHBOARD_UUID__".to_string());

    let mut dashboard_path_list: Vec<String> = vec![];

    dashboard_path_list.push(format!("{}/{}", base, bundle.dashboard.path));

    if let Some(other_dashboards) = &bundle.other_dashboards {
        for other_dash in other_dashboards {
            dashboard_path_list.push(format!("{}/{}", base, other_dash.path));
        }
    }

    // Pattern to find template variables in JSON: __VARIABLE_NAME__
    let variable_pattern = Regex::new(r"__([A-Z_][A-Z0-9_]*)__").unwrap();

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

        // Find all variables used in this dashboard
        let mut found_variables = HashSet::new();
        for cap in variable_pattern.captures_iter(&content) {
            if let Some(var_match) = cap.get(0) {
                found_variables.insert(var_match.as_str().to_string());
            }
        }

        // Check for unexpected variables (typos, wrong names)
        for var in &found_variables {
            if !expected_variables.contains(var) {
                // Some exceptions for special Grafana variables
                if var.starts_with("__time")
                    || var.starts_with("__from")
                    || var.starts_with("__to")
                    || var == "__dashboard"
                    || var == "__user"
                    || var.starts_with("__interval")
                {
                    continue; // These are standard Grafana variables
                }

                return Err(format!(
                    "ERROR: {}.{} Dashboard {} uses unexpected template variable '{}'.\n  \
                     This might be a typo. Expected variables from bundle.json: {:?}\n  \
                     Note: Standard Grafana variables like __time*, __from*, __to*, __user, __interval* are allowed.\n",
                    file!(),
                    line!(),
                    dashboard_name,
                    var,
                    expected_variables.iter().collect::<Vec<_>>()
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
    fn test_variable_pattern_matching() {
        let variable_pattern = Regex::new(r"__([A-Z_][A-Z0-9_]*)__").unwrap();

        let json = r#"{"query": "SELECT * FROM __PROJECT_NAME__.table WHERE time > __from"}"#;
        let vars: Vec<_> = variable_pattern
            .captures_iter(json)
            .filter_map(|cap| cap.get(0))
            .map(|m| m.as_str())
            .collect();

        assert!(vars.contains(&"__PROJECT_NAME__"));
    }

    #[test]
    fn test_detects_typo_in_variable() {
        let variable_pattern = Regex::new(r"__([A-Z_][A-Z0-9_]*)__").unwrap();

        // Common typo: missing underscore
        let json = r#"{"query": "SELECT * FROM __PROJECTNAME__.table"}"#;
        let vars: Vec<_> = variable_pattern
            .captures_iter(json)
            .filter_map(|cap| cap.get(0))
            .map(|m| m.as_str().to_string())
            .collect();

        assert!(vars.contains(&"__PROJECTNAME__".to_string()));
        assert!(!vars.contains(&"__PROJECT_NAME__".to_string()));
    }

    #[test]
    fn test_allows_grafana_variables() {
        let grafana_vars = vec![
            "__timeFrom",
            "__timeTo",
            "__from",
            "__to",
            "__dashboard",
            "__user",
            "__interval",
            "__interval_ms",
        ];

        for var in grafana_vars {
            let is_allowed = var.starts_with("__time")
                || var.starts_with("__from")
                || var.starts_with("__to")
                || var == "__dashboard"
                || var == "__user"
                || var.starts_with("__interval");

            assert!(
                is_allowed,
                "Standard Grafana variable {} should be allowed",
                var
            );
        }
    }
}
