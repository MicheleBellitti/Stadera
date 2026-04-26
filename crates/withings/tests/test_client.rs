use chrono::{TimeZone, Utc};
use stadera_withings::types::measure_type;
use stadera_withings::{WithingsClient, WithingsError};
use wiremock::matchers::{bearer_token, body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client_pointing_to(server: &MockServer) -> WithingsClient {
    WithingsClient::with_base_url(server.uri()).unwrap()
}

fn make_window() -> (chrono::DateTime<Utc>, chrono::DateTime<Utc>) {
    let from = Utc.with_ymd_and_hms(2026, 4, 18, 0, 0, 0).unwrap();
    let to = Utc.with_ymd_and_hms(2026, 4, 25, 0, 0, 0).unwrap();
    (from, to)
}

#[tokio::test]
async fn get_measurements_parses_envelope_and_groups() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/measure"))
        .and(bearer_token("access-token-abc"))
        .and(body_string_contains("action=getmeas"))
        .and(body_string_contains("meastypes=1%2C5%2C6")) // url-encoded "1,5,6"
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 0,
            "body": {
                "updatetime": 1745539200,
                "timezone": "Europe/Rome",
                "measuregrps": [
                    {
                        "grpid": 100,
                        "attrib": 0,
                        "date": 1745452800,
                        "created": 1745452801,
                        "category": 1,
                        "deviceid": "abc",
                        "measures": [
                            { "value": 80000, "type": 1, "unit": -3 },  // 80.000 kg
                            { "value": 200, "type": 6, "unit": -1 },    // 20.0 %
                            { "value": 64000, "type": 5, "unit": -3 }   // 64.000 kg
                        ],
                        "comment": null
                    }
                ]
            }
        })))
        .mount(&server)
        .await;

    let client = client_pointing_to(&server);
    let (from, to) = make_window();
    let groups = client
        .get_measurements("access-token-abc", from, to)
        .await
        .unwrap();

    assert_eq!(groups.len(), 1);
    let g = &groups[0];
    assert_eq!(g.grpid, 100);
    assert_eq!(g.measures.len(), 3);

    // Find weight measure and verify decoded value.
    let weight = g
        .measures
        .iter()
        .find(|m| m.measure_type == measure_type::WEIGHT_KG)
        .unwrap();
    assert!((weight.as_f64() - 80.0).abs() < 1e-9);

    let body_fat = g
        .measures
        .iter()
        .find(|m| m.measure_type == measure_type::BODY_FAT_PERCENT)
        .unwrap();
    assert!((body_fat.as_f64() - 20.0).abs() < 1e-9);

    let lean = g
        .measures
        .iter()
        .find(|m| m.measure_type == measure_type::LEAN_MASS_KG)
        .unwrap();
    assert!((lean.as_f64() - 64.0).abs() < 1e-9);
}

#[tokio::test]
async fn get_measurements_handles_empty_result() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/measure"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 0,
            "body": {
                "updatetime": 1745539200,
                "measuregrps": []
            }
        })))
        .mount(&server)
        .await;

    let client = client_pointing_to(&server);
    let (from, to) = make_window();
    let groups = client.get_measurements("any", from, to).await.unwrap();
    assert!(groups.is_empty());
}

#[tokio::test]
async fn get_measurements_maps_api_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/measure"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 401,
            "error": "Invalid Token"
        })))
        .mount(&server)
        .await;

    let client = client_pointing_to(&server);
    let (from, to) = make_window();
    let result = client.get_measurements("expired-token", from, to).await;

    match result {
        Err(WithingsError::Api { status, message }) => {
            assert_eq!(status, 401);
            assert_eq!(message, "Invalid Token");
        }
        other => panic!("expected WithingsError::Api, got {other:?}"),
    }
}

/// Regression for: freshly-paired Withings accounts with zero readings
/// return `{"status":0,"body":{"measuregrps":[]}}` — no `updatetime`.
/// Required field made the deserializer fail and aborted the entire
/// sync. `updatetime` is now optional.
#[tokio::test]
async fn get_measurements_handles_body_without_updatetime() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/measure"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 0,
            "body": {
                "measuregrps": []
            }
        })))
        .mount(&server)
        .await;

    let client = client_pointing_to(&server);
    let (from, to) = make_window();
    let groups = client.get_measurements("any", from, to).await.unwrap();
    assert!(groups.is_empty());
}

/// Regression for: same accounts also return a fully empty body
/// `{"status":0,"body":{}}` — neither `updatetime` nor `measuregrps`.
/// Both fields are now `#[serde(default)]` so the deserializer accepts
/// the empty object and `measuregrps` defaults to an empty Vec.
#[tokio::test]
async fn get_measurements_handles_empty_body_object() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/measure"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 0,
            "body": {}
        })))
        .mount(&server)
        .await;

    let client = client_pointing_to(&server);
    let (from, to) = make_window();
    let groups = client.get_measurements("any", from, to).await.unwrap();
    assert!(groups.is_empty());
}

/// Regression for: even more degenerate case where Withings drops the
/// `body` field entirely on a successful status. Treated as "no
/// measurements" rather than a hard error.
#[tokio::test]
async fn get_measurements_handles_missing_body_field() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/measure"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": 0
        })))
        .mount(&server)
        .await;

    let client = client_pointing_to(&server);
    let (from, to) = make_window();
    let groups = client.get_measurements("any", from, to).await.unwrap();
    assert!(groups.is_empty());
}
