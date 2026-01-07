use bundle_validator::hdx;
use serde_json::Value;
use tokio::fs;

use crate::grafana::http;
use crate::models::bundle::Bundle;
use crate::GRAFANA_LOCATION;

pub async fn create(dashboard_data: &str) -> Result<String, String> {
    let url = format!("http://{GRAFANA_LOCATION}/api/dashboards/import");

    let boxed_str: Box<str> = dashboard_data.into();

    let result_data = match http::post_string_basic_auth(
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

pub async fn create_others(
    bundle: &Bundle,
    project_name: &str,
    base: &str,
    datalink: &str,
) -> Result<Vec<String>, String> {
    let mut all_dashboard_uuids = vec![];
    // Create other dashboards if present
    let dashboards = match &bundle.other_dashboards {
        Some(v) => v,
        None => return Ok(all_dashboard_uuids),
    };
    let shared_project_name = hdx::shared_proj::get_name();

    for d in dashboards {
        println!("Creating additional dashboard: {}", d.path);

        let full_path = format!("{}/{}", base, d.path);
        let mut contents = fs::read_to_string(&full_path)
            .await
            .map_err(|e| format!("Failed to read other dashboard {}: {}", full_path, e))?;

        contents = contents.replace("__PROJECT_NAME__", project_name);
        contents = contents.replace("__DATASOURCE__", datalink);
        contents = contents.replace("__DASHBOARD_UUID__", &uuid::Uuid::new_v4().to_string());
        contents = contents.replace("__SHARED_PROJECT__", &shared_project_name);

        // Replace summary table variables if present
        if let Some(summary_tables) = &bundle.summary_tables {
            if !summary_tables.is_empty() {
                let summary_min = format!("{}.{}", project_name, summary_tables[0].name);
                contents = contents
                    .replace("${VAR_SUMMARY_MIN}", &summary_min)
                    .replace("$VAR_SUMMARY_MIN", &summary_min);
            }
            if summary_tables.len() > 1 {
                let summary_hour = format!("{}.{}", project_name, summary_tables[1].name);
                contents = contents
                    .replace("${VAR_SUMMARY_HOUR}", &summary_hour)
                    .replace("$VAR_SUMMARY_HOUR", &summary_hour);
            }
        }

        // Replace table dashboard vars
        for table in &bundle.tables {
            contents = contents.replace(&table.dashboard_var, &table.name);
        }

        // Replace summary table dashboard vars
        if let Some(summary_tables) = &bundle.summary_tables {
            for summary in summary_tables {
                contents = contents.replace(&summary.dashboard_var, &summary.name);
            }
        }

        let other_dash_uid = create(&contents)
            .await
            .map_err(|e| format!("Failed to create other dashboard: {}", e))?;

        all_dashboard_uuids.push(other_dash_uid.clone());
        println!("✓ Created dashboard: {} (UID: {})", d.path, other_dash_uid);
    }

    Ok(all_dashboard_uuids)
}

pub async fn load_template(
    base: &str,
    bundle: &Bundle,
    project_name: &str,
    datalink: &str,
) -> Result<String, String> {
    let path = format!("{base}/{}", bundle.dashboard.path);

    let mut dashboard = match fs::read_to_string(&path).await {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Failed to read dashboard path={path}: {e}",
                file!(),
                line!()
            ));
        }
    };

    dashboard = dashboard.replace("__PROJECT_NAME__", project_name);
    dashboard = dashboard.replace("__DATASOURCE__", datalink);
    dashboard = dashboard.replace("__DASHBOARD_UUID__", &uuid::Uuid::new_v4().to_string());

    Ok(dashboard)
}
