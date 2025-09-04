// File: src/sip/orchestrator.rs
use crate::config::AppConfig;
use crate::error::ServiceError;
use crate::grpc::client::create_secure_grpc_channel;
use crate::rabbitmq::connection::RABBITMQ_EXCHANGE_NAME;
use crate::sip::call_context::CallContext;
use crate::sip::utils::extract_sdp_media_info_from_body;
use crate::state::{ActiveCallInfo, ActiveCalls};
use lapin::{options::*, BasicProperties, Channel as LapinChannel};
use sentiric_contracts::sentiric::{
    dialplan::v1::{dialplan_service_client::DialplanServiceClient, ResolveDialplanRequest, ResolveDialplanResponse},
    media::v1::{media_service_client::MediaServiceClient, AllocatePortRequest},
};
use std::collections::HashMap;
use std::sync::Arc;
use tonic::Request as TonicRequest;
use tracing::{info, instrument};

pub struct CallOrchestrator {
    config: Arc<AppConfig>,
    rabbit_channel: Arc<LapinChannel>,
    active_calls: ActiveCalls,
}

impl CallOrchestrator {
    pub fn new(
        config: Arc<AppConfig>,
        rabbit_channel: Arc<LapinChannel>,
        active_calls: ActiveCalls,
    ) -> Self {
        Self { config, rabbit_channel, active_calls }
    }

    #[instrument(skip(self, context), fields(trace_id = %context.trace_id))]
    pub async fn setup_call(
        &self,
        context: &CallContext,
    ) -> Result<(u32, ResolveDialplanResponse), ServiceError> {
        let dialplan_response = self.resolve_dialplan(context).await?;
        info!("Dialplan başarıyla çözüldü.");
        let rtp_port = self.allocate_media_port(context).await?;
        info!(rtp_port, "Medya portu başarıyla ayrıldı.");
        Ok((rtp_port, dialplan_response))
    }

    #[instrument(skip(self, context, response_headers), fields(trace_id = %context.trace_id))]
    pub async fn finalize_call_setup(
        &self,
        context: CallContext,
        rtp_port: u32,
        dialplan_res: ResolveDialplanResponse,
        response_headers: HashMap<String, String>,
    ) -> Result<(), ServiceError> {
        let to_tag_full = response_headers.get("To").cloned().unwrap_or_default();
        let to_tag = to_tag_full.split(";tag=").nth(1).unwrap_or("").to_string();

        let call_info = ActiveCallInfo {
            remote_addr: context.remote_addr,
            rtp_port,
            trace_id: context.trace_id,
            to_tag,
            created_at: std::time::Instant::now(),
            headers: response_headers,
            call_id: context.call_id,
            from_header: context.from_header,
            to_header: context.to_header,
            contact_header: context.contact_header,
            record_route_header: context.record_route_header,
            raw_body: context.raw_body, // <-- YENİ EKLENDİ
        };
        
        self.active_calls.lock().await.insert(call_info.call_id.clone(), call_info.clone());
        info!("Aktif çağrı durumu başarıyla kaydedildi.");

        self.publish_call_event("call.started", &call_info, Some(&dialplan_res)).await?;
        info!("'call.started' olayı yayınlandı.");

        self.publish_call_event("call.answered", &call_info, None).await?;
        info!("'call.answered' olayı yayınlandı.");

        Ok(())
    }

    #[instrument(skip(self, context))]
    async fn resolve_dialplan(&self, context: &CallContext) -> Result<ResolveDialplanResponse, ServiceError> {
        let mut dialplan_req = TonicRequest::new(ResolveDialplanRequest {
            caller_contact_value: context.caller_id.clone(),
            destination_number: context.destination_number.clone(),
        });
        dialplan_req.metadata_mut().insert("x-trace-id", context.trace_id.parse()?);
        let dialplan_channel = create_secure_grpc_channel(&self.config.dialplan_service_url, "dialplan-service").await?;
        let dialplan_res = DialplanServiceClient::new(dialplan_channel).resolve_dialplan(dialplan_req).await?.into_inner();
        Ok(dialplan_res)
    }

    #[instrument(skip(self, context))]
    async fn allocate_media_port(&self, context: &CallContext) -> Result<u32, ServiceError> {
        let mut media_req = TonicRequest::new(AllocatePortRequest { call_id: context.call_id.clone() });
        media_req.metadata_mut().insert("x-trace-id", context.trace_id.parse()?);
        let media_channel = create_secure_grpc_channel(&self.config.media_service_url, "media-service").await?;
        let rtp_port = MediaServiceClient::new(media_channel).allocate_port(media_req).await?.into_inner().rtp_port;
        Ok(rtp_port)
    }

    #[instrument(skip(self, call_info, dialplan_res))]
    async fn publish_call_event(&self, event_type: &str, call_info: &ActiveCallInfo, dialplan_res: Option<&ResolveDialplanResponse>) -> Result<(), ServiceError> {
        let sdp_info = extract_sdp_media_info_from_body(&call_info.raw_body);

        let mut event_payload = serde_json::json!({
            "eventType": event_type, "traceId": &call_info.trace_id, "callId": &call_info.call_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        if event_type == "call.started" {
            event_payload["from"] = serde_json::Value::String(call_info.from_header.clone());
            event_payload["to"] = serde_json::Value::String(call_info.to_header.clone());
            event_payload["media"] = serde_json::json!({ "server_rtp_port": call_info.rtp_port, "caller_rtp_addr": sdp_info });
            if let Some(res) = dialplan_res { event_payload["dialplan"] = serde_json::to_value(res)?; }
        }
        self.rabbit_channel.basic_publish(RABBITMQ_EXCHANGE_NAME, event_type, BasicPublishOptions::default(), event_payload.to_string().as_bytes(), BasicProperties::default().with_delivery_mode(2)).await?.await?;
        Ok(())
    }
}