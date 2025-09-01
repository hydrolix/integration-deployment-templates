// Pointless comment

use std::path::PathBuf;
use tokio::fs;
use walkdir::WalkDir;

mod bundle_struct;
mod dashboard_is_valid;
mod no_bad_checksums;
mod no_duplicate_tokens;

use crate::bundle_struct::Bundle;

#[tokio::main]
async fn main() {
    // We only check bundles at the root directory
    let bundle_list = find_bundle_files();

    println!("list={:?}", bundle_list);
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

        let base_dir = file_path.replace("./", "").replace("/bundle.json", "");
        println!("base_dir={base_dir}");

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
    match no_duplicate_tokens::run(bundle).await {
        Ok(_) => (),
        Err(e) => return Err(format!("Found duplicate tokens: error={e}")),
    }

    match no_bad_checksums::run(base, bundle).await {
        Ok(_) => (),
        Err(e) => return Err(format!("Found bad checksum: error={e}")),
    }

    match dashboard_is_valid::run(base, bundle).await {
        Ok(_) => (),
        Err(e) => return Err(format!("Found bad checksum: error={e}")),
    }

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
