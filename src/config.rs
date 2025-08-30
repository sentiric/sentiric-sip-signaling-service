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
    pub redis_url: String, // <-- YENİ SATIR
    pub env: String,
    pub sip_realm: String,
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
            .field("redis_url", &"***REDACTED***") // <-- YENİ SATIR
            .field("env", &self.env)
            .finish()
    }
}

impl AppConfig {
    pub fn load_from_env() -> Result<Self, Box<dyn Error>> {
        dotenv::dotenv().ok();
        let sip_host = env::var("SIP_SIGNALING_SERVICE_LISTEN_ADDRESS").unwrap_or_else(|_| "0.0.0.0".to_string());
        let sip_port_str = env::var("SIP_SIGNALING_SERVICE_PORT").unwrap_or_else(|_| "5060".to_string());
        let sip_port = sip_port_str.parse::<u16>()?;

        Ok(AppConfig {
            sip_listen_addr: format!("{}:{}", sip_host, sip_port).parse()?,
            sip_public_ip: env::var("PUBLIC_IP")?,
            rabbitmq_url: env::var("RABBITMQ_URL")?,
            redis_url: env::var("REDIS_URL")?, // <-- YENİ SATIR
            media_service_url: env::var("MEDIA_SERVICE_GRPC_URL")?,
            user_service_url: env::var("USER_SERVICE_GRPC_URL")?,
            dialplan_service_url: env::var("DIALPLAN_SERVICE_GRPC_URL")?,
            env: env::var("ENV").unwrap_or_else(|_| "production".to_string()),
            sip_realm: env::var("SIP_REALM").unwrap_or_else(|_| "sentiric_demo".to_string()),
        })
    }
}