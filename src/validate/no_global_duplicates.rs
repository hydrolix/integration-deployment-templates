use std::collections::HashSet;

use crate::bundle_struct::Bundle;

pub fn run(bundles: &Vec<Bundle>) -> Result<(), String> {
    let mut tables: HashSet<String> = HashSet::new();
    let mut titles: HashSet<String> = HashSet::new();

    // Checking for duplicated source names in the UI
    for b in bundles {
        if titles.contains(&b.ui.source.full_title) {
            return Err(format!(
                "ERROR: {}.{} Duplicated UI source name in bundle name={} error={}",
                file!(),
                line!(),
                b.name,
                b.ui.source.full_title
            ));
        }
        titles.insert(b.ui.source.full_title.clone());
    }

    // Checking for duplicated table names
    for b in bundles {
        for t in &b.tables {
            if tables.contains(&t.name) {
                return Err(format!(
                    "ERROR: {}.{} Duplicated table name in bundle name={} error={}",
                    file!(),
                    line!(),
                    b.name,
                    t.name
                ));
            }
            tables.insert(t.name.clone());
        }
    }

    Ok(())
}
