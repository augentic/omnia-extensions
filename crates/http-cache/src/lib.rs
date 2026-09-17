#![doc = include_str!("../README.md")]

mod control;
mod envelope;

use std::any::Any;
use std::error::Error;

use anyhow::Result;
use bytes::Bytes;
use http::header::{ETAG, IF_NONE_MATCH};
use http::{HeaderValue, Request, Response};
use http_body::Body;
use omnia_sdk::{HttpRequest, StateStore};

pub use crate::control::Control;
use crate::envelope::{deserialize, serialize};

/// Decorates an [`HttpRequest`] so responses are served from, and written
/// back through, a [`StateStore`] when the request carries `Cache-Control`.
#[derive(Clone, Debug, Default)]
pub struct HttpCache<H, S> {
    http: H,
    store: S,
}

impl<H, S> HttpCache<H, S> {
    /// Wrap `http`, storing cacheable responses in `store`.
    #[must_use]
    pub const fn new(http: H, store: S) -> Self {
        Self { http, store }
    }
}

impl<H: HttpRequest, S: StateStore> HttpRequest for HttpCache<H, S> {
    async fn fetch<T>(&self, mut request: Request<T>) -> Result<Response<Bytes>>
    where
        T: Body + Any + Send,
        T::Data: Into<Vec<u8>>,
        T::Error: Into<Box<dyn Error + Send + Sync + 'static>>,
    {
        // No `Cache-Control`: pure pass-through, headers untouched.
        let Some(control) = Control::maybe_from(request.headers())? else {
            return self.http.fetch(request).await;
        };

        // The raw etag is the storage key: no prefix, no hashing.
        let etag = control.etag();
        if control.reads()
            && let Some(hit) = self.store.get(etag).await?
        {
            tracing::debug!("cache hit for etag `{etag}`");
            return deserialize(&hit);
        }

        // The cache owns conditional semantics: forwarding `If-None-Match`
        // would let the origin answer `304 Not Modified`, whose empty body
        // must not become the cached resource.
        request.headers_mut().remove(IF_NONE_MATCH);
        let mut response = self.http.fetch(request).await?;

        // The request etag identifies the resource the caller asked for, so it
        // replaces whatever the origin sent.
        response.headers_mut().insert(ETAG, HeaderValue::from_str(etag)?);

        // Only successful responses are cacheable: storing a 5xx body would
        // serve it as the resource for `max_age` seconds.
        if control.writes() && response.status().is_success() {
            tracing::debug!("caching response for etag `{etag}`");
            self.store.set(etag, &serialize(&response)?, Some(control.max_age())).await?;
        }

        Ok(response)
    }
}
