use serde_json::Value;
use std::time::Duration;

const HTTP_TIMEOUT: u64 = 120;

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
