// sentiric-sip-signaling-service/src/config.rs
use std::env;
use std::error::Error;
use std::fmt;
use std::net::SocketAddr;

#[derive(Clone)]
pub struct AppConfig {
    pub sip_listen_addr: SocketAddr,
    pub dialplan_service_url: String,
    pub media_service_url: String,
    pub user_service_url: String,
    pub rabbitmq_url: String,
    pub redis_url: String,
    pub env: String,
    pub sip_realm: String,
    pub service_version: String,
    pub cert_path: String,
    pub key_path: String,
    pub ca_path: String,
}

impl fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppConfig")
            .field("sip_listen_addr", &self.sip_listen_addr)
            .field("dialplan_service_url", &self.dialplan_service_url)
            .field("media_service_url", &self.media_service_url)
            .field("user_service_url", &self.user_service_url)
            .field("rabbitmq_url", &"***REDACTED***")
            .field("redis_url", &"***REDACTED***")
            .field("env", &self.env)
            .field("service_version", &self.service_version)
            .finish()
    }
}

impl AppConfig {
    pub fn load_from_env() -> Result<Self, Box<dyn Error>> {
        
        dotenvy::dotenv().ok();
        
        let sip_port_str = env::var("SIP_SIGNALING_UDP_PORT")
            .unwrap_or_else(|_| "13024".to_string());
        let sip_port = sip_port_str.parse::<u16>()?;

        let redis_use_ssl_str = env::var("REDIS_USE_SSL").unwrap_or_else(|_| "false".to_string());
        let redis_use_ssl = redis_use_ssl_str.parse::<bool>().unwrap_or(false);
        let redis_url_from_env = env::var("REDIS_URL")?;
        
        let redis_url = if redis_use_ssl && redis_url_from_env.starts_with("redis://") {
            redis_url_from_env.replacen("redis://", "rediss://", 1)
        } else {
            redis_url_from_env
        };

        let service_version = env::var("SERVICE_VERSION").unwrap_or_else(|_| "0.1.0".to_string());

        Ok(AppConfig {
            env: env::var("ENV").unwrap_or_else(|_| "production".to_string()),
            service_version,
            sip_listen_addr: format!("0.0.0.0:{}", sip_port).parse()?,
            sip_realm: env::var("SIP_SIGNALING_REALM").unwrap_or_else(|_| "sentiric_demo".to_string()),
            rabbitmq_url: env::var("RABBITMQ_URL")?,
            redis_url,
            
            media_service_url: env::var("MEDIA_SERVICE_TARGET_GRPC_URL")
                .expect("ZORUNLU: MEDIA_SERVICE_TARGET_GRPC_URL eksik"),
            user_service_url: env::var("USER_SERVICE_TARGET_GRPC_URL")
                .expect("ZORUNLU: USER_SERVICE_TARGET_GRPC_URL eksik"),
            dialplan_service_url: env::var("DIALPLAN_SERVICE_TARGET_GRPC_URL")
                .expect("ZORUNLU: DIALPLAN_SERVICE_TARGET_GRPC_URL eksik"),
            
            cert_path: env::var("SIP_SIGNALING_CERT_PATH")?,
            key_path: env::var("SIP_SIGNALING_KEY_PATH")?,
            ca_path: env::var("GRPC_TLS_CA_PATH")?,
        })
    }
}