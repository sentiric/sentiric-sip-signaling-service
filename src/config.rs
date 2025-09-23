// sentiric-sip-signaling-service/src/config.rs
use anyhow::{Context, Result}; // DÜZELTME: Bu satır en kritik eklemedir. .context() metodunu scope'a dahil eder.
use std::env;
use std::fmt;
use std::net::SocketAddr;

#[derive(Clone)]
pub struct AppConfig {
    // --- Gözlemlenebilirlik & Ortam ---
    pub env: String,
    pub service_version: String,

    // --- Güvenlik (mTLS Sertifikaları) ---
    pub cert_path: String,
    pub key_path: String,
    pub ca_path: String,

    // --- Ağ Adresleri (İç) ---
    pub sip_listen_addr: SocketAddr,
    pub sip_realm: String,

    // --- Ağ Adresleri (Dış ve Hedefler) ---
    pub media_service_public_ip: String,
    pub media_service_url: String,
    pub dialplan_service_url: String,
    pub user_service_url: String,

    // --- Altyapı Bağlantıları ---
    pub rabbitmq_url: String,
    pub redis_url: String,
}

impl fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppConfig")
            .field("env", &self.env)
            .field("service_version", &self.service_version)
            .field("sip_listen_addr", &self.sip_listen_addr)
            .field("media_service_public_ip", &self.media_service_public_ip)
            .field("user_service_url", &self.user_service_url)
            .field("dialplan_service_url", &self.dialplan_service_url)
            .field("media_service_url", &self.media_service_url)
            .finish_non_exhaustive()
    }
}

impl AppConfig {
    // DÜZELTME: Fonksiyon imzası artık Box<dyn Error> yerine anyhow::Result kullanıyor.
    pub fn load_from_env() -> Result<Self> { 
        dotenvy::dotenv().ok();
        
        let service_version = env::var("SERVICE_VERSION").unwrap_or_else(|_| "0.1.0".to_string());
        let sip_port_str = env::var("SIP_SIGNALING_UDP_PORT").unwrap_or_else(|_| "13024".to_string());
        let sip_port = sip_port_str.parse::<u16>()?;

        let redis_use_ssl_str = env::var("REDIS_USE_SSL").unwrap_or_else(|_| "false".to_string());
        let redis_use_ssl = redis_use_ssl_str.parse::<bool>().unwrap_or(false);
        let redis_url_from_env = env::var("REDIS_URL")?;
        
        let redis_url = if redis_use_ssl && redis_url_from_env.starts_with("redis://") {
            redis_url_from_env.replacen("redis://", "rediss://", 1)
        } else {
            redis_url_from_env
        };

        Ok(AppConfig {
            env: env::var("ENV").unwrap_or_else(|_| "production".to_string()),
            service_version,
            
            cert_path: env::var("SIP_SIGNALING_CERT_PATH").context("ZORUNLU: SIP_SIGNALING_CERT_PATH eksik")?,
            key_path: env::var("SIP_SIGNALING_KEY_PATH").context("ZORUNLU: SIP_SIGNALING_KEY_PATH eksik")?,
            ca_path: env::var("GRPC_TLS_CA_PATH").context("ZORUNLU: GRPC_TLS_CA_PATH eksik")?,
            
            sip_listen_addr: format!("0.0.0.0:{}", sip_port).parse()?,
            sip_realm: env::var("SIP_SIGNALING_REALM").unwrap_or_else(|_| "sentiric_demo".to_string()),
            
            media_service_public_ip: env::var("MEDIA_SERVICE_PUBLIC_IP").context("ZORUNLU: MEDIA_SERVICE_PUBLIC_IP eksik")?,
            
            media_service_url: env::var("MEDIA_SERVICE_TARGET_GRPC_URL").context("ZORUNLU: MEDIA_SERVICE_TARGET_GRPC_URL eksik")?,
            dialplan_service_url: env::var("DIALPLAN_SERVICE_TARGET_GRPC_URL").context("ZORUNLU: DIALPLAN_SERVICE_TARGET_GRPC_URL eksik")?,
            user_service_url: env::var("USER_SERVICE_TARGET_GRPC_URL").context("ZORUNLU: USER_SERVICE_TARGET_GRPC_URL eksik")?,
            
            rabbitmq_url: env::var("RABBITMQ_URL")?,
            redis_url,
        })
    }
}