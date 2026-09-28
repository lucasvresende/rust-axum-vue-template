mod auth;
mod db;
mod error;
mod mail;
mod models;
mod routes;
mod state;

#[cfg(test)]
mod tests;

use axum::http::HeaderValue;
use std::{env, net::SocketAddr, sync::Arc};
use tracing::info;

use crate::state::AppState;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("info,tower_http=info")
        .init();

    let db = db::initialize().await?;

    let origin = env::var("FRONTEND_ORIGIN")
        .unwrap_or_else(|_| "http://localhost:5173".into())
        .parse::<HeaderValue>()?;

    let mail_key = Arc::new(mail::key_from_env()?);
    let s = AppState {
        db: db.clone(),
        mail_key: mail_key.clone(),
        public_url: Arc::new(
            env::var("PUBLIC_URL").unwrap_or_else(|_| "http://localhost:5173".into()),
        ),
        secret: Arc::new(
            env::var("JWT_SECRET").unwrap_or_else(|_| "development-only-change-me".into()),
        ),
    };

    mail::start_worker(db, mail_key)?;
    let app = routes::router(s, origin);

    let addr: SocketAddr = env::var("APP_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8000".into())
        .parse()?;

    info!(%addr, "API listening");

    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;

    Ok(())
}
