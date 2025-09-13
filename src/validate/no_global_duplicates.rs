use std::collections::HashSet;

use crate::bundle_struct::Bundle;

pub fn run(bundles: &Vec<Bundle>) -> Result<(), String> {
    // Checking for duplicated bundle names
    {
        let mut tokens: HashSet<String> = HashSet::new();
        for b in bundles {
            if tokens.contains(&b.name) {
                return Err(format!(
                    "ERROR: {}.{} Duplicated-Bundle-Name url={} error={}",
                    file!(),
                    line!(),
                    b.base_url,
                    b.name,
                ));
            }
            tokens.insert(b.name.clone());
        }
    }

    // Checking for duplicated source names in the UI
    {
        let mut tokens: HashSet<String> = HashSet::new();
        for b in bundles {
            if tokens.contains(&b.ui.source.full_title) {
                return Err(format!(
                    "ERROR: {}.{} Duplicated-UI-Source-Name url={} error={}",
                    file!(),
                    line!(),
                    b.base_url,
                    b.ui.source.full_title
                ));
            }
            tokens.insert(b.ui.source.full_title.clone());
        }
    }

    // Checking for duplicated table names
    {
        let mut tokens: HashSet<String> = HashSet::new();
        for b in bundles {
            for t in &b.tables {
                if tokens.contains(&t.name) {
                    return Err(format!(
                        "ERROR: {}.{} Duplicated-Table-Name url={} error={}",
                        file!(),
                        line!(),
                        b.base_url,
                        t.name
                    ));
                }
                tokens.insert(t.name.clone());
            }
        }
    }

    Ok(())
}
