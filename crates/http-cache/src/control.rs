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
    /// Parse the caching directives when `Cache-Control` carries at least one
    /// this cache acts on: `no-store`, `no-cache` or `max-age`.
    ///
    /// Returns `None` when the header is absent or carries only directives
    /// this cache does not recognise: RFC 9111 §5.2.3 requires a cache to
    /// ignore those, so the request passes through as if none were present.
    ///
    /// # Errors
    ///
    /// Returns an error if the directives are malformed or conflict.
    pub fn maybe_from(headers: &HeaderMap) -> Result<Option<Self>> {
        if headers.get(CACHE_CONTROL).is_none() {
            tracing::debug!("no Cache-Control header present");
            return Ok(None);
        }
        Self::parse(headers).context("issue parsing Cache-Control headers")
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

impl Control {
    fn parse(headers: &HeaderMap) -> Result<Option<Self>> {
        let mut control = Self::default();
        // `Some` whenever the directive appeared, so `max-age=0` still counts
        // as combining it with `no-store`.
        let mut max_age = None;

        // `Cache-Control` is a list field (RFC 9111 §5.2), so several field
        // lines are equivalent to one comma-joined line (RFC 9110 §5.3):
        // every line is read, and the directives form a set whose order
        // carries no meaning.
        for line in headers.get_all(CACHE_CONTROL) {
            for directive in line.to_str()?.split(',') {
                let directive = directive.trim().to_ascii_lowercase();

                if directive == "no-store" {
                    control.no_store = true;
                } else if directive == "no-cache" {
                    control.no_cache = true;
                } else if let Some(value) = directive.strip_prefix("max-age=") {
                    // RFC 9111 §4.2.1: a repeated directive is either taken
                    // from its first occurrence or treated as invalid. A
                    // caller sending two lifetimes has made a mistake worth
                    // surfacing, so it is invalid here.
                    if max_age.is_some() {
                        bail!("`max-age` directive given more than once");
                    }
                    let Ok(secs) = value.trim().parse() else {
                        bail!("`max-age` directive is malformed");
                    };
                    max_age = Some(secs);
                }

                // RFC 9111 §5.2.3: unrecognised directives (and empty list
                // elements) are ignored.
            }
        }

        // Nothing this cache acts on: RFC 9111 §5.2.3 says ignore the rest,
        // so the request is treated as if it carried no `Cache-Control`.
        if !control.no_store && !control.no_cache && max_age.is_none() {
            tracing::debug!("no recognised Cache-Control directive present");
            return Ok(None);
        }

        // Conflicts are judged on the whole set, so `max-age=0, no-store` and
        // `no-store, max-age=0` are refused alike.
        if control.no_store && (control.no_cache || max_age.is_some()) {
            bail!("`no-store` cannot be combined with `no-cache` or `max-age`");
        }
        control.max_age = max_age.unwrap_or(0);

        // `no-cache` refreshes the stored copy whenever `max-age` accompanies
        // it, and every cached-path response is stamped with the etag, so it
        // needs the key too; only `no-store` can do without one.
        if !control.no_store {
            // `If-None-Match` is a list field too (`#entity-tag`), so a second
            // line is a second etag, refused exactly like a comma.
            let mut lines = headers.get_all(IF_NONE_MATCH).iter();
            let Some(etag) = lines.next() else {
                bail!(
                    "`If-None-Match` header required when using `Cache-Control: max-age` or `no-cache`"
                );
            };
            if lines.next().is_some() {
                bail!("multiple `etag` values in `If-None-Match` header are not supported");
            }
            // The etag doubles as the `&str` store key, so `obs-text` octets
            // (RFC 9110 §5.5), though grammatically valid, are not accepted.
            let Ok(etag) = etag.to_str() else {
                bail!("`If-None-Match` contains `obs-text` octets, which cannot form a cache key");
            };
            control.etag = strong_etag(etag)?.to_string();
        }

        Ok(Some(control))
    }
}

/// Validate `value` as exactly one strong entity-tag and return it trimmed of
/// surrounding whitespace.
///
/// RFC 9110 §8.8.3: `entity-tag = [ weak ] opaque-tag`, `opaque-tag = DQUOTE
/// *etagc DQUOTE`, `etagc = %x21 / %x23-7E / obs-text`. A comma is a legal
/// `etagc`, so a list is detected by finding content after the closing quote,
/// not by looking for commas. `obs-text` is excluded by the caller, which
/// needs the tag as a `&str`, so only the ASCII range is checked here.
fn strong_etag(value: &str) -> Result<&str> {
    let value = value.trim_matches([' ', '\t']);
    if value.is_empty() {
        bail!("`If-None-Match` header is empty");
    }
    if value == "*" {
        bail!("`If-None-Match: *` is not a usable cache key");
    }
    if value.starts_with("W/") {
        bail!("weak `etag` values in `If-None-Match` header are not supported");
    }
    let Some(body) = value.strip_prefix('"') else {
        bail!("`If-None-Match` value is not a quoted entity-tag");
    };
    // `etagc` excludes DQUOTE, so the first one closes the tag.
    let Some(close) = body.find('"') else {
        bail!("`If-None-Match` entity-tag is missing its closing quote");
    };
    let (opaque, rest) = body.split_at(close);
    if !rest[1..].trim_matches([' ', '\t']).is_empty() {
        bail!("multiple `etag` values in `If-None-Match` header are not supported");
    }
    if !opaque.bytes().all(|b| b == 0x21 || (0x23..=0x7E).contains(&b)) {
        bail!("`If-None-Match` entity-tag contains characters outside `etagc`");
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    use http::HeaderValue;

    /// Parse headers expected to carry at least one recognised directive.
    fn parse(headers: &HeaderMap) -> Result<Control> {
        Control::maybe_from(headers)?.context("no recognised directive present")
    }

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

        let control = parse(&headers).expect("should parse");

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

        let control = parse(&headers).expect("should parse");

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

        let control = parse(&headers).expect("should parse");

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

        let control = parse(&headers).expect("should parse");

        assert!(!control.reads());
        assert!(control.writes());
        assert_eq!(control.max_age(), 60);
    }

    #[test]
    fn store_enabled() {
        let mut headers = HeaderMap::new();
        headers.append(CACHE_CONTROL, "no-cache".parse().unwrap());

        let Err(_) = parse(&headers) else {
            panic!("expected missing etag error");
        };
    }

    #[test]
    fn conflicting_directives() {
        let mut headers = HeaderMap::new();
        headers.append(CACHE_CONTROL, "no-store, no-cache, max-age=10".parse().unwrap());
        headers.append(IF_NONE_MATCH, "\"etag\"".parse().unwrap());

        let Err(_) = parse(&headers) else {
            panic!("expected conflicting directives error");
        };
    }

    #[test]
    fn unrecognised_directives_pass_through() {
        // RFC 9111 §5.2.3: a cache MUST ignore unrecognised directives, so a
        // header carrying only those (or nothing) is as good as absent, and
        // does not demand an etag.
        for value in ["no-transform", "public, ext=1", "", ", ,"] {
            let mut headers = HeaderMap::new();
            headers.append(CACHE_CONTROL, value.parse().unwrap());

            assert!(
                Control::maybe_from(&headers).expect("should parse").is_none(),
                "expected `{value}` to pass through"
            );
        }

        // Alongside a recognised directive they are simply skipped.
        let mut headers = HeaderMap::new();
        headers.append(CACHE_CONTROL, "no-transform, max-age=60, ext=1".parse().unwrap());
        headers.append(IF_NONE_MATCH, "\"etag\"".parse().unwrap());

        let control = parse(&headers).expect("should parse");
        assert_eq!(control.max_age(), 60);
        assert!(control.reads());
    }

    #[test]
    fn duplicate_max_age_refused() {
        // RFC 9111 §4.2.1: a repeated directive is first-occurrence or
        // invalid; here it is invalid, whichever order the values come in.
        for value in ["max-age=0, max-age=60", "max-age=60, max-age=0", "max-age=60, max-age=60"] {
            let mut headers = HeaderMap::new();
            headers.append(CACHE_CONTROL, value.parse().unwrap());
            headers.append(IF_NONE_MATCH, "\"etag\"".parse().unwrap());

            let Err(_) = parse(&headers) else {
                panic!("expected `{value}` to be refused");
            };
        }

        // Across field lines too (RFC 9110 §5.3).
        let mut headers = HeaderMap::new();
        headers.append(CACHE_CONTROL, "max-age=60".parse().unwrap());
        headers.append(CACHE_CONTROL, "max-age=0".parse().unwrap());
        headers.append(IF_NONE_MATCH, "\"etag\"".parse().unwrap());

        let Err(_) = parse(&headers) else {
            panic!("expected duplicate `max-age` across lines to be refused");
        };
    }

    #[test]
    fn obs_text_etag_refused_explicitly() {
        // Grammatically valid `etagc` (RFC 9110 §8.8.3), but the etag doubles
        // as a `&str` store key, so the contract refuses it and says why.
        let mut headers = HeaderMap::new();
        headers.append(CACHE_CONTROL, "max-age=60".parse().unwrap());
        headers.append(IF_NONE_MATCH, HeaderValue::from_bytes(b"\"v\xE9\"").unwrap());

        let Err(e) = parse(&headers) else {
            panic!("expected obs-text etag to be refused");
        };
        assert!(format!("{e:#}").contains("obs-text"), "unexpected error: {e:#}");
    }

    #[test]
    fn no_store_conflict_is_order_independent() {
        // `max-age=0` is still `max-age`: present is what matters, not > 0.
        for value in ["max-age=0, no-store", "no-store, max-age=0", "no-cache, no-store"] {
            let mut headers = HeaderMap::new();
            headers.append(CACHE_CONTROL, value.parse().unwrap());
            headers.append(IF_NONE_MATCH, "\"etag\"".parse().unwrap());

            let Err(_) = parse(&headers) else {
                panic!("expected `{value}` to be refused");
            };
        }
    }

    #[test]
    fn cache_control_lines_are_combined() {
        // RFC 9110 §5.3: two field lines read as one comma-joined list.
        let mut headers = HeaderMap::new();
        headers.append(CACHE_CONTROL, "no-cache".parse().unwrap());
        headers.append(CACHE_CONTROL, "max-age=60".parse().unwrap());
        headers.append(IF_NONE_MATCH, "\"etag\"".parse().unwrap());

        let control = parse(&headers).expect("should parse");
        assert!(control.no_cache);
        assert_eq!(control.max_age(), 60);
        assert!(control.writes());

        // A later line carrying `no-store` is not silently dropped.
        let mut headers = HeaderMap::new();
        headers.append(CACHE_CONTROL, "max-age=60".parse().unwrap());
        headers.append(CACHE_CONTROL, "no-store".parse().unwrap());
        headers.append(IF_NONE_MATCH, "\"etag\"".parse().unwrap());

        let Err(_) = parse(&headers) else {
            panic!("expected conflicting directives error across lines");
        };
    }

    #[test]
    fn quoted_comma_is_one_etag() {
        // RFC 9110 §8.8.3: `,` (0x2C) is within `etagc`, so this is a single
        // strong entity-tag, not a list.
        let mut headers = HeaderMap::new();
        headers.append(CACHE_CONTROL, "max-age=60".parse().unwrap());
        headers.append(IF_NONE_MATCH, "\"v1,v2\"".parse().unwrap());

        let control = parse(&headers).expect("should parse");
        assert_eq!(control.etag(), "\"v1,v2\"");
        assert!(control.reads());
    }

    #[test]
    fn etag_grammar_enforced() {
        // Each is not exactly one strong entity-tag: a list with no comma
        // whitespace, a list, a bare token, the match-anything form, an
        // unterminated tag, and a space inside the opaque-tag.
        for value in ["\"a\" \"b\"", "\"a\",\"b\"", "v1", "*", "\"v1", "\"v 1\""] {
            let mut headers = HeaderMap::new();
            headers.append(CACHE_CONTROL, "max-age=60".parse().unwrap());
            headers.append(IF_NONE_MATCH, value.parse().unwrap());

            let Err(_) = parse(&headers) else {
                panic!("expected `{value}` to be refused");
            };
        }
    }

    #[test]
    fn if_none_match_lines_are_multiple_etags() {
        let mut headers = HeaderMap::new();
        headers.append(CACHE_CONTROL, "max-age=60".parse().unwrap());
        headers.append(IF_NONE_MATCH, "\"etag1\"".parse().unwrap());
        headers.append(IF_NONE_MATCH, "\"etag2\"".parse().unwrap());

        let Err(_) = parse(&headers) else {
            panic!("expected multiple etag values rejection across lines");
        };
    }

    #[test]
    fn weak_etag() {
        let mut headers = HeaderMap::new();
        headers.append(CACHE_CONTROL, "no-cache".parse().unwrap());
        headers.append(IF_NONE_MATCH, "W/\"weak-etag\"".parse().unwrap());

        let Err(_) = parse(&headers) else {
            panic!("expected weak etag rejection");
        };
    }

    #[test]
    fn multiple_etags() {
        let mut headers = HeaderMap::new();
        headers.append(CACHE_CONTROL, "no-cache".parse().unwrap());
        headers.append(IF_NONE_MATCH, "\"etag1\", \"etag2\"".parse().unwrap());

        let Err(_) = parse(&headers) else {
            panic!("expected multiple etag values rejection");
        };
    }
}
