use crate::common::*;

#[tokio::test]
async fn test_claim_account() {
    let ctx = setup_test_context().await;

    let resp = ctx.post_json(
        "/api/v1/claim",
        &serde_json::json!({"email": "admin@test.com", "password": "admin123"}),
    ).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["token"].as_str().unwrap().len() > 10);
}

#[tokio::test]
async fn test_claim_already_claimed() {
    let ctx = setup_test_context().await;

    let resp = ctx.post_json(
        "/api/v1/claim",
        &serde_json::json!({"email": "admin@test.com", "password": "admin123"}),
    ).await;
    assert_eq!(resp.status(), 200);

    let resp = ctx.post_json(
        "/api/v1/claim",
        &serde_json::json!({"email": "admin2@test.com", "password": "admin123"}),
    ).await;
    assert_eq!(resp.status(), 409);
}

#[tokio::test]
async fn test_claim_status() {
    let ctx = setup_test_context().await;

    let resp = ctx.get("/api/v1/claim").await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["isClaimed"].as_bool().unwrap() == false);

    register_and_login(&ctx).await;

    let resp = ctx.get("/api/v1/claim").await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["isClaimed"].as_bool().unwrap() == true);
}

#[tokio::test]
async fn test_login() {
    let ctx = setup_test_context().await;
    register_and_login(&ctx).await;

    let resp = ctx.post_json(
        "/api/v1/login",
        &serde_json::json!({"email": "test@test.com", "password": "test123"}),
    ).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["token"].as_str().unwrap().len() > 10);
}

#[tokio::test]
async fn test_login_invalid_password() {
    let ctx = setup_test_context().await;
    register_and_login(&ctx).await;

    let resp = ctx.post_json(
        "/api/v1/login",
        &serde_json::json!({"email": "test@test.com", "password": "wrongpassword"}),
    ).await;

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn test_me_endpoint() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_get(&token, "/api/v2/users/me").await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["id"].as_str().is_some());
    assert!(body["email"].as_str().is_some());
    assert!(body["roles"].is_array());
}

#[tokio::test]
async fn test_list_users() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_get(&token, "/api/v2/users").await;
    assert_eq!(resp.status(), 200);
    let body: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(body.len() >= 1);
}

#[tokio::test]
async fn test_register_user() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_post_json(&token, "/api/v2/users",
        &serde_json::json!({"email": "user2@test.com", "password": "pass123"}),
    ).await;
    assert_eq!(resp.status(), 200);

    let resp = ctx.auth_get(&token, "/api/v2/users").await;
    assert_eq!(resp.status(), 200);
    let body: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(body.len() >= 2);
}

#[tokio::test]
async fn test_delete_user() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_post_json(&token, "/api/v2/users",
        &serde_json::json!({"email": "todelete@test.com", "password": "pass123"}),
    ).await;
    assert_eq!(resp.status(), 200);

    let resp = ctx.auth_get(&token, "/api/v2/users").await;
    let body: Vec<serde_json::Value> = resp.json().await.unwrap();
    let user_id = body.iter()
        .find(|u| u["email"] == "todelete@test.com")
        .map(|u| u["id"].as_str().unwrap().to_string())
        .unwrap();

    let resp = ctx.auth_delete(&token, &format!("/api/v2/users/{}", user_id)).await;
    assert_eq!(resp.status(), 204);

    let resp = ctx.auth_get(&token, "/api/v2/users").await;
    let body: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(body.iter().all(|u| u["email"] != "todelete@test.com"));
}

#[tokio::test]
async fn test_update_password() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_put_json(&token, "/api/v2/users/me/password",
        &serde_json::json!({"password": "newpassword123"}),
    ).await;
    assert_eq!(resp.status(), 204);

    let resp = ctx.post_json(
        "/api/v1/login",
        &serde_json::json!({"email": "test@test.com", "password": "newpassword123"}),
    ).await;
    assert_eq!(resp.status(), 200);

    let resp = ctx.post_json(
        "/api/v1/login",
        &serde_json::json!({"email": "test@test.com", "password": "test123"}),
    ).await;
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn test_cookie_login() {
    let ctx = setup_test_context().await;
    register_and_login(&ctx).await;

    let resp = ctx.get("/api/v2/login/set-cookie").await;
    assert_eq!(resp.status(), 200);
    let cookie = resp.headers().get("set-cookie");
    assert!(cookie.is_some());
    assert!(cookie.unwrap().to_str().unwrap().contains("SESSION="));
}

#[tokio::test]
async fn test_logout() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_get(&token, "/api/v2/logout").await;
    assert_eq!(resp.status(), 200);
    let cookie = resp.headers().get("set-cookie");
    assert!(cookie.is_some());
}

#[tokio::test]
async fn test_auth_activity() {
    let ctx = setup_test_context().await;
    let token = register_and_login(&ctx).await;

    let resp = ctx.auth_get(&token, "/api/v2/users/me/authentication-activity").await;
    assert_eq!(resp.status(), 200);
    let body: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(body.len() >= 0usize);
}

#[tokio::test]
async fn test_releases() {
    let ctx = setup_test_context().await;

    let resp = ctx.get("/api/v1/releases").await;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["version"].as_str().is_some());
}

#[tokio::test]
async fn test_oauth2_providers() {
    let ctx = setup_test_context().await;

    let resp = ctx.get("/api/v2/oauth2/providers").await;
    assert_eq!(resp.status(), 200);
    let _body: Vec<serde_json::Value> = resp.json().await.unwrap();
}
