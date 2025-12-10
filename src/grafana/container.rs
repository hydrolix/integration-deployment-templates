use bollard::Docker;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{sleep, timeout, Duration};

#[allow(deprecated)]
use bollard::container::KillContainerOptions;
use std::default::Default;

use crate::GRAFANA_LOCATION;

#[allow(deprecated)]
pub async fn kill() -> Result<(), String> {
    let docker = match Docker::connect_with_socket_defaults() {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Failed to connect with Docker: {e}",
                file!(),
                line!()
            ));
        }
    };

    let mut list_container_filters = HashMap::new();
    list_container_filters.insert(String::from("status"), vec![String::from("running")]);

    let containers = match &docker
        .list_containers(Some(
            bollard::query_parameters::ListContainersOptionsBuilder::default()
                .all(true)
                .filters(&list_container_filters)
                .build(),
        ))
        .await
    {
        Ok(v) => v.clone(),
        Err(_) => {
            return Err(format!(
                "ERROR: {}.{} Failed to find containers",
                file!(),
                line!()
            ));
        }
    };

    for c in &containers {
        let image = match &c.image {
            Some(s) => s.clone(),
            None => continue,
        };
        let container_id = match &c.id {
            Some(s) => s.clone(),
            None => continue,
        };
        if image == "javiani/grafana:latest" {
            println!("Killing old Grafana container {:?} {container_id}", image);

            let _ = docker
                .kill_container(&container_id, None::<KillContainerOptions<String>>)
                .await;
            return Ok(());
        }
    }
    Ok(())
}

pub async fn start() -> Result<(), String> {
    match Command::new("docker")
        .args([
            "run",
            "--rm",
            "-d",
            "-p",
            "3000:3000",
            "javiani/grafana:latest",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(_) => {}
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Failed to start Grafana container: {e}",
                file!(),
                line!()
            ));
        }
    };
    wait_for_grafana_ready(GRAFANA_LOCATION, 60).await
}

async fn wait_for_grafana_ready(base_url: &str, max_wait_secs: u64) -> Result<(), String> {
    let client = reqwest::Client::new();
    let health_url = format!("http://{}/api/health", base_url);
    let start_time = std::time::Instant::now();

    loop {
        if start_time.elapsed().as_secs() > max_wait_secs {
            return Err("Grafana failed to become ready within timeout".into());
        }

        match timeout(Duration::from_secs(2), client.get(&health_url).send()).await {
            Ok(Ok(response)) if response.status().is_success() => {
                println!("Grafana is ready!");
                return Ok(());
            }
            _ => {
                println!("Grafana not ready yet, waiting...");
                sleep(Duration::from_secs(2)).await;
            }
        }
    }
}
