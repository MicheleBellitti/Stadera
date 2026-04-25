use stadera_withings::{WithingsError, WithingsOauth};
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn oauth_pointing_to(server: &MockServer) -> WithingsOauth {
    WithingsOauth::with_urls(
        "test-client-id".to_string(),
        "test-client-secret".to_string(),
        "http://localhost:7878/callback".to_string(),
        format!("{}/oauth2_user/authorize2", server.uri()),
        format!("{}/v2/oauth2", server.uri()),
    )
    .unwrap()
}

#[test]
fn authorization_url_contains_expected_params() {
    // Use a dummy server URI; we don't hit the network in this test.
    let oauth = WithingsOauth::with_urls(
        "test-client-id".to_string(),
        "test-secret".to_string(),
        "http://localhost:7878/callback".to_string(),
        "https://example.com/authorize".to_string(),
        "https://example.com/token".to_string(),
    )
    .unwrap();

    let (url, csrf) = oauth.authorization_url(&["user.metrics", "user.info"]);
    let pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    let get = |k: &str| {
        pairs
            .iter()
            .find(|(key, _)| key == k)
            .map(|(_, v)| v.clone())
    };

    assert_eq!(get("response_type").as_deref(), Some("code"));
    assert_eq!(get("client_id").as_deref(), Some("test-client-id"));
    assert_eq!(
        get("scope").as_deref(),
        Some("user.metrics,user.info"),
        "scopes joined with comma per Withings convention"
    );
    assert_eq!(
        get("redirect_uri").as_deref(),
        Some("http://localhost:7878/callback")
    );
    assert_eq!(get("state").as_deref(), Some(csrf.secret().as_str()));
}

#[tokio::test]
async fn exchange_code_returns_token_on_success() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v2/oauth2"))
        .and(header("content-type", "application/x-www-form-urlencoded"))
        .and(body_string_contains("action=requesttoken"))
        .and(body_string_contains("grant_type=authorization_code"))
        .and(body_string_contains("code=auth-code-xyz"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 0,
            "body": {
                "userid": "12345",
                "access_token": "access-abc",
                "refresh_token": "refresh-xyz",
                "expires_in": 10800,
                "scope": "user.metrics",
                "token_type": "Bearer"
            }
        })))
        .mount(&server)
        .await;

    let oauth = oauth_pointing_to(&server);
    let token = oauth.exchange_code("auth-code-xyz").await.unwrap();

    assert_eq!(token.userid, "12345");
    assert_eq!(token.access_token, "access-abc");
    assert_eq!(token.refresh_token, "refresh-xyz");
    assert_eq!(token.expires_in, 10800);
    assert_eq!(token.scope, "user.metrics");
}

#[tokio::test]
async fn exchange_code_maps_non_zero_status_to_api_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v2/oauth2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 503,
            "error": "invalid_grant"
        })))
        .mount(&server)
        .await;

    let oauth = oauth_pointing_to(&server);
    let result = oauth.exchange_code("bad-code").await;

    match result {
        Err(WithingsError::Api { status, message }) => {
            assert_eq!(status, 503);
            assert_eq!(message, "invalid_grant");
        }
        other => panic!("expected WithingsError::Api, got {other:?}"),
    }
}

#[tokio::test]
async fn refresh_uses_refresh_token_grant() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v2/oauth2"))
        .and(body_string_contains("grant_type=refresh_token"))
        .and(body_string_contains("refresh_token=old-refresh"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 0,
            "body": {
                "userid": "12345",
                "access_token": "new-access",
                "refresh_token": "new-refresh",
                "expires_in": 10800,
                "scope": "user.metrics",
                "token_type": "Bearer"
            }
        })))
        .mount(&server)
        .await;

    let oauth = oauth_pointing_to(&server);
    let token = oauth.refresh("old-refresh").await.unwrap();

    assert_eq!(token.access_token, "new-access");
    assert_eq!(token.refresh_token, "new-refresh");
}

#[tokio::test]
async fn missing_body_on_success_is_unexpected_response() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v2/oauth2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 0
        })))
        .mount(&server)
        .await;

    let oauth = oauth_pointing_to(&server);
    let result = oauth.exchange_code("any").await;

    assert!(matches!(result, Err(WithingsError::UnexpectedResponse(_))));
}
