// sentiric-sip-signaling-service/src/config.rs
use anyhow::Result;
use serde::Deserialize;
use std::{env, fmt, net::SocketAddr, sync::Arc};

// ===================================================================
//  Bölüm 1: Platform Seviyesi Yapılandırma
// ===================================================================

/// Tüm Sentiric platformu tarafından kullanılan ortam değişkenlerini
/// tip-güvenli bir şekilde temsil eder. "Tek Doğruluk Kaynağı" budur.
/// Bu struct, doğrudan ortamdan `serde` ile doldurulur.
#[derive(Deserialize, Debug, Clone)]
pub struct PlatformConfig {
    // --- TEMEL AYARLAR ---
    #[serde(default = "default_env")]
    pub env: String,
    #[serde(default = "default_rust_log")]
    pub rust_log: String,
    #[serde(default = "default_version")]
    pub service_version: String,

    // --- GENEL SERTİFİKA YOLLARI ---
    pub grpc_tls_ca_path: String,

    // --- BAĞIMLILIK URL'LERİ ---
    pub redis_url: String,
    pub rabbitmq_url: String,
    
    // --- BAĞIMLI SERVİS HEDEFLERİ ---
    pub media_service_public_ip: String,
    pub media_service_target_grpc_url: String,
    pub user_service_target_grpc_url: String,
    pub dialplan_service_target_grpc_url: String,

    // --- BU SERVİSE ÖZEL AYARLAR ---
    pub sip_signaling_udp_port: u16,
    pub sip_signaling_realm: String,
    pub sip_signaling_cert_path: String,
    pub sip_signaling_key_path: String,
}

impl PlatformConfig {
    /// Ortam değişkenlerinden tüm platform yapılandırmasını yükler.
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();
        let cfg = config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()?;
        Ok(cfg.try_deserialize()?)
    }
}

// Serde için varsayılan değer fonksiyonları
fn default_env() -> String { "development".to_string() }
fn default_rust_log() -> String { "info,h2=warn,hyper=warn".to_string() }
fn default_version() -> String { env::var("SERVICE_VERSION").unwrap_or_else(|_| "0.1.0".to_string()) }


// ===================================================================
//  Bölüm 2: Servise Özel Yapılandırma
// ===================================================================

/// `sip-signaling-service`'in çalışması için ihtiyaç duyduğu,
/// `PlatformConfig`'ten türetilmiş, kullanıma hazır yapılandırma.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub env: String,
    pub service_version: String,
    pub rust_log: String,
    
    pub cert_path: String,
    pub key_path: String,
    pub ca_path: String,
    
    pub sip_listen_addr: SocketAddr,
    pub sip_realm: String,
    
    pub media_service_public_ip: String,
    pub media_service_url: String,
    pub dialplan_service_url: String,
    pub user_service_url: String,
    pub rabbitmq_url: String,
    pub redis_url: String,
}

/// `PlatformConfig`'ten bu servise özel `AppConfig`'i oluşturan dönüşüm.
impl From<Arc<PlatformConfig>> for AppConfig {
    fn from(pc: Arc<PlatformConfig>) -> Self {
        Self {
            env: pc.env.clone(),
            service_version: pc.service_version.clone(),
            rust_log: pc.rust_log.clone(),
            cert_path: pc.sip_signaling_cert_path.clone(),
            key_path: pc.sip_signaling_key_path.clone(),
            ca_path: pc.grpc_tls_ca_path.clone(),
            sip_listen_addr: format!("0.0.0.0:{}", pc.sip_signaling_udp_port)
                .parse()
                .expect("Geçersiz SIP dinleme adresi"),
            sip_realm: pc.sip_signaling_realm.clone(),
            media_service_public_ip: pc.media_service_public_ip.clone(),
            media_service_url: pc.media_service_target_grpc_url.clone(),
            dialplan_service_url: pc.dialplan_service_target_grpc_url.clone(),
            user_service_url: pc.user_service_target_grpc_url.clone(),
            rabbitmq_url: pc.rabbitmq_url.clone(),
            redis_url: pc.redis_url.clone(),
        }
    }
}