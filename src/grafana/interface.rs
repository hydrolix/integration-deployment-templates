use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;

use crate::{GRAFANA_TOKEN, GRAFANA_FOLDER_UID, GRAFANA_USERNAME, GRAFANA_PASSWORD};
use crate::{BUNDLE_TESTING_CLUSTER, BUNDLE_TESTING_PASSWORD, BUNDLE_TESTING_USERNAME};
use crate::get_grafana_base_url;

const HDX_DATABASE_PORT: &str = "9440";
const HTTP_TIMEOUT: u64 = 120;

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct CreateDataSourceRequest {
    pub name: String,
    #[serde(rename = "type")]
    pub datasource_type: String,
    pub access: String,
    pub jsonData: JsonData,
    pub secureJsonData: SecureJsonData,
    pub readOnly: bool,
}

#[derive(Serialize, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonData {
    pub default_database: String,
    pub host: String,  // Changed from 'server' to 'host'
    pub port: String,
    pub protocol: String,  // Added protocol field
    pub query_timeout: String,
    pub secure: bool,
    pub timeout: String,
    pub username: String,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct SecureJsonData {
    pub password: String,
}

async fn delete_existing_datasource(name: &str) -> Result<(), String> {
    // List all datasources
    let list_url = format!("{}/api/datasources", get_grafana_base_url());
    println!("  🔍 Checking for existing datasource at: {}", list_url);

    // Create client with explicit timeouts and accept invalid certs for testing
    println!("  🔧 Creating HTTP client...");
    let client = match reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .danger_accept_invalid_certs(true)  // Temporarily for debugging
        .build()
    {
        Ok(c) => {
            println!("  ✓ HTTP client created successfully");
            c
        },
        Err(e) => return Err(format!("Failed to create HTTP client: {}", e)),
    };

    let mut request = client.get(&list_url);

    // Use token auth if available, otherwise fall back to basic auth
    if !GRAFANA_TOKEN.is_empty() {
        println!("  🔑 Using token authentication");
        request = request.header("Authorization", format!("Bearer {}", *GRAFANA_TOKEN));
    } else {
        println!("  🔑 Using basic authentication");
        let auth = base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", *GRAFANA_USERNAME, *GRAFANA_PASSWORD));
        request = request.header("Authorization", format!("Basic {}", auth));
    }

    println!("  📡 Sending request to Grafana...");

    // Wrap in tokio timeout as additional safety
    let response = match tokio::time::timeout(
        Duration::from_secs(15),
        request.send()
    ).await {
        Ok(Ok(r)) => {
            println!("  ✓ Got response from Grafana");
            r
        },
        Ok(Err(e)) => {
            println!("  ❌ Request failed: {}", e);
            return Err(format!("Failed to list datasources: {}", e));
        },
        Err(_) => {
            println!("  ❌ Request timed out after 15 seconds");
            return Err("Request timed out after 15 seconds".to_string());
        }
    };

    if !response.status().is_success() {
        return Ok(()); // If we can't list, just continue
    }

    let datasources: Value = match response.json().await {
        Ok(v) => v,
        Err(_) => return Ok(()),
    };

    // Find datasource with matching name
    if let Some(datasources_array) = datasources.as_array() {
        for ds in datasources_array {
            if let Some(ds_name) = ds.get("name").and_then(|n| n.as_str()) {
                if ds_name == name {
                    if let Some(id) = ds.get("id").and_then(|i| i.as_i64()) {
                        // Delete this datasource
                        let delete_url = format!("{}/api/datasources/{}", get_grafana_base_url(), id);
                        let mut delete_request = client.delete(&delete_url);

                        // Use token auth if available, otherwise fall back to basic auth
                        if !GRAFANA_TOKEN.is_empty() {
                            delete_request = delete_request.header("Authorization", format!("Bearer {}", *GRAFANA_TOKEN));
                        } else {
                            let auth2 = base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", *GRAFANA_USERNAME, *GRAFANA_PASSWORD));
                            delete_request = delete_request.header("Authorization", format!("Basic {}", auth2));
                        }

                        let _ = delete_request.send().await;
                        println!("  Deleted existing datasource: {}", name);
                    }
                }
            }
        }
    }

    Ok(())
}

pub async fn create_datalink(project_name: &str) -> Result<String, String> {
    // Delete any existing datasource with the same name
    let _ = delete_existing_datasource("Bundle Testing").await;

    let datasource_request = CreateDataSourceRequest {
        name: "Bundle Testing".to_string(),
        datasource_type: "hydrolix-hydrolix-datasource".to_string(),
        access: "proxy".to_string(),
        jsonData: JsonData {
            default_database: project_name.to_string(),
            host: BUNDLE_TESTING_CLUSTER.to_string(),  // Changed from 'server' to 'host'
            port: HDX_DATABASE_PORT.to_string(),
            protocol: "native".to_string(),  // Added protocol (native or http)
            query_timeout: "600".to_string(),
            secure: true,
            timeout: "10".to_string(),
            username: BUNDLE_TESTING_USERNAME.to_string(),
        },
        secureJsonData: SecureJsonData {
            password: BUNDLE_TESTING_PASSWORD.to_string(),
        },
        readOnly: true,
    };

    let payload = match serde_json::to_value(&datasource_request) {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Failed to serialize: {e}",
                file!(),
                line!()
            ));
        }
    };

    let url = format!("{}/api/datasources", get_grafana_base_url());

    let response = match post_basic_auth(
        &url,
        &GRAFANA_USERNAME,
        &GRAFANA_PASSWORD,
        &payload,
        Some(vec![("X-Grafana-Org-Id", "1")]), // Add the X-Grafana-Org-Id header
    )
    .await
    {
        Ok(v) => v.to_string(),
        Err(e) => return Err(format!("ERROR: {}.{}  {e}", file!(), line!())),
    };

    let response_json: Value = match serde_json::from_str(&response) {
        Ok(v) => v,
        Err(e) => return Err(format!("ERROR: {}.{}  {e}", file!(), line!())),
    };

    // Access the "uid" field inside the "datasource" object
    if let Some(uid) = response_json["datasource"]["uid"].as_str() {
        sleep(Duration::from_secs(2)).await;
        // acct.project.table[index].grafana_datalink = uid.to_string();
        return Ok(uid.to_string());
    };

    Err(format!(
        "ERROR: {}.{} Failed to create Grafana Datalink ",
        file!(),
        line!()
    ))
}

pub async fn create_dashboard(dashboard_data: &str) -> Result<String, String> {
    let url = format!("{}/api/dashboards/import", get_grafana_base_url());

    // Parse the dashboard JSON - it might already be wrapped or just the dashboard
    let mut dashboard_json: Value = match serde_json::from_str(dashboard_data) {
        Ok(v) => v,
        Err(e) => return Err(format!("ERROR: {}.{} Failed to parse dashboard JSON: {e}", file!(), line!())),
    };

    // Check if it's already an import payload or just a dashboard
    let import_request = if dashboard_json.get("dashboard").is_some() {
        // Already in import format, just add folder if needed
        if !GRAFANA_FOLDER_UID.is_empty() {
            dashboard_json["folderUid"] = Value::String(GRAFANA_FOLDER_UID.to_string());
            println!("  📁 Creating dashboard in folder: {}", *GRAFANA_FOLDER_UID);
        }
        dashboard_json
    } else {
        // Raw dashboard, wrap it in import format
        let mut import_req = serde_json::json!({
            "dashboard": dashboard_json,
            "overwrite": true,
            "inputs": []
        });

        if !GRAFANA_FOLDER_UID.is_empty() {
            import_req["folderUid"] = Value::String(GRAFANA_FOLDER_UID.to_string());
            println!("  📁 Creating dashboard in folder: {}", *GRAFANA_FOLDER_UID);
        }
        import_req
    };

    let import_payload = match serde_json::to_string(&import_request) {
        Ok(v) => v,
        Err(e) => return Err(format!("ERROR: {}.{} Failed to serialize import request: {e}", file!(), line!())),
    };

    let boxed_str: Box<str> = import_payload.into();

    let result_data = match post_string_basic_auth(
        &url,
        &GRAFANA_USERNAME,
        &GRAFANA_PASSWORD,
        boxed_str.clone(),
        Some(vec![("X-Grafana-Org-Id", "1")]), // Add the X-Grafana-Org-Id header
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            return Err(format!("ERROR: {}.{} {e}", file!(), line!()));
        }
    };

    let result_json: Value = match serde_json::from_str(&result_data) {
        Ok(v) => v,
        Err(e) => return Err(format!("ERROR: {}.{}  {e}", file!(), line!())),
    };

    // Access the "uid"
    if let Some(uid) = result_json["uid"].as_str() {
        return Ok(uid.to_string());
    };

    Err(format!(
        "ERROR: No UID in the dashboard {}.{}",
        file!(),
        line!()
    ))
}

pub async fn post_basic_auth(
    url: &str,
    username: &str,
    password: &str,
    payload: &Value,
    additional_headers: Option<Vec<(&str, &str)>>, // Add support for additional headers
) -> Result<String, String> {
    let client = reqwest::Client::new();

    let mut request = client
        .post(url)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .timeout(Duration::from_secs(HTTP_TIMEOUT))
        .json(payload);

    // Use token auth if available, otherwise fall back to basic auth
    if !GRAFANA_TOKEN.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", *GRAFANA_TOKEN));
    } else {
        request = request.basic_auth(username, Some(password));
    }

    // Add additional headers if provided
    if let Some(headers) = additional_headers {
        for (key, value) in headers {
            request = request.header(key, value);
        }
    }

    let response = request.send().await;

    let resp = match response {
        Ok(v) => v,
        Err(e) => return Err(format!("{}.{} Failed to get data: {e}", file!(), line!())),
    };

    // Get the status code before consuming the body
    let status = resp.status();

    // Extract the response body text
    let text = match resp.text().await {
        Ok(v) => v,
        Err(e) => return Err(format!("{}.{} error {e}", file!(), line!())),
    };

    if !status.is_success() {
        return Err(format!(
            "{}.{} Failed post url={url} status_code={:?} text={:?} payload={:?}",
            file!(),
            line!(),
            status,
            text,
            payload,
        ));
    }

    Ok(text)
}

pub async fn post_string_basic_auth(
    url: &str,
    username: &str,
    password: &str,
    payload: Box<str>,
    additional_headers: Option<Vec<(&str, &str)>>,
) -> Result<String, String> {
    let client = reqwest::Client::new();
    let mut request = client
        .post(url)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .timeout(Duration::from_secs(HTTP_TIMEOUT))
        .body(payload.to_string());

    // Use token auth if available, otherwise fall back to basic auth
    if !GRAFANA_TOKEN.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", *GRAFANA_TOKEN));
    } else {
        request = request.basic_auth(username, Some(password));
    }

    // Add additional headers if provided
    if let Some(headers) = additional_headers {
        for (key, value) in headers {
            request = request.header(key, value);
        }
    }

    let response = request.send().await;

    let resp = match response {
        Ok(v) => v,
        Err(e) => return Err(format!("{}.{} Failed to get data: {e}", file!(), line!())),
    };

    //println!("Grafana response: {:?}", resp);
    // Get the status code before consuming the body
    let status = resp.status();

    // Extract the response body text
    let text = match resp.text().await {
        Ok(v) => v,
        Err(e) => return Err(format!("{}.{} error: {e}", file!(), line!())),
    };

    if !status.is_success() {
        return Err(format!(
            "{}.{} Failed post url={url} status_code={:?} text={:?}",
            file!(),
            line!(),
            status,
            text
        ));
    }

    Ok(text)
}
