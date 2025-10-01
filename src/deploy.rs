use serde_json::Value;
use tokio::fs;
use tokio::time::sleep;
use tokio::time::Duration;
use uuid::Uuid;

use crate::bundle_struct::Bundle;
use crate::grafana;
use crate::hdx;
use crate::output_struct::Output;
use crate::output_struct::OutputTable;
use crate::output_struct::OutputTransformation;
use crate::BUNDLE_TESTING_CLUSTER;

use crate::GRAFANA_LOCATION;

pub async fn run(base: &str, bundle: &Bundle, output: &mut Output) -> Result<String, String> {
    let bearer_token = match hdx::get_auth_token().await {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Failed to get HDX bearer token: {e}",
                file!(),
                line!()
            ));
        }
    };

    output.cluster_domain = BUNDLE_TESTING_CLUSTER.to_string();

    let project_name = hdx::create_project_name();

    output.project_name = project_name.to_string();

    let datalink = match grafana::interface::create_datalink(&project_name).await {
        Ok(v) => v.to_string(),
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Failed to create datalink. {e}",
                file!(),
                line!()
            ));
        }
    };

    output.grafana_domain = format!("{GRAFANA_LOCATION}/");
    output.datalink = datalink.to_string();

    let full_path = format!("{base}/{}", bundle.dashboard.path);

    let mut dashboard_data = match fs::read_to_string(&full_path).await {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Failed to read file_path={full_path}: error={e}",
                file!(),
                line!()
            ));
        }
    };

    dashboard_data = dashboard_data.replace("__PROJECT_NAME__", &project_name);

    dashboard_data = dashboard_data.replace("__DATASOURCE__", &datalink);

    dashboard_data = dashboard_data.replace("__DASHBOARD_UUID__", &format!("{}", Uuid::new_v4()));

    for t in &bundle.tables {
        let table_name = hdx::create_table_name();

        println!("Replacing {} with {}", t.dashboard_var, table_name);

        dashboard_data = dashboard_data.replace(&t.dashboard_var, &table_name);

        let table_uuid = match hdx::create_table(&bearer_token, &table_name).await {
            Ok(v) => v,
            Err(e) => {
                return Err(format!(
                    "ERROR: {}.{} Failed to create table {table_name}: {e}",
                    file!(),
                    line!()
                ));
            }
        };

        let mut output_table: OutputTable = OutputTable {
            table_name: table_name.to_string(),
            ..Default::default()
        };
        //let mut output_table: OutputTable = OutputTable::default();
        //output_table.table_name = table_name.to_string();

        println!("Sleeping for 30 seconds to let table get ready for transformations...");
        sleep(Duration::from_secs(30)).await;

        for tt in &t.transforms {
            let full_path = format!("{base}/{}", tt.path);

            let content = match fs::read_to_string(&full_path).await {
                Ok(v) => v,
                Err(e) => {
                    return Err(format!(
                        "ERROR: {}.{} Failed to read file_path={full_path}: error={e}",
                        file!(),
                        line!()
                    ));
                }
            };

            let transform_json = match serde_json::from_str(&content) {
                Ok(v) => v,
                Err(e) => {
                    return Err(format!(
                        "ERROR: {}.{} Failed to read file_path={full_path}: error={e}",
                        file!(),
                        line!()
                    ));
                }
            };

            let full_table_name = format!("{}.{}", project_name, table_name);

            let transformation_name = match hdx::add_transform_to_table(
                &bearer_token,
                &table_uuid,
                &transform_json,
            )
            .await
            {
                Ok(v) => v,
                Err(e) => {
                    return Err(format!(
                        "ERROR: {}.{} Failed to add transformation full_path={full_path} full_table_name={full_table_name}: error={e}",
                        file!(),
                        line!()
                    ));
                }
            };

            let sample_data = get_sample_data_as_json(&transform_json);
            if !sample_data.is_null() {
                println!("Sleeping for 30 seconds to let table get ready for data...");
                sleep(Duration::from_secs(30)).await;

                match hdx::insert_into_table(
                    &bearer_token,
                    &full_table_name,
                    &transformation_name,
                    &sample_data,
                )
                .await
                {
                    Ok(_) => (),
                    Err(e) => {
                        return Err(format!(
                            "ERROR: {}.{} Failed to send data to HDX {full_table_name}: {e}",
                            file!(),
                            line!()
                        ));
                    }
                }
            }
            let data_type = get_transformation_type(&transform_json);
            let data_sub_type = get_transformation_subtype(&transform_json);

            output_table.transforms.push(OutputTransformation {
                name: transformation_name.to_string(),
                data_type: data_type.to_string(),
                data_sub_type: data_sub_type.to_string(),
            });

            output.tables.push(output_table.clone());
        }
    }

    // All of the tables are now built and have data
    // Build Grafana dashboard and return its id so we can check it out

    let grafana_dashboard_id = match grafana::interface::create_dashboard(&dashboard_data).await {
        Ok(v) => v.to_string(),
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Failed to create dashboard. {e}",
                file!(),
                line!()
            ));
        }
    };

    output.dashboard_id = grafana_dashboard_id.to_string();
    Ok(grafana_dashboard_id)
}

fn get_transformation_subtype(transform_json: &Value) -> String {
    transform_json
        .get("settings")
        .and_then(|s| s.get("format_details"))
        .and_then(|fd| fd.get("subtype"))
        .and_then(|st| st.as_str())
        .map(|s| s.to_string())
        .unwrap_or_default()
}

fn get_transformation_type(transform_json: &Value) -> String {
    match transform_json["type"].as_str() {
        Some(v) => v.to_string(),
        None => "".to_string(),
    }
}

fn get_sample_data_as_json(transform_json: &Value) -> Value {
    let sample_data = &transform_json["settings"]["sample_data"];
    if let Some(obj) = sample_data.as_object() {
        if !obj.is_empty() {
            return sample_data.clone();
        }
    }
    Value::Null
}

#[allow(dead_code)]
fn get_transformation_name(transform_json: &Value) -> String {
    match transform_json["name"].as_str() {
        Some(v) => v.to_string(),
        None => "".to_string(),
    }
}
