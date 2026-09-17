//! The stored form of a cached response.

use anyhow::{Context, Result};
use bytes::Bytes;
use http::Response;
use serde::{Deserialize, Serialize};

pub fn serialize(response: &Response<Bytes>) -> Result<Vec<u8>> {
    let ser = Serialized::try_from(response)?;
    serde_json::to_vec(&ser).context("serializing response")
}

pub fn deserialize(data: &[u8]) -> Result<Response<Bytes>> {
    let ser: Serialized = serde_json::from_slice(data).context("deserializing cached response")?;
    Response::<Bytes>::try_from(ser)
}

#[derive(Deserialize, Serialize)]
struct Serialized {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl TryFrom<&Response<Bytes>> for Serialized {
    type Error = anyhow::Error;

    fn try_from(response: &Response<Bytes>) -> Result<Self> {
        Ok(Self {
            status: response.status().as_u16(),
            headers: response
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or_default().to_string()))
                .collect(),
            body: response.body().to_vec(),
        })
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
