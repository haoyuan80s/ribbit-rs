use std::net::SocketAddr;

use crate::error::{Error, Result};
use axum::{routing::get, Router};
use dotenvy::dotenv;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod config;
mod ctx;
mod error;
mod model;

pub use config::config;
pub mod _dev_utils;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().map_err(|_| Error::DotEnvNotFound)?;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .without_time()
        .with_target(false)
        .init();
    _dev_utils::init_dev().await;

    let routes_all = Router::new().route("/hello", get(hello));

    // region:    --- Start Server
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    info!("LISTENING on {addr}");
    axum::Server::bind(&addr)
        .serve(routes_all.into_make_service())
        .await
        .unwrap();
    // endregion: --- Start Server

    Ok(())
}

async fn hello() -> &'static str {
    "Hello, World!"
}
