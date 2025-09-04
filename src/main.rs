// File: src/main.rs
use std::env;
use std::process;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::select;
use tokio::signal;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

mod app_state;
mod config;
mod error;
mod grpc;
mod rabbitmq;
mod redis;
mod sip;
mod state;

use app_state::AppState;
use config::AppConfig;
use error::ServiceError;
use sip::responses::create_response;
use sip::utils::parse_complex_headers;
use state::cleanup_old_transactions;

type SharedAppState = Arc<Mutex<Option<Arc<AppState>>>>;

#[tokio::main]
async fn main() -> Result<(), ServiceError> {
    // ... config ve logger kurulumu aynı ...
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

    let shared_state: SharedAppState = Arc::new(Mutex::new(None));

    // Arka planda AppState'i başlat
    let state_clone_for_init = shared_state.clone();
    let config_clone_for_init = config.clone();

    // DÜZELTME: Termination task'a soketi verebilmek için onu burada oluşturuyoruz.
    let sock = UdpSocket::bind(config.sip_listen_addr).await.map_err(|e| ServiceError::SocketBind {
        addr: config.sip_listen_addr,
        source: e,
    })?;
    let sock = Arc::new(sock);
    info!(address = %config.sip_listen_addr, "✅ SIP dinleyici hemen başlatıldı. Gelen isteklere yanıt verilecek.");
    
    let sock_clone_for_init = sock.clone();
    tokio::spawn(async move {
        info!("Arka planda uygulama durumu (bağlantılar) başlatılıyor...");
        match AppState::new_critical(config_clone_for_init).await {
            Ok(mut state) => {
                state.connect_rabbitmq().await;
                if state.rabbit.is_some() {
                    info!("✅ Kritik olmayan bağımlılık (RabbitMQ) başarıyla kuruldu.");
                } else {
                    warn!("Kritik olmayan bağımlılık (RabbitMQ) kurulamadı. Servis düşük işlevsellik modunda çalışacak.");
                }
                
                let final_state = Arc::new(state);
                
                // Arka plan görevlerini burada, state hazır olunca başlat
                tokio::spawn(cleanup_old_transactions(final_state.active_calls.clone()));
                tokio::spawn(rabbitmq::terminate::listen_for_termination_requests(sock_clone_for_init, final_state.clone()));
                
                *state_clone_for_init.lock().await = Some(final_state);
                info!("✅ Tüm bağımlılıklar başarıyla kuruldu. Servis tam işlevsel.");
            }
            Err(e) => {
                error!(error = %e, "Kritik bağımlılıklar başlatılamadı. Servis başlatılamıyor ve sonlandırılacak.");
                process::exit(1);
            }
        }
    });
    
    let main_loop = async {
        let mut buf = [0; 65535];
        loop {
            let (len, addr) = sock.recv_from(&mut buf).await?;
            let request_bytes = buf[..len].to_vec();
            
            let locked_state = shared_state.lock().await;
            if let Some(state) = locked_state.as_ref() {
                tokio::spawn(sip::handler::handle_sip_request(
                    request_bytes,
                    Arc::clone(&sock),
                    addr,
                    state.clone(),
                ));
            } else {
                warn!(from = %addr, "Servis henüz başlatılıyor, isteğe 503 Service Unavailable yanıtı veriliyor.");
                let request_str = String::from_utf8_lossy(&request_bytes);
                if request_str.starts_with("INVITE") {
                    if let Some(headers) = parse_complex_headers(&request_str) {
                        let response = create_response("503 Service Unavailable", &headers, None, &config, addr);
                        let _ = sock.send_to(response.as_bytes(), addr).await;
                    }
                }
            }
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
    
    info!("✅ Servis başarıyla kapatıldı.");
    Ok(())
}