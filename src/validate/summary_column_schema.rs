// Validation: Check that summary tables output required columns (especially 'timestamp')

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

        // Verify it outputs 'timestamp' column (standard for summary tables)
        if !output_columns.contains("timestamp") {
            return Err(format!(
                "ERROR: {}.{} Summary table {} does not output 'timestamp' column.\n  \
                 Summary tables should always have a timestamp column for time-range filtering.\n  \
                 Add 'timestamp' column or alias the time column as 'timestamp' in the SELECT clause.\n",
                file!(),
                line!(),
                summary_table.name
            ));
        }
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
            // No alias - extract column name
            let col = part.split_whitespace().last().unwrap_or(part).trim();
            if !col.is_empty() && !col.contains('(') {
                columns.insert(col.to_string());
            }
        }
    }

    Ok(columns)
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
    fn test_parse_select_with_timestamp() {
        let sql = "SELECT toStartOfMinute(reqTimeSec) as timestamp, count() as cnt FROM table";
        let columns = parse_select_columns(sql).unwrap();

        assert!(columns.contains("timestamp"));
        assert!(columns.contains("cnt"));
    }

    #[test]
    fn test_parse_select_without_timestamp() {
        let sql = "SELECT count() as cnt FROM table";
        let columns = parse_select_columns(sql).unwrap();

        assert!(!columns.contains("timestamp"));
        assert!(columns.contains("cnt"));
    }
}
