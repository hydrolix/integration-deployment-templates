use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;

use crate::{BUNDLE_TESTING_CLUSTER, BUNDLE_TESTING_PASSWORD, BUNDLE_TESTING_USERNAME};

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
pub struct JsonData {
    pub default_database: String,
    pub port: String,
    pub server: String,
    pub query_timeout: String,
    pub secure: bool,
    pub timeout: String,
    pub username: String,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct SecureJsonData {
    pub password: String,
}

pub async fn create_datalink(project_name: &str) -> Result<String, String> {
    Ok("fake".to_string())
}

/* 
pub async fn create_datalink(project_name: &str) -> Result<String, String> {
    let datasource_request = CreateDataSourceRequest {
        name: "Bundle Testing".to_string(),
        datasource_type: "grafana-clickhouse-datasource".to_string(),
        access: "proxy".to_string(),
        jsonData: JsonData {
            default_database: project_name.to_string(),
            port: HDX_DATABASE_PORT.to_string(),
            server: BUNDLE_TESTING_CLUSTER.to_string(),
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

    let url = "http://localhost:3000/api/datasources".to_string();

    let response = match post_basic_auth(
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
*/

pub async fn create_dashboard(dashboard_data: &str) -> Result<String, String> {
    let url = "http://localhost:3000/api/dashboards/import".to_string();

    let boxed_str: Box<str> = dashboard_data.into();

    let result_data = match post_string_basic_auth(
        &url,
        "admin",
        "admin",
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
        .basic_auth(username, Some(password))
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .timeout(Duration::from_secs(HTTP_TIMEOUT))
        .json(payload);

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
    let text = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read response text: {}", e))?;

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
        .basic_auth(username, Some(password))
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .timeout(Duration::from_secs(HTTP_TIMEOUT))
        .body(payload.to_string());

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
    let text = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read response text: {}", e))?;

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

/*
pub async fn get_basic_auth(
    url: &str,
    username: &str,
    password: &str,
    additional_headers: Option<Vec<(&str, &str)>>, // Add support for additional headers
) -> Result<String, String> {
    let client = reqwest::Client::new();

    let mut request = client
        .get(url)
        .basic_auth(username, Some(password))
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .timeout(Duration::from_secs(HTTP_TIMEOUT));

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
    let text = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read response text: {}", e))?;

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
    */
