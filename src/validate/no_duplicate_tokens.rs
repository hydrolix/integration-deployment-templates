use std::collections::HashSet;

use crate::models::bundle::Bundle;

const MIN_TABLE_NAME: usize = 3;

pub async fn run(bundle: &Bundle) -> Result<(), String> {
    let mut tables: HashSet<String> = HashSet::new();

    // Check every table name...
    for t in &bundle.tables {
        // Validate table name starts with a letter
        if t.name.is_empty() || t.name.len() < MIN_TABLE_NAME {
            return Err(format!(
                "ERROR: {}.{} Missing or truncated table name '{:?}'",
                file!(),
                line!(),
                t
            ));
        }

        if let Some(first_char) = t.name.chars().next() {
            if !first_char.is_alphabetic() {
                return Err(format!(
                    "ERROR: {}.{} Invalid table name '{}' - must start with a letter",
                    file!(),
                    line!(),
                    t.name
                ));
            }
        }

        // Validate table name contains only alphanumeric characters and underscores
        if !t.name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(format!(
            "ERROR: {}.{} Invalid table name '{}' - only letters, digits, and underscores allowed",
            file!(),
            line!(),
            t.name
        ));
        }

        if tables.contains(&t.name) {
            return Err(format!(
                "ERROR: {}.{} Duplicate table name {}",
                file!(),
                line!(),
                t.name
            ));
        }
        tables.insert(t.name.to_string());
    }
    println!("tables={:?}", tables);

    // Check for duplicate database_var values
    let mut tokens: HashSet<String> = HashSet::new();
    for t in &bundle.tables {
        if tokens.contains(&t.dashboard_var) {
            return Err(format!(
                "ERROR: {}.{} Duplicate database_var {}",
                file!(),
                line!(),
                t.dashboard_var
            ));
        }
        tokens.insert(t.dashboard_var.to_string());
    }

    Ok(())
}
