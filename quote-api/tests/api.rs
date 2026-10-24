use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, Response, StatusCode, header};
use http_body_util::BodyExt;
use near_me_quote::{MAX_BODY_BYTES, PER_CLIENT_PER_MINUTE, app};
use serde_json::Value;
use tower::ServiceExt;

const ORIGIN: &str = "https://milxliv.github.io";

fn router() -> Router {
    app(ORIGIN, None).expect("router builds")
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

// Live lookup, against a fake Wikiquote on localhost.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{RawQuery, State};
use axum::http::HeaderMap;
use near_me_quote::wikiquote::{CITATION_SUFFIX, Wikiquote};

/// Synthetic page in Wikiquote's format. No real page text.
const DINNER_BELL: &str = "\
'''Dinner Bell''' is a 2001 film.
== Dialogue ==
:'''Mabel''': Pass the salt, Jack, before it gets cold.
:'''Jack''': Coming, coming.
";

#[derive(Clone, Copy)]
enum Fake {
    Page(&'static str),
    Status(StatusCode),
    Slow,
    Huge,
}

/// Every request the fake saw: query string and headers.
type Seen = Arc<Mutex<Vec<(String, HeaderMap)>>>;

fn reply(title: &str, wikitext: &str) -> String {
    serde_json::json!({
        "batchcomplete": true,
        "query": { "pages": [{
            "pageid": 1, "ns": 0, "title": title, "index": 1,
            "revisions": [{ "slots": { "main": { "content": wikitext } } }]
        }] }
    })
    .to_string()
}

async fn fake_api(
    State((fake, seen)): State<(Fake, Seen)>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
) -> (StatusCode, String) {
    seen.lock()
        .expect("lock")
        .push((query.unwrap_or_default(), headers));
    match fake {
        Fake::Page(wikitext) => (StatusCode::OK, reply("Dinner Bell (2001 film)", wikitext)),
        Fake::Status(status) => (status, String::new()),
        Fake::Slow => {
            tokio::time::sleep(Duration::from_secs(3)).await;
            (
                StatusCode::OK,
                reply("Dinner Bell (2001 film)", DINNER_BELL),
            )
        }
        Fake::Huge => {
            let padding = "\n".repeat(4 * 1024 * 1024);
            (
                StatusCode::OK,
                reply(
                    "Dinner Bell (2001 film)",
                    &format!("{DINNER_BELL}{padding}"),
                ),
            )
        }
    }
}

/// Starts a fake Wikiquote and returns a router that uses it.
async fn live_router(fake: Fake, budget: Duration) -> (Router, Seen) {
    let seen: Seen = Arc::default();
    let api = Router::new()
        .route("/w/api.php", axum::routing::get(fake_api))
        .with_state((fake, seen.clone()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let url = format!("http://{}/w/api.php", listener.local_addr().expect("addr"));
    tokio::spawn(async move { axum::serve(listener, api).await });
    let live = Wikiquote::with_endpoint(&url, budget).expect("client");
    (app(ORIGIN, Some(live)).expect("router builds"), seen)
}

fn jack_from_a_visitor() -> Request<Body> {
    post()
        .header("cf-connecting-ip", "203.0.113.9")
        .header(header::COOKIE, "visitor=secret")
        .body(Body::from(r#"{"name":"jack"}"#))
        .unwrap()
}

#[tokio::test]
async fn live_lookup_returns_a_cited_line_and_sends_only_the_name() {
    let (router, seen) = live_router(Fake::Page(DINNER_BELL), Duration::from_secs(2)).await;
    let (status, response) = send(&router, jack_from_a_visitor()).await;
    assert_eq!(status, StatusCode::OK);
    let v = json(response).await;
    assert_eq!(
        v["quote"], "Pass the salt, Jack, before it gets cold.",
        "{v}"
    );
    assert_eq!(
        v["citation"],
        format!("Mabel, Dinner Bell (2001 film){CITATION_SUFFIX}"),
        "{v}"
    );

    let seen = seen.lock().expect("lock");
    let mut searches: Vec<String> = Vec::new();
    for (query, headers) in seen.iter() {
        let url = reqwest::Url::parse(&format!("http://fake/?{query}")).expect("query");
        for (key, value) in url.query_pairs() {
            assert!(
                !value.contains("203.0.113"),
                "client IP sent upstream: {query}"
            );
            if key == "gsrsearch" {
                searches.push(value.into_owned());
            }
        }
        for name in [
            "origin",
            "cookie",
            "referer",
            "cf-connecting-ip",
            "x-forwarded-for",
        ] {
            assert!(!headers.contains_key(name), "{name} sent upstream");
        }
        let agent = headers[header::USER_AGENT].to_str().expect("ascii");
        assert!(agent.starts_with("near-me-quote/"), "{agent}");
    }
    searches.sort();
    assert_eq!(searches, ["\"Jack\"", "\"Jack\" intitle:film"]);
}

#[tokio::test]
async fn an_upstream_error_falls_back_to_the_curated_list() {
    let (router, seen) = live_router(
        Fake::Status(StatusCode::INTERNAL_SERVER_ERROR),
        Duration::from_secs(2),
    )
    .await;
    let (status, response) = send(&router, jack_from_a_visitor()).await;
    assert_eq!(status, StatusCode::OK);
    let v = json(response).await;
    assert!(v["quote"].as_str().unwrap().contains("Jack"), "{v}");
    assert!(
        !v["citation"].as_str().unwrap().ends_with(CITATION_SUFFIX),
        "{v}"
    );
    assert_eq!(seen.lock().expect("lock").len(), 2);
}

#[tokio::test]
async fn a_slow_upstream_falls_back_within_the_budget() {
    let (router, _) = live_router(Fake::Slow, Duration::from_millis(300)).await;
    let started = Instant::now();
    let (status, response) = send(&router, jack_from_a_visitor()).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        started.elapsed() < Duration::from_millis(1500),
        "{:?}",
        started.elapsed()
    );
    let v = json(response).await;
    assert!(
        !v["citation"].as_str().unwrap().ends_with(CITATION_SUFFIX),
        "{v}"
    );
}

#[tokio::test]
async fn an_oversized_reply_is_dropped() {
    // The page holds a usable line, so only the size cap can explain a fallback.
    let (router, _) = live_router(Fake::Huge, Duration::from_secs(3)).await;
    let (_, response) = send(&router, jack_from_a_visitor()).await;
    let v = json(response).await;
    assert!(
        !v["citation"].as_str().unwrap().ends_with(CITATION_SUFFIX),
        "{v}"
    );
}

#[tokio::test]
async fn no_usable_line_falls_back() {
    let page = "'''Dinner Bell''' is a 2001 film.\n== Dialogue ==\n:'''Jack''': Coming.\n";
    let (router, _) = live_router(Fake::Page(page), Duration::from_secs(2)).await;
    let (_, response) = send(&router, jack_from_a_visitor()).await;
    let v = json(response).await;
    assert!(
        !v["citation"].as_str().unwrap().ends_with(CITATION_SUFFIX),
        "{v}"
    );
}

#[test]
fn plain_http_is_refused_except_on_loopback() {
    assert!(
        Wikiquote::with_endpoint("http://en.wikiquote.org/w/api.php", Duration::from_secs(1))
            .is_err()
    );
    assert!(
        Wikiquote::with_endpoint("http://127.0.0.1:9/w/api.php", Duration::from_secs(1)).is_ok()
    );
}
