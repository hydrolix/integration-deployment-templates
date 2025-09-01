use headless_chrome::protocol::cdp::types::Event;
use headless_chrome::protocol::cdp::Network;
use headless_chrome::{Browser, LaunchOptionsBuilder};
use std::ffi::OsStr;

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

    println!("Got here: {}.{}", file!(), line!());

    let launch_options = match LaunchOptionsBuilder::default()
        .path(Some("/opt/hostedtoolcache/setup-chrome/chromium/stable/x64/chrome".into())) 
        .window_size(Some((1920, 4080))) // Set to 1920x1080 (Full HD)
        .headless(true)
        .port(None)
        .args(vec![
            OsStr::new("--no-sandbox"),
            OsStr::new("--disable-setuid-sandbox"),
            OsStr::new("--disable-dev-shm-usage"),
            OsStr::new("--disable-gpu"),
            OsStr::new("--remote-debugging-port=0"),
            OsStr::new("--headless=new"),
            OsStr::new("--single-process"), // Critical for CI
            OsStr::new("--no-zygote"),      // Critical for CI
        ])
        .build()
    {
        Ok(v) => v,
        Err(e) => return Err(format!("ERROR: {}.{} {e}", file!(), line!())),
    };

    println!("Got here: {}.{}", file!(), line!());
    let browser = match Browser::new(launch_options) {
        Ok(v) => v,
        Err(e) => return Err(format!("ERROR: {}.{} {e}", file!(), line!())),
    };

    println!("Got here: {}.{}", file!(), line!());

    let tab = match browser.new_tab() {
        Ok(v) => v,
        Err(e) => return Err(format!("ERROR: {}.{} {e}", file!(), line!())),
    };
    /*
        #[allow(deprecated)]
        let tab = match browser.wait_for_initial_tab() {
            Ok(v) => v,
            Err(e) => return Err(format!("ERROR: {}.{} {e}", file!(), line!())),
        };
    */
    println!("Got here: {}.{}", file!(), line!());

    match tab.enable_runtime() {
        Ok(_) => (),
        Err(e) => return Err(format!("ERROR: {}.{} {e}", file!(), line!())),
    }

    println!("Got here: {}.{}", file!(), line!());

    match tab.enable_log() {
        Ok(_) => (),
        Err(e) => return Err(format!("ERROR: {}.{} {e}", file!(), line!())),
    }

    println!("Got here: {}.{}", file!(), line!());
    let bad_datasource_regex = match Regex::new(r"Datasource \w+ was not found") {
        Ok(v) => v,
        Err(e) => return Err(format!("ERROR: {}.{} {e}", file!(), line!())),
    };

    let nodata_regex = match Regex::new(r"api/ds/query\?ds_type=[^&]+-clickhouse-datasource") {
        Ok(v) => v,
        Err(e) => return Err(format!("ERROR: {}.{} {e}", file!(), line!())),
    };

    println!("Got here: {}.{}", file!(), line!());

    let datasource_error_count = Arc::new(Mutex::new(0));
    let datasource_error_count_clone = Arc::clone(&datasource_error_count);

    let nodata_error_count = Arc::new(Mutex::new(0));
    let nodata_error_count_clone = Arc::clone(&nodata_error_count);

    println!("Got here: {}.{}", file!(), line!());

    #[allow(clippy::match_single_binding)]
    tab.add_event_listener(Arc::new(move |event: &Event| match event {
        e => {
            let raw_event = format!("{:?}", e.clone());
            if bad_datasource_regex.is_match(&raw_event) {
                *datasource_error_count_clone.lock().unwrap() += 1;
            }
            if nodata_regex.is_match(&raw_event) {
                *nodata_error_count_clone.lock().unwrap() += 1;
            }
        }
    }))
    .unwrap();

    //tab.enable_network().unwrap();

    println!("Got here: {}.{}", file!(), line!());

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

    println!("Got here: {}.{}", file!(), line!());

    // Navigate to the domain first
    let url = format!("http://{GRAFANA_LOCATION}/d/{grafana_dashboard_id}");
    let _x = match tab.navigate_to(&url) {
        Ok(v) => v,
        Err(e) => return Err(format!("ERROR: {}.{} {e}", file!(), line!())),
    };

    println!("Got here: {}.{}", file!(), line!());

    match tab.wait_until_navigated() {
        Ok(_) => println!("Page navigation completed"),
        Err(e) => return Err(format!("ERROR: {}.{} {e}", file!(), line!())),
    }

    // Wait for page to load completely (Grafana needs to pull the data)
    sleep(Duration::from_secs(30)).await;

    println!("Got here: {}.{}", file!(), line!());

    let datasource_error_count = *datasource_error_count.lock().unwrap();
    let nodata_error_count = *nodata_error_count.lock().unwrap();

    Ok((datasource_error_count, nodata_error_count))

    /*
    let png_data = tab
        .capture_screenshot(Page::CaptureScreenshotFormatOption::Png, None, None, true)
        .unwrap();

    // Save to file
    fs::write("screenshot.png", png_data).unwrap();
    */
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
