use std::collections::HashSet;

use crate::bundle_struct::Bundle;

pub async fn run(bundle: &Bundle) -> Result<(), String> {
    let mut tokens: HashSet<String> = HashSet::new();

    // Check for duplicate table names
    for t in &bundle.tables {
        if tokens.contains(&t.name) {
            return Err(format!(
                "ERROR: {}.{} Duplicate table name {}",
                file!(),
                line!(),
                t.name
            ));
        }
        tokens.insert(t.name.to_string());
    }

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
