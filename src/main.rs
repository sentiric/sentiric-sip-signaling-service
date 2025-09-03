// File: src/main.rs
use std::env;
use std::error::Error;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod config;
mod grpc;
mod rabbitmq;
mod redis;
mod sip;
mod state;

use config::AppConfig;
use state::{ActiveCalls, cleanup_old_transactions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Arc::new(AppConfig::load_from_env()?);

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let subscriber_builder = tracing_subscriber::fmt().with_env_filter(env_filter);
    if config.env == "development" {
        subscriber_builder.init();
    } else {
        subscriber_builder.json().init();
    }

    // YENİ: Build-time değişkenlerini environment'tan oku
    let service_version = env::var("SERVICE_VERSION").unwrap_or_else(|_| "0.0.0".to_string());
    let git_commit = env::var("GIT_COMMIT").unwrap_or_else(|_| "unknown".to_string());
    let build_date = env::var("BUILD_DATE").unwrap_or_else(|_| "unknown".to_string());

    // YENİ: Başlangıçta versiyon bilgisini logla
    info!(
        service_name = "sentiric-sip-signaling-service",
        version = %service_version,
        commit = %git_commit,
        build_date = %build_date,
        profile = %config.env,
        "🚀 Servis başlatılıyor..."
    );

    let active_calls: ActiveCalls = Arc::new(Default::default());
    let redis_client = Arc::new(redis::connect_with_retry(&config.redis_url).await);
    
    let rabbit_channel = rabbitmq::connection::connect_with_retry(&config.rabbitmq_url).await;
    rabbitmq::connection::declare_exchange(&rabbit_channel).await?;
    info!(exchange_name = rabbitmq::connection::RABBITMQ_EXCHANGE_NAME, "RabbitMQ exchange'i deklare edildi.");
    
    let sock = Arc::new(UdpSocket::bind(config.sip_listen_addr).await?);
    info!(address = %config.sip_listen_addr, "SIP Signaling dinlemeye başladı.");

    tokio::spawn(rabbitmq::terminate::listen_for_termination_requests(
        Arc::clone(&sock), Arc::clone(&rabbit_channel), Arc::clone(&active_calls),
    ));

    tokio::spawn(cleanup_old_transactions(Arc::clone(&active_calls)));

    let mut buf = [0; 65535];
    loop {
        let (len, addr) = sock.recv_from(&mut buf).await?;
        
        tokio::spawn(sip::handler::handle_sip_request(
            buf[..len].to_vec(),
            Arc::clone(&sock),
            addr,
            Arc::clone(&config),
            Arc::clone(&rabbit_channel),
            Arc::clone(&active_calls),
            Arc::clone(&redis_client),
        ));
    }
}