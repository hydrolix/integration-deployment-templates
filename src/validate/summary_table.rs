use std::collections::HashSet;

use crate::bundle::Bundle;

pub fn run(b: &Bundle) -> Result<(), String> {
    let summary_tables = match &b.summary_tables {
        Some(st) => st,
        None => return Ok(()),
    };

    // Build set of valid table names from this bundle
    let mut valid_tables: HashSet<String> = HashSet::new();
    for t in &b.tables {
        valid_tables.insert(t.name.clone());
    }

    // Check for duplicate dashboard_var in summary tables
    let mut dashboard_vars: HashSet<String> = HashSet::new();
    for st in summary_tables {
        if dashboard_vars.contains(&st.dashboard_var) {
            return Err(format!(
                "ERROR: {}.{} Duplicated-Summary-Dashboard-Var bundle={} summary_table={} dashboard_var={} url={}",
                file!(),
                line!(),
                b.name,
                st.name,
                st.dashboard_var,
                b.base_url
            ));
        }
        dashboard_vars.insert(st.dashboard_var.clone());
    }

    // Check each summary table references a valid parent
    for st in summary_tables {
        if !valid_tables.contains(&st.parent_table_name) {
            return Err(format!(
                "ERROR: {}.{} Invalid-Parent-Table-Reference bundle={} summary_table={} parent_table_name={} url={}",
                file!(),
                line!(),
                b.name,
                st.name,
                st.parent_table_name,
                b.base_url
            ));
        }
    }

    Ok(())
}
