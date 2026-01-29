// Validation: Check that template variables using complex ClickHouse functions have datasource configured

use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;
use tokio::fs;

use crate::models::bundle::Bundle;

pub async fn run(base: &str, bundle: &Bundle) -> Result<(), String> {
    let mut dashboard_path_list: Vec<String> = vec![];

    dashboard_path_list.push(format!("{}/{}", base, bundle.dashboard.path));

    if let Some(other_dashboards) = &bundle.other_dashboards {
        for other_dash in other_dashboards {
            dashboard_path_list.push(format!("{}/{}", base, other_dash.path));
        }
    }

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

        let dashboard_json: Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                return Err(format!(
                    "ERROR: {}.{} Invalid dashboard JSON full_path={full_path} error={e}\n",
                    file!(),
                    line!()
                ));
            }
        };

        // Extract dashboard name from path
        let dashboard_name = full_path.split('/').next_back().unwrap_or("unknown");

        // Find which variables are actually referenced in the dashboard
        let referenced_vars = find_referenced_variables(&content);

        // Check template variables
        if let Some(variables) = dashboard_json["dashboard"]["templating"]["list"].as_array() {
            for var in variables {
                if var["type"].as_str() == Some("query") {
                    let var_name = var["name"].as_str().unwrap_or("unknown");
                    let query = var["query"].as_str().unwrap_or("");

                    // Skip validation for unused variables
                    if !referenced_vars.contains(var_name) {
                        continue;
                    }

                    // Check if query uses ClickHouse-specific functions that require database execution
                    let complex_clickhouse_functions = vec![
                        "arrayFilter",
                        "arrayConcat",
                        "arrayMap",
                        "arrayStringConcat",
                        "arrayElement",
                        "toStartOfMinute",
                        "toStartOfHour",
                        "toStartOfDay",
                        "quantiles",
                        "countMerge",
                        "sumMerge",
                        "avgMerge",
                        "dictGet",
                        "dictHas",
                    ];

                    let uses_complex_functions = complex_clickhouse_functions
                        .iter()
                        .any(|func| query.contains(func));

                    // Check if it's a simple query that Grafana can handle client-side
                    let _uses_grafana_macros = query.contains("$__fromTime")
                        || query.contains("$__toTime")
                        || query.contains("$__from")
                        || query.contains("$__to");

                    let _is_simple_case_when = query.contains("CASE WHEN")
                        && query.contains("${")
                        && !uses_complex_functions;

                    let is_simple_string_template = query.contains("$$")
                        || (query.contains("SELECT")
                            && query.contains("'${")
                            && !uses_complex_functions);

                    // Only flag if it uses complex functions AND isn't a simple template
                    let needs_datasource = uses_complex_functions && !is_simple_string_template;

                    if needs_datasource {
                        // Check if datasource is configured
                        let has_datasource = !var["datasource"].is_null();

                        if !has_datasource {
                            return Err(format!(
                                "ERROR: {}.{} Dashboard {} variable '{}' uses complex ClickHouse functions but has no datasource.\n  \
                                 Query: {}\n  \
                                 Add 'datasource': {{'type': 'hydrolix-hydrolix-datasource', 'uid': '__DATASOURCE__'}} to this variable.\n",
                                file!(),
                                line!(),
                                dashboard_name,
                                var_name,
                                &query[..query.len().min(100)]
                            ));
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn find_referenced_variables(dashboard_json: &str) -> HashSet<String> {
    let mut referenced = HashSet::new();

    // Match ${variable_name} pattern
    let var_regex = Regex::new(r"\$\{([a-zA-Z_][a-zA-Z0-9_]*)\}").unwrap();

    for cap in var_regex.captures_iter(dashboard_json) {
        if let Some(var_name) = cap.get(1) {
            referenced.insert(var_name.as_str().to_string());
        }
    }

    referenced
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

    #[test]
    fn test_find_referenced_variables() {
        let json = r#"
            {
                "panels": [
                    {
                        "targets": [
                            {
                                "rawSql": "SELECT * FROM ${table} WHERE time > ${__from}"
                            }
                        ]
                    }
                ]
            }
        "#;

        let vars = find_referenced_variables(json);
        assert!(vars.contains("table"));
        assert!(vars.contains("__from"));
    }
}
