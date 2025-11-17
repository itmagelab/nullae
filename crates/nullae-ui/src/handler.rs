use gloo::net::http::Request;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ShortenResponse {
    pub short_url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ShortenRequest {
    pub url: String,
}

impl ShortenRequest {
    pub fn validate_url(&self) -> bool {
        url::Url::parse(&self.url).is_ok()
    }
}

pub async fn api_post_shorten(
    url: &str,
    request: ShortenRequest,
) -> Result<ShortenResponse, anyhow::Error> {
    let body = serde_json::to_string(&request)?;

    let response = Request::post(url)
        .header("Content-Type", "application/json")
        .body(body)?
        .send()
        .await?;

    if response.ok() {
        let response_data: ShortenResponse = response.json().await?;
        Ok(response_data)
    } else {
        Err(anyhow::anyhow!("HTTP error: {}", response.status()))
    }
}
