# omnia-http-cache

An HTTP response cache for Omnia guests. `HttpCache` decorates any
`omnia_sdk::HttpRequest` so that requests carrying `Cache-Control` and
`If-None-Match` are answered from, and written back through, an
`omnia_sdk::StateStore`. Caching is opt-in per call site: wrap the provider
where cached fetches are wanted and use the bare provider everywhere else.

## Header contract

- A request without `Cache-Control` passes through untouched.
- `Cache-Control` directives: `max-age=<secs>` (serve a stored response, or
  store a successful one for that long), `no-cache` (bypass the stored copy
  and contact the origin), `no-store` (contact the origin and store nothing).
  `no-store` cannot be combined with the other two.
- `max-age` is the only lifetime the cache knows, so it gates the store in
  both directions. `max-age=0` neither serves nor stores: per RFC 9111
  §5.2.1.1 a request `max-age` is the oldest response the client will accept,
  which makes zero a revalidation. Likewise `no-cache` alone only bypasses
  (RFC 9111 §5.2.1.4 says nothing about storing); `no-cache, max-age=<secs>`
  is the forced refresh that replaces the stored copy.
- `max-age` and `no-cache` require `If-None-Match` carrying a single strong
  etag; weak (`W/`) and comma-separated values are refused before any request
  leaves.
- Only 2xx responses are stored; a 304 or 5xx is returned but never cached.
- `If-None-Match` is not forwarded to the origin: the cache owns conditional
  semantics, so the origin always answers with a full body.
- The request's etag is written onto the response `ETag` on every cached-path
  response (hit or miss), replacing whatever the origin sent. `no-store`
  carries no request etag, so the origin's `ETag` passes through unchanged.
- The storage key is the raw `If-None-Match` value, quotes included
  (`"\"v1\""` for `If-None-Match: "v1"`); the injected `StateStore` decides
  where entries live and the TTL is `max-age`. Header values are stored as
  raw bytes, so `obs-text` (RFC 9110 §5.5) survives a round trip.
- The store never fails a request. A read error or an entry that no longer
  deserializes is treated as a miss; a write error hands back the origin
  response uncached. Both are logged at `warn`. Malformed `Cache-Control` is
  the caller's error and still fails before any request leaves.

## Usage

On `wasm32` the SDK's capability traits carry default WASI-backed method
bodies, so a unit provider needs one empty impl per capability. The same
provider can serve as both transport and store:

```rust,ignore
use omnia_http_cache::HttpCache;
use omnia_sdk::{HttpRequest, StateStore};

struct Provider;

impl HttpRequest for Provider {}
impl StateStore for Provider {}

let cache = HttpCache::new(Provider, Provider);
let request = http::Request::get("https://origin.example/resource")
    .header(http::header::CACHE_CONTROL, "max-age=300")
    .header(http::header::IF_NONE_MATCH, "\"v1\"")
    .body(http_body_util::Empty::<bytes::Bytes>::new())?;
let response = cache.fetch(request).await?;
```

Natively, wrap an `omnia_test::guest::Provider` (or any other `HttpRequest`
and `StateStore` implementations) the same way; `&P` implements both traits,
so `HttpCache::new(&provider, &provider)` shares one provider.
