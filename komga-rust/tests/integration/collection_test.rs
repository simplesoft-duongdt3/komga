use crate::common::*;

#[tokio::test]
async fn test_create_collection() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_post_json(&token, "/api/v1/collections",
        &serde_json::json!({"name": "My Collection", "ordered": true}),
    ).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "My Collection");
    assert_eq!(body["ordered"], true);
    assert!(body["id"].as_str().unwrap().len() > 0);
}

#[tokio::test]
async fn test_list_collections() {
    let ctx = setup_test_context().await;

    let resp = ctx.get("/api/v1/collections").await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["content"].is_array());
}

#[tokio::test]
async fn test_get_collection() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_post_json(&token, "/api/v1/collections",
        &serde_json::json!({"name": "Get Coll"}),
    ).await;
    assert_eq!(resp.status(), 200, "Failed to create collection");
    let created: serde_json::Value = resp.json().await.unwrap();
    let id = created["id"].as_str().unwrap();

    let resp = ctx.get(&format!("/api/v1/collections/{}", id)).await;
    assert_eq!(resp.status(), 200, "Failed to GET collection {}", id);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "Get Coll");
}

#[tokio::test]
async fn test_update_collection() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_post_json(&token, "/api/v1/collections",
        &serde_json::json!({"name": "Update Coll"}),
    ).await;
    assert_eq!(resp.status(), 200, "Failed to create collection");
    let created: serde_json::Value = resp.json().await.unwrap();
    let id = created["id"].as_str().unwrap();

    let resp = ctx.auth_patch_json(&token, &format!("/api/v1/collections/{}", id),
        &serde_json::json!({"name": "Updated Collection"}),
    ).await;
    assert_eq!(resp.status(), 200, "Failed to PATCH collection");
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "Updated Collection");
}

#[tokio::test]
async fn test_delete_collection() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_post_json(&token, "/api/v1/collections",
        &serde_json::json!({"name": "Delete Coll"}),
    ).await;
    assert_eq!(resp.status(), 200, "Failed to create collection");
    let created: serde_json::Value = resp.json().await.unwrap();
    let id = created["id"].as_str().unwrap();

    let resp = ctx.auth_delete(&token, &format!("/api/v1/collections/{}", id)).await;
    assert!(resp.status() == 200 || resp.status() == 204, "DELETE returned {}", resp.status());

    let resp = ctx.get(&format!("/api/v1/collections/{}", id)).await;
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn test_collection_series_empty() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_post_json(&token, "/api/v1/collections",
        &serde_json::json!({"name": "Series Coll"}),
    ).await;
    assert_eq!(resp.status(), 200);
    let created: serde_json::Value = resp.json().await.unwrap();
    let id = created["id"].as_str().unwrap();

    let resp = ctx.get(&format!("/api/v1/collections/{}/series", id)).await;
    assert_eq!(resp.status(), 200);
    let body: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(body.is_empty());
}

#[tokio::test]
async fn test_search() {
    let ctx = setup_test_context().await;

    let resp = ctx.get("/api/v1/search?query=test").await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["books"].is_array());
    assert!(body["series"].is_array());
}

#[tokio::test]
async fn test_search_empty_query() {
    let ctx = setup_test_context().await;

    let resp = ctx.get("/api/v1/search?query=").await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["total"], 0);
}

#[tokio::test]
async fn test_api_keys_crud() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_get(&token, "/api/v2/users/me/api-keys").await;
    assert_eq!(resp.status(), 200);

    let resp = ctx.auth_post_json(&token, "/api/v2/users/me/api-keys",
        &serde_json::json!({"name": "test-key"}),
    ).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let key_id = body["id"].as_str().unwrap().to_string();

    let resp = ctx.auth_get(&token, "/api/v2/users/me/api-keys").await;
    assert_eq!(resp.status(), 200);
    let keys: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(keys.len() >= 1);

    let resp = ctx.auth_delete(&token, &format!("/api/v2/users/me/api-keys/{}", key_id)).await;
    assert_eq!(resp.status(), 204);
}
