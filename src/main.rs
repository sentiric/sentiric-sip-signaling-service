use anyhow::Result;
use crate::app::App;

// Proje modüllerini burada bildiriyoruz
mod app;
mod app_state;
mod config;
mod error;
mod grpc;
mod rabbitmq;
mod redis;
mod sip;
mod state;

#[tokio::main]
async fn main() -> Result<()> {
    App::bootstrap().await?.run().await
}