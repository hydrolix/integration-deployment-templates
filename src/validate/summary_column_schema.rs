use crate::bundle_struct::Bundle;
use regex::Regex;
use std::collections::HashSet;
use std::path::Path;
use tokio::fs;

pub async fn run(base: &str, bundle: &Bundle) -> Result<(), String> {
    println!("Validating summary table column schemas...");

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

    for summary_table in summary_tables {
        let sql_path = format!("{}/{}", base, summary_table.sql.path);

        if !Path::new(&sql_path).exists() {
            return Err(format!("Summary SQL file not found: {}", sql_path));
        }

        let sql_content = match fs::read_to_string(&sql_path).await {
            Ok(content) => content,
            Err(e) => return Err(format!("Failed to read summary SQL {}: {}", sql_path, e)),
        };

        println!("  Checking summary table: {}", summary_table.name);

        // Parse SELECT clause to extract output column names
        let output_columns = parse_select_columns(&sql_content)?;
        println!("    Output columns ({}):", output_columns.len());
        for col in &output_columns {
            println!("      - {}", col);
        }

        // Verify it outputs 'timestamp' column (standard for summary tables)
        if !output_columns.contains("timestamp") {
            return Err(format!(
                "Summary table {} does not output 'timestamp' column. Summary tables should always have a timestamp column.",
                summary_table.name
            ));
        }

        // Check for common naming issues
        check_column_naming_standards(&output_columns, &summary_table.name)?;
    }

    println!("  ✓ Summary table column schemas are valid");
    Ok(())
}

fn parse_select_columns(sql: &str) -> Result<HashSet<String>, String> {
    let mut columns = HashSet::new();

    // Find SELECT clause (handle multiline)
    let select_regex = Regex::new(r"(?is)SELECT\s+(.*?)\s+FROM").unwrap();
    let select_clause = select_regex.captures(sql)
        .ok_or_else(|| "Could not find SELECT clause in SQL".to_string())?
        .get(1)
        .ok_or_else(|| "Could not extract SELECT clause".to_string())?
        .as_str();

    // Split by comma, handling nested function calls
    let parts = split_select_columns(select_clause);

    for part in parts {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        // Extract alias if present (e.g., "statusCode as response_status_code")
        // Look for "as column_name" or just "column_name" at the end
        let alias_regex = Regex::new(r"(?i)\s+as\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();

        if let Some(captures) = alias_regex.captures(part) {
            // Has explicit alias
            let alias = captures.get(1).unwrap().as_str();
            columns.insert(alias.to_string());
        } else {
            // No alias, extract column name
            // Handle cases like "count() as cnt_all" or just "hdx_cdn"
            let tokens: Vec<&str> = part.split_whitespace().collect();
            if let Some(last_token) = tokens.last() {
                // Skip function names ending with ()
                if !last_token.ends_with(')') && !last_token.contains('(') {
                    columns.insert(last_token.to_string());
                }
            }
        }
    }

    Ok(columns)
}

fn split_select_columns(select_clause: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut paren_depth = 0;

    for ch in select_clause.chars() {
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

fn check_column_naming_standards(columns: &HashSet<String>, table_name: &str) -> Result<(), String> {
    // Standard column names that dashboards expect
    let standard_names: HashSet<&str> = vec![
        "timestamp",
        "response_status_code",
        "cache_was_cached",
        "request_host",
        "client_country_iso_code",
        "client_asn",
        "client_city",
        "edge_pop",
        "user_agent_category",
        "hdx_cdn",
        "cnt_all",
        "response_total_bytes",
        "response_ttfb_ms",
        "response_ttlb_ms",
    ].into_iter().collect();

    // Non-standard names that might indicate issues
    let non_standard_patterns = vec![
        ("statusCode", "response_status_code"),
        ("cacheStatus", "cache_was_cached"),
        ("reqHost", "request_host"),
        ("country", "client_country_iso_code"),
        ("Edge_GeoInfo", "client_asn"),
        ("city", "client_city"),
    ];

    let mut warnings = Vec::new();

    for (bad_name, good_name) in non_standard_patterns {
        if columns.contains(bad_name) {
            warnings.push(format!(
                "Column '{}' should be renamed to '{}' for dashboard compatibility",
                bad_name, good_name
            ));
        }
    }

    if !warnings.is_empty() {
        return Err(format!(
            "Summary table {} uses non-standard column names:\n{}",
            table_name,
            warnings.join("\n  ")
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_select() {
        let sql = "SELECT timestamp, hdx_cdn, cnt_all FROM table";
        let columns = parse_select_columns(sql).unwrap();
        assert!(columns.contains("timestamp"));
        assert!(columns.contains("hdx_cdn"));
        assert!(columns.contains("cnt_all"));
    }

    #[test]
    fn test_parse_select_with_aliases() {
        let sql = "SELECT statusCode as response_status_code, cacheStatus as cache_was_cached FROM table";
        let columns = parse_select_columns(sql).unwrap();
        assert!(columns.contains("response_status_code"));
        assert!(columns.contains("cache_was_cached"));
        assert!(!columns.contains("statusCode"));
    }

    #[test]
    fn test_parse_select_with_functions() {
        let sql = "SELECT count() as cnt_all, sum(bytes) as total_bytes FROM table";
        let columns = parse_select_columns(sql).unwrap();
        assert!(columns.contains("cnt_all"));
        assert!(columns.contains("total_bytes"));
    }
}
