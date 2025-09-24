// sentiric-sip-signaling-service/src/grpc/service.rs
use crate::app_state::AppState;
use crate::sip::utils::create_bye_request;
use sentiric_contracts::sentiric::sip::v1::{
    sip_signaling_service_server::SipSignalingService, TerminateCallRequest, TerminateCallResponse,
};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tonic::{Request, Response, Status};
use tracing::{info, instrument, warn};

pub struct MySipSignalingService {
    pub app_state: Arc<AppState>,
    pub sock: Arc<UdpSocket>,
}

#[tonic::async_trait]
impl SipSignalingService for MySipSignalingService {
    #[instrument(skip(self), fields(call_id = %request.get_ref().call_id))]
    async fn terminate_call(
        &self,
        request: Request<TerminateCallRequest>,
    ) -> Result<Response<TerminateCallResponse>, Status> {
        let req = request.into_inner();
        info!("gRPC üzerinden çağrı sonlandırma isteği alındı.");

        let mut active_calls = self.app_state.active_calls.lock().await;
        if let Some(call_info) = active_calls.remove(&req.call_id) {
            let bye_request = create_bye_request(&call_info, &self.app_state.config);

            if let Err(e) = self.sock.send_to(bye_request.as_bytes(), call_info.remote_addr).await {
                warn!(error = %e, "gRPC TerminateCall: BYE paketi gönderilemedi.");
            } else {
                info!("gRPC TerminateCall: BYE paketi başarıyla gönderildi.");
            }
            
            // TODO: call.ended olayını burada yayınla. BYE'a gelen 200 OK yanıtını beklemek daha doğru olur
            // ancak bu, state yönetimini karmaşıklaştırır. Şimdilik BYE gönderimini başarılı kabul ediyoruz.
            
            Ok(Response::new(TerminateCallResponse {
                success: true,
                message: "Termination signal sent.".to_string(),
            }))
        } else {
            warn!("Sonlandırılmak istenen çağrı aktif değil veya zaten sonlandırılmış.");
            Err(Status::not_found(format!(
                "Aktif çağrı bulunamadı: {}",
                req.call_id
            )))
        }
    }
}