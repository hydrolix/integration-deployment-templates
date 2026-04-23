use crate::models::bundle::Bundle;
use serde_json::Value;
use tokio::fs;

pub async fn run(base: &str, bundle: &Bundle) -> Result<(), String> {
    for t in &bundle.tables {
        for tt in &t.transforms {
            let full_path = format!("{base}/{}", tt.path);

            let content = match fs::read_to_string(&full_path).await {
                Ok(v) => v,
                Err(e) => {
                    return Err(format!(
                        "ERROR: {}.{} Failed to read {full_path}: error={e}",
                        file!(),
                        line!()
                    ))
                }
            };

            let transform_json: Value = match serde_json::from_str(&content) {
                Ok(v) => v,
                Err(e) => {
                    return Err(format!(
                        "ERROR: {}.{} Failed to parse {full_path}: error={e}",
                        file!(),
                        line!()
                    ))
                }
            };

            let settings = match transform_json.get("settings") {
                Some(v) => v,
                None => continue,
            };
            let sample_data = match settings.get("sample_data") {
                Some(v) => v,
                None => continue,
            };

            if sample_data.is_array() {
                return Err(format!(
                    "sample_data for transform {} is a JSON array; deploy expects a single object. \
                     Run scripts/configure_bundle.py to normalize, or manually reduce to sample_data[0].",
                    tt.path
                ));
            }

            match sample_data.as_object() {
                Some(obj) if obj.is_empty() => {
                    return Err(format!(
                        "sample_data for transform {} is an empty object; deploy expects a non-empty single object.",
                        tt.path
                    ));
                }
                Some(_) => {}
                None => {
                    return Err(format!(
                        "sample_data for transform {} is {}; deploy expects a single object.",
                        tt.path,
                        describe(sample_data)
                    ));
                }
            }
        }
    }

    Ok(())
}

fn describe(v: &Value) -> &'static str {
    match v {
        Value::Null => "null/missing",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "a JSON array",
        Value::Object(_) => "a JSON object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::bundle::Bundle;
    use tokio::fs;

    fn make_bundle(transform_path: &str) -> Bundle {
        serde_json::from_str(&format!(
            r#"{{
                "name": "test-bundle",
                "source": "test",
                "method": "http_streaming",
                "beta": false,
                "base_url": "https://example.com/test",
                "dashboard": {{
                    "path": "dashboards/test.json",
                    "project_var": "__PROJECT_NAME__"
                }},
                "tables": [{{
                    "dashboard_var": "__TABLE_NAME__",
                    "name": "logs",
                    "transforms": [{{
                        "path": "{}"
                    }}]
                }}],
                "ui": {{
                    "primary_url": "https://example.com",
                    "method": {{ "full_title": "HTTP Streaming", "icon_url": "https://example.com/icon.png" }},
                    "source": {{ "full_title": "Test Source", "icon_url": "https://example.com/icon.png" }},
                    "data_category": "cdn"
                }},
                "metadata": {{
                    "version": "1.0.0",
                    "maintainer": "test",
                    "description": "test bundle",
                    "channel_type": "AWS"
                }}
            }}"#,
            transform_path
        ))
        .unwrap()
    }

    async fn write_transform(dir: &std::path::Path, transform_path: &str, contents: &str) {
        let full_path = dir.join(transform_path);
        fs::create_dir_all(full_path.parent().unwrap())
            .await
            .unwrap();
        fs::write(&full_path, contents).await.unwrap();
    }

    #[tokio::test]
    async fn non_empty_object_passes() {
        let dir = tempfile::tempdir().unwrap();
        let transform_path = "transformations/t.json";
        let json = r#"{
            "name": "t",
            "settings": {
                "sample_data": { "timestamp": 1700000000, "bytes": 1234 }
            }
        }"#;
        write_transform(dir.path(), transform_path, json).await;

        let bundle = make_bundle(transform_path);
        let result = run(dir.path().to_str().unwrap(), &bundle).await;

        assert!(result.is_ok(), "non-empty object should pass: {:?}", result);
    }

    #[tokio::test]
    async fn missing_sample_data_passes() {
        let dir = tempfile::tempdir().unwrap();
        let transform_path = "transformations/t.json";
        let json = r#"{
            "name": "t",
            "settings": {
                "output_columns": []
            }
        }"#;
        write_transform(dir.path(), transform_path, json).await;

        let bundle = make_bundle(transform_path);
        let result = run(dir.path().to_str().unwrap(), &bundle).await;

        assert!(
            result.is_ok(),
            "missing sample_data should pass (legitimate no-op): {:?}",
            result
        );
    }

    #[tokio::test]
    async fn empty_object_fails() {
        let dir = tempfile::tempdir().unwrap();
        let transform_path = "transformations/t.json";
        let json = r#"{
            "name": "t",
            "settings": {
                "sample_data": {}
            }
        }"#;
        write_transform(dir.path(), transform_path, json).await;

        let bundle = make_bundle(transform_path);
        let result = run(dir.path().to_str().unwrap(), &bundle).await;

        let err = result.expect_err("empty-object sample_data must fail");
        assert!(
            err.contains(transform_path),
            "error should name the offending transform path: {err}"
        );
        assert!(
            err.to_lowercase().contains("empty"),
            "error should mention emptiness: {err}"
        );
    }

    #[tokio::test]
    async fn scalar_fails() {
        let dir = tempfile::tempdir().unwrap();
        let transform_path = "transformations/t.json";
        let json = r#"{
            "name": "t",
            "settings": {
                "sample_data": 42
            }
        }"#;
        write_transform(dir.path(), transform_path, json).await;

        let bundle = make_bundle(transform_path);
        let result = run(dir.path().to_str().unwrap(), &bundle).await;

        let err = result.expect_err("scalar sample_data must fail");
        assert!(
            err.contains(transform_path),
            "error should name the offending transform path: {err}"
        );
        assert!(
            err.contains("single object"),
            "error should describe expected shape: {err}"
        );
    }

    #[tokio::test]
    async fn array_form_fails() {
        let dir = tempfile::tempdir().unwrap();
        let transform_path = "transformations/t.json";
        let json = r#"{
            "name": "t",
            "settings": {
                "sample_data": [{ "timestamp": 1700000000 }, { "timestamp": 1700000001 }]
            }
        }"#;
        write_transform(dir.path(), transform_path, json).await;

        let bundle = make_bundle(transform_path);
        let result = run(dir.path().to_str().unwrap(), &bundle).await;

        let err = result.expect_err("array-form sample_data must fail");
        assert!(
            err.contains("JSON array"),
            "error should mention JSON array: {err}"
        );
        assert!(
            err.contains("scripts/configure_bundle.py"),
            "error should suggest running configure_bundle.py: {err}"
        );
        assert!(
            err.contains(transform_path),
            "error should name the offending transform path: {err}"
        );
    }
}
