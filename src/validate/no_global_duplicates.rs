use std::collections::{HashMap, HashSet};

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

    // Checking for duplicated names
    let mut tokens: HashMap<String, usize> = HashMap::new();
    for b in bundles {
        *tokens.entry(b.name.clone()).or_insert(0) += 1;

        for t in &b.tables {
            *tokens.entry(t.name.clone()).or_insert(0) += 1;
        }
        *tokens.entry(b.ui.source.full_title.clone()).or_insert(0) += 1;
        *tokens.entry(b.base_url.clone()).or_insert(0) += 1;
    }

    // Check for duplicates
    for (name, count) in &tokens {
        if *count > 1 {
            return Err(format!(
                "ERROR: {}.{} Duplicated-Name count={} table={}",
                file!(),
                line!(),
                count,
                name
            ));
        }
    }
    Ok(())
}
