
use tokio::fs;
use uuid::Uuid;

use crate::bundle_struct::Bundle;
use crate::grafana;
use crate::output_struct::Output;

use crate::GRAFANA_LOCATION;

pub async fn run(base: &str, bundle: &Bundle, output: &mut Output) -> Result<String, String> {
   
    output.grafana_domain = format!("{GRAFANA_LOCATION}/");


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

