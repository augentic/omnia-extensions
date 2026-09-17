//! # HTTP Cache Wasm Guest
//!
//! Serves `GET /cached` by fetching `{UPSTREAM_URL}/resource` through
//! `HttpCache`, so repeated requests within `max-age` are answered from
//! `wasi:keyvalue` instead of the origin.

#![cfg(target_arch = "wasm32")]

use anyhow::Context;
use axum::Router;
use axum::body::Body;
use axum::response::IntoResponse;
use axum::routing::get;
use bytes::Bytes;
use http::Method;
use http::header::{CACHE_CONTROL, IF_NONE_MATCH};
use http_body_util::Empty;
use omnia_http_cache::HttpCache;
use omnia_sdk::{Config, HttpRequest, HttpResult, StateStore};
use tracing::Level;
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

/// On wasm32 every capability trait carries a WASI-backed default, so the
/// empty impls are the whole wiring: `wasi:config`, `wasi:http` and
/// `wasi:keyvalue` respectively.
struct Provider;

impl Config for Provider {}
impl HttpRequest for Provider {}
impl StateStore for Provider {}

struct HttpGuest;
wasip3::http::service::export!(HttpGuest);

impl Guest for HttpGuest {
    #[omnia_wasi_otel::instrument(name = "http_guest_handle", level = Level::DEBUG)]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let router = Router::new().route("/cached", get(cached));
        omnia_wasi_http::serve(router, request).await
    }
}

/// The first call reaches the origin; later calls within `max-age` are served
/// from the store. Either way the response carries `ETag: "v1"`.
#[omnia_wasi_otel::instrument]
async fn cached() -> HttpResult<impl IntoResponse> {
    let upstream = Config::get(&Provider, "UPSTREAM_URL").await?;
    tracing::info!("fetching {upstream}/resource through the cache");

    let request = http::Request::builder()
        .method(Method::GET)
        .uri(format!("{upstream}/resource"))
        .header(CACHE_CONTROL, "max-age=300")
        .header(IF_NONE_MATCH, "\"v1\"")
        .body(Empty::<Bytes>::new())
        .context("building request")?;

    let response = HttpCache::new(Provider, Provider).fetch(request).await?;
    let (parts, body) = response.into_parts();

    Ok(http::Response::from_parts(parts, Body::from(body)))
}
