use super::orchestrator;
use crate::app_state::AppState;
use crate::error::ServiceError;
use crate::redis::AsyncCommands;
use crate::sip::call_context::CallContext;
use crate::sip::responses;
use rand::distributions::{Alphanumeric, DistString};
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{error, instrument, warn, Span};

#[instrument(skip_all, fields(remote_addr = %addr, call_id, trace_id, caller, destination))]
pub async fn handle(
    request_str: &str,
    sock: Arc<UdpSocket>,
    addr: SocketAddr,
    state: Arc<AppState>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let trace_id = format!("trace-{}", Alphanumeric.sample_string(&mut rand::thread_rng(), 12));
    let context = CallContext::from_request(request_str, addr, trace_id)?;

    Span::current().record("call_id", &context.call_id as &str);
    Span::current().record("trace_id", &context.trace_id as &str);
    Span::current().record("caller", &context.caller_id as &str);
    Span::current().record("destination", &context.destination_number as &str);

    if check_and_handle_duplicate(&context.call_id, &state.redis).await? {
        return Ok(());
    }

    sock.send_to(responses::create_response("100 Trying", &context, None, &state.config).as_bytes(), addr).await?;

    match orchestrator::setup_and_finalize_call(&context, state.clone()).await {
        Ok(call_info) => {
            let ringing_response = responses::build_180_ringing(&call_info.headers, &call_info.via_headers, &state.config, call_info.remote_addr);
            sock.send_to(ringing_response.as_bytes(), call_info.remote_addr).await?;
            
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            
            let ok_response = responses::build_200_ok_with_sdp(&call_info.headers, &call_info.via_headers, call_info.rtp_port, &state.config, call_info.remote_addr);
            sock.send_to(ok_response.as_bytes(), call_info.remote_addr).await?;
        }
        Err(e) => {
            error!(error = %e, "Çağrı kurulumu orkestrasyonu başarısız oldu.");
            let error_response = responses::create_response("503 Service Unavailable", &context, None, &state.config);
            sock.send_to(error_response.as_bytes(), addr).await?;
        }
    }
    
    Ok(())
}

#[instrument(skip(redis_client))]
async fn check_and_handle_duplicate(call_id: &str, redis_client: &Arc<crate::redis::Client>) -> Result<bool, ServiceError> {
    let mut conn = redis_client.get_multiplexed_async_connection().await?;
    let invite_lock_key = format!("processed_invites:{}", call_id);
    let is_first_invite: bool = conn.set_nx(&invite_lock_key, true).await?;
    if !is_first_invite {
        warn!("Yinelenen INVITE isteği alındı (Redis atomik kilit), görmezden geliniyor.");
        return Ok(true);
    }
    conn.expire::<_, ()>(&invite_lock_key, 30).await?;
    Ok(false)
}