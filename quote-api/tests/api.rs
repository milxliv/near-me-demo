use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, Response, StatusCode, header};
use http_body_util::BodyExt;
use near_me_quote::{MAX_BODY_BYTES, PER_CLIENT_PER_MINUTE, app};
use serde_json::Value;
use tower::ServiceExt;

const ORIGIN: &str = "https://milxliv.github.io";

fn router() -> Router {
    app(ORIGIN).expect("router builds")
}

fn post() -> axum::http::request::Builder {
    Request::builder()
        .method(Method::POST)
        .uri("/api/quote")
        .header(header::ORIGIN, ORIGIN)
        .header(header::CONTENT_TYPE, "application/json")
}

async fn send(router: &Router, request: Request<Body>) -> (StatusCode, Response<Body>) {
    let response = router.clone().oneshot(request).await.expect("infallible");
    (response.status(), response)
}

async fn json(response: Response<Body>) -> Value {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    serde_json::from_slice(&bytes).expect("JSON body")
}

#[tokio::test]
async fn a_name_gets_a_quote_and_citation_only() {
    let body = r#"{"name":"Jack"}"#;
    let (status, response) = send(&router(), post().body(Body::from(body)).unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    let h = response.headers();
    assert_eq!(h[header::ACCESS_CONTROL_ALLOW_ORIGIN], ORIGIN);
    assert_eq!(h[header::CACHE_CONTROL], "no-store");
    assert_eq!(h[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
    let v = json(response).await;
    let obj = v.as_object().expect("object");
    assert_eq!(obj.len(), 2, "{v}");
    assert!(
        obj["quote"].as_str().expect("quote").contains("Jack"),
        "{v}"
    );
    assert!(
        !obj["citation"].as_str().expect("citation").is_empty(),
        "{v}"
    );
}

#[tokio::test]
async fn an_unknown_name_gets_an_adapted_quote() {
    let body = r#"{"name":"  zebulon  "}"#;
    let (status, response) = send(&router(), post().body(Body::from(body)).unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    let v = json(response).await;
    assert!(v["quote"].as_str().unwrap().contains("Zebulon"), "{v}");
    assert!(
        v["citation"].as_str().unwrap().starts_with("Adapted from "),
        "{v}"
    );
}

#[tokio::test]
async fn other_origins_are_refused() {
    let body = r#"{"name":"Jack"}"#;
    for origin in [
        Some("https://evil.example"),
        Some("http://milxliv.github.io"),
        None,
    ] {
        let mut b = Request::builder()
            .method(Method::POST)
            .uri("/api/quote")
            .header(header::CONTENT_TYPE, "application/json");
        if let Some(o) = origin {
            b = b.header(header::ORIGIN, o);
        }
        let (status, response) = send(&router(), b.body(Body::from(body)).unwrap()).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{origin:?}");
        // The allowed origin may appear; the caller's own origin never does.
        let allow = response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN);
        assert!(allow.is_none_or(|v| v == ORIGIN), "{allow:?}");
    }
}

#[tokio::test]
async fn extra_fields_are_refused() {
    for body in [
        r#"{"name":"Jack","lat":26.6,"lon":-81.9}"#,
        r#"{"name":"Jack","x":1}"#,
        r#"{}"#,
        r#"{"name":5}"#,
        r#"["Jack"]"#,
        r#""Jack""#,
        r#"{"name":null}"#,
        r#"not json"#,
    ] {
        let (status, response) = send(&router(), post().body(Body::from(body)).unwrap()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert_eq!(json(response).await["error"], "bad request", "{body}");
    }
}

#[tokio::test]
async fn bad_names_are_refused_without_echo() {
    for name in ["", "<script>alert(1)</script>", "1234", &"a".repeat(41)] {
        let body = serde_json::json!({ "name": name }).to_string();
        let (status, response) =
            send(&router(), post().body(Body::from(body.clone())).unwrap()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{name}");
        let v = json(response).await;
        assert_eq!(v["error"], "name must be 1-40 letters");
    }
}

#[tokio::test]
async fn only_json_is_accepted() {
    let body = r#"{"name":"Jack"}"#;
    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/quote")
        .header(header::ORIGIN, ORIGIN)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from(body))
        .unwrap();
    let (status, _) = send(&router(), request).await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[tokio::test]
async fn large_bodies_are_refused() {
    let body = format!(r#"{{"name":"{}"}}"#, "a".repeat(MAX_BODY_BYTES));
    let (status, _) = send(&router(), post().body(Body::from(body.clone())).unwrap()).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn preflight_allows_only_the_origin_post_and_content_type() {
    let request = Request::builder()
        .method(Method::OPTIONS)
        .uri("/api/quote")
        .header(header::ORIGIN, ORIGIN)
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
        .body(Body::empty())
        .unwrap();
    let (status, response) = send(&router(), request).await;
    assert_eq!(status, StatusCode::OK);
    let h = response.headers();
    assert_eq!(h[header::ACCESS_CONTROL_ALLOW_ORIGIN], ORIGIN);
    assert_eq!(h[header::ACCESS_CONTROL_ALLOW_METHODS], "POST");
    assert_eq!(h[header::ACCESS_CONTROL_ALLOW_HEADERS], "content-type");
    assert!(h.get(header::ACCESS_CONTROL_ALLOW_CREDENTIALS).is_none());
}

#[tokio::test]
async fn get_is_not_allowed_and_health_answers() {
    let get = Request::builder()
        .uri("/api/quote")
        .body(Body::empty())
        .unwrap();
    let (status, _) = send(&router(), get).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);

    let health = Request::builder()
        .uri("/api/health")
        .body(Body::empty())
        .unwrap();
    let (status, _) = send(&router(), health).await;
    assert_eq!(status, StatusCode::OK);

    let other = Request::builder().uri("/").body(Body::empty()).unwrap();
    let (status, _) = send(&router(), other).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn each_client_is_rate_limited() {
    let router = router();
    let body = r#"{"name":"Jack"}"#;
    let from = |ip: &str| {
        post()
            .header("cf-connecting-ip", ip)
            .body(Body::from(body))
            .unwrap()
    };
    for _ in 0..PER_CLIENT_PER_MINUTE {
        let (status, _) = send(&router, from("203.0.113.7")).await;
        assert_eq!(status, StatusCode::OK);
    }
    let (status, response) = send(&router, from("203.0.113.7")).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
        ORIGIN
    );
    let (status, _) = send(&router, from("203.0.113.8")).await;
    assert_eq!(status, StatusCode::OK);
}
