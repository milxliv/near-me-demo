//! near-me-quote: `POST /api/quote` with `{"name": "..."}` returns
//! `{"quote": "...", "citation": "..."}` for a real quote that uses the name,
//! or `{"message": "No famous quote found for <name>."}`.
//!
//! The service stores nothing and logs no requests. It accepts calls only from
//! one browser origin, only as JSON, only with a `name` field, and only a few
//! per client per minute. It looks the name up on Wikiquote, then in a curated
//! list. It never puts a name into a quote that does not already contain it.

pub mod error;
pub mod limit;
pub mod quotes;
pub mod wikiquote;

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, header};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;
use serde_json::{Map, Value};
use tower_http::cors::CorsLayer;
use tower_http::set_header::SetResponseHeaderLayer;

use error::ApiError;
use limit::RateLimiter;
use wikiquote::Wikiquote;

pub const MAX_BODY_BYTES: usize = 512;
pub const PER_CLIENT_PER_MINUTE: u32 = 10;
pub const TOTAL_PER_MINUTE: u32 = 300;

/// Cloudflare sets this to the visitor's IP. The service listens on
/// localhost behind the tunnel, so a visitor cannot supply it.
const CLIENT_IP: HeaderName = HeaderName::from_static("cf-connecting-ip");

struct AppState {
    origin: HeaderValue,
    limiter: RateLimiter,
    live: Option<Wikiquote>,
}

#[derive(Debug, Serialize)]
pub struct QuoteResponse {
    pub quote: String,
    pub citation: String,
}

/// The body of a successful `POST /api/quote`. The message holds the cleaned
/// name, which `clean_name` limits to letters, spaces, `-`, `'`, and `.`.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum QuoteReply {
    Found(QuoteResponse),
    NotFound { message: String },
}

/// Builds the router. `allowed_origin` is the one browser origin that may
/// call the API, for example `https://milxliv.github.io`. With `live` set,
/// each name is looked up there first; without it, only the curated list
/// answers.
pub fn app(allowed_origin: &str, live: Option<Wikiquote>) -> anyhow::Result<Router> {
    let origin = HeaderValue::from_str(allowed_origin)?;
    let state = Arc::new(AppState {
        origin: origin.clone(),
        limiter: RateLimiter::new(
            PER_CLIENT_PER_MINUTE,
            TOTAL_PER_MINUTE,
            Duration::from_secs(60),
        ),
        live,
    });
    let cors = CorsLayer::new()
        .allow_origin(origin)
        .allow_methods([Method::POST])
        .allow_headers([header::CONTENT_TYPE])
        .max_age(Duration::from_secs(600));

    Ok(Router::new()
        .route("/api/quote", post(quote))
        .route("/api/health", get(|| async { "ok" }))
        .with_state(state)
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(cors)
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        )))
}

async fn quote(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<QuoteReply>, ApiError> {
    if headers.get(header::ORIGIN) != Some(&state.origin) {
        return Err(ApiError::Forbidden);
    }
    let is_json = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("application/json"));
    if !is_json {
        return Err(ApiError::UnsupportedMediaType);
    }
    // to_str() admits visible ASCII only, so byte slicing is safe. 45 bytes
    // holds the longest IPv6 text form.
    let client = headers
        .get(&CLIENT_IP)
        .and_then(|v| v.to_str().ok())
        .map_or("local", |ip| &ip[..ip.len().min(45)]);
    if !state.limiter.allow(client) {
        return Err(ApiError::TooManyRequests);
    }
    let name = name_field(&body).ok_or(ApiError::BadRequest)?;
    let name = quotes::clean_name(&name).ok_or(ApiError::InvalidName)?;
    let seed = seed();
    if let Some(live) = &state.live
        && let Some(found) = live.find(&name, seed).await
    {
        return Ok(Json(QuoteReply::Found(found)));
    }
    Ok(Json(match quotes::pick(&name, seed) {
        Some(found) => QuoteReply::Found(found),
        None => QuoteReply::NotFound {
            message: quotes::not_found(&name),
        },
    }))
}

/// Returns the name from a body that is exactly `{"name": "<string>"}`.
/// A struct with `deny_unknown_fields` would also accept `["Jack"]`.
fn name_field(body: &[u8]) -> Option<String> {
    let mut object: Map<String, Value> = serde_json::from_slice(body).ok()?;
    match (object.len(), object.remove("name")) {
        (1, Some(Value::String(name))) => Some(name),
        _ => None,
    }
}

/// Varies the pick among several quotes for one name. Not security relevant.
fn seed() -> usize {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as usize)
        .unwrap_or(0)
}
