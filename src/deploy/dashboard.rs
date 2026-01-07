use tokio::fs;
use uuid::Uuid;

use crate::grafana;
use crate::models::bundle::Bundle;
use crate::models::output::Output;

use crate::GRAFANA_LOCATION;

pub async fn run(base: &str, bundle: &Bundle, output: &mut Output) -> Result<Vec<String>, String> {
    output.grafana_domain = format!("{GRAFANA_LOCATION}/");

    // Create datasource first to test configuration
    println!("\n🔗 Creating test datasource...");
    let _datasource_uid = match grafana::datasource::create("test_project").await {
        Ok(uid) => {
            println!("✓ Datasource created successfully with UID: {}", uid);
            uid
        }
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Failed to create datasource. {e}",
                file!(),
                line!()
            ));
        }
    };

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

    dashboard_data = dashboard_data.replace("__PROJECT_NAME__", "JUST GRAFANA");

    dashboard_data = dashboard_data.replace("__DASHBOARD_UUID__", &format!("{}", Uuid::new_v4()));

    let grafana_dashboard_id = match grafana::dashboard::create(&dashboard_data).await {
        Ok(v) => v.to_string(),
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Failed to create dashboard. {e}",
                file!(),
                line!()
            ));
        }
    };

    output.dashboard_id = grafana_dashboard_id.clone();

    // Return as Vec for consistency with deploy.rs (even though it's just one dashboard)
    Ok(vec![grafana_dashboard_id])
}
