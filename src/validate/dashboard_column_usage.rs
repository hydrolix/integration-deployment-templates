use crate::bundle_struct::Bundle;
use regex::Regex;
use std::collections::HashSet;
use std::path::Path;
use tokio::fs;

pub async fn run(base: &str, bundle: &Bundle) -> Result<(), String> {
    println!("Validating dashboard column usage against summary tables...");

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

    // Get all columns available in summary tables
    let mut all_summary_columns = HashSet::new();
    for summary_table in summary_tables {
        let sql_path = format!("{}/{}", base, summary_table.sql.path);
        let sql_content = fs::read_to_string(&sql_path).await
            .map_err(|e| format!("Failed to read summary SQL {}: {}", sql_path, e))?;

        let columns = parse_summary_columns(&sql_content)?;
        all_summary_columns.extend(columns);
    }

    println!("  Summary tables provide {} columns", all_summary_columns.len());

    // Get base table columns (for Raw Logs dashboard)
    let base_table_columns = get_base_table_columns(base, bundle)?;
    println!("  Base table provides {} columns", base_table_columns.len());

    // Get all dashboard files
    let dashboard_files = get_dashboard_files(base, bundle).await?;

    for dashboard_path in dashboard_files {
        let dashboard_name = Path::new(&dashboard_path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();

        let dashboard_json = fs::read_to_string(&dashboard_path).await
            .map_err(|e| format!("Failed to read dashboard {}: {}", dashboard_path, e))?;

        println!("  Checking dashboard: {}", dashboard_name);

        // Extract column references from queries
        let referenced_columns = extract_column_references(&dashboard_json)?;

        println!("    Dashboard references {} columns", referenced_columns.len());

        // Check which table each column should come from
        let is_raw_logs = dashboard_name.contains("Raw Logs");

        for col in &referenced_columns {
            // Skip special cases
            if is_special_identifier(col) {
                continue;
            }

            let in_summary = all_summary_columns.contains(col);
            let in_base = base_table_columns.contains(col);

            if is_raw_logs {
                // Raw Logs should query base table
                if !in_base {
                    println!("    ⚠️  Column '{}' not found in base table", col);
                    return Err(format!(
                        "Dashboard {} queries column '{}' but it doesn't exist in base table.\n  \
                         Available base table columns: {:?}",
                        dashboard_name, col, base_table_columns.iter().take(10).collect::<Vec<_>>()
                    ));
                }
            } else {
                // Other dashboards should query summary tables
                if !in_summary && !in_base {
                    println!("    ⚠️  Column '{}' not found in summary or base tables", col);
                    return Err(format!(
                        "Dashboard {} queries column '{}' but it doesn't exist in summary tables.\n  \
                         Available summary columns: {:?}",
                        dashboard_name, col, all_summary_columns.iter().take(10).collect::<Vec<_>>()
                    ));
                }
            }
        }

        println!("    ✓ All column references are valid");
    }

    println!("  ✓ Dashboard column usage validation passed");
    Ok(())
}

fn parse_summary_columns(sql: &str) -> Result<HashSet<String>, String> {
    let mut columns = HashSet::new();

    // Find SELECT clause
    let select_regex = Regex::new(r"(?is)SELECT\s+(.*?)\s+FROM").unwrap();
    let select_clause = select_regex.captures(sql)
        .ok_or_else(|| "Could not find SELECT clause".to_string())?
        .get(1)
        .ok_or_else(|| "Could not extract SELECT clause".to_string())?
        .as_str();

    // Split by comma (accounting for nested functions)
    let parts = split_by_comma(select_clause);

    for part in parts {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        // Extract alias if present (e.g., "statusCode as response_status_code")
        let alias_regex = Regex::new(r"(?i)\s+as\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();

        if let Some(captures) = alias_regex.captures(part) {
            // Has explicit alias - this is the output column name
            let alias = captures.get(1).unwrap().as_str();
            columns.insert(alias.to_string());
        } else {
            // No alias - extract the column name from the expression
            // Handle simple cases like "hdx_cdn" or "count() as cnt_all"
            let tokens: Vec<&str> = part.split_whitespace().collect();
            if let Some(last_token) = tokens.last() {
                // Skip function calls
                if !last_token.ends_with(')') && !last_token.contains('(') {
                    columns.insert(last_token.to_string());
                }
            }
        }
    }

    Ok(columns)
}

fn split_by_comma(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut paren_depth = 0;

    for ch in text.chars() {
        match ch {
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                paren_depth -= 1;
                current.push(ch);
            }
            ',' if paren_depth == 0 => {
                parts.push(current.trim().to_string());
                current = String::new();
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }

    parts
}

fn extract_column_references(dashboard_json: &str) -> Result<HashSet<String>, String> {
    let mut columns = HashSet::new();

    // First, extract template variable definitions to resolve them later
    let template_vars = extract_template_variables(dashboard_json)?;

    // Find all rawSql fields in the dashboard
    let sql_regex = Regex::new(r#""rawSql":\s*"([^"]*(?:\\.[^"]*)*)""#).unwrap();

    for cap in sql_regex.captures_iter(dashboard_json) {
        if let Some(sql) = cap.get(1) {
            let sql = sql.as_str()
                .replace("\\n", " ")
                .replace("\\t", " ")
                .replace("\\r", " ");

            // Remove SETTINGS clause to avoid extracting identifiers from admin comments
            let sql_without_settings = if let Some(settings_pos) = sql.to_uppercase().find("SETTINGS") {
                &sql[..settings_pos]
            } else {
                &sql
            };

            // Remove AS aliases (output column names) to avoid validating them
            let as_alias_regex = Regex::new(r"(?i)\s+AS\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
            let aliases: HashSet<String> = as_alias_regex
                .captures_iter(sql_without_settings)
                .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
                .collect();

            // Extract identifiers that look like column names
            // Pattern: word characters not preceded by $ or function call
            let identifier_regex = Regex::new(r"\b([a-zA-Z_][a-zA-Z0-9_]*)\b").unwrap();

            for ident_cap in identifier_regex.captures_iter(sql_without_settings) {
                if let Some(ident) = ident_cap.get(1) {
                    let ident_str = ident.as_str();

                    // Filter out SQL keywords, functions, special identifiers, and AS aliases
                    if !is_sql_keyword(ident_str) && !is_grafana_macro(ident_str) && !is_special_identifier(ident_str) && !aliases.contains(ident_str) {
                        // Check if this is a template variable name - if so, resolve it
                        if let Some(var_query) = template_vars.get(ident_str) {
                            // Skip if this is a table reference variable (contains other template vars)
                            if var_query.contains("${") {
                                // This is a template variable that references other variables (likely a table name)
                                continue;
                            }
                            // Extract column names from the variable's query
                            let var_columns = extract_columns_from_query(var_query);
                            columns.extend(var_columns);
                        } else {
                            // It's a direct column reference
                            columns.insert(ident_str.to_string());
                        }
                    }
                }
            }
        }
    }

    Ok(columns)
}

fn extract_template_variables(dashboard_json: &str) -> Result<std::collections::HashMap<String, String>, String> {
    use std::collections::HashMap;
    let mut vars = HashMap::new();

    // Simple approach: Extract all "name": "xxx" and all "query": "yyy" pairs separately,
    // then match them up based on proximity
    let name_regex = Regex::new(r#""name":\s*"([a-zA-Z_][a-zA-Z0-9_]*)""#).unwrap();
    let query_regex = Regex::new(r#""query":\s*"([^"]*(?:\\.[^"]*)*)""#).unwrap();

    let names: Vec<(usize, String)> = name_regex
        .captures_iter(dashboard_json)
        .filter_map(|cap| {
            cap.get(1)
                .map(|m| (m.start(), m.as_str().to_string()))
        })
        .collect();

    let queries: Vec<(usize, String)> = query_regex
        .captures_iter(dashboard_json)
        .filter_map(|cap| {
            cap.get(1)
                .map(|m| (m.start(), m.as_str().to_string()))
        })
        .collect();

    // Match names with the closest following query (within 500 chars)
    for (name_pos, name) in names {
        if let Some((_, query)) = queries.iter().find(|(query_pos, _)| {
            *query_pos > name_pos && (*query_pos - name_pos) < 500
        }) {
            vars.insert(name, query.clone());
        }
    }

    Ok(vars)
}

fn extract_columns_from_query(query: &str) -> HashSet<String> {
    let mut columns = HashSet::new();

    // Pattern: column_name[index] or just column_name
    let col_regex = Regex::new(r"\b([a-zA-Z_][a-zA-Z0-9_]*)\b").unwrap();

    for cap in col_regex.captures_iter(query) {
        if let Some(col) = cap.get(1) {
            let col_str = col.as_str();
            if !is_sql_keyword(col_str) && !is_grafana_macro(col_str) && !is_special_identifier(col_str) {
                columns.insert(col_str.to_string());
            }
        }
    }

    columns
}

fn get_base_table_columns(base: &str, bundle: &Bundle) -> Result<HashSet<String>, String> {
    if bundle.tables.is_empty() || bundle.tables[0].transforms.is_empty() {
        return Ok(HashSet::new());
    }

    let transform_path = format!("{}/{}", base, bundle.tables[0].transforms[0].path);
    let transform_json = std::fs::read_to_string(&transform_path)
        .map_err(|e| format!("Failed to read transform: {}", e))?;

    let transform: serde_json::Value = serde_json::from_str(&transform_json)
        .map_err(|e| format!("Failed to parse transform: {}", e))?;

    let mut columns = HashSet::new();

    if let Some(output_columns) = transform["settings"]["output_columns"].as_array() {
        for col in output_columns {
            if let Some(name) = col["name"].as_str() {
                columns.insert(name.to_string());
            }
        }
    }

    Ok(columns)
}

fn is_sql_keyword(word: &str) -> bool {
    let keywords = vec![
        "SELECT", "FROM", "WHERE", "AND", "OR", "NOT", "NULL", "AS", "ON",
        "JOIN", "LEFT", "RIGHT", "INNER", "OUTER", "GROUP", "BY", "ORDER",
        "HAVING", "LIMIT", "OFFSET", "UNION", "CASE", "WHEN", "THEN", "ELSE",
        "END", "IF", "IN", "EXISTS", "BETWEEN", "LIKE", "IS", "ASC", "DESC",
        "DISTINCT", "ALL", "ANY", "SOME", "SETTINGS", "INTERVAL", "MINUTE",
        "HOUR", "DAY", "WEEK", "MONTH", "YEAR", "OVER", "PARTITION", "ROW",
        "ROWS", "RANGE", "UNBOUNDED", "PRECEDING", "FOLLOWING", "CURRENT",
    ];

    keywords.contains(&word.to_uppercase().as_str())
}

fn is_grafana_macro(word: &str) -> bool {
    word.starts_with("__") || word.starts_with('$')
}

fn is_special_identifier(word: &str) -> bool {
    // Filter out common non-column identifiers (ClickHouse functions and Grafana keywords)
    // All lowercase since we compare against word.to_lowercase()
    let special = vec![
        "now", "count", "sum", "avg", "min", "max", "todatetime",
        "concat", "if", "arraymap", "arrayfilter", "position", "length",
        "trim", "replace", "substring", "tostring", "toint", "tofloat",
        "datediff", "formatdatetime", "empty", "notempty", "quantile",
        "quantiles", "round", "floor", "ceil", "abs", "log", "exp",
        "tostartofminute", "tostartofhour", "tostartofday", "tostartofweek",
        "tostartofmonth", "tostartofinterval", "arrayjoin", "arrayconcat",
        "arrayelement", "grouparray", "uniq", "uniqexact", "topk", "median",
        // Merge functions for aggregating states
        "countmerge", "countmergeif", "summerge", "avgmerge", "minmerge",
        "maxmerge", "uniqmerge", "quantilemerge", "quantilesmerge",
        // Grafana format modifiers
        "raw", "csv", "json", "sqlstring", "regex", "pipe", "distributed",
    ];

    special.contains(&word.to_lowercase().as_str())
}

async fn get_dashboard_files(base: &str, bundle: &Bundle) -> Result<Vec<String>, String> {
    let mut files = Vec::new();

    let path = format!("{}/{}", base, bundle.dashboard.path);
    if Path::new(&path).exists() {
        files.push(path);
    }

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
