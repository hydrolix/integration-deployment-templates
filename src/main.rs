// Pointless comment

use lazy_static::lazy_static;
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
    static ref IS_REMOTE: bool = {
        let args: Vec<String> = std::env::args().collect();
        args.contains(&"--remote".to_string())
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
    static ref DELAY_MODE: bool = {
        let args: Vec<String> = std::env::args().collect();
        args.contains(&"--delay".to_string())
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

/// Decide whether a bundle's name matches the user's positional filter.
///
/// An empty filter matches everything (unchanged from prior behavior).
/// Otherwise: when at least one bundle in the repo has a name exactly equal
/// to the filter, we restrict the run to exact-name matches; otherwise we
/// fall back to substring match.
///
/// This resolves the collision where one bundle's name is a substring of
/// another's (e.g. `bot_insights` is a substring of
/// `trafficpeak_bot_insights_ds2`), which previously made it impossible to
/// target the shorter-named bundle in isolation.
fn bundle_matches_filter(name: &str, filter: &str, exact_match_present: bool) -> bool {
    if filter.is_empty() {
        return true;
    }
    if exact_match_present {
        name == filter
    } else {
        name.contains(filter)
    }
}

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

    // Handle --cleanup: delete project (and its Grafana subfolder if creds present), then exit.
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
            }
            Err(e) => {
                eprintln!("ERROR: Failed to delete project '{}': {e}", project_name);
                std::process::exit(1);
            }
        }

        // Idempotent Grafana cleanup. Only attempted when REMOTE_GRAFANA_USER /
        // REMOTE_GRAFANA_PASSWORD are present — older callers cleaning up GUID
        // projects don't have these set and shouldn't be forced to.
        match grafana::remote::RemoteConfig::from_env() {
            Ok(cfg) => {
                let parent = cfg.bundling_folder_uid.clone();
                match grafana::remote::delete_subfolder_by_title(&cfg, &parent, project_name).await
                {
                    Ok(_) => {
                        println!("Grafana subfolder cleanup OK for: {}", project_name);
                    }
                    Err(e) => {
                        eprintln!("WARNING: Grafana subfolder cleanup failed: {e}");
                    }
                }
            }
            Err(_) => {
                // Creds not configured — skip silently. This preserves prior
                // behavior for users who cleanup GUID projects without --remote.
            }
        }

        std::process::exit(0);
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

    let bundle_list = find_bundle_files();

    let mut final_bundle_list: Vec<Bundle> = vec![];
    let mut all_bundle_list: Vec<Bundle> = vec![];

    // Pre-scan: does any bundle's name match MATCH_ONLY exactly? If so, run
    // only that bundle and skip substring fallback (resolves filter collisions
    // where one bundle's name is a substring of another's). See LOTC-1719.
    let exact_match_in_filter = if MATCH_ONLY.is_empty() {
        false
    } else {
        let mut found = false;
        for b in &bundle_list {
            let path = b
                .clone()
                .into_os_string()
                .into_string()
                .unwrap_or_else(|os_str| os_str.to_string_lossy().into_owned());
            if let Ok(bundle) = file_to_bundle(&path).await {
                if bundle.name == *MATCH_ONLY {
                    found = true;
                    break;
                }
            }
        }
        found
    };

    // --remote requires the positional filter to match exactly one bundle.
    // Pre-flight the match count so we can fail-fast with a useful message
    // before spending time on validation.
    if *IS_REMOTE {
        let mut matched_names: Vec<String> = vec![];
        for b in &bundle_list {
            let path = b
                .clone()
                .into_os_string()
                .into_string()
                .unwrap_or_else(|os_str| os_str.to_string_lossy().into_owned());
            if let Ok(bundle) = file_to_bundle(&path).await {
                if bundle_matches_filter(&bundle.name, &MATCH_ONLY, exact_match_in_filter) {
                    matched_names.push(bundle.name);
                }
            }
        }
        if matched_names.is_empty() {
            eprintln!("ERROR: --remote: no bundles match filter '{}'", *MATCH_ONLY);
            std::process::exit(1);
        }
        if matched_names.len() > 1 {
            eprintln!(
                "ERROR: --remote: filter '{}' matches multiple bundles ({}); narrow the filter",
                *MATCH_ONLY,
                matched_names.join(", ")
            );
            std::process::exit(1);
        }
    }

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

        if !bundle_matches_filter(&bundle.name, &MATCH_ONLY, exact_match_in_filter) {
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

        if *IS_REMOTE {
            match deploy::remote::run(&base_dir, &bundle).await {
                Ok(_) => (),
                Err(e) => {
                    eprintln!("ERROR: --remote push failed: {e}");
                    std::process::exit(1);
                }
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

    // Check sample data timestamp freshness (warning only, doesn't fail validation)
    for w in validate::sample_data_freshness::run(base, bundle).await {
        eprintln!("WARNING: {w}");
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

// Find all bundle.json files in the repo
fn find_bundle_files() -> Vec<std::path::PathBuf> {
    let search_path = if *SCAN_WIP { "./WIP" } else { "." };

    WalkDir::new(search_path)
        .max_depth(3)
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

#[cfg(test)]
mod tests {
    use super::bundle_matches_filter;

    #[test]
    fn empty_filter_matches_every_bundle() {
        assert!(bundle_matches_filter("bot_insights", "", false));
        assert!(bundle_matches_filter("trafficpeak_bot_insights_ds2", "", false));
        assert!(bundle_matches_filter("anything", "", true));
    }

    #[test]
    fn exact_match_wins_when_exact_match_exists() {
        // Filter "bot_insights" is the exact name of one bundle in the repo,
        // so trafficpeak_bot_insights_ds2 must NOT match despite containing
        // the substring.
        assert!(bundle_matches_filter(
            "bot_insights",
            "bot_insights",
            true
        ));
        assert!(!bundle_matches_filter(
            "trafficpeak_bot_insights_ds2",
            "bot_insights",
            true
        ));
    }

    #[test]
    fn substring_fallback_when_no_exact_match() {
        // Partial filter "bot" matches no bundle exactly; both bot bundles
        // should run via substring fallback (current/backward-compatible).
        assert!(bundle_matches_filter("bot_insights", "bot", false));
        assert!(bundle_matches_filter(
            "trafficpeak_bot_insights_ds2",
            "bot",
            false
        ));
        assert!(!bundle_matches_filter("cdn_insights", "bot", false));
    }

    #[test]
    fn full_trafficpeak_filter_targets_only_that_bundle() {
        assert!(bundle_matches_filter(
            "trafficpeak_bot_insights_ds2",
            "trafficpeak_bot_insights_ds2",
            true
        ));
        assert!(!bundle_matches_filter(
            "bot_insights",
            "trafficpeak_bot_insights_ds2",
            true
        ));
    }
}
