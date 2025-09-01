use serde_json::Value;
use tokio::fs;

use crate::bundle_struct::Bundle;

pub async fn run(base: &str, bundle: &Bundle) -> Result<(), String> {
    let full_path = format!("{base}/{}", bundle.dashboard.path);

    let content = match fs::read_to_string(&full_path).await {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Failed to read file_path={full_path}: error={e}",
                file!(),
                line!()
            ));
        }
    };

    let mut must_have: Vec<String> = vec![];
    must_have.push("__DASHBOARD_UUID__".to_string());
    must_have.push("__DATASOURCE__".to_string());
    must_have.push("__PROJECT_NAME__".to_string());

    for t in &bundle.tables {
        must_have.push(t.dashboard_var.clone());
    }

    for m in &must_have {
        if !content.contains(m) {
            return Err(format!(
                "ERROR: {}.{} Dashboard Must have {m} full_path={full_path}",
                file!(),
                line!()
            ));
        }
    }

    let dashboard_json: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Invalid JSON full_path={full_path} error={e}",
                file!(),
                line!()
            ));
        }
    };

    match dashboard_json["dashboard"].as_object() {
        Some(_) => (),
        None => {
            return Err(format!(
                "ERROR: {}.{} Invalid dashboard - top element must be dashboard.full_path={full_path}",
                file!(),
                line!()
            ));
        }
    }

    if dashboard_json["id"].as_str().is_some() {
        return Err(format!(
            "ERROR: {}.{} Invalid dashboard - cannot had Id set. full_path={full_path}",
            file!(),
            line!()
        ));
    }

    Ok(())
}
