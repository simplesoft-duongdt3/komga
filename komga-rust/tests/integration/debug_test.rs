use crate::common::*;
use sqlx::Row;

#[tokio::test]
async fn test_debug_create_and_get() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    // Create library - print full response
    let resp = ctx.auth_post_json(&token, "/api/v1/libraries",
        &serde_json::json!({"name": "Debug Lib", "root": "/tmp/debug"}),
    ).await;
    println!("Create status: {}", resp.status());
    let create_body = resp.text().await.unwrap();
    println!("Create body: {}", create_body);

    let created: serde_json::Value = serde_json::from_str(&create_body).unwrap();
    let id = created["id"].as_str().unwrap();
    println!("Library ID: {}", id);

    // Check DB directly BEFORE GET
    let rows: Vec<(String,)> = sqlx::query_as(r#"SELECT "ID" FROM "LIBRARY""#)
        .fetch_all(&ctx.pool).await.unwrap();
    println!("Libraries in DB: {:?}", rows);

    // Direct repo find_by_id test
    let repo = komga_rust::domain::repository::LibraryRepository::new(ctx.pool.clone());
    let lib_uuid = uuid::Uuid::parse_str(&id).unwrap();
    println!("Parsed UUID: {}", lib_uuid);
    match repo.find_by_id(lib_uuid).await {
        Ok(Some(lib)) => println!("Repo found library: {}", lib.name),
        Ok(None) => println!("Repo returned None!"),
        Err(e) => println!("Repo error: {}", e),
    }
    println!("UUID string for query: '{}'", lib_uuid.to_string());

    // List all libraries
    let list_resp = ctx.get("/api/v1/libraries").await;
    println!("List ALL status: {}", list_resp.status());
    let list_body = list_resp.text().await.unwrap();
    println!("List ALL body: {}", list_body);

    // Now try GET
    let resp = ctx.get(&format!("/api/v1/libraries/{}", id)).await;
    let get_status = resp.status();
    let get_body = resp.text().await.unwrap();
    println!("GET status: {}", get_status);
    println!("GET body: {}", get_body);

    assert_eq!(get_status, 200);
}
