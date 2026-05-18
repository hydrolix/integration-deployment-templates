pub mod auth;
pub mod dependencies;
pub mod dictionaries;
pub mod functions;
pub mod shared_proj;
pub mod table;

use lazy_static::lazy_static;
use reqwest::Client;
use serde_json::Value;
use std::sync::OnceLock;
use std::time::Duration;

// These are static but not secret
// const ORG_UUID_MARK: &str = "d867bf48-4281-4496-8432-a93aa989aae6";  // markeplace-dev
// const ORG_UUID_SAND: &str = "b646d78a-5fb2-4d5f-afef-b705bf185174";  // partnersandbox
const REMOTE_ORG_UUID_DEFAULT: &str = "c1834e11-9716-4971-9a5f-d8c07f4f6b3a"; // demo.trafficpeak.live
const REMOTE_HYDROLIX_CLUSTER_DEFAULT: &str = "demo.trafficpeak.live";
const ORG_UUID: &str = "25a47323-9d41-40f7-b2a2-de3fe500135a"; // hdx-se-playpen.hydrolix.live
                                                               // const PROJ_UUID_MARK: &str = "67e79a3c-f7d6-4b33-a207-fef4579a3152";  //  markeplace-dev cdn_test_project
                                                               // const PROJ_UUID_SAND: &str = "469dbd34-6f06-4dfe-8fd1-9adf82123ecf";  // partnersandbox
const PROJ_UUID: &str = "3338ec5e-39f5-496d-a36d-d0ae97216ecb"; // hdx-se-playpen.hydrolix.live
                                                                // const PROJ_NAME: &str = "cdn_test_project";
const PROJ_NAME: &str = "bundle_verification";

// const ORG_UUID_SAND: &str = "b646d78a-5fb2-4d5f-afef-b705bf185174"; // partnersandbox
// const PROJ_UUID_SAND: &str = "469dbd34-6f06-4dfe-8fd1-9adf82123ecf"; // partnersandbox

const HTTP_TIMEOUT: u64 = 120;

lazy_static! {
    static ref CLIENT: Client = reqwest::Client::new();
    static ref BUNDLE_TESTING_CLUSTER: String =
        std::env::var("BUNDLE_TESTING_CLUSTER").unwrap_or_else(|_| "".to_string());
    static ref BUNDLE_TESTING_USERNAME: String =
        std::env::var("BUNDLE_TESTING_USERNAME").unwrap_or_else(|_| "".to_string());
    static ref BUNDLE_TESTING_PASSWORD: String =
        std::env::var("BUNDLE_TESTING_PASSWORD").unwrap_or_else(|_| "".to_string());
    /// Overrides BUNDLE_TESTING_CLUSTER when --remote is active.
    /// Set REMOTE_HYDROLIX_CLUSTER to the demo cluster hostname (e.g. demo.trafficpeak.live).
    static ref REMOTE_HYDROLIX_CLUSTER: String =
        std::env::var("REMOTE_HYDROLIX_CLUSTER").unwrap_or_else(|_| "".to_string());
    /// Overrides the hardcoded ORG_UUID when --remote is active.
    /// Set REMOTE_ORG_UUID to the org UUID on the demo cluster.
    static ref REMOTE_ORG_UUID: String =
        std::env::var("REMOTE_ORG_UUID").unwrap_or_else(|_| "".to_string());
    /// Credentials for the remote (demo) cluster. Only used in --remote mode.
    static ref REMOTE_HYDROLIX_USERNAME: String =
        std::env::var("REMOTE_HYDROLIX_USERNAME").unwrap_or_else(|_| "".to_string());
    static ref REMOTE_HYDROLIX_PASSWORD: String =
        std::env::var("REMOTE_HYDROLIX_PASSWORD").unwrap_or_else(|_| "".to_string());
    static ref IS_REMOTE: bool = {
        let args: Vec<String> = std::env::args().collect();
        args.contains(&"--remote".to_string())
    };
    static ref FOR_MARKETPLACE: bool = {
        let args: Vec<String> = std::env::args().collect();
        args.contains(&"--marketplace".to_string())
    };
}

/// Returns the active Hydrolix cluster hostname. In --remote mode, uses
/// REMOTE_HYDROLIX_CLUSTER if set, otherwise demo.trafficpeak.live.
/// In all other modes, uses BUNDLE_TESTING_CLUSTER.
pub fn cluster() -> &'static str {
    if *IS_REMOTE {
        if !REMOTE_HYDROLIX_CLUSTER.is_empty() {
            &REMOTE_HYDROLIX_CLUSTER
        } else {
            REMOTE_HYDROLIX_CLUSTER_DEFAULT
        }
    } else {
        &BUNDLE_TESTING_CLUSTER
    }
}

/// Returns the active cluster username.
pub fn username() -> &'static str {
    if *IS_REMOTE && !REMOTE_HYDROLIX_USERNAME.is_empty() {
        &REMOTE_HYDROLIX_USERNAME
    } else {
        &BUNDLE_TESTING_USERNAME
    }
}

/// Returns the active cluster password.
pub fn password() -> &'static str {
    if *IS_REMOTE && !REMOTE_HYDROLIX_PASSWORD.is_empty() {
        &REMOTE_HYDROLIX_PASSWORD
    } else {
        &BUNDLE_TESTING_PASSWORD
    }
}

/// Returns the active org UUID. In --remote mode, uses REMOTE_ORG_UUID if set,
/// otherwise the built-in demo.trafficpeak.live default. In all other modes,
/// uses the demo.aws.hydrolix.live default (overridable via the same var for
/// edge cases, though that's uncommon).
pub fn org_uuid() -> &'static str {
    if *IS_REMOTE {
        if !REMOTE_ORG_UUID.is_empty() {
            &REMOTE_ORG_UUID
        } else {
            REMOTE_ORG_UUID_DEFAULT
        }
    } else {
        ORG_UUID
    }
}

/// Dynamic project overrides set by --guid at startup.
static GUID_PROJECT_NAME: OnceLock<String> = OnceLock::new();
static GUID_PROJECT_UUID: OnceLock<String> = OnceLock::new();

/// Set the dynamic project name and UUID (called once at startup when --guid is used).
pub fn set_guid_project(name: String, uuid: String) {
    GUID_PROJECT_NAME
        .set(name)
        .expect("GUID project name already set");
    GUID_PROJECT_UUID
        .set(uuid)
        .expect("GUID project UUID already set");
}

pub fn get_project_name() -> String {
    GUID_PROJECT_NAME
        .get()
        .cloned()
        .unwrap_or_else(|| PROJ_NAME.to_string())
}

pub fn get_project_uuid() -> String {
    GUID_PROJECT_UUID
        .get()
        .cloned()
        .unwrap_or_else(|| PROJ_UUID.to_string())
}

/// Create a new project on the cluster. Returns the project UUID.
pub async fn create_project(bearer_token: &str, name: &str) -> Result<String, String> {
    let url = format!(
        "https://{}/config/v1/orgs/{}/projects/",
        cluster(), org_uuid()
    );

    let payload = serde_json::json!({
        "name": name,
        "description": format!("GUID test project for bundle validation ({})", name),
    });

    let response = CLIENT
        .post(&url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .header("Content-Type", "application/json")
        .timeout(Duration::from_secs(HTTP_TIMEOUT))
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to create project: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!(
            "HTTP {} creating project '{}': {}",
            status, name, body
        ));
    }

    let result: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse create project response: {}", e))?;

    result
        .get("uuid")
        .and_then(|u| u.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "No UUID in create project response".to_string())
}

/// Find a project UUID by name. Returns Err if not found.
pub async fn find_project_uuid(bearer_token: &str, name: &str) -> Result<String, String> {
    let url = format!(
        "https://{}/config/v1/orgs/{}/projects/",
        cluster(), org_uuid()
    );

    let response = CLIENT
        .get(&url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .timeout(Duration::from_secs(HTTP_TIMEOUT))
        .send()
        .await
        .map_err(|e| format!("Failed to list projects: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to list projects: {}", response.status()));
    }

    let projects: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse projects response: {}", e))?;

    let empty_vec = vec![];
    let list = if projects.is_array() {
        projects.as_array().unwrap()
    } else if let Some(results) = projects.get("results") {
        results.as_array().unwrap_or(&empty_vec)
    } else if let Some(data) = projects.get("data") {
        data.as_array().unwrap_or(&empty_vec)
    } else {
        &empty_vec
    };

    for project in list {
        if let Some(pname) = project.get("name").and_then(|n| n.as_str()) {
            if pname == name {
                if let Some(uuid) = project.get("uuid").and_then(|u| u.as_str()) {
                    return Ok(uuid.to_string());
                }
            }
        }
    }

    Err(format!("Project '{}' not found", name))
}

/// Delete a project by name. Finds it first, then deletes by UUID.
pub async fn delete_project(bearer_token: &str, name: &str) -> Result<(), String> {
    let uuid = find_project_uuid(bearer_token, name).await?;

    let url = format!(
        "https://{}/config/v1/orgs/{}/projects/{}",
        cluster(), org_uuid(), uuid
    );

    let response = CLIENT
        .delete(&url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .timeout(Duration::from_secs(HTTP_TIMEOUT))
        .send()
        .await
        .map_err(|e| format!("Failed to delete project: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!(
            "HTTP {} deleting project '{}' (uuid={}): {}",
            status, name, uuid, body
        ))
    }
}

pub fn generate_guid_project_name() -> String {
    let suffix: String = uuid::Uuid::new_v4()
        .to_string()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .take(10)
        .collect();
    format!("bundle_verification_{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guid_project_name_has_correct_prefix() {
        let name = generate_guid_project_name();
        assert!(
            name.starts_with("bundle_verification_"),
            "Expected prefix 'bundle_verification_', got: {}",
            name
        );
    }

    #[test]
    fn test_guid_project_name_has_correct_length() {
        let name = generate_guid_project_name();
        let suffix = name.strip_prefix("bundle_verification_").unwrap();
        assert_eq!(
            suffix.len(),
            10,
            "Expected 10-char suffix, got {}: '{}'",
            suffix.len(),
            suffix
        );
    }

    #[test]
    fn test_guid_project_name_suffix_is_alphanumeric() {
        let name = generate_guid_project_name();
        let suffix = name.strip_prefix("bundle_verification_").unwrap();
        assert!(
            suffix.chars().all(|c| c.is_ascii_alphanumeric()),
            "Suffix should be alphanumeric, got: {}",
            suffix
        );
    }

    #[test]
    fn test_guid_project_names_are_unique() {
        let name1 = generate_guid_project_name();
        let name2 = generate_guid_project_name();
        assert_ne!(name1, name2, "Two generated names should differ");
    }
}
