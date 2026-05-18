use crate::models::bundle::Bundle;
use std::path::Path;
use std::process::Command;

/// Warns if bundle.json's `name` field was ever different in git history.
/// Renaming a bundle can break existing customer deployments that reference the old name.
pub async fn run(base: &str, bundle: &Bundle) -> Vec<String> {
    let mut warnings = Vec::new();
    let current_name = &bundle.name;

    // Get the git repo root so we can build repo-relative paths for `git show`.
    let root_out = match Command::new("git")
        .args(["-C", base, "rev-parse", "--show-toplevel"])
        .output()
    {
        Ok(out) if out.status.success() => out.stdout,
        _ => return warnings,
    };
    let repo_root = std::str::from_utf8(&root_out).unwrap_or("").trim().to_string();
    if repo_root.is_empty() {
        return warnings;
    }

    // Compute the bundle.json path relative to the repo root.
    let base_canon = match Path::new(base).canonicalize() {
        Ok(p) => p,
        Err(_) => return warnings,
    };
    let rel_dir = match base_canon.strip_prefix(&repo_root) {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => return warnings,
    };
    let git_path = if rel_dir.is_empty() {
        "bundle.json".to_string()
    } else {
        format!("{}/bundle.json", rel_dir)
    };

    // Get the two most recent commits that touched bundle.json.
    let log_out = match Command::new("git")
        .args(["-C", base, "log", "--format=%H", "-2", "--", "bundle.json"])
        .output()
    {
        Ok(out) if out.status.success() => out.stdout,
        _ => return warnings,
    };
    let log_str = std::str::from_utf8(&log_out).unwrap_or("").trim().to_string();
    let commits: Vec<&str> = log_str.lines().collect();

    // Need at least two commits to have a previous state to compare against.
    if commits.len() < 2 {
        return warnings;
    }

    let prev_hash = commits[1].trim();
    let show_out = match Command::new("git")
        .args(["-C", base, "show", &format!("{}:{}", prev_hash, git_path)])
        .output()
    {
        Ok(out) if out.status.success() => out.stdout,
        _ => return warnings,
    };
    let content = match std::str::from_utf8(&show_out) {
        Ok(s) => s,
        Err(_) => return warnings,
    };
    let json: serde_json::Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(_) => return warnings,
    };
    if let Some(old_name) = json.get("name").and_then(|n| n.as_str()) {
        if old_name != current_name.as_str() {
            warnings.push(format!(
                "bundle.json name was changed from '{}' to '{}' in the latest commit. \
                 Renaming a bundle can break existing customer deployments.",
                old_name, current_name
            ));
        }
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn git(dir: &str, args: &[&str]) {
        let out = Command::new("git").args(args).current_dir(dir).output().unwrap();
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn write_bundle_json(dir: &str, name: &str, version: &str) {
        let content = format!(
            r#"{{
  "name": "{}",
  "source": "test",
  "method": "http_streaming",
  "solution": false,
  "beta": false,
  "base_url": "https://example.com",
  "dashboard": {{"path": "dashboards/main.json", "project_var": "__PROJECT_NAME__"}},
  "tables": [],
  "ui": {{
    "primary_url": "https://example.com",
    "method": {{"full_title": "HTTP", "icon_url": "https://example.com/icon.png"}},
    "source": {{"full_title": "Test", "icon_url": "https://example.com/icon.png"}},
    "data_category": "cdn"
  }},
  "metadata": {{
    "version": "{}",
    "maintainer": "test",
    "description": "test",
    "channel_type": "AWS"
  }}
}}"#,
            name, version
        );
        std::fs::write(format!("{}/bundle.json", dir), content).unwrap();
    }

    fn make_bundle(name: &str) -> Bundle {
        serde_json::from_str(&format!(
            r#"{{
                "name": "{}",
                "source": "test",
                "method": "http_streaming",
                "solution": false,
                "beta": false,
                "base_url": "https://example.com",
                "dashboard": {{"path": "dashboards/main.json", "project_var": "__PROJECT_NAME__"}},
                "tables": [],
                "ui": {{
                    "primary_url": "https://example.com",
                    "method": {{"full_title": "HTTP", "icon_url": "https://example.com/icon.png"}},
                    "source": {{"full_title": "Test", "icon_url": "https://example.com/icon.png"}},
                    "data_category": "cdn"
                }},
                "metadata": {{
                    "version": "1.0.0",
                    "maintainer": "test",
                    "description": "test",
                    "channel_type": "AWS"
                }}
            }}"#,
            name
        ))
        .unwrap()
    }

    fn init_git_repo(dir: &TempDir) -> String {
        let path = dir.path().to_str().unwrap().to_string();
        git(&path, &["init"]);
        git(&path, &["config", "user.email", "test@example.com"]);
        git(&path, &["config", "user.name", "Test"]);
        path
    }

    #[tokio::test]
    async fn test_no_name_change_no_warning() {
        let dir = tempfile::tempdir().unwrap();
        let path = init_git_repo(&dir);

        // Two commits with same name but different version — no rename
        write_bundle_json(&path, "my_bundle", "1.0.0");
        git(&path, &["add", "bundle.json"]);
        git(&path, &["commit", "-m", "initial"]);

        write_bundle_json(&path, "my_bundle", "1.0.1");
        git(&path, &["add", "bundle.json"]);
        git(&path, &["commit", "-m", "bump version"]);

        let bundle = make_bundle("my_bundle");
        let warnings = run(&path, &bundle).await;
        assert!(warnings.is_empty(), "No rename should produce no warnings: {:?}", warnings);
    }

    #[tokio::test]
    async fn test_name_changed_produces_warning() {
        let dir = tempfile::tempdir().unwrap();
        let path = init_git_repo(&dir);

        // Commit with original name
        write_bundle_json(&path, "old_bundle_name", "1.0.0");
        git(&path, &["add", "bundle.json"]);
        git(&path, &["commit", "-m", "initial"]);

        // Commit renaming the bundle
        write_bundle_json(&path, "new_bundle_name", "1.0.0");
        git(&path, &["add", "bundle.json"]);
        git(&path, &["commit", "-m", "rename bundle"]);

        let bundle = make_bundle("new_bundle_name");
        let warnings = run(&path, &bundle).await;
        assert!(!warnings.is_empty(), "Rename should produce a warning");
        let w = &warnings[0];
        assert!(w.contains("old_bundle_name"), "Warning should mention old name: {w}");
        assert!(w.contains("new_bundle_name"), "Warning should mention new name: {w}");
    }

    #[tokio::test]
    async fn test_single_commit_no_warning() {
        let dir = tempfile::tempdir().unwrap();
        let path = init_git_repo(&dir);

        write_bundle_json(&path, "my_bundle", "1.0.0");
        git(&path, &["add", "bundle.json"]);
        git(&path, &["commit", "-m", "initial"]);

        let bundle = make_bundle("my_bundle");
        let warnings = run(&path, &bundle).await;
        assert!(warnings.is_empty(), "Single commit should produce no warnings: {:?}", warnings);
    }

    #[tokio::test]
    async fn test_not_in_git_repo_no_panic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_str().unwrap().to_string();
        // No git init — not a repo

        let bundle = make_bundle("some_bundle");
        let warnings = run(&path, &bundle).await;
        assert!(warnings.is_empty(), "Non-repo should produce no warnings (graceful): {:?}", warnings);
    }
}
