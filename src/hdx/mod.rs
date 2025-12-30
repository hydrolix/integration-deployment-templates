pub mod auth;
pub mod dictionaries;
pub mod functions;
pub mod table;
use lazy_static::lazy_static;
use reqwest::Client;

// These are static but not secret
const ORG_UUID: &str = "d867bf48-4281-4496-8432-a93aa989aae6"; // markeplace-dev
const PROJ_UUID: &str = "67e79a3c-f7d6-4b33-a207-fef4579a3152"; // markeplace-dev cdn_test_project
const PROJ_NAME: &str = "cdn_test_project";

const HTTP_TIMEOUT: u64 = 120;

lazy_static! {
    static ref CLIENT: Client = reqwest::Client::new();
    static ref BUNDLE_TESTING_CLUSTER: String =
        std::env::var("BUNDLE_TESTING_CLUSTER").unwrap_or_else(|_| "".to_string());
    static ref BUNDLE_TESTING_USERNAME: String =
        std::env::var("BUNDLE_TESTING_USERNAME").unwrap_or_else(|_| "".to_string());
    static ref BUNDLE_TESTING_PASSWORD: String =
        std::env::var("BUNDLE_TESTING_PASSWORD").unwrap_or_else(|_| "".to_string());
    static ref FOR_MARKETPLACE: bool = {
        let args: Vec<String> = std::env::args().collect();
        args.contains(&"--marketplace".to_string())
    };
}

pub fn get_project_name() -> String {
    PROJ_NAME.to_string()
}
