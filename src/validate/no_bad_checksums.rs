use sha2::Digest;
use tokio::fs;

use crate::bundle_struct::Bundle;

pub async fn run(base: &str, bundle: &Bundle) -> Result<(), String> {
    let full_path = format!("{base}/{}", bundle.dashboard.path);

    match check_checksum(&full_path, bundle.dashboard.sha256.as_deref()).await {
        Ok(_) => (),
        Err(e) => {
            return Err(format!(
                "Failed dashboard checksum: full_path={full_path} error={e}"
            ))
        }
    }

    for t in &bundle.tables {
        for tt in &t.transforms {
            let full_path = format!("{base}/{}", tt.path);
            match check_checksum(&full_path, tt.sha256.as_deref()).await {
                Ok(_) => (),
                Err(e) => {
                    return Err(format!(
                        "Failed transformation checksum: path={full_path} error={e}"
                    ))
                }
            }
        }
    }

    Ok(())
}

async fn check_checksum(file_path: &str, checksum: Option<&str>) -> Result<(), String> {
    let content = match fs::read_to_string(file_path).await {
        Ok(v) => v,
        Err(e) => {
            return Err(format!(
                "ERROR: {}.{} Failed to read file_path={}: error={}",
                file!(),
                line!(),
                file_path,
                e
            ));
        }
    };

    // If there is a SHA256 then verify it
    if let Some(v) = checksum {
        let sha256 = generate_sha256(&content);
        if sha256 != v {
            return Err(format!(
                "ERROR: {}.{} SHA256 {} does not match for local file {}",
                file!(),
                line!(),
                sha256,
                file_path
            ));
        }
    }
    Ok(())
}

/// Generate a SHA256 hash from a string
fn generate_sha256(input: &str) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}
