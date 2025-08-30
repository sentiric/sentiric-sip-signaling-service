// File: sentiric-sip-signaling-service/src/sip/register.rs

use crate::config::AppConfig;
use crate::grpc::client::create_secure_grpc_channel;
use crate::redis::{self, AsyncCommands};
use crate::sip::utils::{create_response, parse_complex_headers};
use lazy_static::lazy_static;
use md5::compute;
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
use tokio::sync::Mutex; // <-- TOKIO'NUN MUTEX'İNİ KULLANIYORUZ
use tonic::Request as TonicRequest;
use tracing::{error, info, instrument, warn, Span};

// Tokio'nun asenkron Mutex'i ile lazy_static kullanıyoruz.
lazy_static! {
    static ref PENDING_REGISTRATIONS: Arc<Mutex<HashMap<String, Instant>>> = Arc::new(Mutex::new(HashMap::new()));
}

#[instrument(skip_all, fields(remote_addr = %addr, call_id))]
pub async fn handle_register(
    request_str: &str,
    sock: Arc<UdpSocket>,
    addr: SocketAddr,
    config: Arc<AppConfig>,
    redis_client: Arc<redis::Client>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let headers = parse_complex_headers(request_str).ok_or("Geçersiz başlıklar")?;
    let call_id = headers.get("Call-ID").cloned().unwrap_or_default();
    Span::current().record("call_id", &call_id as &str);

    if let Some(auth_header) = headers.get("Authorization") {
        verify_authentication(auth_header, &headers, sock, addr, config, redis_client).await
    } else {
        // Tokio Mutex'i asenkron olduğu için .lock() çağrısına .await ekliyoruz.
        let mut pending = PENDING_REGISTRATIONS.lock().await;
        if let Some(instant) = pending.get(&call_id) {
            if instant.elapsed() < Duration::from_secs(10) {
                warn!("Kısa süre içinde aynı Call-ID ile tekrar REGISTER isteği alındı, görmezden geliniyor.");
                return Ok(());
            }
        }
        pending.insert(call_id.clone(), Instant::now());
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
    
    let auth_challenge = format!(
        r#"Digest realm="{}", qop="auth", nonce="{}""#,
        config.sip_realm, nonce
    );

    let mut response_headers = headers.clone();
    response_headers.insert("WWW-Authenticate".to_string(), auth_challenge);

    let response = create_response("401 Unauthorized", &response_headers, None, &config, addr);
    sock.send_to(response.as_bytes(), addr).await?;
    Ok(())
}

async fn verify_authentication(
    auth_header: &str,
    headers: &HashMap<String, String>,
    sock: Arc<UdpSocket>,
    addr: SocketAddr,
    config: Arc<AppConfig>,
    redis_client: Arc<redis::Client>,
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
    let qop = auth_parts.get("qop");
    let cnonce = auth_parts.get("cnonce");
    let nc = auth_parts.get("nc");

    let user_channel = create_secure_grpc_channel(&config.user_service_url, "user-service").await?;
    let mut user_client = UserServiceClient::new(user_channel);
    let creds_res = user_client.get_sip_credentials(TonicRequest::new(GetSipCredentialsRequest {
        sip_username: username.to_string(),
    })).await;

    if let Err(e) = creds_res {
        warn!(error = %e, "SIP kullanıcısı bulunamadı veya user-service hatası.");
        let response = create_response("403 Forbidden", headers, None, &config, addr);
        sock.send_to(response.as_bytes(), addr).await?;
        return Ok(());
    }

    let ha1_hash = creds_res.unwrap().into_inner().ha1_hash;
    
    let a2_str = format!("REGISTER:{}", uri);
    let a2_hash = format!("{:x}", compute(a2_str.as_bytes()));

    let expected_response = if let (Some(&qop_val), Some(&cnonce_val), Some(&nc_val)) = (qop, cnonce, nc) {
        let response_str = format!("{}:{}:{}:{}:{}:{}", ha1_hash, nonce, nc_val, cnonce_val, qop_val, a2_hash);
        format!("{:x}", compute(response_str.as_bytes()))
    } else {
        let response_str = format!("{}:{}:{}", ha1_hash, nonce, a2_hash);
        format!("{:x}", compute(response_str.as_bytes()))
    };

    if *client_response == expected_response {
        info!("Kimlik doğrulama başarılı. Kullanıcı kaydediliyor.");
        let contact_uri = headers.get("Contact").cloned().unwrap_or_default();
        let expires = headers.get("Expires").and_then(|e| e.parse::<u64>().ok()).unwrap_or(3600);

        if expires > 0 {
            let aor = format!("sip_registration:sip:{}@{}", username, realm);
            if let Err(e) = redis::set_registration(&redis_client, &aor, &contact_uri, expires).await {
                error!(error = %e, "Redis'e kayıt yapılamadı.");
                let response = create_response("500 Server Internal Error", headers, None, &config, addr);
                sock.send_to(response.as_bytes(), addr).await?;
                return Err(e.into());
            }
        } else {
            info!("Kullanıcı kaydı siliniyor (Expires=0).");
            let aor = format!("sip_registration:sip:{}@{}", username, realm);
            let mut conn = redis_client.get_multiplexed_async_connection().await?;
            let _: () = conn.del(&aor).await?;
        }

        let mut response_headers = headers.clone();
        response_headers.insert("Contact".to_string(), format!("{};expires={}", contact_uri, expires));
        
        let response = create_response("200 OK", &response_headers, None, &config, addr);
        sock.send_to(response.as_bytes(), addr).await?;
    } else {
        warn!(client_response, expected_response, "Kimlik doğrulama başarısız. Yanlış şifre.");
        let response = create_response("403 Forbidden", headers, None, &config, addr);
        sock.send_to(response.as_bytes(), addr).await?;
    }

    Ok(())
}