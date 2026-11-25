use super::*;
use crate::auth::{LoginOutcome, Session};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::Method;
use secrecy::{ExposeSecret, SecretString};
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path},
};

fn secret(s: &str) -> SecretString {
    s.to_owned().into()
}
fn session() -> Session {
    Session::from_parts("123".into(), secret("test-token")).unwrap()
}
fn jwt(exp: u64) -> String {
    format!(
        "e30.{}.test-signature",
        URL_SAFE_NO_PAD.encode(json!({"phone":"123","exp":exp}).to_string())
    )
}
const ACCOUNT: &str = include_str!("../../../tests/fixtures/account.json");
const USAGE: &str = include_str!("../../../tests/fixtures/usage.json");
const PLANS: &str = include_str!("../../../tests/fixtures/plans.json");

#[tokio::test]
async fn login_success_and_mfa_require_separate_explicit_calls() {
    let server = MockServer::start().await;
    let client = Client::test_client(&server.uri());
    Mock::given(method("POST"))
        .and(path("/v2/mint/login"))
        .and(body_json(
            json!({"msisdn":"2025550100","password":"fake-password","subscriberType":"PHONE"}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"mfa":true})))
        .expect(1)
        .mount(&server)
        .await;
    assert!(matches!(
        client
            .login(&secret("2025550100"), &secret("fake-password"), None)
            .await
            .unwrap(),
        LoginOutcome::MfaRequired
    ));
    Mock::given(method("POST")).and(path("/v2/mint/login"))
        .and(body_json(json!({"msisdn":"2025550100","password":"fake-password","subscriberType":"PHONE","mfaToken":"123456"})))
        .respond_with(ResponseTemplate::new(200).insert_header("set-cookie", "session=fake-cookie; Path=/; HttpOnly")
            .set_body_json(json!({"id":123,"token":"test-token","subscriberType":"PHONE"})))
        .expect(1).mount(&server).await;
    let LoginOutcome::Authenticated(session) = client
        .login(
            &secret("2025550100"),
            &secret("fake-password"),
            Some(&secret("123456")),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    let saved = client.snapshot(&session).unwrap();
    assert!(saved.expose_secret().contains("fake-cookie"));
    assert!(!saved.expose_secret().contains("fake-password"));
    assert!(!saved.expose_secret().contains("2025550100"));
    let (_, restored) = Client::restore(&saved).unwrap();
    assert_eq!(restored.account_id, "123");
}

#[tokio::test]
async fn all_error_statuses_are_redacted_and_never_retried() {
    for (status, expected) in [
        (401, Error::Unauthorized),
        (403, Error::AccessDenied),
        (412, Error::ChallengeRequired),
        (
            429,
            Error::RateLimited {
                retry_after_seconds: Some(60),
            },
        ),
        (500, Error::Http(500)),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(status)
                    .insert_header("retry-after", "60")
                    .set_body_string("secret-password secret-token"),
            )
            .expect(1)
            .mount(&server)
            .await;
        let error = Client::test_client(&server.uri())
            .login(&secret("2025550100"), &secret("secret-password"), None)
            .await
            .unwrap_err();
        assert_eq!(error, expected);
        assert!(!format!("{error:?} {error}").contains("secret-"));
    }
}
#[tokio::test]
async fn account_and_usage_parse_without_forwarding_unknown_fields() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/mint/account/123"))
        .and(header("authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(ACCOUNT, "application/json"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v2/mint/account/123/usage"))
        .and(body_json(
            json!({"types":["data"],"subscriberType":"PHONE"}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_raw(USAGE, "application/json"))
        .expect(1)
        .mount(&server)
        .await;
    let client = Client::test_client(&server.uri());
    let account = client.account(&session()).await.unwrap();
    assert_eq!(account.plan.id, "7000001");
    let output = serde_json::to_string(&account).unwrap();
    assert!(!output.contains("token"));
    assert!(!output.contains("email"));
    let usage = client.usage(&session()).await.unwrap();
    assert_eq!(usage.used_mb, 2048.0);
    assert_eq!(usage.remaining_mb, 8192.0);
}
#[tokio::test]
async fn plans_preserve_source_price_without_inventing_taxes() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/mint/account/123/plans"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(PLANS, "application/json"))
        .expect(1)
        .mount(&server)
        .await;
    let plans = Client::test_client(&server.uri())
        .plans(&session())
        .await
        .unwrap();
    assert_eq!(
        plans.available_plans[0].cost.as_ref().unwrap().as_u64(),
        Some(18000)
    );
    assert_eq!(plans.next_plan.as_deref(), Some("7000001"));
    assert!(
        plans.available_plans[0]
            .contract_renewal_period_count
            .is_none()
    );
}
#[tokio::test]
async fn malformed_responses_fail_closed() {
    for body in [
        "not json secret",
        "{}",
        "{\"remainingHighSpeedData\":-1,\"usageHighSpeedData\":2}",
        "{\"remainingHighSpeedData\":1}",
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body, "application/json"))
            .expect(1)
            .mount(&server)
            .await;
        assert_eq!(
            Client::test_client(&server.uri())
                .usage(&session())
                .await
                .unwrap_err(),
            Error::InvalidResponse
        );
    }
}
#[tokio::test]
async fn expired_token_makes_no_request() {
    let server = MockServer::start().await;
    let s = Session::from_parts("123".into(), secret(&jwt(1))).unwrap();
    assert_eq!(
        Client::test_client(&server.uri())
            .account(&s)
            .await
            .unwrap_err(),
        Error::Expired
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}
#[tokio::test]
async fn explicit_refresh_rotates_tokens_and_cookies() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/mint/refresh"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("set-cookie", "refresh=rotated; Path=/; HttpOnly")
                .set_body_json(json!({"token":jwt(4102444800)})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = Client::test_client(&server.uri());
    let mut s = session();
    client.refresh(&mut s).await.unwrap();
    assert!(
        client
            .snapshot(&s)
            .unwrap()
            .expose_secret()
            .contains("rotated")
    );
    assert_ne!(s.token.expose_secret(), "test-token");
}
#[tokio::test]
async fn invalidation_does_not_trigger_login_or_refresh() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(401))
        .expect(1)
        .mount(&server)
        .await;
    assert_eq!(
        Client::test_client(&server.uri())
            .account(&session())
            .await
            .unwrap_err(),
        Error::Unauthorized
    );
}
#[tokio::test]
async fn redirects_are_not_followed() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(302).insert_header("location", "https://example.invalid/steal"),
        )
        .expect(1)
        .mount(&server)
        .await;
    assert_eq!(
        Client::test_client(&server.uri())
            .account(&session())
            .await
            .unwrap_err(),
        Error::Http(302)
    );
}
#[test]
fn session_debug_and_errors_never_reveal_credentials() {
    let s = session();
    let text = format!("{s:?}");
    assert!(!text.contains("123"));
    assert!(!text.contains("test-token"));
    assert!(Session::from_parts("../victim".into(), secret("test")).is_err());
    assert!(Session::from_parts("123".into(), secret("test\r\nInjected: value")).is_err());
}
#[tokio::test]
async fn oversize_response_is_rejected() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("x".repeat(1024 * 1024 + 1)))
        .expect(1)
        .mount(&server)
        .await;
    assert_eq!(
        Client::test_client(&server.uri())
            .service_status()
            .await
            .unwrap_err(),
        Error::ResponseTooLarge
    );
}
#[tokio::test]
async fn unknown_retry_after_is_not_an_invented_delay() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "Wed, 01 Jan 2031 00:00:00 GMT"),
        )
        .expect(1)
        .mount(&server)
        .await;
    assert_eq!(
        Client::test_client(&server.uri())
            .service_status()
            .await
            .unwrap_err(),
        Error::RateLimited {
            retry_after_seconds: None
        }
    );
}
#[tokio::test]
async fn request_metadata_is_per_request_not_a_persistent_device_id() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"outage":false,"message":"Service is up","planned":false})),
        )
        .expect(2)
        .mount(&server)
        .await;
    let client = Client::test_client(&server.uri());
    client.service_status().await.unwrap();
    client.service_status().await.unwrap();
    let r = server.received_requests().await.unwrap();
    assert_ne!(r[0].headers["kaena-sid"], r[1].headers["kaena-sid"]);
    assert_eq!(r[0].headers["kaena-sid"], r[0].headers["kaena-tid"]);
    assert!(!r[0].headers.contains_key("authorization"));
}
#[tokio::test]
async fn logout_is_explicit_and_not_retried() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/mint/logout"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;
    Client::test_client(&server.uri())
        .logout(&session())
        .await
        .unwrap();
}
#[tokio::test]
async fn malformed_login_cannot_be_reported_as_success() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":123})))
        .expect(1)
        .mount(&server)
        .await;
    assert_eq!(
        Client::test_client(&server.uri())
            .login(&secret("2025550100"), &secret("fake"), None)
            .await
            .unwrap_err(),
        Error::InvalidResponse
    );
}
#[tokio::test]
async fn unparseable_success_does_not_expose_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("SECRET"))
        .expect(1)
        .mount(&server)
        .await;
    let result: Result<Value, _> = Client::test_client(&server.uri())
        .request(Method::GET, "test", None, None)
        .await;
    assert_eq!(result.unwrap_err(), Error::InvalidResponse);
}

#[tokio::test]
async fn renewal_quote_is_get_only_and_preserves_api_values() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/mint/account/123/checkout/recharge"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("../../../tests/fixtures/quote.json"),
            "application/json",
        ))
        .expect(1)
        .mount(&server)
        .await;
    let quote = Client::test_client(&server.uri())
        .renewal_quote(&session())
        .await
        .unwrap();
    assert_eq!(quote.credit_card.total.as_u64(), Some(19234));
    assert_eq!(quote.credit_card.tax.as_u64(), Some(234));
    assert_eq!(quote.credit_card.recovery_fee.as_u64(), Some(1000));
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}
#[tokio::test]
async fn incomplete_quote_is_not_a_zero_dollar_offer() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"itemCost":18000,"creditCard":{"total":18000}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    assert_eq!(
        Client::test_client(&server.uri())
            .renewal_quote(&session())
            .await
            .unwrap_err(),
        Error::InvalidResponse
    );
}
#[tokio::test]
async fn refresh_cannot_switch_account_identity() {
    let server = MockServer::start().await;
    let token = format!(
        "e30.{}.sig",
        URL_SAFE_NO_PAD.encode(json!({"phone":"456"}).to_string())
    );
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"token":token})))
        .expect(1)
        .mount(&server)
        .await;
    let mut session = session();
    assert_eq!(
        Client::test_client(&server.uri())
            .refresh(&mut session)
            .await
            .unwrap_err(),
        Error::InvalidSession
    );
    assert_eq!(session.account_id, "123");
}

#[tokio::test]
async fn forbidden_diagnostics_do_not_blame_password_or_echo_server_data() {
    for (response, expected) in [
        (
            ResponseTemplate::new(403).set_body_raw("secret-html", "text/html"),
            Error::HtmlAccessDenied,
        ),
        (
            ResponseTemplate::new(403).insert_header("cf-mitigated", "challenge"),
            Error::BrowserChallenge,
        ),
        (
            ResponseTemplate::new(403).set_body_json(json!("BAD_ISP")),
            Error::ProviderRestricted,
        ),
        (
            ResponseTemplate::new(403)
                .set_body_json(json!({"reason":"portInPending","secret":"hidden"})),
            Error::PortInPending,
        ),
        (
            ResponseTemplate::new(403)
                .set_body_json(json!({"reason":"secret-reason","password":"hidden"})),
            Error::AccessDenied,
        ),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(response)
            .expect(1)
            .mount(&server)
            .await;
        let error = Client::test_client(&server.uri())
            .login(&secret("2025550100"), &secret("hidden"), None)
            .await
            .unwrap_err();
        assert_eq!(error, expected);
        let text = error.to_string();
        assert!(text.contains("403"));
        assert!(!text.contains("secret-"));
        assert!(!text.contains("hidden"));
    }
}
