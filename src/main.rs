// sentiric-sip-signaling-service/src/main.rs
use std::env;
use std::process;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::select;
use tokio::signal;
use tokio::sync::Mutex;
use tonic::transport::{Certificate, Identity, Server as GrpcServer, ServerTlsConfig};
use tracing::{error, info, warn};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    prelude::*,
    Registry, EnvFilter,
};

use rustls::crypto::CryptoProvider;
use rustls::crypto::ring::default_provider;

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
use grpc::service::MySipSignalingService;
use sentiric_contracts::sentiric::sip::v1::sip_signaling_service_server::SipSignalingServiceServer;
use sip::responses::create_response;
use sip::utils::parse_complex_headers;
use state::cleanup_old_transactions;

type SharedAppState = Arc<Mutex<Option<Arc<AppState>>>>;

#[tokio::main]
async fn main() -> Result<(), ServiceError> {
    let provider = default_provider();
    CryptoProvider::install_default(provider)
        .expect("Crypto provider (ring) kurulamadı.");

    let config = match AppConfig::load_from_env() {
        Ok(cfg) => Arc::new(cfg),
        Err(e) => {
            eprintln!("### BAŞLANGIÇ HATASI: Yapılandırma yüklenemedi: {}", e);
            process::exit(1);
        }
    };

    let rust_log_env = env::var("RUST_LOG")
        .unwrap_or_else(|_| env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()));
    
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&rust_log_env))?;
    
    let subscriber = Registry::default().with(env_filter);
    
    if config.env == "development" {
        subscriber.with(fmt::layer().with_target(true).with_line_number(true).with_span_events(FmtSpan::NONE)).init();
    } else {
        subscriber.with(fmt::layer().json().with_current_span(true).with_span_list(true).with_span_events(FmtSpan::NONE)).init();
    }

    info!(
        service_name = "sentiric-sip-signaling-service",
        version = %config.service_version,
        commit = %env::var("GIT_COMMIT").unwrap_or_else(|_| "unknown".to_string()),
        build_date = %env::var("BUILD_DATE").unwrap_or_else(|_| "unknown".to_string()),
        profile = %config.env,
        "🚀 Servis başlatılıyor..."
    );

    let shared_state: SharedAppState = Arc::new(Mutex::new(None));
    // DÜZELTME: Derleyiciye yardımcı olmak için tipini açıkça belirtiyoruz.
    let sock: Arc<UdpSocket> = Arc::new(UdpSocket::bind(config.sip_listen_addr).await?);
    info!(address = %config.sip_listen_addr, "✅ UDP SIP dinleyici hemen başlatıldı.");
    
    let state_clone_for_init = shared_state.clone();
    let config_clone_for_init = config.clone();

    // gRPC sunucusunu ayrı bir görevde başlat
    let grpc_server_task = {
        let state_clone = shared_state.clone();
        let sock_clone = sock.clone();
        let config_clone = config.clone();
        tokio::spawn(async move {
            loop {
                // Sadece AppState hazır olduğunda gRPC sunucusunu başlat
                if let Some(state) = state_clone.lock().await.as_ref() {
                    let grpc_service = MySipSignalingService {
                        app_state: state.clone(),
                        sock: sock_clone.clone(),
                    };
                    
                    let grpc_port = env::var("SIP_SIGNALING_GRPC_PORT").unwrap_or_else(|_| "13021".to_string());
                    let addr_str = format!("[::]:{}", grpc_port);
                    let addr = addr_str.parse().unwrap();

                    let cert = tokio::fs::read(&config_clone.cert_path).await.expect("Sunucu sertifikası okunamadı");
                    let key = tokio::fs::read(&config_clone.key_path).await.expect("Sunucu anahtarı okunamadı");
                    let identity = Identity::from_pem(cert, key);
                    
                    let ca_cert = tokio::fs::read(&config_clone.ca_path).await.expect("CA sertifikası okunamadı");
                    let client_ca_cert = Certificate::from_pem(ca_cert);

                    let tls_config = ServerTlsConfig::new()
                        .identity(identity)
                        .client_ca_root(client_ca_cert);
                    
                    info!(address = %addr, "gRPC sunucusu (mTLS ile) başlatılıyor...");
                    if let Err(e) = GrpcServer::builder()
                        .tls_config(tls_config).expect("TLS yapılandırması başarısız")
                        .add_service(SipSignalingServiceServer::new(grpc_service))
                        .serve(addr)
                        .await
                    {
                        error!(error = %e, "gRPC sunucusu çöktü.");
                    }
                    break; // Sunucu çökerse döngüden çık
                }
                // AppState hazır değilse, kısa bir süre bekle ve tekrar kontrol et
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        })
    };

    // Bağımlılıkları kurmak için ayrı bir görev başlat
    tokio::spawn(async move {
        info!("Arka planda uygulama durumu (bağımlılıklar) başlatılıyor...");
        match AppState::new_critical(config_clone_for_init).await {
            Ok(mut state) => {
                state.connect_rabbitmq().await;
                let final_state = Arc::new(state);

                // Zaman aşımına uğramış çağrıları temizlemek için arka plan görevini başlat                
                tokio::spawn(cleanup_old_transactions(final_state.active_calls.clone()));
                
                *state_clone_for_init.lock().await = Some(final_state);
                info!("✅ Tüm bağımlılıklar başarıyla kuruldu. Servis tam işlevsel.");
            }
            Err(e) => {
                error!(error = %e, "Kritik bağımlılıklar başlatılamadı. Servis sonlandırılacak.");
                process::exit(1);
            }
        }
    });
        
        // Ana UDP dinleme döngüsü
        let main_loop = async {
        let mut buf = [0; 65535];
        loop {
            let (len, addr) = sock.recv_from(&mut buf).await?;
            let request_bytes = buf[..len].to_vec();
            
            let locked_state = shared_state.lock().await;
            if let Some(state) = locked_state.as_ref() {
                // Durum hazır, isteği işlemesi için yeni bir task başlat
                tokio::spawn(sip::handler::handle_sip_request(
                    request_bytes,
                    Arc::clone(&sock),
                    addr,
                    state.clone(),
                ));
            } else {
                // Durum henüz hazır değil, 503 yanıtı ver
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

    // Graceful shutdown mekanizması
    select! {
        res = main_loop => {
            if let Err(e) = res {
                error!(error = %ServiceError::from(e), "Kritik UDP ağ hatası, servis durduruluyor.");
            }
        },
        res = grpc_server_task => {
             if let Err(e) = res {
                error!(error = %e, "gRPC sunucu görevi hatayla sonlandı.");
             }
        },
        _ = signal::ctrl_c() => {
            warn!("Kapatma sinyali (Ctrl+C) alındı. Servis gracefully kapatılıyor...");
        }
    }
    
    info!("✅ Servis başarıyla kapatıldı.");
    Ok(())
}