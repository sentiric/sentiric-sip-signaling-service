// sentiric-sip-signaling-service/src/app.rs
use crate::{
    app_state::AppState,
    config::{AppConfig, PlatformConfig}, // PlatformConfig'i de import ediyoruz
    grpc::service::MySipSignalingService,
    sip::handler::handle_sip_request,
    state::cleanup_old_transactions,
};
use anyhow::Result;
use rustls::crypto::{ring::default_provider, CryptoProvider};
use sentiric_contracts::sentiric::sip::v1::sip_signaling_service_server::SipSignalingServiceServer;
use std::{env, panic, process, sync::Arc};
use tokio::{net::UdpSocket, select, signal};
use tonic::transport::{Certificate, Identity, Server as GrpcServer, ServerTlsConfig};
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, prelude::*, EnvFilter, Registry};

/// Uygulamanın ana yapısı. Konfigürasyonu ve durumunu içerir.
pub struct App {
    config: Arc<AppConfig>,
    state: Arc<AppState>,
}

impl App {
    /// Uygulamayı başlatır: config'i yükler, loglamayı ayarlar ve App state'ini oluşturur.
    pub async fn bootstrap() -> Result<Self> {
        setup_panic_hook();
        let config = initialize_config_and_logging()?;

        info!(
            service_name = "sentiric-sip-signaling-service",
            version = %config.service_version,
            commit = %env::var("GIT_COMMIT").unwrap_or_else(|_| "unknown".to_string()),
            build_date = %env::var("BUILD_DATE").unwrap_or_else(|_| "unknown".to_string()),
            profile = %config.env,
            "🚀 Servis başlatılıyor..."
        );

        let state = initialize_app_state(config.clone()).await;
        info!("✅ Tüm bağımlılıklar başarıyla kuruldu. Servis tam işlevsel.");

        Ok(Self { config, state })
    }

    /// Uygulamanın ana döngüsünü çalıştırır: ağ dinleyicilerini ve görevleri başlatır.
    pub async fn run(self) -> Result<()> {
        let sock = Arc::new(UdpSocket::bind(self.config.sip_listen_addr).await?);
        info!(address = %self.config.sip_listen_addr, "✅ UDP SIP dinleyici başlatıldı.");

        tokio::spawn(cleanup_old_transactions(self.state.active_calls.clone()));
        let grpc_server_task = spawn_grpc_server(self.state.clone(), sock.clone(), self.config.clone());
        let udp_listener_task = spawn_udp_listener(self.state.clone(), sock);

        select! {
            res = udp_listener_task => { if let Err(e) = res { error!(error = ?e, "UDP dinleyici görevi hatayla sonlandı."); } },
            res = grpc_server_task => { if let Err(e) = res { error!(error = ?e, "gRPC sunucu görevi hatayla sonlandı."); } },
            _ = signal::ctrl_c() => { warn!("Kapatma sinyali (Ctrl+C) alındı. Servis kapatılıyor..."); }
        }

        info!("✅ Servis başarıyla kapatıldı.");
        Ok(())
    }
}

// --- Yardımcı Fonksiyonlar ---

fn setup_panic_hook() {
    let default_panic_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        error!(%panic_info, "Kritik bir panik oluştu!");
        default_panic_hook(panic_info);
    }));
}

fn initialize_config_and_logging() -> Result<Arc<AppConfig>> {
    CryptoProvider::install_default(default_provider()).expect("Crypto provider (ring) kurulamadı.");
    
    // Adım 1: Tüm platformun yapılandırmasını merkezi struct'a yükle.
    let platform_config = match PlatformConfig::from_env() {
        Ok(cfg) => Arc::new(cfg),
        Err(e) => {
            eprintln!("### BAŞLANGIÇ HATASI: Platform yapılandırması yüklenemedi: {}", e);
            process::exit(1);
        }
    };
    
    // Adım 2: Bu servise özel yapılandırmayı, platform yapılandırmasından türet.
    let config = Arc::new(AppConfig::from(platform_config));
    
    // Adım 3: Loglamayı başlat.
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&config.rust_log))?;
    let subscriber = Registry::default().with(env_filter);
    if config.env == "development" {
        subscriber.with(fmt::layer().with_target(true).with_line_number(true)).init();
    } else {
        subscriber.with(fmt::layer().json().with_current_span(true).with_span_list(true)).init();
    }
    Ok(config)
}

async fn initialize_app_state(config: Arc<AppConfig>) -> Arc<AppState> {
    info!("Tüm kritik bağımlılıklar başlatılıyor...");
    let mut app_state = match AppState::new_critical(config).await {
        Ok(state) => state,
        Err(e) => {
            error!(error = %e, "Kritik bağımlılıklar başlatılamadı. Servis sonlandırılacak.");
            process::exit(1);
        }
    };
    app_state.connect_rabbitmq().await;
    Arc::new(app_state)
}

fn spawn_udp_listener(app_state: Arc<AppState>, sock: Arc<UdpSocket>) -> tokio::task::JoinHandle<Result<(), std::io::Error>> {
    tokio::spawn(async move {
        let mut buf = [0; 65535];
        loop {
            let (len, addr) = sock.recv_from(&mut buf).await?;
            let request_bytes = buf[..len].to_vec();
            tokio::spawn(handle_sip_request(request_bytes, Arc::clone(&sock), addr, app_state.clone()));
        }
    })
}

fn spawn_grpc_server(app_state: Arc<AppState>, sock: Arc<UdpSocket>, config: Arc<AppConfig>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let grpc_service = MySipSignalingService { app_state, sock };
        let grpc_port_str = env::var("SIP_SIGNALING_SERVICE_GRPC_PORT").unwrap_or_else(|_| "13021".to_string());
        let addr = format!("[::]:{}", grpc_port_str).parse().unwrap();
        
        let tls_config = match load_tls_config(&config).await {
            Ok(cfg) => cfg,
            Err(e) => {
                error!(error = %e, "mTLS yapılandırması yüklenemedi.");
                return;
            }
        };
        info!(address = %addr, "gRPC sunucusu (mTLS ile) başlatılıyor...");
        if let Err(e) = GrpcServer::builder()
            .tls_config(tls_config).expect("TLS yapılandırması başarısız olmamalı")
            .add_service(SipSignalingServiceServer::new(grpc_service))
            .serve(addr).await {
            error!(error = %e, "gRPC sunucusu çöktü.");
        }
    })
}

async fn load_tls_config(config: &AppConfig) -> Result<tonic::transport::ServerTlsConfig> {
    let cert = tokio::fs::read(&config.cert_path).await?;
    let key = tokio::fs::read(&config.key_path).await?;
    let identity = Identity::from_pem(cert, key);
    let ca_cert = tokio::fs::read(&config.ca_path).await?;
    let client_ca_cert = Certificate::from_pem(ca_cert);
    Ok(ServerTlsConfig::new().identity(identity).client_ca_root(client_ca_cert))
}