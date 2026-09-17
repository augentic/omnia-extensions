//! Tests for the `HttpCache` decorator driven exactly as a guest invokes it,
//! with the outbound request recorded by `MatchedHttp` and the stored copy
//! read back from `Memory`.

use bytes::Bytes;
use http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH};
use http::{HeaderValue, Method, Request, Response, StatusCode};
use http_body_util::Empty;
use omnia_http_cache::HttpCache;
use omnia_sdk::HttpRequest as _;
use omnia_test::guest::{MatchedHttp, Provider};

const URL: &str = "https://origin.test/resource";
const ETAG_V1: &str = "\"v1\"";
const ORIGIN_ETAG: &str = "\"origin\"";
const PAYLOAD: &[u8] = b"payload";

/// A provider whose origin answers `GET /resource` with `status` and the
/// fixture body, carrying its own `ETag` so stamping is observable.
fn provider(status: StatusCode) -> Provider {
    let mut response = Response::new(Bytes::from_static(PAYLOAD));
    *response.status_mut() = status;
    response.headers_mut().insert(ETAG, HeaderValue::from_static(ORIGIN_ETAG));
    response.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static("text/plain"));
    Provider::default().http(MatchedHttp::default().on(Method::GET, URL, response))
}

fn request(headers: &[(http::HeaderName, &'static str)]) -> Request<Empty<Bytes>> {
    let mut request = Request::get(URL);
    for (name, value) in headers {
        request = request.header(name, *value);
    }
    request.body(Empty::new()).expect("valid request")
}

fn etag(response: &Response<Bytes>) -> Option<&str> {
    response.headers().get(ETAG).map(|v| v.to_str().expect("ascii etag"))
}

/// The stored envelope, as the JSON the crate writes.
fn stored(provider: &Provider) -> Option<serde_json::Value> {
    provider.storage.state(ETAG_V1).map(|bytes| serde_json::from_slice(&bytes).expect("valid JSON"))
}

#[tokio::test]
async fn max_age_miss_then_hit() {
    let provider = provider(StatusCode::OK);
    let cache = HttpCache::new(&provider, &provider);
    let headers = [(CACHE_CONTROL, "max-age=60"), (IF_NONE_MATCH, ETAG_V1)];

    let miss = cache.fetch(request(&headers)).await.expect("should succeed");
    assert_eq!(miss.status(), StatusCode::OK);
    assert_eq!(miss.body(), PAYLOAD);
    assert_eq!(etag(&miss), Some(ETAG_V1));

    // The conditional header stays with the cache: the origin must answer
    // with the full resource, never a 304.
    let requests = provider.http.requests();
    assert_eq!(requests.len(), 1);
    assert!(!requests[0].headers.contains_key(IF_NONE_MATCH));

    // The stored copy is keyed by the raw etag, quotes included, and is the
    // stamped response rather than the origin's.
    let envelope = stored(&provider).expect("response cached");
    assert_eq!(envelope["status"], 200);
    assert_eq!(envelope["body"], serde_json::json!(PAYLOAD));
    assert!(
        envelope["headers"]
            .as_array()
            .expect("headers array")
            .contains(&serde_json::json!(["etag", ETAG_V1]))
    );

    let hit = cache.fetch(request(&headers)).await.expect("should succeed");
    assert_eq!(hit.status(), StatusCode::OK);
    assert_eq!(hit.body(), PAYLOAD);
    assert_eq!(etag(&hit), Some(ETAG_V1));
    assert_eq!(
        hit.headers().get(CONTENT_TYPE).map(HeaderValue::as_bytes),
        Some(b"text/plain".as_slice())
    );
    assert_eq!(provider.http.requests().len(), 1);
}

#[tokio::test]
async fn no_cache_refreshes() {
    let provider = provider(StatusCode::OK);
    let cache = HttpCache::new(&provider, &provider);
    let headers = [(CACHE_CONTROL, "no-cache, max-age=60"), (IF_NONE_MATCH, ETAG_V1)];

    for round in 1..=2 {
        // A stale entry under the key must be bypassed and then replaced.
        provider.storage.insert_state(ETAG_V1, b"stale");

        let response = cache.fetch(request(&headers)).await.expect("should succeed");
        assert_eq!(response.body(), PAYLOAD);
        assert_eq!(etag(&response), Some(ETAG_V1));
        assert_eq!(provider.http.requests().len(), round);
        assert_eq!(stored(&provider).expect("response cached")["status"], 200);
    }
}

#[tokio::test]
async fn no_cache_alone_bypasses_without_writing() {
    let provider = provider(StatusCode::OK);
    let cache = HttpCache::new(&provider, &provider);
    let headers = [(CACHE_CONTROL, "no-cache"), (IF_NONE_MATCH, ETAG_V1)];

    // RFC 9111 §5.2.1.4: the stored copy must not be served unvalidated, but
    // with no `max-age` there is no lifetime to write the response under, so
    // the existing entry is left as it was.
    provider.storage.insert_state(ETAG_V1, b"existing");

    let response = cache.fetch(request(&headers)).await.expect("should succeed");
    assert_eq!(response.body(), PAYLOAD);
    assert_eq!(etag(&response), Some(ETAG_V1));
    assert_eq!(provider.http.requests().len(), 1);
    assert_eq!(provider.storage.state(ETAG_V1).as_deref(), Some(b"existing".as_slice()));
}

#[tokio::test]
async fn zero_max_age_revalidates() {
    let provider = provider(StatusCode::OK);
    let cache = HttpCache::new(&provider, &provider);
    let headers = [(CACHE_CONTROL, "max-age=0"), (IF_NONE_MATCH, ETAG_V1)];

    // RFC 9111 §5.2.1.1: `max-age=0` accepts no stored response, however
    // fresh, so a populated entry is bypassed rather than served.
    provider.storage.insert_state(ETAG_V1, b"stale");

    let response = cache.fetch(request(&headers)).await.expect("should succeed");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.body(), PAYLOAD);
    assert_eq!(etag(&response), Some(ETAG_V1));
    assert_eq!(provider.http.requests().len(), 1);

    // Nor is there a lifetime to store the fresh copy under.
    assert_eq!(provider.storage.state(ETAG_V1).as_deref(), Some(b"stale".as_slice()));
}

#[tokio::test]
async fn no_store_bypasses() {
    let provider = provider(StatusCode::OK);
    let cache = HttpCache::new(&provider, &provider);
    let headers = [(CACHE_CONTROL, "no-store"), (IF_NONE_MATCH, ETAG_V1)];

    let response = cache.fetch(request(&headers)).await.expect("should succeed");
    assert_eq!(response.body(), PAYLOAD);
    assert_eq!(provider.http.requests().len(), 1);
    assert!(stored(&provider).is_none());

    // The origin's `ETag` is still replaced. `Control` records no etag under
    // `no-store`, so the stamped value is empty.
    assert_eq!(etag(&response), Some(""));
}

#[tokio::test]
async fn non_success_not_stored() {
    let provider = provider(StatusCode::INTERNAL_SERVER_ERROR);
    let cache = HttpCache::new(&provider, &provider);
    let headers = [(CACHE_CONTROL, "max-age=60"), (IF_NONE_MATCH, ETAG_V1)];

    let response = cache.fetch(request(&headers)).await.expect("should succeed");
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(response.body(), PAYLOAD);
    assert_eq!(etag(&response), Some(ETAG_V1));
    assert_eq!(provider.http.requests().len(), 1);
    assert!(stored(&provider).is_none());
}

#[tokio::test]
async fn without_cache_control_passes_through() {
    let provider = provider(StatusCode::OK);
    let cache = HttpCache::new(&provider, &provider);

    let response = cache.fetch(request(&[(IF_NONE_MATCH, ETAG_V1)])).await.expect("should succeed");
    assert_eq!(etag(&response), Some(ORIGIN_ETAG));

    let requests = provider.http.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].headers.get(IF_NONE_MATCH).map(HeaderValue::as_bytes),
        Some(ETAG_V1.as_bytes())
    );
    assert!(stored(&provider).is_none());
}

#[tokio::test]
async fn malformed_directives_refused() {
    let provider = provider(StatusCode::OK);
    let cache = HttpCache::new(&provider, &provider);

    let Err(_) = cache.fetch(request(&[(CACHE_CONTROL, "max-age=60")])).await else {
        panic!("expected missing If-None-Match error");
    };
    assert!(provider.http.requests().is_empty());
    assert!(stored(&provider).is_none());
}
