//! An HTTP response cache for Omnia guests.
//!
//! Decorates an `omnia_sdk::HttpRequest` so that responses to requests
//! carrying `Cache-Control` and `If-None-Match` are served from, and written
//! back through, an `omnia_sdk::StateStore`.
