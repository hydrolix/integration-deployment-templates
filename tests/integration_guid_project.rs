//! Integration tests for GUID project isolation (LOTC-803).
//!
//! These tests require a live Hydrolix cluster. Set the following env vars:
//!   BUNDLE_TESTING_CLUSTER, BUNDLE_TESTING_USERNAME, BUNDLE_TESTING_PASSWORD
//!
//! Run with: cargo test --test integration_guid_project -- --ignored

use bundle_validator::hdx;

fn skip_if_no_cluster() -> bool {
    std::env::var("BUNDLE_TESTING_CLUSTER")
        .unwrap_or_default()
        .is_empty()
}

#[tokio::test]
#[ignore]
async fn test_create_guid_project_returns_uuid() {
    if skip_if_no_cluster() {
        eprintln!("SKIPPED: No BUNDLE_TESTING_CLUSTER set");
        return;
    }

    let bearer_token = hdx::auth::get_token().await.expect("auth failed");
    let project_name = hdx::generate_guid_project_name();

    println!("Creating test project: {}", project_name);
    let uuid = hdx::create_project(&bearer_token, &project_name)
        .await
        .expect("create_project failed");

    assert!(!uuid.is_empty(), "UUID should not be empty");
    println!("Created project {} with UUID {}", project_name, uuid);

    // Clean up: delete the project we just created
    hdx::delete_project(&bearer_token, &project_name)
        .await
        .expect("cleanup delete_project failed");
    println!("Cleaned up project {}", project_name);
}

#[tokio::test]
#[ignore]
async fn test_delete_project_removes_it() {
    if skip_if_no_cluster() {
        eprintln!("SKIPPED: No BUNDLE_TESTING_CLUSTER set");
        return;
    }

    let bearer_token = hdx::auth::get_token().await.expect("auth failed");
    let project_name = hdx::generate_guid_project_name();

    // Create
    let _uuid = hdx::create_project(&bearer_token, &project_name)
        .await
        .expect("create_project failed");

    // Delete
    hdx::delete_project(&bearer_token, &project_name)
        .await
        .expect("delete_project failed");

    // Verify it's gone by trying to find it
    let result = hdx::find_project_uuid(&bearer_token, &project_name).await;
    assert!(
        result.is_err(),
        "Project should not exist after deletion, but found: {:?}",
        result
    );
}

#[tokio::test]
#[ignore]
async fn test_create_table_in_guid_project() {
    if skip_if_no_cluster() {
        eprintln!("SKIPPED: No BUNDLE_TESTING_CLUSTER set");
        return;
    }

    let bearer_token = hdx::auth::get_token().await.expect("auth failed");
    let project_name = hdx::generate_guid_project_name();

    // Create project
    let project_uuid = hdx::create_project(&bearer_token, &project_name)
        .await
        .expect("create_project failed");

    println!("Created project {} (UUID: {})", project_name, project_uuid);

    // Create a table in that project
    let table_name = "logs";
    let table_uuid = hdx::table::create_in_project(&bearer_token, &project_uuid, table_name)
        .await
        .expect("create table failed");

    assert!(!table_uuid.is_empty(), "Table UUID should not be empty");
    println!(
        "Created table '{}' (UUID: {}) in project {}",
        table_name, table_uuid, project_name
    );

    // Clean up
    hdx::delete_project(&bearer_token, &project_name)
        .await
        .expect("cleanup delete_project failed");
    println!("Cleaned up project {}", project_name);
}
