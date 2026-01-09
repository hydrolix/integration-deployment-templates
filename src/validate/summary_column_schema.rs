// Validation: Check that summary tables' hdx_primary_key references an actual output column

use regex::Regex;
use std::collections::HashSet;
use tokio::fs;

use crate::models::bundle::Bundle;

pub async fn run(base: &str, bundle: &Bundle) -> Result<(), String> {
    let summary_tables = match &bundle.summary_tables {
        Some(tables) => tables,
        None => {
            // No summary tables, skip validation
            return Ok(());
        }
    };

    if summary_tables.is_empty() {
        return Ok(());
    }

    for summary_table in summary_tables {
        let sql_path = format!("{}/{}", base, summary_table.sql.path);

        let sql_content = match fs::read_to_string(&sql_path).await {
            Ok(v) => v,
            Err(e) => {
                return Err(format!(
                    "ERROR: {}.{} Failed to read summary SQL {}: error={e}\n",
                    file!(),
                    line!(),
                    sql_path
                ));
            }
        };

        // Parse SELECT clause to extract output column names
        let output_columns = parse_select_columns(&sql_content)?;

        // Parse hdx_primary_key from SETTINGS clause
        if let Some(primary_key) = parse_primary_key(&sql_content) {
            // Verify the primary key column exists in the output
            if !output_columns.contains(&primary_key) {
                return Err(format!(
                    "ERROR: {}.{} Summary table {} has hdx_primary_key='{}' but this column is not in the SELECT output.\n  \
                     Available columns: {}\n  \
                     Fix: Either add '{}' to the SELECT clause or change hdx_primary_key to match an existing column.\n",
                    file!(),
                    line!(),
                    summary_table.name,
                    primary_key,
                    output_columns.iter().cloned().collect::<Vec<_>>().join(", "),
                    primary_key
                ));
            }
        }
        // If no hdx_primary_key is specified, that's okay - ClickHouse will use defaults
    }

    Ok(())
}

fn parse_select_columns(sql: &str) -> Result<HashSet<String>, String> {
    let mut columns = HashSet::new();

    // Remove SQL comments first
    let sql = remove_sql_comments(sql);

    // Find SELECT clause (handle multiline)
    let select_regex = Regex::new(r"(?is)SELECT\s+(.*?)\s+FROM").unwrap();
    let select_clause = select_regex
        .captures(&sql)
        .ok_or_else(|| "Could not find SELECT clause in SQL".to_string())?
        .get(1)
        .ok_or_else(|| "Could not extract SELECT clause".to_string())?
        .as_str();

    // Split by comma
    for part in select_clause.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        // Extract alias if present (e.g., "statusCode as response_status_code")
        if let Some(alias_pos) = part.to_lowercase().rfind(" as ") {
            let alias = part[alias_pos + 4..].trim();
            columns.insert(alias.to_string());
        } else {
            // No alias - extract column name (last token before any function calls)
            let col = part.split_whitespace().last().unwrap_or(part).trim();
            if !col.is_empty() && !col.contains('(') {
                columns.insert(col.to_string());
            }
        }
    }

    Ok(columns)
}

fn parse_primary_key(sql: &str) -> Option<String> {
    // Remove comments
    let sql = remove_sql_comments(sql);

    // Look for SETTINGS hdx_primary_key = 'value' or hdx_primary_key='value'
    let pk_regex = Regex::new(r#"(?i)hdx_primary_key\s*=\s*['"]([^'"]+)['"]"#).unwrap();

    pk_regex
        .captures(&sql)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
}

fn remove_sql_comments(sql: &str) -> String {
    let mut result = String::new();
    let mut chars = sql.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '-' => {
                if chars.peek() == Some(&'-') {
                    chars.next();
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch == '\n' || next_ch == '\r' {
                            break;
                        }
                        chars.next();
                    }
                    result.push(' ');
                } else {
                    result.push(ch);
                }
            }
            '/' => {
                if chars.peek() == Some(&'*') {
                    chars.next();
                    while let Some(inner_ch) = chars.next() {
                        if inner_ch == '*' && chars.peek() == Some(&'/') {
                            chars.next();
                            break;
                        }
                    }
                    result.push(' ');
                } else {
                    result.push(ch);
                }
            }
            _ => {
                result.push(ch);
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_primary_key() {
        let sql = "SELECT timestamp FROM table SETTINGS hdx_primary_key = 'timestamp'";
        assert_eq!(parse_primary_key(sql), Some("timestamp".to_string()));

        let sql2 = "SELECT reqTimeSec FROM table SETTINGS hdx_primary_key='reqTimeSec'";
        assert_eq!(parse_primary_key(sql2), Some("reqTimeSec".to_string()));
    }

    #[test]
    fn test_parse_select_with_alias() {
        let sql = "SELECT toStartOfMinute(reqTimeSec) as timestamp_min, count() as cnt FROM table";
        let columns = parse_select_columns(sql).unwrap();

        assert!(columns.contains("timestamp_min"));
        assert!(columns.contains("cnt"));
    }

    #[test]
    fn test_matching_primary_key() {
        let sql = "SELECT toStartOfHour(reqTimeSec) AS reqTimeSec FROM t GROUP BY reqTimeSec SETTINGS hdx_primary_key = 'reqTimeSec'";

        let columns = parse_select_columns(sql).unwrap();
        let pk = parse_primary_key(sql).unwrap();

        assert!(columns.contains(&pk));
    }
}
