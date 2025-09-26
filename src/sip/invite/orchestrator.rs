// sentiric-sip-signaling-service/src/sip/invite/orchestrator.rs
use crate::app_state::AppState;
use crate::error::ServiceError;
use crate::rabbitmq::connection::RABBITMQ_EXCHANGE_NAME;
use crate::sip::call_context::CallContext;
use crate::sip::utils::extract_sdp_media_info_from_body;
use crate::state::ActiveCallInfo;
use lapin::{options::*, BasicProperties, Channel as LapinChannel};
use prost_types::Timestamp;
use rand::Rng;
// ==================== DÜZELTİLMİŞ IMPORT'LAR ====================
use sentiric_contracts::sentiric::{
    dialplan::v1::{ResolveDialplanRequest, ResolveDialplanResponse},
    // Hatalı `call_started_event::` kısmı kaldırıldı.
    event::v1::{CallStartedEvent, MediaInfo},
    media::v1::AllocatePortRequest,
};
// ==================== IMPORT SONU ====================
use std::sync::Arc;
use std::time::SystemTime;
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
    
    let event_payload_str = if event_type == "call.started" && dialplan_res.is_some() {
        let event = CallStartedEvent {
            event_type: event_type.to_string(),
            trace_id: call_info.trace_id.clone(),
            call_id: call_info.call_id.clone(),
            from_uri: call_info.from_header.clone(),
            to_uri: call_info.to_header.clone(),
            timestamp: Some(Timestamp::from(SystemTime::now())),
            dialplan_resolution: dialplan_res.cloned(),
            media_info: Some(MediaInfo {
                caller_rtp_addr: sdp_info,
                server_rtp_port: call_info.rtp_port,
            }),
        };
        serde_json::to_string(&event)?
    } else {
        serde_json::to_string(&serde_json::json!({
            "eventType": event_type,
            "traceId": &call_info.trace_id,
            "callId": &call_info.call_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }))?
    };
    
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