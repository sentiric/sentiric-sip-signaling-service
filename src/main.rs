// File: src/main.rs
use std::env;
use std::process;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::select;
use tokio::signal;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

mod config;
mod error;
mod grpc;
mod rabbitmq;
mod redis;
mod sip;
mod state;

use config::AppConfig;
use error::ServiceError;
use state::{cleanup_old_transactions, ActiveCalls};

#[tokio::main]
async fn main() -> Result<(), ServiceError> {
    let config = match AppConfig::load_from_env() {
        Ok(cfg) => Arc::new(cfg),
        Err(e) => {
            eprintln!("### BAŞLANGIÇ HATASI: Yapılandırma yüklenemedi: {}", e);
            process::exit(1);
        }
    };
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber_builder = tracing_subscriber::fmt().with_env_filter(env_filter);
    if config.env == "development" {
        subscriber_builder.with_target(true).with_line_number(true).init();
    } else {
        subscriber_builder.json().with_current_span(true).with_span_list(true).init();
    }
    info!(
        service_name = "sentiric-sip-signaling-service",
        version = %env::var("SERVICE_VERSION").unwrap_or_else(|_| "0.1.0".to_string()),
        commit = %env::var("GIT_COMMIT").unwrap_or_else(|_| "unknown".to_string()),
        build_date = %env::var("BUILD_DATE").unwrap_or_else(|_| "unknown".to_string()),
        profile = %config.env,
        "🚀 Servis başlatılıyor..."
    );

    let active_calls: ActiveCalls = Arc::new(Default::default());
    let redis_client = Arc::new(redis::connect_with_retry(&config.redis_url).await);
    let rabbit_channel = Arc::new(rabbitmq::connection::connect_with_retry(&config.rabbitmq_url).await);
    rabbitmq::connection::declare_exchange(&rabbit_channel).await?;
    let sock = UdpSocket::bind(config.sip_listen_addr).await.map_err(|e| ServiceError::SocketBind { addr: config.sip_listen_addr, source: e })?;
    let sock = Arc::new(sock);
    info!(address = %config.sip_listen_addr, "✅ SIP dinleyici başlatıldı.");

    let termination_task = tokio::spawn(rabbitmq::terminate::listen_for_termination_requests(
        Arc::clone(&sock), Arc::clone(&rabbit_channel), Arc::clone(&active_calls)
    ));
    let cleanup_task = tokio::spawn(cleanup_old_transactions(Arc::clone(&active_calls)));
    
    let main_loop = async {
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
        #[allow(unreachable_code)]
        Ok::<(), std::io::Error>(())
    };

    select! {
        res = main_loop => {
            if let Err(e) = res {
                error!(error = %ServiceError::from(e), "Kritik ağ hatası, servis durduruluyor.");
                process::exit(1);
            }
        },
        _ = signal::ctrl_c() => {
            warn!("Kapatma sinyali (Ctrl+C) alındı. Servis gracefully kapatılıyor...");
        }
    }
    termination_task.abort();
    cleanup_task.abort();
    info!("✅ Servis başarıyla kapatıldı.");
    Ok(())
}