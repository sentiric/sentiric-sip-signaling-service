// ========== FILE: src/main.rs (Düzeltilmiş) ==========
use std::error::Error;
use std::sync::Arc;
use tokio::net::UdpSocket;

use tracing::info;

mod config;
mod grpc;
mod rabbitmq;
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
        subscriber_builder
            .with_target(true)
            .with_line_number(true)
            .init();
    } else {
        subscriber_builder
            .json()
            .with_current_span(true)
            .with_span_list(true)
            .init();
    }

    info!(config = ?config, "Konfigürasyon yüklendi.");

    let active_calls: ActiveCalls = Arc::new(Default::default());
    let rabbit_channel = rabbitmq::connection::connect_with_retry(&config.rabbitmq_url).await;
    rabbitmq::connection::declare_exchange(&rabbit_channel).await?;
    info!(exchange_name = rabbitmq::connection::RABBITMQ_EXCHANGE_NAME, "RabbitMQ exchange'i deklare edildi.");
    
    let sock = Arc::new(UdpSocket::bind(config.sip_listen_addr).await?);
    info!(address = %config.sip_listen_addr, "SIP Signaling başlatıldı.");

    tokio::spawn(rabbitmq::terminate::listen_for_termination_requests(
        Arc::clone(&sock),
        Arc::clone(&rabbit_channel),
        Arc::clone(&active_calls),
    ));

    tokio::spawn(cleanup_old_transactions(Arc::clone(&active_calls)));

    let mut buf = [0; 65535];
    loop {
        let (len, addr) = sock.recv_from(&mut buf).await?;
        
        // HATA DÜZELTMESİ: `buf[..len].to_vec()` çağrısı şimdi `handle_sip_request`'in 
        // beklediği `Vec<u8>` tipini doğru bir şekilde üretiyor.
        tokio::spawn(sip::handler::handle_sip_request(
            buf[..len].to_vec(), // Bu satır artık doğru.
            Arc::clone(&sock),
            addr,
            Arc::clone(&config),
            Arc::clone(&rabbit_channel),
            Arc::clone(&active_calls),
        ));
    }
}