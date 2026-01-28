// Cleanup script for Hydrolix resources
// Usage:
//   cargo run --bin cleanup -- --functions <bundle-name>
//   cargo run --bin cleanup -- --dictionaries <bundle-name>
//   cargo run --bin cleanup -- --dictionary-files <bundle-name>
//   cargo run --bin cleanup -- --tables <bundle-name>
//   cargo run --bin cleanup -- --all <bundle-name>
//   cargo run --bin cleanup -- --all <bundle-name> --dry-run

use bundle_validator::hdx;
use bundle_validator::models::bundle::Bundle;
use lazy_static::lazy_static;
use serde_json::Value;
use std::collections::HashSet;

// const ORG_UUID_MARK: &str = "d867bf48-4281-4496-8432-a93aa989aae6";  // markeplace-dev
// const ORG_UUID_SAND: &str = "b646d78a-5fb2-4d5f-afef-b705bf185174";  // partnersandbox
const ORG_UUID: &str = "2b8cbbf8-dcb8-4c28-bd94-cb46147296d1"; // demo.aws.hydrolix.live
                                                               // const PROJ_UUID_MARK: &str = "67e79a3c-f7d6-4b33-a207-fef4579a3152";  // markeplace-dev cdn_test_project
                                                               // const PROJ_UUID_SAND: &str = "469dbd34-6f06-4dfe-8fd1-9adf82123ecf";  // partnersandbox
const PROJ_UUID: &str = "6debffd1-3c88-4d5e-afc8-9e1a770f6a7a"; // demo.aws.hydrolix.live
                                                                //const PROJ_NAME: &str = "cdn_test_project";
const PROJ_NAME: &str = "bundle_verification";

// const ORG_UUID_SAND: &str = "b646d78a-5fb2-4d5f-afef-b705bf185174"; // partnersandbox
// const PROJ_UUID_SAND: &str = "469dbd34-6f06-4dfe-8fd1-9adf82123ecf"; // partnersandbox

lazy_static! {
    static ref BUNDLE_TESTING_CLUSTER: String =
        std::env::var("BUNDLE_TESTING_CLUSTER").unwrap_or_else(|_| "".to_string());
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    let delete_functions =
        args.contains(&"--functions".to_string()) || args.contains(&"--all".to_string());
    let delete_dictionaries =
        args.contains(&"--dictionaries".to_string()) || args.contains(&"--all".to_string());
    let delete_dictionary_files =
        args.contains(&"--dictionary-files".to_string()) || args.contains(&"--all".to_string());
    let delete_tables =
        args.contains(&"--tables".to_string()) || args.contains(&"--all".to_string());
    let dry_run = args.contains(&"--dry-run".to_string());

    // Get bundle name (first non-flag argument)
    let bundle_name = args
        .iter()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .cloned()
        .unwrap_or_default();

    if !delete_functions && !delete_dictionaries && !delete_tables && !delete_dictionary_files {
        print_usage();
        std::process::exit(1);
    }

    println!("\n🧹 Cleanup Script for project: {}", PROJ_NAME);
    println!("   Cluster: {}", *BUNDLE_TESTING_CLUSTER);

    // Load bundle if specified
    let bundle = if !bundle_name.is_empty() {
        match load_bundle(&bundle_name).await {
            Ok(b) => {
                println!("   Bundle: {}", bundle_name);
                println!("   Scope: Only resources from this bundle");
                Some(b)
            }
            Err(e) => {
                eprintln!("\n❌ Could not read bundle: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        println!("   ⚠️  WARNING: No bundle specified - will delete ALL resources!");
        println!("   Press Ctrl+C to cancel...");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        None
    };

    if dry_run {
        println!("   🔍 DRY RUN MODE - Nothing will actually be deleted\n");
    }

    // Get authentication token
    let bearer_token = match hdx::auth::get_token().await {
        Ok(token) => {
            println!("✓ Authenticated successfully");
            token
        }
        Err(e) => {
            eprintln!("\n❌ Authentication failed: {}", e);
            std::process::exit(1);
        }
    };

    // Execute cleanup operations
    if delete_functions {
        if let Err(e) =
            delete_functions_impl(&bearer_token, bundle.as_ref(), &bundle_name, dry_run).await
        {
            eprintln!("\n❌ Failed to delete functions: {}", e);
            std::process::exit(1);
        }
    }

    if delete_dictionaries {
        if let Err(e) = delete_dictionaries_impl(&bearer_token, dry_run).await {
            eprintln!("\n❌ Failed to delete dictionaries: {}", e);
            std::process::exit(1);
        }
    }

    if delete_dictionary_files {
        if let Err(e) =
            delete_dictionary_files_impl(&bearer_token, bundle.as_ref(), &bundle_name, dry_run)
                .await
        {
            eprintln!("\n❌ Failed to delete dictionary files: {}", e);
            std::process::exit(1);
        }
    }

    if delete_tables {
        if let Err(e) = delete_tables_impl(&bearer_token, bundle.as_ref(), dry_run).await {
            eprintln!("\n❌ Failed to delete tables: {}", e);
            std::process::exit(1);
        }
    }

    println!("\n✅ Cleanup complete!");
    std::process::exit(0);
}

fn print_usage() {
    println!("Cleanup Script for Hydrolix Resources");
    println!("\nUsage:");
    println!(
        "  cargo run --bin cleanup -- --functions <bundle-name>       # Delete bundle's functions"
    );
    println!("  cargo run --bin cleanup -- --dictionaries <bundle-name>    # Delete bundle's dictionaries");
    println!("  cargo run --bin cleanup -- --dictionary-files <bundle-name> # Delete bundle's dictionary files");
    println!(
        "  cargo run --bin cleanup -- --tables <bundle-name>          # Delete bundle's tables"
    );
    println!("  cargo run --bin cleanup -- --all <bundle-name>             # Delete everything for bundle");
    println!(
        "  cargo run --bin cleanup -- --all <bundle-name> --dry-run   # Show what would be deleted"
    );
    println!("\nNote: --all includes functions, dictionaries, dictionary files, and tables");
    println!("      Provide bundle name to delete only that bundle's resources (recommended)");
    println!("      Omit bundle name to delete ALL resources in project (dangerous!)");
}

async fn load_bundle(bundle_name: &str) -> Result<Bundle, String> {
    let bundle_path = format!("{}/bundle.json", bundle_name);
    let content = tokio::fs::read_to_string(&bundle_path)
        .await
        .map_err(|e| format!("Failed to read bundle file: {}", e))?;

    serde_json::from_str::<Bundle>(&content).map_err(|e| format!("Failed to parse bundle: {}", e))
}

async fn delete_functions_impl(
    bearer_token: &str,
    bundle: Option<&Bundle>,
    bundle_name: &str,
    dry_run: bool,
) -> Result<(), String> {
    println!("\n🗑️  Deleting functions...");

    // Build list of functions to delete
    let functions_to_delete: Option<HashSet<String>> = if let Some(b) = bundle {
        let mut set = HashSet::new();
        let (bundle_funcs, _) = b.get_all_functions();
        for func_name in &bundle_funcs {
            set.insert(func_name.clone());
        }

        // Try to discover from bundle directory
        if let Ok(discovered) = hdx::functions::discover(bundle_name).await {
            for func_name in discovered {
                set.insert(func_name);
            }
        }

        if set.is_empty() {
            println!("  No functions to delete (none declared in bundle)");
            return Ok(());
        }

        println!("  Scope: {} function(s) from bundle", set.len());
        Some(set)
    } else {
        println!("  Scope: ALL functions in project");
        None
    };

    let list_url = format!(
        "https://{}/config/v1/orgs/{}/projects/{}/functions/",
        *BUNDLE_TESTING_CLUSTER, ORG_UUID, PROJ_UUID
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&list_url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .send()
        .await
        .map_err(|e| format!("Failed to list functions: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to list functions: {}", response.status()));
    }

    let response_data: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let functions = if let Some(results) = response_data.get("results").and_then(|r| r.as_array()) {
        results.clone()
    } else if let Some(arr) = response_data.as_array() {
        arr.clone()
    } else {
        vec![]
    };

    if functions.is_empty() {
        println!("  No functions found on cluster");
        return Ok(());
    }

    println!("  Found {} total function(s) on cluster", functions.len());

    let mut deleted = 0;
    for func in functions {
        let name = func
            .get("name")
            .or_else(|| func.get("function_name"))
            .or_else(|| func.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let uuid = func
            .get("uuid")
            .or_else(|| func.get("id"))
            .or_else(|| func.get("uid"))
            .or_else(|| func.get("function_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Filter by bundle if specified
        if let Some(ref filter_set) = functions_to_delete {
            if !filter_set.contains(name) {
                continue; // Skip - not in this bundle
            }
        }

        if dry_run {
            println!("  [DRY RUN] Would delete: {}", name);
            continue;
        }

        if uuid.is_empty() {
            eprintln!("  ⚠️  Skipping {} - no UUID found", name);
            continue;
        }

        let delete_url = format!("{}{}", list_url, uuid);

        let delete_response = client
            .delete(&delete_url)
            .header("Authorization", format!("Bearer {}", bearer_token))
            .send()
            .await;

        match delete_response {
            Ok(resp) if resp.status().is_success() => {
                println!("  ✓ Deleted: {}", name);
                deleted += 1;
            }
            Ok(resp) => {
                eprintln!("  ⚠️  Failed to delete {}: {}", name, resp.status());
            }
            Err(e) => {
                eprintln!("  ⚠️  Error deleting {}: {}", name, e);
            }
        }
    }

    println!("✓ Deleted {} function(s)", deleted);
    Ok(())
}

async fn delete_dictionaries_impl(bearer_token: &str, dry_run: bool) -> Result<(), String> {
    println!("\n🗑️  Deleting all dictionary definitions...");
    println!("  (Note: Uploaded dictionary files will NOT be deleted)");

    let list_url = format!(
        "https://{}/config/v1/orgs/{}/projects/{}/dictionaries/",
        *BUNDLE_TESTING_CLUSTER, ORG_UUID, PROJ_UUID
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&list_url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .send()
        .await
        .map_err(|e| format!("Failed to list dictionaries: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to list dictionaries: {}",
            response.status()
        ));
    }

    let response_data: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let dictionaries =
        if let Some(results) = response_data.get("results").and_then(|r| r.as_array()) {
            results.clone()
        } else if let Some(arr) = response_data.as_array() {
            arr.clone()
        } else {
            vec![]
        };

    if dictionaries.is_empty() {
        println!("  No dictionaries to delete");
        return Ok(());
    }

    println!("  Found {} dictionar(y/ies)", dictionaries.len());

    let mut deleted = 0;
    for dict in dictionaries {
        let name = dict
            .get("name")
            .or_else(|| dict.get("dictionary_name"))
            .or_else(|| dict.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let uuid = dict
            .get("uuid")
            .or_else(|| dict.get("id"))
            .or_else(|| dict.get("uid"))
            .or_else(|| dict.get("dictionary_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if dry_run {
            println!("  [DRY RUN] Would delete: {} (uuid: {})", name, uuid);
            continue;
        }

        if uuid.is_empty() {
            eprintln!("  ⚠️  Skipping {} - no UUID found", name);
            continue;
        }

        let delete_url = format!("{}{}", list_url, uuid);

        let delete_response = client
            .delete(&delete_url)
            .header("Authorization", format!("Bearer {}", bearer_token))
            .send()
            .await;

        match delete_response {
            Ok(resp) if resp.status().is_success() => {
                println!("  ✓ Deleted: {}", name);
                deleted += 1;
            }
            Ok(resp) => {
                eprintln!("  ⚠️  Failed to delete {}: {}", name, resp.status());
            }
            Err(e) => {
                eprintln!("  ⚠️  Error deleting {}: {}", name, e);
            }
        }
    }

    println!("✓ Deleted {} dictionar(y/ies)", deleted);
    Ok(())
}

async fn delete_dictionary_files_impl(
    bearer_token: &str,
    bundle: Option<&Bundle>,
    bundle_name: &str,
    dry_run: bool,
) -> Result<(), String> {
    println!("\n🗑️  Deleting uploaded dictionary files...");
    println!("  ⚠️  WARNING: This will delete uploaded CSV/YAML files!");

    // Build list of file names to delete
    let file_names_to_delete: Option<HashSet<String>> = if let Some(b) = bundle {
        let mut set = HashSet::new();
        let (bundle_dicts, _) = b.get_all_dictionaries();

        let mut all_dict_names = bundle_dicts;

        // Try to discover from bundle directory
        if let Ok(discovered) = hdx::dictionaries::discover(bundle_name).await {
            all_dict_names.extend(discovered);
        }

        // For each dictionary, add possible file names (with and without extensions)
        for dict_name in &all_dict_names {
            set.insert(dict_name.clone());
            set.insert(format!("{}.csv", dict_name));
            set.insert(format!("{}.yaml", dict_name));
            set.insert(format!("{}.yml", dict_name));
            set.insert(format!("{}.tsv", dict_name));
        }

        println!(
            "  Scope: Files for {} dictionar(y/ies) from bundle",
            all_dict_names.len()
        );
        Some(set)
    } else {
        println!("  Scope: ALL dictionary files in project");
        None
    };

    let list_url = format!(
        "https://{}/config/v1/orgs/{}/projects/{}/dictionaries/files/",
        *BUNDLE_TESTING_CLUSTER, ORG_UUID, PROJ_UUID
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&list_url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .send()
        .await
        .map_err(|e| format!("Failed to list dictionary files: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to list dictionary files: {}",
            response.status()
        ));
    }

    let response_data: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let files: Vec<String> = if let Some(arr) = response_data.as_array() {
        arr.iter()
            .map(|f| {
                if let Some(s) = f.as_str() {
                    s.to_string()
                } else {
                    f.get("name")
                        .or_else(|| f.get("id"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string()
                }
            })
            .collect()
    } else if let Some(files_arr) = response_data.get("files").and_then(|f| f.as_array()) {
        files_arr
            .iter()
            .map(|f| {
                if let Some(s) = f.as_str() {
                    s.to_string()
                } else {
                    f.get("name")
                        .or_else(|| f.get("id"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string()
                }
            })
            .collect()
    } else if let Some(results) = response_data.get("results").and_then(|r| r.as_array()) {
        results
            .iter()
            .map(|f| {
                if let Some(s) = f.as_str() {
                    s.to_string()
                } else {
                    f.get("name")
                        .or_else(|| f.get("id"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string()
                }
            })
            .collect()
    } else {
        vec![]
    };

    if files.is_empty() {
        println!("  No dictionary files found on cluster");
        return Ok(());
    }

    println!("  Found {} total file(s) on cluster", files.len());

    let mut deleted = 0;
    for file_name in files {
        // Filter by bundle if specified
        if let Some(ref filter_set) = file_names_to_delete {
            if !filter_set.contains(&file_name) {
                continue; // Skip - not used by this bundle
            }
        }

        if dry_run {
            println!("  [DRY RUN] Would delete: {}", file_name);
            continue;
        }

        let delete_url = format!("{}{}", list_url, file_name);

        let delete_response = client
            .delete(&delete_url)
            .header("Authorization", format!("Bearer {}", bearer_token))
            .send()
            .await;

        match delete_response {
            Ok(resp) if resp.status().is_success() => {
                println!("  ✓ Deleted: {}", file_name);
                deleted += 1;
            }
            Ok(resp) => {
                eprintln!("  ⚠️  Failed to delete {}: {}", file_name, resp.status());
            }
            Err(e) => {
                eprintln!("  ⚠️  Error deleting {}: {}", file_name, e);
            }
        }
    }

    println!("✓ Deleted {} file(s)", deleted);
    Ok(())
}

async fn delete_tables_impl(
    bearer_token: &str,
    bundle: Option<&Bundle>,
    dry_run: bool,
) -> Result<(), String> {
    println!("\n🗑️  Deleting tables...");
    println!("  ⚠️  WARNING: This will delete table data!");

    // Build list of tables to delete
    let tables_to_delete: Option<HashSet<String>> = if let Some(b) = bundle {
        let mut set = HashSet::new();

        // Add main tables
        for table in &b.tables {
            set.insert(table.name.clone());
        }

        // Add summary tables
        if let Some(summary_tables) = &b.summary_tables {
            for summary in summary_tables {
                set.insert(summary.name.clone());
            }
        }

        if set.is_empty() {
            println!("  No tables to delete (none declared in bundle)");
            return Ok(());
        }

        println!("  Scope: {} table(s) from bundle", set.len());
        Some(set)
    } else {
        println!("  Scope: ALL tables in project");
        None
    };

    let table_list_text = hdx::table::get_list(bearer_token, false)
        .await
        .map_err(|e| format!("Failed to get table list: {}", e))?;

    let response_data: Value = serde_json::from_str(&table_list_text)
        .map_err(|e| format!("Failed to parse table list: {}", e))?;

    let tables: Vec<(String, String)> =
        if let Some(results) = response_data.get("results").and_then(|r| r.as_array()) {
            results
                .iter()
                .filter_map(|t| {
                    let name = t.get("name").and_then(|n| n.as_str())?;
                    let uuid = t.get("uuid").and_then(|u| u.as_str())?;
                    Some((name.to_string(), uuid.to_string()))
                })
                .collect()
        } else if let Some(arr) = response_data.as_array() {
            arr.iter()
                .filter_map(|t| {
                    let name = t.get("name").and_then(|n| n.as_str())?;
                    let uuid = t.get("uuid").and_then(|u| u.as_str())?;
                    Some((name.to_string(), uuid.to_string()))
                })
                .collect()
        } else {
            vec![]
        };

    if tables.is_empty() {
        println!("  No tables found on cluster");
        return Ok(());
    }

    println!("  Found {} total table(s) on cluster", tables.len());

    let mut deleted = 0;
    for (name, uuid) in tables {
        // Filter by bundle if specified
        if let Some(ref filter_set) = tables_to_delete {
            if !filter_set.contains(&name) {
                continue; // Skip - not in this bundle
            }
        }

        if dry_run {
            println!("  [DRY RUN] Would delete: {}", name);
            continue;
        }

        match hdx::table::delete(bearer_token, &uuid).await {
            Ok(_) => {
                println!("  ✓ Deleted: {}", name);
                deleted += 1;
            }
            Err(e) => {
                eprintln!("  ⚠️  Error deleting {}: {}", name, e);
            }
        }
    }

    println!("✓ Deleted {} table(s)", deleted);
    Ok(())
}
