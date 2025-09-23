// sentiric-sip-signaling-service/src/main.rs
use anyhow::Result;
use rustls::crypto::{ring::default_provider, CryptoProvider};
use sentiric_contracts::sentiric::sip::v1::sip_signaling_service_server::SipSignalingServiceServer;
use sip::{handler::handle_sip_request, responses::create_response, utils::parse_complex_headers}; // `utils`'i buraya ekliyoruz
use state::cleanup_old_transactions;
use std::{env, process, sync::Arc, time::Duration};
use tokio::{net::UdpSocket, select, signal, sync::Mutex};
use tonic::transport::{Certificate, Identity, Server as GrpcServer, ServerTlsConfig};
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, prelude::*, EnvFilter, Registry};

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

type SharedAppState = Arc<Mutex<Option<Arc<AppState>>>>;

#[tokio::main]
async fn main() -> Result<()> {
    CryptoProvider::install_default(default_provider()).expect("Crypto provider (ring) kurulamadı.");
    let config = match AppConfig::load_from_env() {
        Ok(cfg) => Arc::new(cfg),
        Err(e) => {
            eprintln!("### BAŞLANGIÇ HATASI: Yapılandırma yüklenemedi: {}", e);
            process::exit(1);
        }
    };
    let rust_log_env = env::var("RUST_LOG").unwrap_or_else(|_| "info,h2=warn,hyper=warn,tower=warn,rustls=warn,lapin=warn".to_string());
    let env_filter = EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new(&rust_log_env))?;
    let subscriber = Registry::default().with(env_filter);
    if config.env == "development" {
        subscriber.with(fmt::layer().with_target(true).with_line_number(true)).init();
    } else {
        subscriber.with(fmt::layer().json().with_current_span(true).with_span_list(true)).init();
    }
    info!(
        service_name = "sentiric-sip-signaling-service", version = %config.service_version,
        commit = %env::var("GIT_COMMIT").unwrap_or_else(|_| "unknown".to_string()),
        build_date = %env::var("BUILD_DATE").unwrap_or_else(|_| "unknown".to_string()),
        profile = %config.env, "🚀 Servis başlatılıyor..."
    );
    let shared_state: SharedAppState = Arc::new(Mutex::new(None));
    let sock: Arc<UdpSocket> = Arc::new(UdpSocket::bind(config.sip_listen_addr).await?);
    info!(address = %config.sip_listen_addr, "✅ UDP SIP dinleyici hemen başlatıldı.");
    let state_clone_for_init = shared_state.clone();
    let config_clone_for_init = config.clone();
    let grpc_server_task = {
        let state_clone = shared_state.clone();
        let sock_clone = sock.clone();
        let config_clone = config.clone();
        tokio::spawn(async move {
            loop {
                if let Some(state) = state_clone.lock().await.as_ref() {
                    let grpc_service = MySipSignalingService { app_state: state.clone(), sock: sock_clone.clone() };
                    let grpc_port = env::var("SIP_SIGNALING_GRPC_PORT").unwrap_or_else(|_| "13021".to_string());
                    let addr = format!("[::]:{}", grpc_port).parse().unwrap();
                    let cert = tokio::fs::read(&config_clone.cert_path).await.expect("Sunucu sertifikası okunamadı");
                    let key = tokio::fs::read(&config_clone.key_path).await.expect("Sunucu anahtarı okunamadı");
                    let identity = Identity::from_pem(cert, key);
                    let ca_cert = tokio::fs::read(&config_clone.ca_path).await.expect("CA sertifikası okunamadı");
                    let client_ca_cert = Certificate::from_pem(ca_cert);
                    let tls_config = ServerTlsConfig::new().identity(identity).client_ca_root(client_ca_cert);
                    info!(address = %addr, "gRPC sunucusu (mTLS ile) başlatılıyor...");
                    if let Err(e) = GrpcServer::builder().tls_config(tls_config).expect("TLS yapılandırması başarısız").add_service(SipSignalingServiceServer::new(grpc_service)).serve(addr).await {
                        error!(error = %e, "gRPC sunucusu çöktü.");
                    }
                    break;
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        })
    };
    tokio::spawn(async move {
        info!("Arka planda uygulama durumu (bağımlılıklar) başlatılıyor...");
        match AppState::new_critical(config_clone_for_init).await {
            Ok(mut state) => {
                state.connect_rabbitmq().await;
                let final_state = Arc::new(state);
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
    let main_loop = async {
        let mut buf = [0; 65535];
        loop {
            let (len, addr) = sock.recv_from(&mut buf).await?;
            let request_bytes = buf[..len].to_vec();
            info!(from = %addr, bytes_received = len, "UDP paketi alındı, işleyiciye yönlendiriliyor.");
            let locked_state = shared_state.lock().await;
            if let Some(state) = locked_state.as_ref() {
                tokio::spawn(handle_sip_request(request_bytes, Arc::clone(&sock), addr, state.clone()));
            } else {
                warn!(from = %addr, "Servis henüz başlatılıyor, isteğe 503 Service Unavailable yanıtı veriliyor.");
                let request_str = String::from_utf8_lossy(&request_bytes);
                if request_str.starts_with("INVITE") {
                    if let Some((headers, via_headers)) = parse_complex_headers(&request_str) {
                        // DÜZELTME: CallContext oluşturmak yerine, yanıt için gerekli minimal yapıyı kullanıyoruz.
                        let mut temp_headers = headers;
                        temp_headers.insert("via".to_string(), via_headers.join(",")); // Geçici olarak birleştir
                        let response = create_response("503 Service Unavailable", &temp_headers, None, &config, addr);
                        let _ = sock.send_to(response.as_bytes(), addr).await;
                    }
                }
            }
        }
        #[allow(unreachable_code)]
        Ok::<(), std::io::Error>(())
    };
    select! {
        res = main_loop => { if let Err(e) = res { error!(error = %ServiceError::from(e), "Kritik UDP ağ hatası, servis durduruluyor."); } },
        res = grpc_server_task => { if let Err(e) = res { error!(error = %e, "gRPC sunucu görevi hatayla sonlandı."); } },
        _ = signal::ctrl_c() => { warn!("Kapatma sinyali (Ctrl+C) alındı. Servis gracefully kapatılıyor..."); }
    }
    info!("✅ Servis başarıyla kapatıldı.");
    Ok(())
}