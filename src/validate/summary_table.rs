use std::collections::HashSet;

use crate::bundle_struct::Bundle;

pub fn run(b: &Bundle) -> Result<(), String> {
   
        if b.summary_tables.is_none() {
            return Ok(());
        }

        // Build set of valid table names from this bundle
        let mut valid_tables: HashSet<String> = HashSet::new();
        for t in &b.tables {
            valid_tables.insert(t.name.clone());
        }

        // Check each summary table references a valid parent
        for st in b.summary_tables.as_ref().unwrap() {
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