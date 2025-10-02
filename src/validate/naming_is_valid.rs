use crate::bundle_struct::Bundle;

pub async fn run(bundle: &Bundle) -> Result<(), String> {
    // Check method consistency
    let method = &bundle.method;
    let method_title = &bundle.ui.method.full_title;

    let expected_titles = match method.as_str() {
        "firehose" => vec![
            "Amazon Data Firehose",
            "AWS Firehose",
            "Kinesis Data Firehose",
        ],
        "s3" => vec!["Amazon S3", "AWS S3"],
        "kinesis" => vec!["Amazon Kinesis", "AWS Kinesis"],
        _ => vec![],
    };

    if !expected_titles.is_empty()
        && !expected_titles
            .iter()
            .any(|&title| method_title.contains(title))
    {
        return Err(format!(
            "ERROR: {}.{} docs.method.full_title '{method_title}' does not match method '{method}'",
            file!(),
            line!()
        ));
    }

    let source = &bundle.source;
    let source_title = &bundle.ui.source.full_title;

    if source == "waf" && !source_title.to_lowercase().contains("waf") {
        return Err(format!(
            "ERROR: {}.{} Source title should contain 'WAF' when source is 'waf'",
            file!(),
            line!()
        ));
    }

    // Check name consistency with source and method
    let name = &bundle.name;

    if !name.to_lowercase().contains(&source.to_lowercase())
        || !name.to_lowercase().contains(&method.to_lowercase())
    {
        return Err(format!(
            "ERROR: {}.{} Name '{name}' must include '{source}' and '{method}'",
            file!(),
            line!()
        ));
    }

    // Check version format (basic semantic versioning)
    let version = &bundle.metadata.version;
    if version.matches('.').count() != 2 {
        return Err(format!(
            "ERROR: {}.{} Version {version} should follow semantic versioning format (e.g., 1.0.0)",
            file!(),
            line!()
        ));
    }

    // Check maintainer email format
    let maintainer = &bundle.metadata.maintainer;
    if !maintainer.contains('@') || !maintainer.contains('.') {
        return Err(format!(
            "ERROR: {}.{} Maintainer should be a valid email address",
            file!(),
            line!()
        ));
    }

    // Check description is not empty
    if bundle.metadata.description.trim().is_empty() {
        return Err(format!(
            "ERROR: {}.{} Description cannot be empty",
            file!(),
            line!()
        ));
    }

    Ok(())
}
