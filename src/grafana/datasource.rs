use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;

use crate::grafana::http;
use crate::GRAFANA_LOCATION;
use crate::{BUNDLE_TESTING_CLUSTER, BUNDLE_TESTING_PASSWORD, BUNDLE_TESTING_USERNAME};

const HDX_DATABASE_PORT: &str = "9440";

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
    pub host: String, // Changed from 'server' to 'host'
    pub port: String,
    pub protocol: String, // Added protocol field
    pub query_timeout: String,
    pub secure: bool,
    pub timeout: String,
    pub username: String,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct SecureJsonData {
    pub password: String,
}

pub async fn create(project_name: &str) -> Result<String, String> {
    // Delete any existing datasource with the same name
    let _ = delete("Bundle Testing").await;

    let datasource_request = CreateDataSourceRequest {
        name: "Bundle Testing".to_string(),
        datasource_type: "hydrolix-hydrolix-datasource".to_string(),
        access: "proxy".to_string(),
        jsonData: JsonData {
            default_database: project_name.to_string(),
            host: BUNDLE_TESTING_CLUSTER.to_string(), // Changed from 'server' to 'host'
            port: HDX_DATABASE_PORT.to_string(),
            protocol: "native".to_string(), // Added protocol (native or http)
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

    let url = format!("http://{GRAFANA_LOCATION}/api/datasources");

    let response = match http::post_basic_auth(
        &url,
        "admin",
        "admin",
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

async fn delete(name: &str) -> Result<(), String> {
    // List all datasources
    let list_url = format!("http://{GRAFANA_LOCATION}/api/datasources");
    let client = reqwest::Client::new();

    let auth = base64::engine::general_purpose::STANDARD.encode("admin:admin");

    let response = match client
        .get(&list_url)
        .header("Authorization", format!("Basic {}", auth))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to list datasources: {}", e)),
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
                        let delete_url =
                            format!("http://{GRAFANA_LOCATION}/api/datasources/{}", id);
                        let auth2 = base64::engine::general_purpose::STANDARD.encode("admin:admin");
                        let _ = client
                            .delete(&delete_url)
                            .header("Authorization", format!("Basic {}", auth2))
                            .send()
                            .await;
                        println!("  Deleted existing datasource: {}", name);
                    }
                }
            }
        }
    }

    Ok(())
}
