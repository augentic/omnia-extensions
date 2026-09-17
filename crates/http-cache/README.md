# omnia-http-cache

An HTTP response cache for Omnia guests. `HttpCache` decorates any
`omnia_sdk::HttpRequest` so that requests carrying `Cache-Control` and
`If-None-Match` are answered from, and written back through, an
`omnia_sdk::StateStore`. Caching is opt-in per call site: wrap the provider
where cached fetches are wanted and use the bare provider everywhere else.

## Header contract

- A request without `Cache-Control` passes through untouched.
- `Cache-Control` directives: `max-age=<secs>` (store a successful response
  for that long), `no-cache` (always contact the origin, then refresh the
  stored copy), `no-store` (contact the origin and store nothing). `no-store`
  cannot be combined with the other two.
- `max-age` and `no-cache` require `If-None-Match` carrying a single strong
  etag; weak (`W/`) and comma-separated values are refused before any request
  leaves.
- Only 2xx responses are stored; a 304 or 5xx is returned but never cached.
- `If-None-Match` is not forwarded to the origin: the cache owns conditional
  semantics, so the origin always answers with a full body.
- The request's etag is written onto the response `ETag` on every cached-path
  response (hit or miss), replacing whatever the origin sent.
- The storage key is the raw `If-None-Match` value, quotes included
  (`"\"v1\""` for `If-None-Match: "v1"`); the injected `StateStore` decides
  where entries live and the TTL is `max-age`.

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
