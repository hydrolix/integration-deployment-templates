use crate::bundle::Bundle;
use serde_json::Value;
use std::collections::HashSet;
use tokio::fs;

pub async fn run(base: &str, bundle: &Bundle) -> Result<(), String> {
    for table in &bundle.tables {
        let mut transform_names: HashSet<String> = HashSet::new();

        for transform in &table.transforms {
            let full_path = format!("{base}/{}", transform.path);

            // Read the transform file
            let content = match fs::read_to_string(&full_path).await {
                Ok(v) => v,
                Err(e) => {
                    return Err(format!(
                        "ERROR: {}.{} Failed to read transform file: path={full_path} error={e}",
                        file!(),
                        line!()
                    ));
                }
            };

            // Parse as JSON
            let transform_json: Value = match serde_json::from_str(&content) {
                Ok(v) => v,
                Err(e) => {
                    return Err(format!(
                        "ERROR: {}.{} Transform file is not valid JSON: path={full_path} error={e}",
                        file!(),
                        line!()
                    ));
                }
            };

            // Check for non-empty name field
            let this_name = match transform_json.get("name") {
                Some(name_value) => match name_value.as_str() {
                    Some(name_str) => {
                        if name_str.trim().is_empty() {
                            return Err(format!(
                    "ERROR: {}.{} Transform file has empty 'name' field: path={full_path}",
                    file!(),
                    line!()
                ));
                        }
                        name_str // Return the string if valid
                    }
                    None => {
                        return Err(format!(
                "ERROR: {}.{} Transform file 'name' field is not a string: path={full_path}",
                file!(),
                line!()
            ));
                    }
                },
                None => {
                    return Err(format!(
            "ERROR: {}.{} Transform file missing required 'name' field: path={full_path}",
            file!(),
            line!()
        ));
                }
            };

            if transform_names.contains(this_name) {
                return Err(format!(
                    "ERROR: {}.{} Duplicated transform name '{this_name}' path={full_path}",
                    file!(),
                    line!()
                ));
            }
            transform_names.insert(this_name.to_string());

            // Check subtype if present
            if let Some(subtype_value) = transform_json.get("subtype") {
                match subtype_value.as_str() {
                    Some(subtype_str) => {
                        if subtype_str != "firehose" {
                            return Err(format!(
                                "ERROR: {}.{} Transform file has invalid subtype '{}', must be 'firehose': path={full_path}",
                                file!(),
                                line!(),
                                subtype_str
                            ));
                        }
                    }
                    None => {
                        return Err(format!(
                            "ERROR: {}.{} Transform file 'subtype' field is not a string: path={full_path}",
                            file!(),
                            line!()
                        ));
                    }
                }
            }
        }
    }

    Ok(())
}
