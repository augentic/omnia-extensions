//! `Cache-Control` and `If-None-Match` request header parsing.

use anyhow::{Context, Result, bail};
use http::HeaderMap;
use http::header::{CACHE_CONTROL, IF_NONE_MATCH};

/// Caching directives parsed from a request's `Cache-Control` and
/// `If-None-Match` headers.
#[derive(Clone, Debug, Default)]
pub struct Control {
    // If true, bypass any stored copy and make the HTTP request. Per RFC 9111
    // §5.2.1.4 the request directive only forbids serving a stored response
    // without validation; whether the new response is written back still
    // depends on `max_age` supplying a lifetime.
    no_cache: bool,

    // If true, make the HTTP request and do not cache the response.
    no_store: bool,

    // Length of time to cache the response in seconds. Zero (also the value
    // when the directive is absent) disables the store in both directions:
    // RFC 9111 §5.2.1.1 defines a request `max-age` as the oldest response
    // the client will accept, so `max-age=0` cannot be satisfied from the
    // store and there is no lifetime to write a fresh copy under.
    max_age: u64,

    // ETag to use as the cache key, derived from the `If-None-Match` header.
    etag: String,
}

impl Control {
    /// Parse the caching directives when a `Cache-Control` header is present.
    ///
    /// # Errors
    ///
    /// Returns an error if the directives are malformed or conflict.
    pub fn maybe_from(headers: &HeaderMap) -> Result<Option<Self>> {
        if headers.get(CACHE_CONTROL).is_none() {
            tracing::debug!("no Cache-Control header present");
            return Ok(None);
        }
        Self::try_from(headers).context("issue parsing Cache-Control headers").map(Some)
    }

    /// Whether a stored response may be served without contacting the origin.
    ///
    /// Requires a positive `max-age`: `max-age=0` asks for a response no
    /// older than zero seconds, which only the origin can provide.
    #[must_use]
    pub const fn reads(&self) -> bool {
        !self.no_cache && !self.no_store && self.max_age > 0 && !self.etag.is_empty()
    }

    /// Whether a successful origin response should be stored.
    ///
    /// `no-cache` on its own does not write: it bypasses the stored copy but
    /// supplies no lifetime, so only `no-cache, max-age=<secs>` refreshes it.
    #[must_use]
    pub const fn writes(&self) -> bool {
        !self.no_store && self.max_age > 0 && !self.etag.is_empty()
    }

    /// The raw `If-None-Match` value, doubling as the storage key.
    #[must_use]
    pub fn etag(&self) -> &str {
        &self.etag
    }

    /// The `max-age` directive in seconds (zero when absent).
    #[must_use]
    pub const fn max_age(&self) -> u64 {
        self.max_age
    }
}

impl TryFrom<&HeaderMap> for Control {
    type Error = anyhow::Error;

    fn try_from(headers: &HeaderMap) -> Result<Self> {
        let mut control = Self::default();

        // `Control::maybe_from` only parses when the header is present.
        let cache_control = headers.get(CACHE_CONTROL).context("missing Cache-Control header")?;

        if cache_control.is_empty() {
            bail!("Cache-Control header is empty");
        }

        for directive in cache_control.to_str()?.split(',') {
            let directive = directive.trim().to_ascii_lowercase();
            if directive.is_empty() {
                continue;
            }

            if directive == "no-store" {
                if control.no_cache || control.max_age > 0 {
                    bail!("`no-store` cannot be combined with other cache directives");
                }
                control.no_store = true;
                continue;
            }

            if directive == "no-cache" {
                if control.no_store {
                    bail!("`no-cache` cannot be combined with `no-store`");
                }
                control.no_cache = true;
                continue;
            }

            if let Some(value) = directive.strip_prefix("max-age=") {
                if control.no_store {
                    bail!("`max-age` cannot be combined with `no-store`");
                }
                let Ok(max_age) = value.trim().parse() else {
                    bail!("`max-age` directive is malformed");
                };
                control.max_age = max_age;
            }

            // ... other directives ignored
        }

        // `no-cache` refreshes the stored copy whenever `max-age` accompanies
        // it, and every cached-path response is stamped with the etag, so it
        // needs the key too; only `no-store` can do without one.
        if !control.no_store {
            let Some(etag) = headers.get(IF_NONE_MATCH) else {
                bail!(
                    "`If-None-Match` header required when using `Cache-Control: max-age` or `no-cache`"
                );
            };
            if etag.is_empty() {
                bail!("`If-None-Match` header is empty");
            }

            let etag_str = etag.to_str()?;
            if etag_str.contains(',') {
                bail!("multiple `etag` values in `If-None-Match` header are not supported");
            }
            if etag_str.starts_with("W/") {
                bail!("weak `etag` values in `If-None-Match` header are not supported");
            }
            control.etag = etag_str.to_string();
        }

        Ok(control)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_cache_control() {
        let mut headers = HeaderMap::new();
        headers.append(IF_NONE_MATCH, "\"etag\"".parse().unwrap());

        assert!(Control::maybe_from(&headers).expect("should parse").is_none());
    }

    #[test]
    fn max_age_with_etag() {
        let mut headers = HeaderMap::new();
        headers.append(CACHE_CONTROL, "max-age=120".parse().unwrap());
        headers.append(IF_NONE_MATCH, "\"strong-etag\"".parse().unwrap());

        let control = Control::try_from(&headers).expect("should parse");

        assert!(!control.no_store);
        assert_eq!(control.max_age, 120);
        assert_eq!(control.etag, "\"strong-etag\"");
        assert!(control.reads());
        assert!(control.writes());
    }

    #[test]
    fn zero_max_age_revalidates() {
        let mut headers = HeaderMap::new();
        headers.append(CACHE_CONTROL, "max-age=0".parse().unwrap());
        headers.append(IF_NONE_MATCH, "\"etag\"".parse().unwrap());

        let control = Control::try_from(&headers).expect("should parse");

        // RFC 9111 §5.2.1.1: no stored response is young enough, and there is
        // no lifetime to store a fresh one under.
        assert!(!control.reads());
        assert!(!control.writes());
    }

    #[test]
    fn no_cache_bypasses_without_writing() {
        let mut headers = HeaderMap::new();
        headers.append(CACHE_CONTROL, "no-cache".parse().unwrap());
        headers.append(IF_NONE_MATCH, "\"etag\"".parse().unwrap());

        let control = Control::try_from(&headers).expect("should parse");

        // RFC 9111 §5.2.1.4 only forbids serving the stored copy unvalidated;
        // with no `max-age` there is nothing to write the response under.
        assert!(control.no_cache);
        assert!(!control.reads());
        assert!(!control.writes());
    }

    #[test]
    fn no_cache_with_max_age_refreshes() {
        let mut headers = HeaderMap::new();
        headers.append(CACHE_CONTROL, "no-cache, max-age=60".parse().unwrap());
        headers.append(IF_NONE_MATCH, "\"etag\"".parse().unwrap());

        let control = Control::try_from(&headers).expect("should parse");

        assert!(!control.reads());
        assert!(control.writes());
        assert_eq!(control.max_age(), 60);
    }

    #[test]
    fn store_enabled() {
        let mut headers = HeaderMap::new();
        headers.append(CACHE_CONTROL, "no-cache".parse().unwrap());

        let Err(_) = Control::try_from(&headers) else {
            panic!("expected missing etag error");
        };
    }

    #[test]
    fn conflicting_directives() {
        let mut headers = HeaderMap::new();
        headers.append(CACHE_CONTROL, "no-store, no-cache, max-age=10".parse().unwrap());
        headers.append(IF_NONE_MATCH, "\"etag\"".parse().unwrap());

        let Err(_) = Control::try_from(&headers) else {
            panic!("expected conflicting directives error");
        };
    }

    #[test]
    fn weak_etag() {
        let mut headers = HeaderMap::new();
        headers.append(CACHE_CONTROL, "no-cache".parse().unwrap());
        headers.append(IF_NONE_MATCH, "W/\"weak-etag\"".parse().unwrap());

        let Err(_) = Control::try_from(&headers) else {
            panic!("expected weak etag rejection");
        };
    }

    #[test]
    fn multiple_etags() {
        let mut headers = HeaderMap::new();
        headers.append(CACHE_CONTROL, "no-cache".parse().unwrap());
        headers.append(IF_NONE_MATCH, "\"etag1\", \"etag2\"".parse().unwrap());

        let Err(_) = Control::try_from(&headers) else {
            panic!("expected multiple etag values rejection");
        };
    }
}
