use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;

use bollard::Docker;

#[allow(deprecated)]
use bollard::container::KillContainerOptions;

use std::default::Default;

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
        .args(["run", "--rm", "-p", "3000:3000", "javiani/grafana:latest"])
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
    Ok(())
}
