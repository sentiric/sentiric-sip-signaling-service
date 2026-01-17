// sentiric-sip-signaling-service/src/sip/invite/orchestrator.rs

use crate::app_state::AppState;
use crate::error::ServiceError;
use crate::rabbitmq::connection::RABBITMQ_EXCHANGE_NAME;
use crate::sip::call_context::CallContext;
use crate::sip::utils::extract_sdp_media_info_from_body;
use crate::state::ActiveCallInfo;
use lapin::{options::*, BasicProperties, Channel as LapinChannel};
use rand::Rng;
use sentiric_contracts::sentiric::{
    dialplan::v1::{ResolveDialplanRequest, ResolveDialplanResponse},
    media::v1::{AllocatePortRequest, PlayAudioRequest}, // PlayAudioRequest EKLENDİ
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::Request as TonicRequest;
use tracing::{debug, info, instrument, warn};

#[instrument(skip_all, fields(trace_id = %context.trace_id))]
pub async fn setup_and_finalize_call(
    context: &CallContext,
    state: Arc<AppState>,
) -> Result<ActiveCallInfo, ServiceError> {
    let dialplan_response = resolve_dialplan(context, state.clone()).await?;
    info!(dialplan_id = %dialplan_response.dialplan_id, "Dialplan başarıyla çözüldü.");

    let rtp_port = allocate_media_port(context, state.clone()).await?;
    info!(rtp_port, "Medya portu başarıyla ayrıldı.");

    // --- YENİ EKLENEN KISIM: NAT DELME (HOLE PUNCHING) ---
    // Operatörün SDP'sinden IP ve Portu bul
    if let Some(target_addr) = extract_sdp_media_info_from_body(&context.raw_body) {
        info!(target = %target_addr, "SDP'den hedef RTP adresi bulundu. NAT delme işlemi başlatılıyor...");
        
        // Media Service'e "Bu adrese boş bir ses paketi at" diyoruz.
        // Bu sayede firewall açılacak ve operatör bize ses gönderebilecek.
        // 'data:...' URI'si ile 100ms'lik sessizlik gönderiyoruz.
        // (Base64 encoded 160 byte sessizlik - PCMU)
        // let silence_uri = "data:audio/basic;base64,////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////"; 
        
        // // Media Service'e kısa, tatlı AI bip tonu gönderiyoruz.
        // // 60ms'lik yumuşak "Neural Pulse" (PCMU / 8kHz)
        // // Firewall açılır, ama rahatsız edici bip olmaz.
        // let silence_uri = "data:audio/basic;base64,/////////////////////////////////////////////6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+vr6+v";

        // --- GÜVENLİ NAT DELME PAKETİ (Standart PCMU Sessizlik) ---
        // 160 byte (20ms) 0xFF (PCMU sessizlik)
        // Base64: /wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/w==
        let silence_uri = "data:audio/basic;base64,//8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/wD/AP8A/w==";

        let mut media_client = state.grpc.media.clone();
        let play_req = TonicRequest::new(PlayAudioRequest {
            audio_uri: silence_uri.to_string(),
            server_rtp_port: rtp_port,
            rtp_target_addr: target_addr.clone(),
        });
        
        // Bu işlemi arka planda yap, ana akışı bloklama
        tokio::spawn(async move {
            if let Err(e) = media_client.play_audio(play_req).await {
                warn!("NAT delme (PlayAudio) başarısız oldu: {}", e);
            } else {
                info!("NAT delme paketi gönderildi.");
            }
        });
    } else {
        warn!("SDP içinde geçerli RTP adresi bulunamadı. NAT delme yapılamıyor.");
    }
    // -----------------------------------------------------

    let mut response_headers = context.headers.clone();
    let to_tag: u32 = rand::thread_rng().gen();
    response_headers
        .entry("to".to_string())
        .and_modify(|v| *v = format!("{};tag={}", v, to_tag));

    let call_info = ActiveCallInfo {
        remote_addr: context.remote_addr,
        rtp_port,
        trace_id: context.trace_id.clone(),
        to_tag: to_tag.to_string(),
        created_at: std::time::Instant::now(),
        headers: response_headers.clone(),
        via_headers: context.via_headers.clone(),
        call_id: context.call_id.clone(),
        from_header: context.from_header.clone(),
        to_header: context.to_header.clone(),
        contact_header: context.contact_header.clone(),
        record_route_header: context.record_route_header.clone(),
        raw_body: context.raw_body.clone(),
        answered_event_published: Arc::new(Mutex::new(false)),
    };

    state
        .active_calls
        .lock()
        .await
        .insert(call_info.call_id.clone(), call_info.clone());
    info!("Aktif çağrı durumu başarıyla kaydedildi.");

    if let Some(rabbit_channel) = &state.rabbit {
        publish_call_event("call.started", &call_info, Some(&dialplan_response), rabbit_channel)
            .await?;
    } else {
        warn!("RabbitMQ bağlantısı aktif değil, 'call.started' olayı yayınlanamadı.");
    }

    Ok(call_info)
}

#[instrument(skip(context, state))]
async fn resolve_dialplan(
    context: &CallContext,
    state: Arc<AppState>,
) -> Result<ResolveDialplanResponse, ServiceError> {
    let mut dialplan_client = state.grpc.dialplan.clone();
    let mut dialplan_req = TonicRequest::new(ResolveDialplanRequest {
        caller_contact_value: context.caller_id.clone(),
        destination_number: context.destination_number.clone(),
    });
    dialplan_req.metadata_mut().insert("x-trace-id", context.trace_id.parse()?);
    let dialplan_res = dialplan_client.resolve_dialplan(dialplan_req).await?.into_inner();
    Ok(dialplan_res)
}

#[instrument(skip(context, state))]
async fn allocate_media_port(context: &CallContext, state: Arc<AppState>) -> Result<u32, ServiceError> {
    let mut media_client = state.grpc.media.clone();
    let mut media_req = TonicRequest::new(AllocatePortRequest {
        call_id: context.call_id.clone(),
    });
    media_req.metadata_mut().insert("x-trace-id", context.trace_id.parse()?);
    let rtp_port = media_client.allocate_port(media_req).await?.into_inner().rtp_port;
    Ok(rtp_port)
}

#[instrument(skip(call_info, dialplan_res, rabbit_channel))]
async fn publish_call_event(
    event_type: &str,
    call_info: &ActiveCallInfo,
    dialplan_res: Option<&ResolveDialplanResponse>,
    rabbit_channel: &Arc<LapinChannel>,
) -> Result<(), ServiceError> {
    let sdp_info = extract_sdp_media_info_from_body(&call_info.raw_body).unwrap_or_default();
    
    // --- MANUEL JSON OLUŞTURMA ---
    let mut event_payload = serde_json::json!({
        "eventType": event_type,
        "traceId": &call_info.trace_id,
        "callId": &call_info.call_id,
        "fromUri": &call_info.from_header,
        "toUri": &call_info.to_header,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    if event_type == "call.started" {
        let media_info = serde_json::json!({
            "callerRtpAddr": sdp_info,
            "serverRtpPort": call_info.rtp_port,
        });

        let dialplan_json = if let Some(res) = dialplan_res {
             serde_json::json!({
                 "dialplanId": res.dialplan_id,
                 "tenantId": res.tenant_id,
                 "action": {
                     "action": res.action.as_ref().map(|a| &a.action).unwrap_or(&"".to_string()),
                     "actionData": res.action.as_ref()
                        .and_then(|a| a.action_data.as_ref())
                        .map(|d| &d.data)
                        .unwrap_or(&std::collections::HashMap::new())
                 }
             })
        } else {
             serde_json::Value::Null
        };

        if let serde_json::Value::Object(ref mut map) = event_payload {
            map.insert("mediaInfo".to_string(), media_info);
            map.insert("dialplanResolution".to_string(), dialplan_json);
        }
    }
    
    let event_payload_str = serde_json::to_string(&event_payload)?;
    
    debug!(
        event_payload = %event_payload_str,
        "{} olayı yayınlanıyor (tam içerik).", event_type
    );
    
    info!("'{}' olayı yayınlanıyor.", event_type);

    rabbit_channel.basic_publish(
        RABBITMQ_EXCHANGE_NAME,
        event_type,
        BasicPublishOptions::default(),
        event_payload_str.as_bytes(),
        BasicProperties::default().with_delivery_mode(2).with_content_type("application/json".into()),
    ).await?.await?;
    
    Ok(())
}