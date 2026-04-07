use std::collections::HashMap;

use crate::models::bundle::Bundle;

pub fn run(bundles: &Vec<Bundle>) -> Result<(), String> {
    // Checking for duplicated bundle names
    {
        let mut seen: HashMap<String, String> = HashMap::new(); // name -> base_url
        for b in bundles {
            if seen.contains_key(&b.name) {
                return Err(format!(
                    "ERROR: {}.{} Duplicated-Bundle-Name url={} error={}",
                    file!(),
                    line!(),
                    b.base_url,
                    b.name,
                ));
            }
            seen.insert(b.name.clone(), b.base_url.clone());
        }
    }

    // Checking for duplicated source names in the UI
    {
        let mut seen: HashMap<String, String> = HashMap::new(); // full_title -> bundle name
        for b in bundles {
            if let Some(existing_name) = seen.get(&b.ui.source.full_title) {
                if existing_name != &b.name {
                    return Err(format!(
                        "ERROR: {}.{} Duplicated-UI-Source-Name url={} error={}",
                        file!(),
                        line!(),
                        b.base_url,
                        b.ui.source.full_title
                    ));
                }
            } else {
                seen.insert(b.ui.source.full_title.clone(), b.name.clone());
            }
        }
    }

    // Checking for duplicated base_urls (each must be unique)
    {
        let mut seen: HashMap<String, usize> = HashMap::new();
        for b in bundles {
            *seen.entry(b.base_url.clone()).or_insert(0) += 1;
        }
        for (url, count) in &seen {
            if *count > 1 {
                return Err(format!(
                    "ERROR: {}.{} Duplicated-Base-URL count={} url={}",
                    file!(),
                    line!(),
                    count,
                    url
                ));
            }
        }
    }

    Ok(())
}
