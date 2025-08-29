// File: src/sip/register.rs (Uyarı Düzeltmesi)

use crate::config::AppConfig;
use crate::grpc::client::create_secure_grpc_channel;
use crate::sip::md5::Context;
use crate::sip::utils::{create_response, parse_complex_headers};
use crate::state::{RegistrationInfo, Registrations};
use rand::distributions::{Alphanumeric, DistString};
use sentiric_contracts::sentiric::user::v1::{
    user_service_client::UserServiceClient, GetSipCredentialsRequest,
};
use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tonic::Request as TonicRequest;
use tracing::{info, instrument, warn, Span};

#[instrument(skip_all, fields(remote_addr = %addr, call_id))]
pub async fn handle_register(
    request_str: &str,
    sock: Arc<UdpSocket>,
    addr: SocketAddr,
    config: Arc<AppConfig>,
    registrations: Registrations,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let headers = parse_complex_headers(request_str).ok_or("Geçersiz başlıklar")?;
    let call_id = headers.get("Call-ID").cloned().unwrap_or_default();
    Span::current().record("call_id", &call_id as &str);

    if let Some(auth_header) = headers.get("Authorization") {
        verify_authentication(auth_header, &headers, sock, addr, config, registrations).await
    } else {
        challenge_client(&headers, sock, addr, config).await
    }
}

async fn challenge_client(
    headers: &HashMap<String, String>,
    sock: Arc<UdpSocket>,
    addr: SocketAddr,
    config: Arc<AppConfig>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    info!("Authorization başlığı yok, 401 Unauthorized ile challenge gönderiliyor.");
    let nonce = Alphanumeric.sample_string(&mut rand::thread_rng(), 32);
    let auth_challenge = format!(r#"Digest realm="{}", nonce="{}""#, config.sip_realm, nonce);
    let mut response_headers = headers.clone();
    response_headers.insert("WWW-Authenticate".to_string(), auth_challenge);

    let response = create_response("401 Unauthorized", &response_headers, None, &config);
    sock.send_to(response.as_bytes(), addr).await?;
    Ok(())
}

async fn verify_authentication(
    auth_header: &str,
    headers: &HashMap<String, String>,
    sock: Arc<UdpSocket>,
    addr: SocketAddr,
    config: Arc<AppConfig>,
    registrations: Registrations,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    info!("Authorization başlığı bulundu, kimlik bilgileri doğrulanıyor.");
    let auth_parts: HashMap<_, _> = auth_header
        .strip_prefix("Digest ")
        .unwrap_or("")
        .split(',')
        .filter_map(|s| s.trim().split_once('='))
        .map(|(k, v)| (k.trim(), v.trim().trim_matches('"')))
        .collect();

    let username = auth_parts.get("username").ok_or("username eksik")?;
    let realm = auth_parts.get("realm").ok_or("realm eksik")?;
    let nonce = auth_parts.get("nonce").ok_or("nonce eksik")?;
    let uri = auth_parts.get("uri").ok_or("uri eksik")?;
    let client_response = auth_parts.get("response").ok_or("response eksik")?;

    let user_channel = create_secure_grpc_channel(&config.user_service_url, "user-service").await?;
    let mut user_client = UserServiceClient::new(user_channel);
    let creds_res = user_client.get_sip_credentials(TonicRequest::new(GetSipCredentialsRequest {
        sip_username: username.to_string(),
    })).await;

    if let Err(e) = creds_res {
        warn!(error = %e, "SIP kullanıcısı bulunamadı veya user-service hatası.");
        let response = create_response("403 Forbidden", headers, None, &config);
        sock.send_to(response.as_bytes(), addr).await?;
        return Ok(());
    }

    let ha1_hash = creds_res.unwrap().into_inner().ha1_hash;
    
    // DÜZELTME 2: Hasher'ı doğru şekilde oluştur ve doğru metotları kullan.
    // a2 = MD5("REGISTER:sip_uri")
    let mut hasher_a2 = Context::new();
    hasher_a2.consume(format!("REGISTER:{}", uri).as_bytes());
    let a2_hash = format!("{:x}", hasher_a2.finalize());
    
    // expected_response = MD5(ha1_hash:nonce:a2_hash)
    let mut hasher_response = Context::new();
    hasher_response.consume(format!("{}:{}:{}", ha1_hash, nonce, a2_hash).as_bytes());
    let expected_response = format!("{:x}", hasher_response.finalize());

        if *client_response == expected_response {
            info!("Kimlik doğrulama başarılı. Kullanıcı kaydediliyor.");
            let contact_uri = headers.get("Contact").cloned().unwrap_or_default();
            let expires = headers.get("Expires").and_then(|e| e.parse::<u64>().ok()).unwrap_or(3600);

            // DÜZELTME: Alan adlarını _ önekiyle güncelledik.
            let registration_info = RegistrationInfo {
                _contact_uri: contact_uri.clone(),
                _expires_at: Instant::now() + Duration::from_secs(expires),
            };
            registrations.lock().await.insert(format!("sip:{}@{}", username, realm), registration_info);

            let mut response_headers = headers.clone();
            response_headers.insert("Contact".to_string(), format!("{};expires={}", contact_uri, expires));
            
            let response = create_response("200 OK", &response_headers, None, &config);
            sock.send_to(response.as_bytes(), addr).await?;
        } else {
            warn!("Kimlik doğrulama başarısız. Yanlış şifre.");
            let response = create_response("403 Forbidden", headers, None, &config);
            sock.send_to(response.as_bytes(), addr).await?;
        }

        Ok(())
    }
