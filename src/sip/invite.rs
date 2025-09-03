// File: src/sip/invite.rs (TAM VE DÜZELTİLMİŞ HALİ)

use super::utils::{
    create_response, extract_sdp_media_info, extract_user_from_uri, parse_complex_headers,
};
use crate::config::AppConfig;
use crate::grpc::client::create_secure_grpc_channel;
use crate::rabbitmq::connection::RABBITMQ_EXCHANGE_NAME;
use crate::redis::{self, AsyncCommands}; // AsyncCommands'i import ettiğimizden emin olalım
use crate::state::{ActiveCallInfo, ActiveCalls};
use lapin::{options::*, BasicProperties, Channel as LapinChannel};
use rand::distributions::{Alphanumeric, DistString};
use rand::Rng;
use sentiric_contracts::sentiric::{
    dialplan::v1::{dialplan_service_client::DialplanServiceClient, ResolveDialplanRequest},
    media::v1::{media_service_client::MediaServiceClient, AllocatePortRequest},
};
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::time::sleep;
use tonic::Request as TonicRequest;
use tracing::{debug, error, info, instrument, warn, Span};

#[instrument(skip_all, fields(remote_addr = %addr, call_id, trace_id, caller, destination))]
pub async fn handle_invite(
    request_str: &str,
    sock: Arc<UdpSocket>,
    addr: SocketAddr,
    config: Arc<AppConfig>,
    rabbit_channel: Arc<LapinChannel>,
    active_calls: ActiveCalls,
    redis_client: Arc<redis::Client>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut headers = parse_complex_headers(request_str).ok_or("Geçersiz başlıklar")?;
    let call_id = headers.get("Call-ID").cloned().unwrap_or_default();

    // --- YENİ ve GÜÇLENDİRİLMİŞ YARIŞ DURUMU KONTROLÜ ---
    let mut conn = redis_client.get_multiplexed_async_connection().await?;
    let invite_lock_key = format!("processed_invites:{}", call_id);
    
    // SETNX komutunu kullanarak atomik bir şekilde kilidi ayarlamaya çalış.
    // Başarılı olursa `true` (kilit yeni oluşturuldu), başarısız olursa `false` döner.
    let is_first_invite: bool = conn.set_nx(&invite_lock_key, true).await?;

    if !is_first_invite {
        warn!(call_id = %call_id, "Yinelenen INVITE isteği alındı (Redis atomik kilit), görmezden geliniyor.");
        return Ok(());
    }
    
    // Kilidi 30 saniye sonra otomatik olarak silinmesi için ayarla.
    let _: () = conn.expire(&invite_lock_key, 30).await?;
    // --- YENİ KONTROL SONU ---

    // Hafızadaki aktif çağrı kontrolü hala bir güvenlik katmanı olarak kalabilir.
    if active_calls.lock().await.contains_key(&call_id) {
        warn!(call_id = %call_id, "Yinelenen INVITE isteği (aktif çağrı), görmezden geliniyor.");
        return Ok(());
    }

    let from_uri = headers.get("From").cloned().unwrap_or_default();
    let to_uri = headers.get("To").cloned().unwrap_or_default();
    let caller_id = extract_user_from_uri(&from_uri).unwrap_or_else(|| "unknown".to_string());
    let destination_number =
        extract_user_from_uri(&to_uri).unwrap_or_else(|| "unknown".to_string());

    let trace_id = format!(
        "trace-{}",
        Alphanumeric.sample_string(&mut rand::thread_rng(), 12)
    );
    Span::current().record("call_id", &call_id as &str);
    Span::current().record("trace_id", &trace_id as &str);
    Span::current().record("caller", &caller_id as &str);
    Span::current().record("destination", &destination_number as &str);

    if !destination_number.starts_with("90") {
        info!(destination_user = %destination_number, "Kullanıcıdan kullanıcıya arama tespit edildi. Redis'ten adres sorgulanıyor...");

        let aor = format!("sip_registration:sip:{}@{}", destination_number, config.sip_realm);
        let target_contact_uri: Option<String> = conn.get(aor).await?;

        if let Some(contact_uri) = target_contact_uri {
            info!(contact = %contact_uri, "Hedef kullanıcı bulundu.");
            
            warn!("P2P proxy mantığı henüz implemente edilmedi. Çağrı reddediliyor.");
            sock.send_to(
                create_response("404 Not Found", &headers, None, &config, addr).as_bytes(),
                addr,
            ).await?;
            return Ok(());
        } else {
            warn!(destination_user = %destination_number, "Hedef kullanıcı kayıtlı değil veya çevrimdışı. Çağrı reddediliyor.");
            sock.send_to(
                create_response("404 Not Found", &headers, None, &config, addr).as_bytes(),
                addr,
            ).await?;
            return Ok(());
        }
    }

    debug!("100 Trying yanıtı gönderiliyor...");
    sock.send_to(
        create_response("100 Trying", &headers, None, &config, addr).as_bytes(),
        addr,
    )
    .await?;

    let mut dialplan_req = TonicRequest::new(ResolveDialplanRequest {
        caller_contact_value: caller_id.clone(),
        destination_number: destination_number.clone(),
    });
    dialplan_req
        .metadata_mut()
        .insert("x-trace-id", trace_id.parse()?);

    let dialplan_channel =
        create_secure_grpc_channel(&config.dialplan_service_url, "dialplan-service").await?;
    let dialplan_result = DialplanServiceClient::new(dialplan_channel)
        .resolve_dialplan(dialplan_req)
        .await;

    if let Err(e) = dialplan_result {
        error!(error = %e, "Dialplan'den karar alınamadı.");
        sock.send_to(
            create_response("503 Service Unavailable", &headers, None, &config, addr).as_bytes(),
            addr,
        )
        .await?;
        return Err(e.into());
    }

    let dialplan_res = dialplan_result.unwrap().into_inner();

    let mut media_req = TonicRequest::new(AllocatePortRequest {
        call_id: call_id.clone(),
    });
    media_req
        .metadata_mut()
        .insert("x-trace-id", trace_id.parse()?);

    let media_channel =
        create_secure_grpc_channel(&config.media_service_url, "media-service").await?;
    let rtp_port = match MediaServiceClient::new(media_channel)
        .allocate_port(media_req)
        .await
    {
        Ok(res) => res.into_inner().rtp_port,
        Err(e) => {
            error!(error = %e, "Media service'ten port alınamadı.");
            sock.send_to(
                create_response("503 Service Unavailable", &headers, None, &config, addr).as_bytes(),
                addr,
            )
            .await?;
            return Err(e.into());
        }
    };

    info!(rtp_port, "Medya portu ayrıldı.");
    
    let to_tag = format!(";tag={}", rand::thread_rng().gen::<u32>());
    headers
        .entry("To".to_string())
        .and_modify(|v| *v = format!("{}{}", v, to_tag));
    
    let call_info = ActiveCallInfo {
        remote_addr: addr,
        rtp_port,
        trace_id: trace_id.clone(),
        created_at: std::time::Instant::now(),
        headers: headers.clone(),
    };
    active_calls.lock().await.insert(call_id.clone(), call_info);

    let sdp_body = format!(
        "v=0\r\no=- {0} {0} IN IP4 {1}\r\ns=Sentiric\r\nc=IN IP4 {1}\r\nt=0 0\r\nm=audio {2} RTP/AVP 0\r\na=rtpmap:0 PCMU/8000\r\n",
        rand::thread_rng().gen::<u32>(),
        config.sip_public_ip,
        rtp_port
    );

    debug!("180 Ringing yanıtı gönderiliyor...");
    sock.send_to(
        create_response("180 Ringing", &headers, None, &config, addr).as_bytes(),
        addr,
    )
    .await?;
    sleep(std::time::Duration::from_millis(100)).await;
    let ok_response = create_response("200 OK", &headers, Some(&sdp_body), &config, addr);
    sock.send_to(ok_response.as_bytes(), addr).await?;

    info!("Çağrı başarıyla yanıtlandı (200 OK gönderildi).");
    
    let event_payload = serde_json::json!({
        "eventType": "call.started", "traceId": trace_id, "callId": call_id, "from": from_uri, "to": to_uri,
        "media": { "server_rtp_port": rtp_port, "caller_rtp_addr": extract_sdp_media_info(request_str).unwrap_or_default() },
        "dialplan": dialplan_res, "timestamp": chrono::Utc::now().to_rfc3339()
    });

    rabbit_channel
        .basic_publish(
            RABBITMQ_EXCHANGE_NAME,
            "call.started",
            BasicPublishOptions::default(),
            event_payload.to_string().as_bytes(),
            BasicProperties::default().with_delivery_mode(2),
        )
        .await?
        .await?;
    info!("'call.started' olayı yayınlandı.");


    let answered_event_payload = serde_json::json!({
        "eventType": "call.answered",
        "traceId": trace_id,
        "callId": call_id,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    let _ = rabbit_channel
        .basic_publish(
            RABBITMQ_EXCHANGE_NAME,
            "call.answered",
            BasicPublishOptions::default(),
            answered_event_payload.to_string().as_bytes(),
            BasicProperties::default().with_delivery_mode(2),
        )
        .await?
        .await?;
    info!("'call.answered' olayı yayınlandı.");

    Ok(())
}