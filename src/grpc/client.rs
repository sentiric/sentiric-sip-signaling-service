// ========== FILE: src/grpc/client.rs ==========
use std::env;
use std::error::Error;
use std::time::Duration;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};

pub async fn create_secure_grpc_channel(
    url: &str,
    server_name: &str,
) -> Result<Channel, Box<dyn Error + Send + Sync>> {
    let cert_path = env::var("SIP_SIGNALING_SERVICE_CERT_PATH")?;
    let key_path = env::var("SIP_SIGNALING_SERVICE_KEY_PATH")?;
    let ca_path = env::var("GRPC_TLS_CA_PATH")?;
    let cert = tokio::fs::read(cert_path).await?;
    let key = tokio::fs::read(key_path).await?;
    let ca_cert = tokio::fs::read(ca_path).await?;
    let identity = Identity::from_pem(cert, key);
    let ca_cert = Certificate::from_pem(ca_cert);
    let tls_config = ClientTlsConfig::new()
        .domain_name(server_name)
        .ca_certificate(ca_cert)
        .identity(identity);
    let endpoint = Channel::from_shared(format!("https://{}", url))?
        .tls_config(tls_config)?
        .connect_timeout(Duration::from_secs(5))
        .keep_alive_while_idle(true)
        .timeout(Duration::from_secs(10));
    let channel = endpoint.connect().await?;
    Ok(channel)
}