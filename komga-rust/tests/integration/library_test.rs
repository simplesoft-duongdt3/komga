use crate::common::*;

#[tokio::test]
async fn test_create_library() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_post_json(&token, "/api/v1/libraries",
        &serde_json::json!({"name": "Test Library", "root": "/tmp/test_lib"}),
    ).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "Test Library");
    assert_eq!(body["root"], "/tmp/test_lib");
    assert!(body["id"].as_str().unwrap().len() > 0);
}

#[tokio::test]
async fn test_list_libraries() {
    let ctx = setup_test_context().await;

    let resp = ctx.get("/api/v1/libraries").await;
    assert_eq!(resp.status(), 200);
    let body: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(body.len() >= 0usize);
}

#[tokio::test]
async fn test_get_library() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_post_json(&token, "/api/v1/libraries",
        &serde_json::json!({"name": "Get Test", "root": "/tmp/get_test"}),
    ).await;
    let created: serde_json::Value = resp.json().await.unwrap();
    let id = created["id"].as_str().unwrap();

    let resp = ctx.get(&format!("/api/v1/libraries/{}", id)).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "Get Test");
}

#[tokio::test]
async fn test_patch_library() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_post_json(&token, "/api/v1/libraries",
        &serde_json::json!({"name": "Patch Test", "root": "/tmp/patch_test"}),
    ).await;
    let created: serde_json::Value = resp.json().await.unwrap();
    let id = created["id"].as_str().unwrap();

    let resp = ctx.auth_patch_json(&token, &format!("/api/v1/libraries/{}", id),
        &serde_json::json!({"name": "Patched Library", "hashFiles": false}),
    ).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "Patched Library");
}

#[tokio::test]
async fn test_delete_library() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_post_json(&token, "/api/v1/libraries",
        &serde_json::json!({"name": "Delete Test", "root": "/tmp/delete_test"}),
    ).await;
    let created: serde_json::Value = resp.json().await.unwrap();
    let id = created["id"].as_str().unwrap();

    let resp = ctx.auth_delete(&token, &format!("/api/v1/libraries/{}", id)).await;
    assert_eq!(resp.status(), 200);

    let resp = ctx.get(&format!("/api/v1/libraries/{}", id)).await;
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn test_library_analyze_and_refresh() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_post_json(&token, "/api/v1/libraries",
        &serde_json::json!({"name": "Scan Test", "root": "/tmp/scan_test"}),
    ).await;
    let created: serde_json::Value = resp.json().await.unwrap();
    let id = created["id"].as_str().unwrap();

    let resp = ctx.auth_post(&token, &format!("/api/v1/libraries/{}/analyze", id)).await;
    assert_eq!(resp.status(), 202);

    let resp = ctx.auth_post(&token, &format!("/api/v1/libraries/{}/metadata/refresh", id)).await;
    assert_eq!(resp.status(), 202);
}

#[tokio::test]
async fn test_empty_trash() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_post_json(&token, "/api/v1/libraries",
        &serde_json::json!({"name": "Trash Test", "root": "/tmp/trash_test"}),
    ).await;
    let created: serde_json::Value = resp.json().await.unwrap();
    let id = created["id"].as_str().unwrap();

    let resp = ctx.auth_post(&token, &format!("/api/v1/libraries/{}/empty-trash", id)).await;
    assert_eq!(resp.status(), 204);
}

#[tokio::test]
async fn test_referential() {
    let ctx = setup_test_context().await;

    let resp = ctx.get("/api/v1/referential/authors").await;
    assert_eq!(resp.status(), 200);

    let resp = ctx.get("/api/v1/referential/genres").await;
    assert_eq!(resp.status(), 200);

    let resp = ctx.get("/api/v1/referential/tags").await;
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_task_list_and_delete() {
    let ctx = setup_test_context().await;

    let resp = ctx.get("/api/v1/tasks").await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["content"].is_array());

    let resp = ctx.delete("/api/v1/tasks").await;
    assert_eq!(resp.status(), 204);
}

#[tokio::test]
async fn test_settings_get_and_update() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_get(&token, "/api/v1/settings").await;
    assert_eq!(resp.status(), 200);

    let resp = ctx.auth_patch_json(&token, "/api/v1/settings",
        &serde_json::json!({"scanStartup": "true", "taskPoolSize": "8"}),
    ).await;
    assert_eq!(resp.status(), 204);

    let resp = ctx.auth_get(&token, "/api/v1/settings").await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["scanStartup"], "true");
    assert_eq!(body["taskPoolSize"], "8");
}

#[tokio::test]
async fn test_page_hashes() {
    let ctx = setup_test_context().await;

    let resp = ctx.get("/api/v1/page-hashes").await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["content"].is_array());
}

#[tokio::test]
async fn test_client_settings() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_get(&token, "/api/v2/client-settings").await;
    assert_eq!(resp.status(), 200);

    let resp = ctx.auth_put_json(&token, "/api/v2/client-settings",
        &serde_json::json!({"locale": "fr", "theme": "dark"}),
    ).await;
    assert_eq!(resp.status(), 204);

    let resp = ctx.auth_delete(&token, "/api/v2/client-settings").await;
    assert_eq!(resp.status(), 204);
}

#[tokio::test]
async fn test_fonts() {
    let ctx = setup_test_context().await;

    let resp = ctx.get("/api/v1/fonts").await;
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_get_library_not_found() {
    let ctx = setup_test_context().await;

    let resp = ctx.get("/api/v1/libraries/00000000-0000-0000-0000-000000000000").await;
    assert_eq!(resp.status(), 404);
}
