// HTTP client for the persistent (review) Grafana at REMOTE_GRAFANA_URL.
//
// Bypasses the localhost:3000 ephemeral container used by --local. All
// requests authenticate with a service-account / API token via Bearer auth
// (the review Grafana is OAuth-only for browser users, so basic auth is not
// available — see Grafana docs: Service accounts / API keys).

use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

const HTTP_TIMEOUT: u64 = 60;

const DEFAULT_GRAFANA_URL: &str = "https://dashboards.trafficpeak.live";
const DEFAULT_BUNDLING_FOLDER_UID: &str = "efko87db3by0we";
const DEFAULT_DATASOURCE_UID: &str = "aelocpydfc9vke";

pub struct RemoteConfig {
    pub url: String,
    pub token: String,
    pub bundling_folder_uid: String,
    pub datasource_uid: String,
}

impl RemoteConfig {
    pub fn from_env() -> Result<Self, String> {
        let token = std::env::var("REMOTE_GRAFANA_TOKEN").map_err(|_| {
            "REMOTE_GRAFANA_TOKEN is required for --remote mode (Grafana service-account or API token)"
                .to_string()
        })?;

        Ok(Self {
            url: std::env::var("REMOTE_GRAFANA_URL")
                .unwrap_or_else(|_| DEFAULT_GRAFANA_URL.to_string()),
            token,
            bundling_folder_uid: std::env::var("REMOTE_BUNDLING_FOLDER_UID")
                .unwrap_or_else(|_| DEFAULT_BUNDLING_FOLDER_UID.to_string()),
            datasource_uid: std::env::var("REMOTE_GRAFANA_DATASOURCE_UID")
                .unwrap_or_else(|_| DEFAULT_DATASOURCE_UID.to_string()),
        })
    }
}

fn client() -> Client {
    Client::new()
}

/// Find a subfolder of `parent_uid` whose title equals `name`. Returns its
/// uid if found, otherwise creates it via POST /api/folders.
pub async fn ensure_subfolder(
    cfg: &RemoteConfig,
    parent_uid: &str,
    name: &str,
) -> Result<String, String> {
    if let Some(uid) = find_subfolder_uid(cfg, parent_uid, name).await? {
        return Ok(uid);
    }

    let url = format!("{}/api/folders", cfg.url);
    let payload = json!({
        "parentUid": parent_uid,
        "title": name,
    });

    let resp = client()
        .post(&url)
        .bearer_auth(&cfg.token)
        .header("content-type", "application/json")
        .timeout(Duration::from_secs(HTTP_TIMEOUT))
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("POST {url} failed: {e}"))?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("read response body: {e}"))?;

    if !status.is_success() {
        return Err(format!(
            "create folder '{}' under '{}' failed: HTTP {} body={}",
            name, parent_uid, status, text
        ));
    }

    let body: Value = serde_json::from_str(&text)
        .map_err(|e| format!("parse create-folder response: {e} body={text}"))?;
    body.get("uid")
        .and_then(|u| u.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("create-folder response missing uid: {text}"))
}

/// Look up subfolder uid by parent + title. Returns Ok(None) if absent.
pub async fn find_subfolder_uid(
    cfg: &RemoteConfig,
    parent_uid: &str,
    name: &str,
) -> Result<Option<String>, String> {
    let url = format!("{}/api/folders?parentUid={}", cfg.url, parent_uid);
    let resp = client()
        .get(&url)
        .bearer_auth(&cfg.token)
        .timeout(Duration::from_secs(HTTP_TIMEOUT))
        .send()
        .await
        .map_err(|e| format!("GET {url} failed: {e}"))?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("read response body: {e}"))?;
    if !status.is_success() {
        return Err(format!(
            "list folders under '{}' failed: HTTP {} body={}",
            parent_uid, status, text
        ));
    }

    let body: Value = serde_json::from_str(&text)
        .map_err(|e| format!("parse list-folders response: {e} body={text}"))?;
    let arr = match body.as_array() {
        Some(a) => a,
        None => return Ok(None),
    };

    for f in arr {
        let title = f.get("title").and_then(|v| v.as_str()).unwrap_or("");
        if title == name {
            if let Some(uid) = f.get("uid").and_then(|u| u.as_str()) {
                return Ok(Some(uid.to_string()));
            }
        }
    }
    Ok(None)
}

/// Upsert a dashboard into `folder_uid`. Returns the dashboard's URL slug.
pub async fn upsert_dashboard(
    cfg: &RemoteConfig,
    folder_uid: &str,
    dashboard: &Value,
    message: &str,
) -> Result<String, String> {
    let url = format!("{}/api/dashboards/db", cfg.url);
    let payload = json!({
        "dashboard": dashboard,
        "folderUid": folder_uid,
        "overwrite": true,
        "message": message,
    });

    let resp = client()
        .post(&url)
        .bearer_auth(&cfg.token)
        .header("content-type", "application/json")
        .timeout(Duration::from_secs(HTTP_TIMEOUT))
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("POST {url} failed: {e}"))?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("read response body: {e}"))?;
    if !status.is_success() {
        return Err(format!(
            "upsert dashboard failed: HTTP {} body={}",
            status, text
        ));
    }

    let body: Value = serde_json::from_str(&text)
        .map_err(|e| format!("parse upsert-dashboard response: {e} body={text}"))?;
    Ok(body
        .get("url")
        .and_then(|u| u.as_str())
        .unwrap_or_default()
        .to_string())
}

/// Delete the subfolder titled `name` under `parent_uid` and its dashboards.
/// No-op (returns Ok) if the subfolder doesn't exist.
pub async fn delete_subfolder_by_title(
    cfg: &RemoteConfig,
    parent_uid: &str,
    name: &str,
) -> Result<(), String> {
    let uid = match find_subfolder_uid(cfg, parent_uid, name).await? {
        Some(u) => u,
        None => return Ok(()),
    };

    let url = format!("{}/api/folders/{}?forceDeleteRules=true", cfg.url, uid);
    let resp = client()
        .delete(&url)
        .bearer_auth(&cfg.token)
        .timeout(Duration::from_secs(HTTP_TIMEOUT))
        .send()
        .await
        .map_err(|e| format!("DELETE {url} failed: {e}"))?;

    let status = resp.status();
    if status.is_success() || status.as_u16() == 404 {
        Ok(())
    } else {
        let text = resp
            .text()
            .await
            .unwrap_or_else(|_| "<no body>".to_string());
        Err(format!(
            "delete folder uid={} failed: HTTP {} body={}",
            uid, status, text
        ))
    }
}
