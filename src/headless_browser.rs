use headless_chrome::protocol::cdp::types::Event;
use headless_chrome::protocol::cdp::Network;
use headless_chrome::{Browser, LaunchOptionsBuilder};

use regex::Regex;
use reqwest::header;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;

use crate::GRAFANA_LOCATION;

// url, grafana_dashboard_id, username, password
pub async fn run(grafana_dashboard_id: &str) -> Result<(i32, i32), String> {
    let (cookie_name, cookie_value) = match get_grafana_session_cookie().await {
        Ok((n, v)) => (n.to_string(), v.to_string()),
        Err(e) => return Err(format!("ERROR: {}.{} {e}", file!(), line!())),
    };

    let launch_options = match LaunchOptionsBuilder::default()
        .window_size(Some((1920, 4080))) // Set to 1920x1080 (Full HD)
        .build()
    {
        Ok(v) => v,
        Err(e) => return Err(format!("ERROR: {}.{} {e}", file!(), line!())),
    };

    let browser = match Browser::new(launch_options) {
        Ok(v) => v,
        Err(e) => return Err(format!("ERROR: {}.{} {e}", file!(), line!())),
    };

    let tab = match browser.new_tab() {
        Ok(v) => v,
        Err(e) => return Err(format!("ERROR: {}.{} {e}", file!(), line!())),
    };

    match tab.enable_runtime() {
        Ok(_) => (),
        Err(e) => return Err(format!("ERROR: {}.{} {e}", file!(), line!())),
    }

    match tab.enable_log() {
        Ok(_) => (),
        Err(e) => return Err(format!("ERROR: {}.{} {e}", file!(), line!())),
    }

    let bad_datasource_regex = match Regex::new(r"Datasource \w+ was not found") {
        Ok(v) => v,
        Err(e) => return Err(format!("ERROR: {}.{} {e}", file!(), line!())),
    };

    let nodata_regex = match Regex::new(r"api/ds/query\?ds_type=[^&]+-clickhouse-datasource") {
        Ok(v) => v,
        Err(e) => return Err(format!("ERROR: {}.{} {e}", file!(), line!())),
    };

    let datasource_error_count = Arc::new(Mutex::new(0));
    let datasource_error_count_clone = Arc::clone(&datasource_error_count);

    let nodata_error_count = Arc::new(Mutex::new(0));
    let nodata_error_count_clone = Arc::clone(&nodata_error_count);

    #[allow(clippy::match_single_binding)]
    tab.add_event_listener(Arc::new(move |event: &Event| match event {
        e => {
            let raw_event = format!("{:?}", e.clone());
            if bad_datasource_regex.is_match(&raw_event) {
                if let Ok(mut count) = datasource_error_count_clone.lock() {
                    *count += 1;
                }
            }
            if nodata_regex.is_match(&raw_event) {
                if let Ok(mut count) = nodata_error_count_clone.lock() {
                    *count += 1;
                }
            }
        }
    }))
    .unwrap();

    let cookie = Network::CookieParam {
        name: cookie_name.to_string(),
        value: cookie_value.to_string(),
        url: Some(format!("http://{GRAFANA_LOCATION}/")),
        //domain: Some("host.docker.internal".to_string()),
        domain: None,
        path: Some("/".to_string()),
        secure: Some(false),
        http_only: Some(true),
        same_site: Some(Network::CookieSameSite::Lax),
        expires: None,
        priority: None,
        same_party: Some(false),
        source_scheme: None,
        source_port: None,
        partition_key: None,
    };

    // Set the cookie
    match tab.set_cookies(vec![cookie]) {
        Ok(v) => v,
        Err(e) => return Err(format!("ERROR: {}.{} {e}", file!(), line!())),
    }

    // Navigate to the domain first going back two weeks to test the time-range collar
    let url = format!("http://{GRAFANA_LOCATION}/d/{grafana_dashboard_id}?from=now-14d&to=now");

    let _x = match tab.navigate_to(&url) {
        Ok(v) => v,
        Err(e) => return Err(format!("ERROR: {}.{} {e}", file!(), line!())),
    };

    match tab.wait_until_navigated() {
        Ok(_) => println!("Page navigation completed"),
        Err(e) => return Err(format!("ERROR: {}.{} {e}", file!(), line!())),
    }

    // Wait much longer for all panels to load and make their queries
    println!("Waiting 120 seconds for all panels to load and query data...");
    sleep(Duration::from_secs(120)).await;

    // Reset counters at the start of each iteration
    if let Ok(mut count) = datasource_error_count.lock() {
        *count = 0;
    }
    if let Ok(mut count) = nodata_error_count.lock() {
        *count = 0;
    }

    // Read the counts safely
    let datasource_count = datasource_error_count
        .lock()
        .map(|count| *count)
        .unwrap_or(0);
    let nodata_count = nodata_error_count.lock().map(|count| *count).unwrap_or(0);

    println!(
        "Datasource errors: {}, NoData errors: {}",
        datasource_count, nodata_count
    );

    if datasource_count == 0 && nodata_count == 0 {
        println!("Success! No errors detected.");
    }

    return Ok((datasource_count, nodata_count));
}

async fn get_grafana_session_cookie() -> Result<(String, String), String> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none()) // Important: disable redirects
        .build()
        .unwrap();

    let response = client
        .post(format!("http://{GRAFANA_LOCATION}/login"))
        .header(header::CONTENT_TYPE, "application/json")
        .json(&serde_json::json!({
            "user": "admin",
            "password": "admin"
        }))
        .send()
        .await
        .unwrap();

    let cookie = match response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .next()
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| format!("ERROR: {}.{} No valid cookie found", file!(), line!()))
    {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Invalid cookie format: {e}",
                file!(),
                line!()
            ));
        }
    };

    // Parse cookie name and value
    let cookie_parts: Vec<&str> = cookie.split(';').next().unwrap().split('=').collect();
    if cookie_parts.len() != 2 {
        return Err(format!(
            "ERROR: {}.{} Invalid cookie format",
            file!(),
            line!()
        ));
    }

    Ok((cookie_parts[0].to_string(), cookie_parts[1].to_string()))
}
