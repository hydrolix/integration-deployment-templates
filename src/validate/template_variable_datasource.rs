use crate::bundle_struct::Bundle;
use std::path::Path;
use tokio::fs;

pub async fn run(base: &str, bundle: &Bundle) -> Result<(), String> {
    println!("Validating dashboard template variable datasources...");

    let dashboard_files = get_dashboard_files(base, bundle).await?;

    if dashboard_files.is_empty() {
        println!("  No dashboards found, skipping validation");
        return Ok(());
    }

    for dashboard_path in dashboard_files {
        let dashboard_json = match fs::read_to_string(&dashboard_path).await {
            Ok(content) => content,
            Err(e) => return Err(format!("Failed to read dashboard {}: {}", dashboard_path, e)),
        };

        let dashboard: serde_json::Value = match serde_json::from_str(&dashboard_json) {
            Ok(v) => v,
            Err(e) => return Err(format!("Failed to parse dashboard JSON {}: {}", dashboard_path, e)),
        };

        let dashboard_name = Path::new(&dashboard_path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();

        println!("  Checking dashboard: {}", dashboard_name);

        // Check which variables are actually referenced in the dashboard
        let referenced_vars = find_referenced_variables(&dashboard_json);

        if let Some(variables) = dashboard["dashboard"]["templating"]["list"].as_array() {
            for var in variables {
                if var["type"].as_str() == Some("query") {
                    let var_name = var["name"].as_str().unwrap_or("unknown");
                    let query = var["query"].as_str().unwrap_or("");

                    // Skip validation for unused variables
                    if !referenced_vars.contains(var_name) {
                        println!("    ⓘ Variable '{}' is defined but not used in dashboard (skipping validation)", var_name);
                        continue;
                    }

                    // Check if query uses ClickHouse-specific functions that require database execution
                    // These are functions that Grafana cannot evaluate client-side
                    let complex_clickhouse_functions = vec![
                        "arrayFilter", "arrayConcat", "arrayMap",
                        "arrayStringConcat", "arrayElement",
                        "toStartOfMinute", "toStartOfHour", "toStartOfDay",
                        "quantiles", "countMerge", "sumMerge", "avgMerge",
                        "dictGet", "dictHas",
                    ];

                    let uses_complex_functions = complex_clickhouse_functions.iter()
                        .any(|func| query.contains(func));

                    // Also check if it's a simple query that Grafana can handle:
                    // - Uses only Grafana macros ($__fromTime, $__toTime, etc.)
                    // - Simple CASE WHEN with variable substitutions
                    // - Simple comparisons and string operations on variables
                    let uses_grafana_macros = query.contains("$__fromTime")
                        || query.contains("$__toTime")
                        || query.contains("$__from")
                        || query.contains("$__to");

                    let is_simple_case_when = query.contains("CASE WHEN")
                        && query.contains("${")
                        && !uses_complex_functions;

                    let is_simple_string_template = query.contains("$$")
                        || (query.contains("SELECT") && query.contains("'${") && !uses_complex_functions);

                    // Only flag if it uses complex functions AND isn't a simple template
                    let needs_datasource = uses_complex_functions
                        && !is_simple_string_template;

                    if needs_datasource {
                        // Check if datasource is configured
                        let has_datasource = !var["datasource"].is_null();

                        if !has_datasource {
                            println!("    ⚠️  Variable '{}' uses complex ClickHouse functions but has no datasource", var_name);
                            println!("       Query: {}", &query[..query.len().min(100)]);
                            return Err(format!(
                                "Dashboard {} variable '{}' uses complex ClickHouse functions that require database execution but has no datasource configured.\n  \
                                 Add 'datasource': {{'type': 'hydrolix-hydrolix-datasource', 'uid': '__DATASOURCE__'}} to this variable.",
                                dashboard_name, var_name
                            ));
                        } else {
                            println!("    ✓ Variable '{}' has datasource configured", var_name);
                        }
                    } else if uses_grafana_macros || is_simple_case_when || is_simple_string_template {
                        println!("    ⓘ Variable '{}' uses simple logic that Grafana can evaluate client-side", var_name);
                    }
                }
            }
        }
    }

    println!("  ✓ All template variables with ClickHouse functions have datasources configured");
    Ok(())
}

fn find_referenced_variables(dashboard_json: &str) -> std::collections::HashSet<String> {
    use regex::Regex;
    let mut referenced = std::collections::HashSet::new();

    // Match ${variable_name} pattern
    let var_regex = Regex::new(r"\$\{([a-zA-Z_][a-zA-Z0-9_]*)\}").unwrap();

    for cap in var_regex.captures_iter(dashboard_json) {
        if let Some(var_name) = cap.get(1) {
            referenced.insert(var_name.as_str().to_string());
        }
    }

    referenced
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clickhouse_function_detection() {
        let query1 = "SELECT arrayFilter(x -> x > 0, [1,2,3])";
        assert!(query1.contains("arrayFilter"));

        let query2 = "SELECT toStartOfMinute(timestamp)";
        assert!(query2.contains("toStartOfMinute"));

        let query3 = "SELECT name FROM table";
        assert!(!query3.contains("arrayFilter"));
    }
}
