use std::path::PathBuf;
use tokio::fs;
use walkdir::WalkDir;

mod bundle_struct;

use crate::bundle_struct::Bundle;

#[tokio::main]
async fn main() {

	// We only check bundles at the root directory
    let bundle_list = find_bundle_files();

    println!("list={:?}", bundle_list);
    for b in &bundle_list {
        let path = PathBuf::from(b);
        let string = path
            .into_os_string()
            .into_string()
            .unwrap_or_else(|os_str| os_str.to_string_lossy().into_owned());

        let bundle = match file_to_bundle(&string).await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("ERROR: Failed to decode the structure: {e}");
                std::process::exit(1);
            }
        };
        println!("Bundle={:?}", bundle);
    }

    println!("Success");
    std::process::exit(0);
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

