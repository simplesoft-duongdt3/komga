use crate::common::*;

#[tokio::test]
async fn test_list_series() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_post_json(&token, "/api/v1/libraries",
        &serde_json::json!({"name": "Series Test Lib", "root": "/tmp/series_test"}),
    ).await;
    let lib: serde_json::Value = resp.json().await.unwrap();
    let lib_id = lib["id"].as_str().unwrap();

    let resp = ctx.get(&format!("/api/v1/series?library_id={}", lib_id)).await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["content"].is_array());
    assert_eq!(body["content"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_series_collections_empty() {
    let ctx = setup_test_context().await;

    let resp = ctx.get("/api/v1/series/00000000-0000-0000-0000-000000000000/collections").await;
    assert_eq!(resp.status(), 200);
    let body: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(body.is_empty());
}

#[tokio::test]
async fn test_series_tachiyomi_progress() {
    let ctx = setup_test_context().await;

    let resp = ctx.get("/api/v2/series/00000000-0000-0000-0000-000000000000/read-progress/tachiyomi").await;
    assert_eq!(resp.status(), 200);

    let resp = ctx.put_json("/api/v2/series/00000000-0000-0000-0000-000000000000/read-progress/tachiyomi",
        &serde_json::json!({"readChapters": []}),
    ).await;
    assert_eq!(resp.status(), 204);
}
