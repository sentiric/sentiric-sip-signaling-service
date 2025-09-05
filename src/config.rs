// File: src/config.rs
use std::env;
use std::error::Error;
use std::fmt;
use std::net::SocketAddr;

#[derive(Clone)]
pub struct AppConfig {
    pub sip_listen_addr: SocketAddr,
    pub sip_public_ip: String,
    pub dialplan_service_url: String,
    pub media_service_url: String,
    pub user_service_url: String,
    pub rabbitmq_url: String,
    pub redis_url: String,
    pub env: String,
    pub sip_realm: String,
    pub service_version: String, // YENİ EKLENDİ
}

impl fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppConfig")
            .field("sip_listen_addr", &self.sip_listen_addr)
            .field("sip_public_ip", &self.sip_public_ip)
            .field("dialplan_service_url", &self.dialplan_service_url)
            .field("media_service_url", &self.media_service_url)
            .field("user_service_url", &self.user_service_url)
            .field("rabbitmq_url", &"***REDACTED***")
            .field("redis_url", &"***REDACTED***")
            .field("env", &self.env)
            .field("service_version", &self.service_version) // YENİ EKLENDİ
            .finish()
    }
}

impl AppConfig {
    pub fn load_from_env() -> Result<Self, Box<dyn Error>> {
        dotenv::dotenv().ok();
        let sip_host = env::var("SIP_SIGNALING_SERVICE_LISTEN_ADDRESS").unwrap_or_else(|_| "0.0.0.0".to_string());
        let sip_port_str = env::var("SIP_SIGNALING_SERVICE_PORT").unwrap_or_else(|_| "5060".to_string());
        let sip_port = sip_port_str.parse::<u16>()?;

        let redis_use_ssl_str = env::var("REDIS_USE_SSL").unwrap_or_else(|_| "false".to_string());
        let redis_use_ssl = redis_use_ssl_str.parse::<bool>().unwrap_or(false);
        let redis_url_from_env = env::var("REDIS_URL")?;
        
        let redis_url = if redis_use_ssl && redis_url_from_env.starts_with("redis://") {
            redis_url_from_env.replacen("redis://", "rediss://", 1)
        } else {
            redis_url_from_env
        };

        // YENİ: Servis versiyonunu çevre değişkenlerinden oku.
        let service_version = env::var("SERVICE_VERSION").unwrap_or_else(|_| "0.1.0".to_string());

        Ok(AppConfig {
            sip_listen_addr: format!("{}:{}", sip_host, sip_port).parse()?,
            sip_public_ip: env::var("PUBLIC_IP")?,
            rabbitmq_url: env::var("RABBITMQ_URL")?,
            redis_url,
            media_service_url: env::var("MEDIA_SERVICE_GRPC_URL")?,
            user_service_url: env::var("USER_SERVICE_GRPC_URL")?,
            dialplan_service_url: env::var("DIALPLAN_SERVICE_GRPC_URL")?,
            env: env::var("ENV").unwrap_or_else(|_| "production".to_string()),
            sip_realm: env::var("SIP_REALM").unwrap_or_else(|_| "sentiric_demo".to_string()),
            service_version, // YENİ EKLENDİ
        })
    }
}