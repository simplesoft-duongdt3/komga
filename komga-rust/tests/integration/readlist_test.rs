use crate::common::*;

#[tokio::test]
async fn test_create_readlist() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_post_json(&token, "/api/v1/readlists",
        &serde_json::json!({"name": "My Reading List", "summary": "A test list", "ordered": true}),
    ).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "My Reading List");
    assert_eq!(body["summary"], "A test list");
    assert_eq!(body["ordered"], true);
    assert!(body["id"].as_str().unwrap().len() > 0);
}

#[tokio::test]
async fn test_list_readlists() {
    let ctx = setup_test_context().await;

    let resp = ctx.get("/api/v1/readlists").await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["content"].is_array());
}

#[tokio::test]
async fn test_get_readlist() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_post_json(&token, "/api/v1/readlists",
        &serde_json::json!({"name": "Get RL Test", "summary": ""}),
    ).await;
    assert_eq!(resp.status(), 200, "Failed to create readlist");
    let created: serde_json::Value = resp.json().await.unwrap();
    let id = created["id"].as_str().unwrap();

    let resp = ctx.get(&format!("/api/v1/readlists/{}", id)).await;
    assert_eq!(resp.status(), 200, "Failed to GET readlist {}", id);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "Get RL Test");
}

#[tokio::test]
async fn test_update_readlist() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_post_json(&token, "/api/v1/readlists",
        &serde_json::json!({"name": "Update RL", "summary": ""}),
    ).await;
    assert_eq!(resp.status(), 200, "Failed to create readlist");
    let created: serde_json::Value = resp.json().await.unwrap();
    let id = created["id"].as_str().unwrap();

    let resp = ctx.auth_patch_json(&token, &format!("/api/v1/readlists/{}", id),
        &serde_json::json!({"name": "Updated ReadList", "summary": "Updated summary"}),
    ).await;
    assert_eq!(resp.status(), 200, "Failed to PATCH readlist");
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "Updated ReadList");
}

#[tokio::test]
async fn test_delete_readlist() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_post_json(&token, "/api/v1/readlists",
        &serde_json::json!({"name": "Delete RL", "summary": ""}),
    ).await;
    assert_eq!(resp.status(), 200, "Failed to create readlist");
    let created: serde_json::Value = resp.json().await.unwrap();
    let id = created["id"].as_str().unwrap();

    let resp = ctx.auth_delete(&token, &format!("/api/v1/readlists/{}", id)).await;
    assert!(resp.status() == 200 || resp.status() == 204, "DELETE returned {}", resp.status());

    let resp = ctx.get(&format!("/api/v1/readlists/{}", id)).await;
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn test_readlist_books_empty() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_post_json(&token, "/api/v1/readlists",
        &serde_json::json!({"name": "Books RL", "summary": ""}),
    ).await;
    assert_eq!(resp.status(), 200, "Failed to create readlist");
    let created: serde_json::Value = resp.json().await.unwrap();
    let id = created["id"].as_str().unwrap();

    let resp = ctx.get(&format!("/api/v1/readlists/{}/books", id)).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["content"].is_array());
}

#[tokio::test]
async fn test_readlist_thumbnails() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_post_json(&token, "/api/v1/readlists",
        &serde_json::json!({"name": "Thumb RL", "summary": ""}),
    ).await;
    assert_eq!(resp.status(), 200);
    let created: serde_json::Value = resp.json().await.unwrap();
    let id = created["id"].as_str().unwrap();

    let resp = ctx.get(&format!("/api/v1/readlists/{}/thumbnails", id)).await;
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_readlist_tachiyomi_progress() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_post_json(&token, "/api/v1/readlists",
        &serde_json::json!({"name": "Tachi RL", "summary": ""}),
    ).await;
    let created: serde_json::Value = resp.json().await.unwrap();
    let id = created["id"].as_str().unwrap();

    let resp = ctx.get(&format!("/api/v1/readlists/{}/read-progress/tachiyomi", id)).await;
    assert_eq!(resp.status(), 200);

    let resp = ctx.put_json(
        &format!("/api/v1/readlists/{}/read-progress/tachiyomi", id),
        &serde_json::json!({"readChapters": []}),
    ).await;
    assert_eq!(resp.status(), 204);
}
