// Pointless comment

use lazy_static::lazy_static;
use regex::Regex;
use std::path::PathBuf;
use tokio::fs;
use walkdir::WalkDir;

mod deploy;
mod grafana;
mod hdx;
mod models;
mod validate;

use crate::models::bundle::Bundle;
use crate::models::output::Output;

mod flags;

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
            // Skip the argument following --cleanup (it's the project name, not a filter)
            if i > 1 && args[i - 1] == "--cleanup" {
                continue;
            }
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
    // Parse flags for --guid and --cleanup
    let args: Vec<String> = std::env::args().collect();
    let parsed_flags = match flags::Flags::parse(&args) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("ERROR: {e}");
            std::process::exit(1);
        }
    };

    // Handle --cleanup: delete project and exit
    if let Some(project_name) = &parsed_flags.cleanup_project {
        println!("Cleaning up project: {}", project_name);
        let bearer_token = match hdx::auth::get_token().await {
            Ok(t) => t,
            Err(e) => {
                eprintln!("ERROR: Authentication failed: {e}");
                std::process::exit(1);
            }
        };
        match hdx::delete_project(&bearer_token, project_name).await {
            Ok(_) => {
                println!("Successfully deleted project: {}", project_name);
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("ERROR: Failed to delete project '{}': {e}", project_name);
                std::process::exit(1);
            }
        }
    }

    // Handle --guid: create a GUID'd project before validation
    if parsed_flags.use_guid {
        let project_name = hdx::generate_guid_project_name();
        println!("Created test project: {}", project_name);

        let bearer_token = match hdx::auth::get_token().await {
            Ok(t) => t,
            Err(e) => {
                eprintln!("ERROR: Authentication failed: {e}");
                std::process::exit(1);
            }
        };

        let project_uuid = match hdx::create_project(&bearer_token, &project_name).await {
            Ok(uuid) => {
                println!("  Project UUID: {}", uuid);
                uuid
            }
            Err(e) => {
                eprintln!(
                    "ERROR: Failed to create GUID project '{}': {e}",
                    project_name
                );
                std::process::exit(1);
            }
        };

        hdx::set_guid_project(project_name, project_uuid);
    }

    let mut bundles_checked = 0;

    // Reject any directories that look like versions but aren't strict X.Y.Z
    reject_invalid_version_dirs();

    let bundle_list = filter_to_latest_versions(find_bundle_files());

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

        // Extract version directory name if this is a versioned path
        let dir_version: Option<String> = {
            let base_path = std::path::Path::new(&base_dir);
            let last_component = base_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if is_semver(last_component) {
                Some(last_component.to_string())
            } else if looks_like_version(last_component) {
                eprintln!(
                    "ERROR: folder name '{}' looks like a version but is not valid \
                     semver (expected X.Y.Z, e.g., 1.0.0). Rename the folder to a strict \
                     X.Y.Z version or a plain bundle name.",
                    last_component
                );
                std::process::exit(1);
            } else {
                None
            }
        };

        match validate_bundle(&base_dir, &bundle, dir_version.as_deref()).await {
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
    match validate::no_global_duplicates::run(&final_bundle_list) {
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
async fn validate_bundle(
    base: &str,
    bundle: &Bundle,
    dir_version: Option<&str>,
) -> Result<(), String> {
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

    match validate::naming_is_valid::run(bundle, dir_version).await {
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

    match validate::datasource_uid_consistency::run(base, bundle).await {
        Ok(_) => (),
        Err(e) => return Err(format!("Datasource UID consistency failed: error={e}")),
    }

    match validate::summary_column_schema::run(base, bundle).await {
        Ok(_) => (),
        Err(e) => {
            return Err(format!(
                "Summary column schema validation failed: error={e}"
            ))
        }
    }

    match validate::summary_table_references::run(base, bundle).await {
        Ok(_) => (),
        Err(e) => {
            return Err(format!(
                "Summary table references validation failed: error={e}"
            ))
        }
    }

    match validate::template_variable_consistency::run(base, bundle).await {
        Ok(_) => (),
        Err(e) => return Err(format!("Template variable consistency failed: error={e}")),
    }

    match validate::template_variable_datasource::run(base, bundle).await {
        Ok(_) => (),
        Err(e) => {
            return Err(format!(
                "Template variable datasource validation failed: error={e}"
            ))
        }
    }

    // Check dependencies (warnings only, doesn't fail validation)
    match validate::check_dependencies::run(base, bundle).await {
        Ok(_) => (),
        Err(e) => eprintln!("WARNING: Dependency check failed: {e}"),
    }

    // Production mode: Check that resources exist on remote cluster
    if *PRODUCTION_MODE {
        println!("Production mode: Checking remote cluster for required resources...");
        match hdx::auth::get_token().await {
            Ok(bearer_token) => match hdx::dependencies::exist(&bearer_token, bundle, base).await {
                Ok(_) => println!("✓ All required resources exist on remote cluster"),
                Err(e) => {
                    eprintln!("ERROR: Production mode dependency check failed: {e}");
                    return Err(format!("Production dependency check failed: {e}"));
                }
            },
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

        let dashboard_ids = match deploy::dashboard::run(base, bundle, &mut output).await {
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

        let dashboard_ids = match deploy::default::run(base, bundle, &mut output).await {
            Ok(v) => v,
            Err(e) => return Err(format!("Failed to deploy error={e}")),
        };

        println!("Total dashboards deployed: {}", dashboard_ids.len());
        println!("Dashboard IDs: {:?}", dashboard_ids);

        // Check for required Grafana plugins
        match grafana::plugins::check_deployed_dashboards(&dashboard_ids, *STRICT_PLUGINS).await {
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
            match grafana::headless_browser::run(&primary_dashboard_id).await {
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

/// Scan for directories that look like versions but aren't strict X.Y.Z semver.
/// This catches folders like "1.0.0-beta", "1.0", "2.0.0rc1" before they silently
/// bypass version detection.
fn reject_invalid_version_dirs() {
    let search_path = if *SCAN_WIP { "./WIP" } else { "." };

    for entry in WalkDir::new(search_path)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
    {
        let dir_name = entry.file_name().to_str().unwrap_or("");
        if looks_like_version(dir_name) {
            eprintln!(
                "ERROR: folder name '{}' looks like a version but is not valid \
                 semver (expected X.Y.Z, e.g., 1.0.0). Rename the folder to a strict \
                 X.Y.Z version or a plain bundle name.\n  Path: {}",
                dir_name,
                entry.path().display()
            );
            std::process::exit(1);
        }
    }
}

// Update find_bundle_files to handle WIP location
fn find_bundle_files() -> Vec<std::path::PathBuf> {
    let search_path = if *SCAN_WIP { "./WIP" } else { "." };

    WalkDir::new(search_path)
        .max_depth(4)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name() == "bundle.json")
        .map(|e| e.path().to_path_buf())
        .collect()
}

/// Check if a directory name is a semver version string (e.g., "1.0.0").
fn is_semver(s: &str) -> bool {
    lazy_static! {
        static ref SEMVER_RE: Regex = Regex::new(r"^\d+\.\d+\.\d+$").unwrap();
    }
    SEMVER_RE.is_match(s)
}

/// Check if a string looks like a version but isn't strict X.Y.Z semver.
/// Catches names like "1.0.0-beta", "1.0", "2.0.0rc1".
fn looks_like_version(s: &str) -> bool {
    lazy_static! {
        static ref VERSION_LIKE_RE: Regex = Regex::new(r"^\d+\.").unwrap();
    }
    VERSION_LIKE_RE.is_match(s) && !is_semver(s)
}

/// Parse a semver string into a comparable tuple.
fn parse_semver(s: &str) -> (u32, u32, u32) {
    let parts: Vec<u32> = s.split('.').filter_map(|p| p.parse().ok()).collect();
    (
        parts.first().copied().unwrap_or(0),
        parts.get(1).copied().unwrap_or(0),
        parts.get(2).copied().unwrap_or(0),
    )
}

/// Filter bundle paths so that only the latest version per bundle identity is kept.
/// Non-versioned (flat) paths are always included.
fn filter_to_latest_versions(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    use std::collections::HashMap;

    let mut versioned: HashMap<PathBuf, (PathBuf, (u32, u32, u32))> = HashMap::new();
    let mut flat: Vec<PathBuf> = Vec::new();

    for path in paths {
        // Parent of bundle.json — could be a version dir or a bundle dir
        let parent = match path.parent() {
            Some(p) => p,
            None => {
                flat.push(path);
                continue;
            }
        };

        let parent_name = parent.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if is_semver(parent_name) {
            // Versioned path: grandparent is the bundle identity
            let bundle_identity = match parent.parent() {
                Some(gp) => gp.to_path_buf(),
                None => {
                    flat.push(path);
                    continue;
                }
            };
            let ver = parse_semver(parent_name);

            match versioned.get(&bundle_identity) {
                Some((_, existing_ver)) if ver > *existing_ver => {
                    versioned.insert(bundle_identity, (path, ver));
                }
                None => {
                    versioned.insert(bundle_identity, (path, ver));
                }
                _ => {} // existing version is higher or equal, skip
            }
        } else {
            flat.push(path);
        }
    }

    let mut result: Vec<PathBuf> = flat;
    result.extend(versioned.into_values().map(|(path, _)| path));
    result.sort();
    result
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
