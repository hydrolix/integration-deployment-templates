// Pointless comment

use lazy_static::lazy_static;
use std::path::PathBuf;
use tokio::fs;
use walkdir::WalkDir;

mod bundle_struct;
mod deploy;
mod deploy_only_dashboard;
mod grafana;
mod hdx;
mod hdx_check_dependencies;
mod hdx_shared;
mod headless_browser;
mod output_struct;
mod validate;

use crate::bundle_struct::Bundle;
use crate::output_struct::Output;

lazy_static! {
    static ref BUNDLE_TESTING_CLUSTER: String =
        std::env::var("BUNDLE_TESTING_CLUSTER").unwrap_or_else(|_| "".to_string());
    static ref BUNDLE_TESTING_USERNAME: String =
        std::env::var("BUNDLE_TESTING_USERNAME").unwrap_or_else(|_| "".to_string());
    static ref BUNDLE_TESTING_PASSWORD: String =
        std::env::var("BUNDLE_TESTING_PASSWORD").unwrap_or_else(|_| "".to_string());
    static ref SCAN_WIP: bool = {
        let args: Vec<String> = std::env::args().collect();
        args.contains(&"--wip".to_string())
    };
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
    static ref STRICT_PLUGINS: bool = {
        let args: Vec<String> = std::env::args().collect();
        args.contains(&"--strict-plugins".to_string())
            || std::env::var("STRICT_PLUGIN_VALIDATION").unwrap_or_default() == "true"
    };
    static ref PRODUCTION_MODE: bool = {
        let args: Vec<String> = std::env::args().collect();
        args.contains(&"--production".to_string())
    };
    static ref MATCH_ONLY: String = {
        let mut value = "".to_string();
        let args: Vec<String> = std::env::args().collect();
        #[allow(clippy::needless_range_loop)]
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
    let mut bundles_checked = 0;

    let bundle_list = find_bundle_files();

    let mut final_bundle_list: Vec<Bundle> = vec![];
    let mut all_bundle_list: Vec<Bundle> = vec![];

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

        // We need this to check for global duplicates at the end.
        all_bundle_list.push(bundle.clone());

        if !MATCH_ONLY.is_empty() && !bundle.name.contains(&*MATCH_ONLY) {
            println!("Ignoring {} {}", bundle.name, *MATCH_ONLY);
            continue;
        }

        let base_dir = file_path.replace("./", "").replace("/bundle.json", "");
        println!("Testing {}", bundle.name);

        bundles_checked += 1;

        match validate_bundle(&base_dir, &bundle).await {
            Ok(_) => (),
            Err(e) => {
                eprintln!("ERROR: Failed bundle validation: {e}");
                std::process::exit(1);
            }
        }

        println!("Bundle={:?}", bundle);
        final_bundle_list.push(bundle.clone());
    }

    if bundles_checked == 0 {
        eprintln!("ERROR: No bundles were checked - nothing matched the filter.");
        std::process::exit(1);
    }

    println!("Final check on all of the bundles for duplicated tokens...");
    match validate::no_global_duplicates::run(&all_bundle_list) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("ERROR: Failed bundle validation: {e}");
            std::process::exit(1);
        }
    }

    println!("SUCCESS");
    std::process::exit(0);
}

// These are all of our tests...
async fn validate_bundle(base: &str, bundle: &Bundle) -> Result<(), String> {
    println!("Base={base} bundle={:?}", bundle);

    let mut output: Output = Output::default();

    match validate::valid_base_url::run(base, bundle).await {
        Ok(_) => (),
        Err(e) => return Err(format!("Found invalid base url: error={e}")),
    }

    match validate::no_duplicate_tokens::run(bundle).await {
        Ok(_) => (),
        Err(e) => return Err(format!("Found duplicate tokens: error={e}")),
    }

    match validate::naming_is_valid::run(bundle).await {
        Ok(_) => (),
        Err(e) => return Err(format!("Found bad naming: error={e}")),
    }

    match validate::no_bad_checksums::run(base, bundle).await {
        Ok(_) => (),
        Err(e) => return Err(format!("Found bad checksum: error={e}")),
    }

    match validate::transforms_are_valid::run(base, bundle).await {
        Ok(_) => (),
        Err(e) => return Err(format!("Found bad transform: error={e}")),
    }

    match validate::dashboard_is_valid::run(base, bundle).await {
        Ok(_) => (),
        Err(e) => return Err(format!("Found bad dasboard: error={e}")),
    }

    match validate::sample_data_exists::run(base, bundle).await {
        Ok(_) => (),
        Err(e) => return Err(format!("No sample data: error={e}")),
    }

    match validate::summary_table::run(bundle) {
        Ok(_) => (),
        Err(e) => return Err(format!("Bad summary table: error={e}")),
    }

    match validate::alert_rules_are_valid::run(base, bundle).await {
        Ok(_) => (),
        Err(e) => return Err(format!("Alert rules validation failed: error={e}")),
    }

    // Check dependencies (warnings only, doesn't fail validation)
    match validate::check_dependencies::run(base, bundle).await {
        Ok(_) => (),
        Err(e) => eprintln!("WARNING: Dependency check failed: {e}"),
    }

    // Production mode: Check that resources exist on remote cluster
    if *PRODUCTION_MODE {
        println!("Production mode: Checking remote cluster for required resources...");
        match hdx::get_auth_token().await {
            Ok(bearer_token) => {
                match hdx_check_dependencies::check_dependencies_exist(&bearer_token, bundle, base)
                    .await
                {
                    Ok(_) => println!("✓ All required resources exist on remote cluster"),
                    Err(e) => {
                        eprintln!("ERROR: Production mode dependency check failed: {e}");
                        return Err(format!("Production dependency check failed: {e}"));
                    }
                }
            }
            Err(e) => {
                eprintln!("ERROR: Failed to authenticate for production check: {e}");
                return Err(format!("Production auth failed: {e}"));
            }
        }
    }

    if *IS_LOCAL_DASHBOARD_ONLY {
        // Kill the previous container if it exists
        _ = grafana::container::kill().await;

        match grafana::container::start().await {
            Ok(_) => (),
            Err(e) => {
                eprintln!("Failed to start the Grafana container... error={e}");
                std::process::exit(1);
            }
        }

        let dashboard_ids = match deploy_only_dashboard::run(base, bundle, &mut output).await {
            Ok(v) => v,
            Err(e) => return Err(format!("Failed to deploy dashboard error={e}")),
        };
        println!("Dashboard IDs: {:?}", dashboard_ids);
        println!(
            "Primary dashboard_id={}",
            dashboard_ids.first().unwrap_or(&"N/A".to_string())
        );
    }

    if *IS_LOCAL {
        // Kill the previous container if it exists
        _ = grafana::container::kill().await;

        match grafana::container::start().await {
            Ok(_) => (),
            Err(e) => {
                eprintln!("Failed to start the Grafana container... error={e}");
                std::process::exit(1);
            }
        }

        let dashboard_ids = match deploy::run(base, bundle, &mut output).await {
            Ok(v) => v,
            Err(e) => return Err(format!("Failed to deploy error={e}")),
        };

        println!("Total dashboards deployed: {}", dashboard_ids.len());
        println!("Dashboard IDs: {:?}", dashboard_ids);

        // Check for required Grafana plugins
        match grafana::grafana_plugins_check::check_deployed_dashboards(
            &dashboard_ids,
            *STRICT_PLUGINS,
        )
        .await
        {
            Ok(_) => (),
            Err(e) => {
                if *STRICT_PLUGINS {
                    return Err(format!("Plugin validation failed: {}", e));
                } else {
                    eprintln!("Plugin check warning: {}", e);
                }
            }
        }

        // Use primary dashboard for headless browser check
        let primary_dashboard_id = dashboard_ids
            .first()
            .cloned()
            .unwrap_or_else(|| "".to_string());

        println!("Checking the Grafana dashboard with headless Chrome");
        let (datasource_error_count, nodata_error_count) =
            match headless_browser::run(&primary_dashboard_id).await {
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

// Update find_bundle_files to handle WIP location
fn find_bundle_files() -> Vec<std::path::PathBuf> {
    let search_path = if *SCAN_WIP { "./WIP" } else { "." };

    WalkDir::new(search_path)
        .max_depth(2)
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
