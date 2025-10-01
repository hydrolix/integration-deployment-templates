use crate::bundle_struct::Bundle;
use serde_json::Value;
use tokio::fs;

pub async fn run(base: &str, bundle: &Bundle) -> Result<(), String> {
    for t in &bundle.tables {
        for tt in &t.transforms {
            let full_path = format!("{base}/{}", tt.path);

            let content = match fs::read_to_string(full_path.clone()).await {
                Ok(v) => v.to_string(),
                Err(e) => {
                    return Err(format!(
                        "ERROR: {}.{} Failed to read full_path={full_path}: error={e}",
                        file!(),
                        line!()
                    ))
                }
            };

            let transform_json: Value = match serde_json::from_str(&content) {
                Ok(v) => v,
                Err(e) => {
                    return Err(format!(
                        "ERROR: {}.{} Failed to read full_path={full_path}: error={e}",
                        file!(),
                        line!()
                    ))
                }
            };

            let sample_data = &transform_json["settings"]["sample_data"];

            // Check if it's a non-empty object
            if sample_data.is_object() && sample_data.as_object().is_some() {
                continue;
            }

            // Check if it's a non-empty string
            if sample_data.as_str().is_some() {
                continue;
            }

            return Err(format!(
                "ERROR: {}.{} No Sample data in transformation full_path={full_path}",
                file!(),
                line!()
            ));
        }
    }

    Ok(())
}
