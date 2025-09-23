// sentiric-sip-signaling-service/src/sip/register.rs
use crate::app_state::AppState;
use crate::redis::{self, AsyncCommands};
use crate::sip::responses::create_response;
use crate::sip::utils::parse_complex_headers;
use md5::compute;
use rand::distributions::{Alphanumeric, DistString};
use sentiric_contracts::sentiric::user::v1::GetSipCredentialsRequest;
use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tonic::Request as TonicRequest;
use tracing::{error, info, instrument, warn, Span};

#[instrument(skip_all, fields(remote_addr = %addr, call_id))]
pub async fn handle(
    request_str: &str,
    sock: Arc<UdpSocket>,
    addr: SocketAddr,
    state: Arc<AppState>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let (headers, via_headers) = parse_complex_headers(request_str).ok_or("Geçersiz başlıklar")?;
    let call_id = headers.get("call-id").cloned().unwrap_or_default();
    Span::current().record("call_id", &call_id as &str);
    
    if let Some(auth_header) = headers.get("authorization") {
        verify_authentication(auth_header, headers, via_headers, sock, addr, state).await
    } else {
        let mut conn = state.redis.get_multiplexed_async_connection().await?;
        let key = format!("pending_reg:{}", call_id);
        if conn.exists(&key).await? {
            warn!("Kısa süre içinde aynı Call-ID ile tekrar REGISTER isteği alındı, görmezden geliniyor.");
            return Ok(());
        }
        let _: () = conn.set_ex(&key, true, 10).await?;
        challenge_client(headers, via_headers, sock, addr, state).await
    }
}

async fn challenge_client(
    mut headers: HashMap<String, String>,
    via_headers: Vec<String>,
    sock: Arc<UdpSocket>,
    addr: SocketAddr,
    state: Arc<AppState>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    info!("Authorization başlığı yok, 401 Unauthorized ile challenge gönderiliyor.");
    let nonce = Alphanumeric.sample_string(&mut rand::thread_rng(), 32);
    let auth_challenge = format!(r#"Digest realm="{}", qop="auth", nonce="{}""#, state.config.sip_realm, nonce);
    headers.insert("www-authenticate".to_string(), auth_challenge);

    let response = create_response("401 Unauthorized", &via_headers, &headers, None, &state.config, addr);
    sock.send_to(response.as_bytes(), addr).await?;
    Ok(())
}

async fn verify_authentication(
    auth_header: &str,
    mut headers: HashMap<String, String>,
    via_headers: Vec<String>,
    sock: Arc<UdpSocket>,
    addr: SocketAddr,
    state: Arc<AppState>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    info!("Authorization başlığı bulundu, kimlik bilgileri doğrulanıyor.");
    let auth_parts: HashMap<_, _> = auth_header.strip_prefix("Digest ").unwrap_or("").split(',')
        .filter_map(|s| s.trim().split_once('='))
        .map(|(k, v)| (k.trim(), v.trim().trim_matches('"')))
        .collect();

    let username = *auth_parts.get("username").ok_or("username eksik")?;
    let realm = *auth_parts.get("realm").ok_or("realm eksik")?;
    let nonce = *auth_parts.get("nonce").ok_or("nonce eksik")?;
    let uri = *auth_parts.get("uri").ok_or("uri eksik")?;
    let client_response = *auth_parts.get("response").ok_or("response eksik")?;
    
    let mut user_client = state.grpc.user.clone();
    let creds_res = user_client.get_sip_credentials(TonicRequest::new(GetSipCredentialsRequest {
        sip_username: username.to_string(), realm: realm.to_string(),
    })).await;

    if let Err(e) = creds_res {
        warn!(error = %e, "SIP kullanıcısı bulunamadı veya user-service hatası.");
        let response = create_response("403 Forbidden", &via_headers, &headers, None, &state.config, addr);
        sock.send_to(response.as_bytes(), addr).await?;
        return Ok(());
    }

    let ha1_hash = creds_res.unwrap().into_inner().ha1_hash;
    let a2_str = format!("REGISTER:{}", uri);
    let a2_hash = format!("{:x}", compute(a2_str.as_bytes()));
    let response_str = format!("{}:{}:{}", ha1_hash, nonce, a2_hash);
    let expected_response = format!("{:x}", compute(response_str.as_bytes()));

    if client_response == expected_response {
        info!("Kimlik doğrulama başarılı. Kullanıcı kaydediliyor.");
        let call_id = headers.get("call-id").cloned().unwrap_or_default();
        let mut conn = state.redis.get_multiplexed_async_connection().await?;
        let key = format!("pending_reg:{}", call_id);
        let _: () = conn.del(key).await?;

        let contact_uri = headers.get("contact").cloned().unwrap_or_default();
        let expires = headers.get("expires").and_then(|e| e.parse::<u64>().ok()).unwrap_or(3600);

        if expires > 0 {
            let aor = format!("sip_registration:sip:{}@{}", username, realm);
            redis::set_registration(&state.redis, &aor, &contact_uri, expires).await?;
        }
        
        headers.insert("Contact".to_string(), format!("{};expires={}", contact_uri, expires));
        let response = create_response("200 OK", &via_headers, &headers, None, &state.config, addr);
        sock.send_to(response.as_bytes(), addr).await?;
    } else {
        warn!("Kimlik doğrulama başarısız. Yanlış şifre.");
        let response = create_response("403 Forbidden", &via_headers, &headers, None, &state.config, addr);
        sock.send_to(response.as_bytes(), addr).await?;
    }
    Ok(())
}