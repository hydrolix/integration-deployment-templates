use crate::bundle_struct::Bundle;
use std::path::Path;
use tokio::fs;

pub async fn run(base: &str, bundle: &Bundle) -> Result<(), String> {
    println!("Validating Raw Logs timestamp column...");

    // Check if Raw Logs dashboard exists
    let raw_logs_path = format!("{}/dashboards/Raw Logs.json", base);
    if !Path::new(&raw_logs_path).exists() {
        println!("  No Raw Logs dashboard found, skipping validation");
        return Ok(());
    }

    // Get primary timestamp column from base table
    let primary_timestamp = get_primary_timestamp_column(base, bundle)?;
    println!("  Base table primary timestamp column: {}", primary_timestamp);

    // Read Raw Logs dashboard
    let dashboard_json = match fs::read_to_string(&raw_logs_path).await {
        Ok(content) => content,
        Err(e) => return Err(format!("Failed to read Raw Logs dashboard: {}", e)),
    };

    // Check for timestamp references in queries
    let patterns_to_check = vec![
        ("WHERE timestamp", "WHERE clause"),
        ("ORDER BY timestamp", "ORDER BY clause"),
        ("$__timeFilter(timestamp)", "Grafana time filter"),
    ];

    let mut found_timestamp_refs = Vec::new();

    for (pattern, description) in &patterns_to_check {
        if dashboard_json.contains(pattern) {
            found_timestamp_refs.push(*description);
        }
    }

    if !found_timestamp_refs.is_empty() && primary_timestamp != "timestamp" {
        return Err(format!(
            "Raw Logs dashboard uses 'timestamp' in {} but base table primary timestamp column is '{}'.\n  \
             Update the Raw Logs dashboard to use '{}' instead of 'timestamp'.",
            found_timestamp_refs.join(", "),
            primary_timestamp,
            primary_timestamp
        ));
    }

    // Also check for correct column usage
    let correct_patterns = vec![
        (format!("WHERE {}", primary_timestamp), "WHERE clause"),
        (format!("ORDER BY {}", primary_timestamp), "ORDER BY clause"),
        (format!("$__timeFilter({})", primary_timestamp), "Grafana time filter"),
    ];

    let mut found_correct = false;
    for (pattern, _) in &correct_patterns {
        if dashboard_json.contains(pattern) {
            found_correct = true;
            break;
        }
    }

    if found_correct {
        println!("  ✓ Raw Logs dashboard correctly uses '{}' timestamp column", primary_timestamp);
    } else {
        println!("  ⚠️  Warning: Raw Logs dashboard may not be querying timestamp column");
    }

    Ok(())
}

fn get_primary_timestamp_column(base: &str, bundle: &Bundle) -> Result<String, String> {
    if bundle.tables.is_empty() {
        return Err("No tables defined in bundle".to_string());
    }

    if bundle.tables[0].transforms.is_empty() {
        return Err("No transforms defined for first table".to_string());
    }

    let transform_path = format!("{}/{}", base, bundle.tables[0].transforms[0].path);

    let transform_json = std::fs::read_to_string(&transform_path)
        .map_err(|e| format!("Failed to read transform file {}: {}", transform_path, e))?;

    let transform: serde_json::Value = serde_json::from_str(&transform_json)
        .map_err(|e| format!("Failed to parse transform JSON: {}", e))?;

    // Find column with primary: true in datatype
    if let Some(columns) = transform["settings"]["output_columns"].as_array() {
        for col in columns {
            if let Some(datatype) = col["datatype"].as_object() {
                if datatype.get("primary").and_then(|v| v.as_bool()) == Some(true) {
                    if let Some(name) = col["name"].as_str() {
                        return Ok(name.to_string());
                    }
                }
            }
        }
    }

    Err(format!("No primary timestamp column found in transform {}", transform_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_detection() {
        let dashboard = r#"
        {
            "rawSql": "SELECT * FROM table WHERE timestamp > now() ORDER BY timestamp DESC"
        }
        "#;

        assert!(dashboard.contains("WHERE timestamp"));
        assert!(dashboard.contains("ORDER BY timestamp"));
    }
}
