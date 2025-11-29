use axum::{
    Router,
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use nullae_core::prelude::*;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Debug, Deserialize)]
struct ShortenRequest {
    url: String,
}

#[derive(Debug, Serialize)]
struct ShortenResponse {
    short_url: String,
}

async fn shorten(Json(payload): Json<ShortenRequest>) -> Result<Json<ShortenResponse>, StatusCode> {
    info!("Received shorten request for URL: {}", payload.url);

    let short_url = match shorten_handler(&payload.url).await {
        Ok(u) => u,
        Err(e) => {
            tracing::error!(?e);
            String::from("[ERROR] When shortening URL")
        }
    };

    let response = ShortenResponse { short_url };

    info!("Sending response: {:?}", response);
    Ok(Json(response))
}

async fn shorten_handler(url: &str) -> anyhow::Result<String> {
    let ctx = Context::new()?;
    let url = Url::create(url, &ctx).await?.short_url()?;
    Ok(url)
}

async fn redirect(
    axum::extract::Path(short_url): axum::extract::Path<String>,
) -> Result<axum::response::Redirect, StatusCode> {
    info!("Received redirect request for short URL: {}", short_url);

    let original_url = match redirect_handler(&short_url).await {
        Ok(u) => u,
        Err(e) => {
            tracing::error!(?e);
            return Ok(axum::response::Redirect::to("/"));
        }
    };
    Ok(axum::response::Redirect::to(&original_url))
}

async fn redirect_handler(hash: &str) -> anyhow::Result<String> {
    let ctx = Context::new()?;
    let original_url = if let Some(entity) = ctx.storage().get_by_hash(hash).await? {
        let EntityKind::Url { inner, .. } = entity.kind else {
            anyhow::bail!("Invalid URL entity");
        };
        inner.url().to_string()
    } else {
        todo!();
    };
    Ok(original_url)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .compact()
        .init();

    info!("Starting nullae-api server...");

    let app = Router::new()
        .route("/api/v1/short", post(shorten))
        .route("/{short_url}", get(redirect))
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Starting server on {}", addr);
    info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("TCP listener bound successfully on {}", addr);
    info!("TCP listener bound successfully");

    info!("Starting axum server on {}", addr);
    if let Err(e) = axum::serve(listener, app).await {
        error!("Server error: {}", e);
        return Err(anyhow::anyhow!("Server error: {}", e));
    }
    info!("Server stopped");
    Ok(())
}
