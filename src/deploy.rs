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

const TABLE_READY_DELAY_SECS: u64 = 30;
const DATA_READY_DELAY_SECS: u64 = 30;

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

    let project_name = hdx::create_project_name();

    let datalink = match grafana::interface::create_datalink(&project_name).await {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Failed to create datalink. {e}",
                file!(),
                line!()
            ));
        }
    };

    output.cluster_domain = BUNDLE_TESTING_CLUSTER.to_string();
    output.project_name = project_name.clone();
    output.grafana_domain = format!("{GRAFANA_LOCATION}/");
    output.datalink = datalink.to_string();

    let mut dashboard_data =
        match load_dashboard_template(base, bundle, &project_name, &datalink).await {
            Ok(v) => v,
            Err(e) => return Err(e),
        };

    // Create base tables and transformations
    for table in &bundle.tables {
        dashboard_data = dashboard_data.replace(&table.dashboard_var, &table.name);
        match process_table(base, &bearer_token, &project_name, table, output).await {
            Ok(_) => (),
            Err(e) => return Err(e),
        }
    }

    // Create summary tables if present
    if let Some(summary_tables) = &bundle.summary_tables {
        for summary in summary_tables {
            match create_summary_table(
                base,
                &bearer_token,
                &project_name,
                summary,
                &mut dashboard_data,
            )
            .await
            {
                Ok(_) => (),
                Err(e) => return Err(e),
            }
        }
    }

    // Second pass: insert data into base tables to populate summaries
    match seed_tables_with_data(base, &bearer_token, &project_name, bundle).await {
        Ok(_) => (),
        Err(e) => return Err(e),
    }

    // Create Grafana dashboard
    let dashboard_id = match grafana::interface::create_dashboard(&dashboard_data).await {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Failed to create dashboard. {e}",
                file!(),
                line!()
            ));
        }
    };

    output.dashboard_id = dashboard_id.clone();
    Ok(dashboard_id)
}

async fn load_dashboard_template(
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
    dashboard = dashboard.replace("__DASHBOARD_UUID__", &Uuid::new_v4().to_string());

    Ok(dashboard)
}

async fn process_table(
    base: &str,
    bearer_token: &str,
    project_name: &str,
    table: &crate::bundle_struct::Table,
    output: &mut Output,
) -> Result<(), String> {
    println!("Creating table: {}", table.name);

    let table_uuid = match hdx::create_table(bearer_token, &table.name).await {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Failed to create table {}: {e}",
                file!(),
                line!(),
                table.name
            ));
        }
    };

    println!("Waiting for table to be ready...");
    sleep(Duration::from_secs(TABLE_READY_DELAY_SECS)).await;

    let mut output_table = OutputTable {
        table_name: table.name.clone(),
        ..Default::default()
    };

    for transform in &table.transforms {
        let transform_json = match read_transform_file(base, &transform.path).await {
            Ok(v) => v,
            Err(e) => return Err(e),
        };

        let transform_name = match add_transformation(
            bearer_token,
            &table_uuid,
            &transform_json,
            &table.name,
            project_name,
            &transform.path,
        )
        .await
        {
            Ok(v) => v,
            Err(e) => return Err(e),
        };

        match insert_sample_data_if_present(
            bearer_token,
            project_name,
            &table.name,
            &transform_name,
            &transform_json,
        )
        .await
        {
            Ok(_) => (),
            Err(e) => return Err(e),
        }

        output_table.transforms.push(OutputTransformation {
            name: transform_name,
            data_type: get_transformation_type(&transform_json),
            data_sub_type: get_transformation_subtype(&transform_json),
        });
    }

    output.tables.push(output_table);
    Ok(())
}

async fn create_summary_table(
    base: &str,
    bearer_token: &str,
    project_name: &str,
    summary: &crate::bundle_struct::SummaryTable,
    dashboard_data: &mut String,
) -> Result<(), String> {
    let path = format!("{base}/{}", summary.sql.path);

    let mut sql = match fs::read_to_string(&path).await {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Failed to read SQL path={path}: {e}",
                file!(),
                line!()
            ));
        }
    };

    sql = sql.replace("__PROJECT_NAME__", project_name);
    sql = sql.replace("__TABLE_NAME__", &summary.parent_table_name);

    match hdx::create_summary_table(bearer_token, &summary.name, &sql).await {
        Ok(_) => (),
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Failed to create summary table {}: {e}",
                file!(),
                line!(),
                summary.name
            ));
        }
    }

    let full_table_name = format!("{}.{}", project_name, summary.name);
    println!(
        "Replacing {} with {}",
        summary.dashboard_var, full_table_name
    );
    *dashboard_data = dashboard_data.replace(&summary.dashboard_var, &full_table_name);

    Ok(())
}

async fn seed_tables_with_data(
    base: &str,
    bearer_token: &str,
    project_name: &str,
    bundle: &Bundle,
) -> Result<(), String> {
    for table in &bundle.tables {
        for transform in &table.transforms {
            let transform_json = match read_transform_file(base, &transform.path).await {
                Ok(v) => v,
                Err(e) => return Err(e),
            };

            let transform_name = get_transformation_name(&transform_json);

            match insert_sample_data_if_present(
                bearer_token,
                project_name,
                &table.name,
                &transform_name,
                &transform_json,
            )
            .await
            {
                Ok(_) => (),
                Err(e) => return Err(e),
            }
        }
    }
    Ok(())
}

async fn read_transform_file(base: &str, relative_path: &str) -> Result<Value, String> {
    let path = format!("{base}/{relative_path}");

    let content = match fs::read_to_string(&path).await {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Failed to read transform path={path}: {e}",
                file!(),
                line!()
            ));
        }
    };

    match serde_json::from_str(&content) {
        Ok(v) => Ok(v),
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Failed to parse JSON path={path}: {e}",
                file!(),
                line!()
            ));
        }
    }
}

async fn add_transformation(
    bearer_token: &str,
    table_uuid: &str,
    transform_json: &Value,
    table_name: &str,
    project_name: &str,
    transform_path: &str,
) -> Result<String, String> {
    let full_table_name = format!("{}.{}", project_name, table_name);

    match hdx::add_transform_to_table(bearer_token, table_uuid, transform_json).await {
        Ok(v) => Ok(v),
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Failed to add transformation path={} table={}: {e}",
                file!(),
                line!(),
                transform_path,
                full_table_name
            ));
        }
    }
}

async fn insert_sample_data_if_present(
    bearer_token: &str,
    project_name: &str,
    table_name: &str,
    transform_name: &str,
    transform_json: &Value,
) -> Result<(), String> {
    let sample_data = get_sample_data_as_json(transform_json);
    if sample_data.is_null() {
        return Ok(());
    }

    println!("Waiting for table to be ready for data...");
    sleep(Duration::from_secs(DATA_READY_DELAY_SECS)).await;

    let full_table_name = format!("{}.{}", project_name, table_name);

    match hdx::insert_into_table(bearer_token, &full_table_name, transform_name, &sample_data).await
    {
        Ok(_) => Ok(()),
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Failed to insert data into {}: {e}",
                file!(),
                line!(),
                full_table_name
            ));
        }
    }
}

// Helper functions
fn get_transformation_subtype(transform_json: &Value) -> String {
    transform_json
        .get("settings")
        .and_then(|s| s.get("format_details"))
        .and_then(|fd| fd.get("subtype"))
        .and_then(|st| st.as_str())
        .map(String::from)
        .unwrap_or_default()
}

fn get_transformation_type(transform_json: &Value) -> String {
    transform_json["type"]
        .as_str()
        .map(String::from)
        .unwrap_or_default()
}

fn get_sample_data_as_json(transform_json: &Value) -> Value {
    transform_json
        .get("settings")
        .and_then(|s| s.get("sample_data"))
        .and_then(|sd| sd.as_object())
        .filter(|obj| !obj.is_empty())
        .map(|_| transform_json["settings"]["sample_data"].clone())
        .unwrap_or(Value::Null)
}

fn get_transformation_name(transform_json: &Value) -> String {
    transform_json["name"]
        .as_str()
        .map(String::from)
        .unwrap_or_default()
}
