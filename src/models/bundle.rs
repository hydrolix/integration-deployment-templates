use serde::{de, Deserialize, Serialize};
use url::Url;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    #[serde(deserialize_with = "deserialize_valid_name")]
    pub name: String,
    #[serde(deserialize_with = "deserialize_valid_source")]
    pub source: String,
    #[serde(deserialize_with = "deserialize_valid_method")]
    pub method: String,
    pub method_overrides: Option<MethodOverrides>,
    #[serde(default)]
    pub solution: bool,
    pub beta: bool,
    #[serde(deserialize_with = "deserialize_https_url")]
    pub base_url: String,
    pub dashboard: Dashboard,
    pub other_dashboards: Option<Vec<Dashboard>>,
    pub tables: Vec<Table>,
    pub summary_tables: Option<Vec<SummaryTable>>,
    pub ui: Ui,
    pub metadata: Metadata,
    #[serde(default)]
    pub dependencies: Option<Dependencies>,
    pub alert_rules: Option<AlertRules>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Dashboard {
    #[serde(deserialize_with = "deserialize_url_path")]
    pub path: String,
    pub project_var: String,
    #[serde(default, deserialize_with = "deserialize_optional_sha256")]
    pub sha256: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct AlertRules {
    #[serde(deserialize_with = "deserialize_url_path")]
    pub path: String,
    #[serde(default, deserialize_with = "deserialize_optional_sha256")]
    pub sha256: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct MethodOverrides {
    pub region: Option<String>,
    pub stream_prefix: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Table {
    #[serde(deserialize_with = "deserialize_macro_name")]
    pub dashboard_var: String,
    pub name: String,
    pub transforms: Vec<Transform>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct SummarySqlFile {
    #[serde(deserialize_with = "deserialize_url_path")]
    pub path: String,
    #[serde(default, deserialize_with = "deserialize_optional_sha256")]
    pub sha256: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct SummaryTable {
    pub name: String,
    pub parent_table_name: String,
    #[serde(deserialize_with = "deserialize_macro_name")]
    pub dashboard_var: String,
    pub sql: SummarySqlFile,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Transform {
    #[serde(deserialize_with = "deserialize_url_path")]
    pub path: String,
    #[serde(default, deserialize_with = "deserialize_optional_sha256")]
    pub sha256: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_url_path")]
    pub sample: Option<String>,
    pub method: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Ui {
    #[serde(deserialize_with = "deserialize_https_url")]
    pub primary_url: String,
    pub method: Graphics,
    pub source: Graphics,
    #[serde(deserialize_with = "deserialize_valid_data_categories")]
    pub data_category: String,
    #[serde(default, deserialize_with = "deserialize_optional_folder")]
    pub folder: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_subfolder")]
    pub subfolder: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Graphics {
    pub full_title: String,
    #[serde(deserialize_with = "deserialize_https_url")]
    pub icon_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub version: String,
    pub maintainer: String,
    pub description: String,
    #[serde(deserialize_with = "deserialize_valid_channel_type")]
    pub channel_type: String,
}

// New structs for dependencies
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Dependencies {
    pub grafana: Option<GrafanaDependencies>,
    pub hydrolix: Option<HydrolixDependencies>,
    #[serde(default)]
    #[serde(rename = "data-sources")]
    pub data_sources: Option<Vec<DataSource>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct GrafanaDependencies {
    pub version: Option<String>,
    pub plugins: Option<Vec<GrafanaPlugin>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct GrafanaPlugin {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct HydrolixDependencies {
    #[serde(rename = "cluster_version")]
    pub cluster_version: Option<String>,
    #[serde(rename = "required_dictionaries")]
    pub required_dictionaries: Option<Vec<String>>,
    #[serde(rename = "required_functions")]
    pub required_functions: Option<Vec<String>>,
    #[serde(rename = "shared_dictionaries")]
    pub shared_dictionaries: Option<Vec<String>>,
    #[serde(rename = "shared_functions")]
    pub shared_functions: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct DataSource {
    pub name: String,
    #[serde(rename = "type")]
    pub data_source_type: String,
    pub url: String,
    pub access: String,
}

// Custom deserializer for HTTPS URLs
fn deserialize_https_url<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s = match String::deserialize(deserializer) {
        Ok(v) => v,
        Err(e) => {
            return Err(de::Error::custom(format!(
                "failed to deserialize url: {}",
                e
            )))
        }
    };

    // Validate URL format
    match Url::parse(&s) {
        Ok(_) => (),
        Err(e) => return Err(de::Error::custom(format!("Failed to parse URL: {}", e))),
    }

    // Ensure HTTPS
    if !s.starts_with("https://") && !s.starts_with("file://") {
        return Err(de::Error::custom("URL must start with https:// or file://"));
    }
    Ok(s)
}

// Custom deserializer for HTTP path
fn deserialize_url_path<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s = match String::deserialize(deserializer) {
        Ok(v) => v,
        Err(e) => {
            return Err(de::Error::custom(format!(
                "failed to deserialize url path: {}",
                e
            )))
        }
    };

    if s.starts_with("/") || s.contains("..") {
        return Err(de::Error::custom(
            "Path cannot start with slash or contain 2 dots".to_string(),
        ));
    }

    if !s.to_lowercase().ends_with(".json")
        && !s.to_lowercase().ends_with(".tsv")
        && !s.to_lowercase().ends_with(".sql")
    {
        return Err(de::Error::custom(
            "Path must end in .json or .tsv or .sql".to_string(),
        ));
    }

    Ok(s)
}

// Custom __VARIABLE__ name
fn deserialize_macro_name<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s = match String::deserialize(deserializer) {
        Ok(v) => v,
        Err(e) => {
            return Err(de::Error::custom(format!(
                "failed to deserialize macro name: {}",
                e
            )))
        }
    };

    // Must be at least 5 characters: "__X__"
    if s.len() < 5 {
        return Err(de::Error::custom(format!(
            "{s} Must be in format __VARIABLE_NAME__"
        )));
    }

    if !s.starts_with("__") || !s.ends_with("__") {
        return Err(de::Error::custom(format!(
            "{s} Must be in format __VARIABLE_NAME__"
        )));
    }

    // Extract the inner content (between the double underscores)
    let inner = &s[2..s.len() - 2];

    // Inner content cannot be empty
    if inner.is_empty() {
        return Err(de::Error::custom(format!(
            "{s} Empty inside __VARIABLE_NAME__"
        )));
    }

    // Check each character in the inner content
    for ch in inner.chars() {
        match ch {
            'A'..='Z' => continue, // Uppercase letters are allowed
            '1'..='9' => continue, // Uppercase letters are allowed
            '_' => continue,       // Single underscores are allowed
            _ => {
                return Err(de::Error::custom(format!(
                    "{s} Must be upper-case or a numeral __VARIABLE_NAME__"
                )));
            }
        }
    }

    // Make sure there are no consecutive underscores (double underscores)
    if inner.contains("__") {
        return Err(de::Error::custom(format!(
            "{s} Inside there are no double underscores"
        )));
    }

    Ok(s)
}

#[allow(dead_code)]
// Custom deserializer
fn deserialize_ends_in_json<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s = match String::deserialize(deserializer) {
        Ok(v) => v,
        Err(e) => {
            return Err(de::Error::custom(format!(
                "failed to deserialize json path: {}",
                e
            )))
        }
    };

    match s.ends_with(".json") {
        true => Ok(s),
        false => Err(de::Error::custom(format!("{s} Must end in .json"))),
    }
}

// Custom deserializer
fn deserialize_valid_method<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s = match String::deserialize(deserializer) {
        Ok(v) => v,
        Err(e) => {
            return Err(de::Error::custom(format!(
                "failed to deserialize method: {}",
                e
            )))
        }
    };

    match s.as_str() {
        // Convert String to &str for matching
        "firehose" | "s3" | "kinesis" | "lambda" | "http_streaming" | "http" | "multi_stream" => {
            Ok(s)
        }
        _ => Err(de::Error::custom(format!("{} is an invalid method", s))),
    }
}

// Custom deserializer
fn deserialize_valid_source<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s = match String::deserialize(deserializer) {
        Ok(v) => v,
        Err(e) => {
            return Err(de::Error::custom(format!(
                "failed to deserialize source: {}",
                e
            )))
        }
    };

    if s.chars()
        .any(|c| !c.is_alphanumeric() && c != '-' && c != '_')
    {
        return Err(de::Error::custom(format!(
            "{} contains invalid characters (only alphanumeric, dashes, and underscores allowed)",
            s
        )));
    }

    Ok(s)
}

// Custom deserializer for channel type
fn deserialize_valid_channel_type<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s = match String::deserialize(deserializer) {
        Ok(v) => v,
        Err(e) => {
            return Err(de::Error::custom(format!(
                "failed to deserialize channel type: {}",
                e
            )))
        }
    };

    match s.as_str() {
        "AWS" | "Azure" | "GCP" | "3rdParty" | "Internal" => Ok(s),
        _ => Err(de::Error::custom(format!(
            "{} is an invalid channel_type",
            s
        ))),
    }
}

// Custom deserializer
fn deserialize_valid_data_categories<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s = match String::deserialize(deserializer) {
        Ok(v) => v,
        Err(e) => {
            return Err(de::Error::custom(format!(
                "failed to deserialize data category: {}",
                e
            )))
        }
    };

    match s.as_str() {
        // Convert String to &str for matching
        "video" | "cdn" | "security" => Ok(s),
        _ => Err(de::Error::custom(format!(
            "{} is an invalid data category",
            s
        ))),
    }
}

// Custom deserializer for optional SHA256
fn deserialize_optional_sha256<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: de::Deserializer<'de>,
{
    let opt = match Option::<String>::deserialize(deserializer) {
        Ok(Some(v)) => Some(v),
        Ok(None) => None,
        Err(e) => return Err(e),
    };

    match opt {
        Some(s) => {
            if s.len() != 64 {
                Err(de::Error::custom(format!(
                    "{s} needs to be 64 characters long"
                )))
            } else if !s.chars().all(|c| c.is_ascii_hexdigit()) {
                Err(de::Error::custom(format!(
                    "{s} must contain only hexadecimal characters"
                )))
            } else {
                Ok(Some(s))
            }
        }
        None => Ok(None),
    }
}

fn deserialize_valid_name<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s = match String::deserialize(deserializer) {
        Ok(v) => v,
        Err(e) => {
            return Err(de::Error::custom(format!(
                "failed to deserialize name: {}",
                e
            )))
        }
    };

    match s.as_str() {
        s_str
            if s_str
                .chars()
                .any(|c| !c.is_alphanumeric() && c != '_' && c != '-') =>
        {
            return Err(de::Error::custom(format!(
                "{} contains invalid characters (only alphanumeric characters, underscores, and dashes allowed)",
                s
            )));
        }
        _ => (),
    }

    Ok(s)
}

// Custom deserializer for optional SHA256
fn deserialize_optional_url_path<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: de::Deserializer<'de>,
{
    let opt = match Option::<String>::deserialize(deserializer) {
        Ok(Some(v)) => Some(v),
        Ok(None) => None,
        Err(e) => return Err(e),
    };

    match opt {
        Some(s) => {
            if s.starts_with("/") || s.contains("..") {
                return Err(de::Error::custom(
                    "Path cannot start with slash or contain 2 dots".to_string(),
                ));
            }

            if !s.to_lowercase().ends_with(".json") && !s.to_lowercase().ends_with(".tsv") {
                return Err(de::Error::custom(
                    "Path must end in .json or .tsv".to_string(),
                ));
            }
            Ok(Some(s))
        }
        None => Ok(None),
    }
}

fn deserialize_optional_folder<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: de::Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    if let Some(ref s) = opt {
        match s.as_str() {
            "api-context" | "cdn" | "dns" | "media" | "security" => {}
            _ => return Err(de::Error::custom(format!("{} is an invalid folder", s))),
        }
    }
    Ok(opt)
}

fn deserialize_optional_subfolder<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: de::Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    if let Some(ref s) = opt {
        match s.as_str() {
            "multi-cdn" | "bots" | "ds2" | "siem" => {}
            _ => return Err(de::Error::custom(format!("{} is an invalid subfolder", s))),
        }
    }
    Ok(opt)
}

// Helper functions to extract functions and dictionaries
impl Bundle {
    /// Get all functions categorized by type (bundle-specific vs shared)
    pub fn get_all_functions(&self) -> (Vec<String>, Vec<String>) {
        let bundle_specific = self
            .dependencies
            .as_ref()
            .and_then(|d| d.hydrolix.as_ref())
            .and_then(|h| h.required_functions.as_ref())
            .cloned()
            .unwrap_or_default();

        let shared = self
            .dependencies
            .as_ref()
            .and_then(|d| d.hydrolix.as_ref())
            .and_then(|h| h.shared_functions.as_ref())
            .cloned()
            .unwrap_or_default();

        (bundle_specific, shared)
    }

    /// Get all dictionaries categorized by type (bundle-specific vs shared)
    pub fn get_all_dictionaries(&self) -> (Vec<String>, Vec<String>) {
        let bundle_specific = self
            .dependencies
            .as_ref()
            .and_then(|d| d.hydrolix.as_ref())
            .and_then(|h| h.required_dictionaries.as_ref())
            .cloned()
            .unwrap_or_default();

        let shared = self
            .dependencies
            .as_ref()
            .and_then(|d| d.hydrolix.as_ref())
            .and_then(|h| h.shared_dictionaries.as_ref())
            .cloned()
            .unwrap_or_default();

        (bundle_specific, shared)
    }
}
