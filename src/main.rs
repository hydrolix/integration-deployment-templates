// Pointless comment

use lazy_static::lazy_static;
use std::path::PathBuf;
use tokio::fs;
use tokio::time::sleep;
use tokio::time::Duration;
use walkdir::WalkDir;

mod bundle_struct;
mod deploy;
mod deploy_only_dashboard;
mod grafana;
mod hdx;
mod headless_browser;
mod output_struct;
mod validate;

use crate::bundle_struct::Bundle;
use crate::output_struct::Output;

lazy_static! {
    static ref BUNDLE_TESTING_CLUSTER: String = std::env::var("BUNDLE_TESTING_CLUSTER")
        .expect("BUNDLE_TESTING_CLUSTER environment variable must be set");
    static ref BUNDLE_TESTING_USERNAME: String = std::env::var("BUNDLE_TESTING_USERNAME")
        .expect("BUNDLE_TESTING_USERNAME environment variable must be set");
    static ref BUNDLE_TESTING_PASSWORD: String = std::env::var("BUNDLE_TESTING_PASSWORD")
        .expect("BUNDLE_TESTING_PASSWORD environment variable must be set");
    static ref IS_LOCAL: bool = {
        let args: Vec<String> = std::env::args().collect();
        args.contains(&"--local".to_string())
    };
    static ref IS_LOCAL_DASHBOARD_ONLY: bool = {
        let args: Vec<String> = std::env::args().collect();
        args.contains(&"--local-dashboard-only".to_string())
    };
    static ref FOR_MARKETPLACE: bool = {
        let args: Vec<String> = std::env::args().collect();
        args.contains(&"--marketplace".to_string())
    };
    static ref DUMP_OUTPUT: bool = {
        let args: Vec<String> = std::env::args().collect();
        args.contains(&"--output".to_string())
    };
    static ref MATCH_ONLY: String = {
        let mut value = "".to_string();
        let args: Vec<String> = std::env::args().collect();
        for i in 1..args.len() {
            if !args[i].starts_with("--") {
                value = args[i].to_string();
                break;
            }
        }
        value.to_string()
    };
}

//pub const GRAFANA_LOCATION: &str = "host.docker.internal:3000";

pub const GRAFANA_LOCATION: &str = "localhost:3000";

#[tokio::main]
async fn main() {
    let bundle_list = find_bundle_files();

    for b in &bundle_list {
        let path = PathBuf::from(b);
        let file_path = path
            .into_os_string()
            .into_string()
            .unwrap_or_else(|os_str| os_str.to_string_lossy().into_owned());

        let bundle = match file_to_bundle(&file_path).await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("ERROR: Failed to decode the structure: file_path={file_path} error={e}");
                std::process::exit(1);
            }
        };

        if !MATCH_ONLY.is_empty() {
            if !bundle.name.contains(&*MATCH_ONLY) {
                println!("Ignoring {}", bundle.name);
                continue;
            }
        }

        let base_dir = file_path.replace("./", "").replace("/bundle.json", "");
        println!("Testing {}", bundle.name);

        match validate_bundle(&base_dir, &bundle).await {
            Ok(_) => (),
            Err(e) => {
                eprintln!("ERROR: Failed bundle validation: {e}");
                std::process::exit(1);
            }
        }
        println!("Bundle={:?}", bundle);
    }

    println!("Success");
    std::process::exit(0);
}

// These are all of our tests...
async fn validate_bundle(base: &str, bundle: &Bundle) -> Result<(), String> {
    println!("Base={base} bundle={:?}", bundle);

    let mut output: Output = Output::default();

    match validate::no_duplicate_tokens::run(bundle).await {
        Ok(_) => (),
        Err(e) => return Err(format!("Found duplicate tokens: error={e}")),
    }

    match validate::naming_is_valid::run(bundle).await {
        Ok(_) => (),
        Err(e) => return Err(format!("Found duplicate tokens: error={e}")),
    }

    match validate::no_bad_checksums::run(base, bundle).await {
        Ok(_) => (),
        Err(e) => return Err(format!("Found bad checksum: error={e}")),
    }

    match validate::transforms_are_valid::run(base, bundle).await {
        Ok(_) => (),
        Err(e) => return Err(format!("Found bad checksum: error={e}")),
    }

    match validate::dashboard_is_valid::run(base, bundle).await {
        Ok(_) => (),
        Err(e) => return Err(format!("Found bad checksum: error={e}")),
    }

    match validate::sample_data_exists::run(base, bundle).await {
        Ok(_) => (),
        Err(e) => return Err(format!("No sample data: error={e}")),
    }

    if *IS_LOCAL_DASHBOARD_ONLY {
        // Kill the previous container if it exists
        match grafana::container::kill().await {
            Ok(_) => (),
            Err(_) => (),
        }
        match grafana::container::start().await {
            Ok(_) => (),
            Err(e) => {
                eprintln!("Failed to start the Grafana container... error={e}");
                std::process::exit(1);
            }
        }

        eprintln!("Sleeping for 10 seconds to let container start up...");
        sleep(Duration::from_secs(10)).await;

        let dashboard_id = match deploy_only_dashboard::run(base, bundle, &mut output).await {
            Ok(v) => v,
            Err(e) => return Err(format!("Failed to deploy dashboard error={e}")),
        };
        println!("Dashboard_id={dashboard_id}");
    }

    if *IS_LOCAL {
        // Kill the previous container if it exists
        match grafana::container::kill().await {
            Ok(_) => (),
            Err(_) => (),
        }

        match grafana::container::start().await {
            Ok(_) => (),
            Err(e) => {
                eprintln!("Failed to start the Grafana container... error={e}");
                std::process::exit(1);
            }
        }

        eprintln!("Sleeping for 30 seconds to let container start up...");
        sleep(Duration::from_secs(30)).await;

        let dashboard_id = match deploy::run(base, bundle, &mut output).await {
            Ok(v) => v,
            Err(e) => return Err(format!("Failed to deploy error={e}")),
        };

        println!("Dashboard_id={dashboard_id}");

        println!("Checking the Grafana dashboard with headless Chrome");
        let (datasource_error_count, nodata_error_count) =
            match headless_browser::run(&dashboard_id).await {
                Ok(v) => v,
                Err(e) => return Err(format!("Failed to run headless browser error={e}")),
            };

        println!("Dashboard Errors={datasource_error_count} NoDataErrors={nodata_error_count}");

        if datasource_error_count > 0 || nodata_error_count > 0 {
            return Err(format!(
                "Dashboard Errors={datasource_error_count} NoDataErrors={nodata_error_count}"
            ));
        }
    }

    if *DUMP_OUTPUT {
        if let Ok(pretty_output) = serde_json::to_string_pretty(&output) {
            println!("OUTPUT FOR TRAFFIC GENERATION: \n\n{}", pretty_output);
        } else {
            println!("{:?}", output);
        }
    }

    println!("SUCCESS");

    Ok(())
}

fn find_bundle_files() -> Vec<std::path::PathBuf> {
    WalkDir::new(".")
        .max_depth(2) // Only search current directory
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name() == "bundle.json")
        .map(|e| e.path().to_path_buf())
        .collect()
}

async fn file_to_bundle(file_path: &str) -> Result<Bundle, String> {
    let content = match fs::read_to_string(file_path).await {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Failed to read local bundle file: {e}",
                file!(),
                line!()
            ));
        }
    };

    match serde_json::from_str::<Bundle>(&content) {
        Ok(v) => Ok(v),
        Err(e) => Err(format!(
            "ERROR: {}.{} Not valid Bundle: {e}",
            file!(),
            line!()
        )),
    }
}
