//! The stored form of a cached response.

use anyhow::{Context, Result};
use bytes::Bytes;
use http::Response;
use serde::{Deserialize, Serialize};

pub fn serialize(response: &Response<Bytes>) -> Result<Vec<u8>> {
    serde_json::to_vec(&Serialized::from(response)).context("serializing response")
}

pub fn deserialize(data: &[u8]) -> Result<Response<Bytes>> {
    let ser: Serialized = serde_json::from_slice(data).context("deserializing cached response")?;
    Response::<Bytes>::try_from(ser)
}

#[derive(Deserialize, Serialize)]
struct Serialized {
    status: u16,
    // Header values travel as raw bytes: RFC 9110 §5.5 permits `obs-text`
    // (0x80-0xFF) in a field value and asks recipients to treat it as opaque,
    // while RFC 9111 §3.1 requires a cache to store every received header
    // field. Round-tripping through `str` would drop those values.
    headers: Vec<(String, Vec<u8>)>,
    body: Vec<u8>,
}

impl From<&Response<Bytes>> for Serialized {
    fn from(response: &Response<Bytes>) -> Self {
        Self {
            status: response.status().as_u16(),
            headers: response
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.as_bytes().to_vec()))
                .collect(),
            body: response.body().to_vec(),
        }
    }
}

impl TryFrom<Serialized> for Response<Bytes> {
    type Error = anyhow::Error;

    fn try_from(s: Serialized) -> Result<Self> {
        let mut response = Response::builder().status(s.status);
        for (k, v) in s.headers {
            response = response.header(k, v);
        }
        response.body(Bytes::from(s.body)).context("building response from cached data")
    }
}

#[cfg(test)]
mod tests {
    use http::HeaderValue;
    use http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};

    use super::*;

    #[test]
    fn obs_text_header_round_trips() {
        // Latin-1 `é` (0xE9): valid `obs-text` per RFC 9110 §5.5, but not
        // `str`, so a lossy `to_str()` would have blanked it.
        let filename: &[u8] = b"attachment; filename=\"r\xE9sum\xE9.pdf\"";
        let mut response = Response::new(Bytes::from_static(b"body"));
        response.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static("text/plain"));
        response
            .headers_mut()
            .insert(CONTENT_DISPOSITION, HeaderValue::from_bytes(filename).expect("obs-text"));

        let stored = serialize(&response).expect("serializes");
        let restored = deserialize(&stored).expect("deserializes");

        assert_eq!(restored.status(), response.status());
        assert_eq!(restored.body(), response.body());
        assert_eq!(
            restored.headers().get(CONTENT_DISPOSITION).map(HeaderValue::as_bytes),
            Some(filename)
        );
        assert_eq!(
            restored.headers().get(CONTENT_TYPE).map(HeaderValue::as_bytes),
            Some(b"text/plain".as_slice())
        );
    }
}
