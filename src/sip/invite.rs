// File: src/sip/invite.rs
use super::call_context::CallContext;
use super::orchestrator::CallOrchestrator;
use super::utils::create_response;
use crate::config::AppConfig;
use crate::error::ServiceError;
use crate::redis::{self, AsyncCommands};
use crate::state::ActiveCalls;
use lapin::Channel as LapinChannel;
use rand::distributions::{Alphanumeric, DistString};
use rand::Rng;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::time::sleep;
use tracing::{error, instrument, warn, Span}; // `debug` ve `info` kaldırıldı

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
    let trace_id = format!("trace-{}", Alphanumeric.sample_string(&mut rand::thread_rng(), 12));
    // Hata durumunda hemen çıkmak için `?` kullanıyoruz.
    let context = CallContext::from_request(request_str, addr, trace_id)?;

    Span::current().record("call_id", &context.call_id as &str);
    Span::current().record("trace_id", &context.trace_id as &str);
    Span::current().record("caller", &context.caller_id as &str);
    Span::current().record("destination", &context.destination_number as &str);

    if check_and_handle_duplicate(&context.call_id, &redis_client).await? {
        return Ok(());
    }
    
    sock.send_to(create_response("100 Trying", &context.headers, None, &config, addr).as_bytes(), addr).await?;

    let orchestrator = CallOrchestrator::new(config.clone(), rabbit_channel, active_calls);

    match orchestrator.setup_call(&context).await {
        Ok((rtp_port, dialplan_res)) => {
            let mut response_headers = context.headers.clone();
            let to_tag: u32 = rand::thread_rng().gen();
            response_headers.entry("To".to_string()).and_modify(|v| *v = format!("{};tag={}", v, to_tag));
            
            let sdp_body = format!(
                "v=0\r\no=- {0} {0} IN IP4 {1}\r\ns=Sentiric\r\nc=IN IP4 {1}\r\nt=0 0\r\nm=audio {2} RTP/AVP 0\r\na=rtpmap:0 PCMU/8000\r\n",
                rand::thread_rng().gen::<u32>(), config.sip_public_ip, rtp_port
            );

            sock.send_to(create_response("180 Ringing", &response_headers, None, &config, addr).as_bytes(), addr).await?;
            sleep(std::time::Duration::from_millis(100)).await;

            let ok_response = create_response("200 OK", &response_headers, Some(&sdp_body), &config, addr);
            sock.send_to(ok_response.as_bytes(), addr).await?;
            
            orchestrator.finalize_call_setup(context, rtp_port, dialplan_res, response_headers).await?;
        }
        Err(e) => {
            error!(error = %e, "Çağrı kurulumu orkestrasyonu başarısız oldu.");
            let error_response = create_response("503 Service Unavailable", &context.headers, None, &config, addr);
            sock.send_to(error_response.as_bytes(), addr).await?;
        }
    }

    Ok(())
}

#[instrument(skip(redis_client))]
async fn check_and_handle_duplicate(call_id: &str, redis_client: &Arc<redis::Client>) -> Result<bool, ServiceError> {
    let mut conn = redis_client.get_multiplexed_async_connection().await?;
    let invite_lock_key = format!("processed_invites:{}", call_id);
    
    let is_first_invite: bool = conn.set_nx(&invite_lock_key, true).await?;
    if !is_first_invite {
        warn!("Yinelenen INVITE isteği alındı (Redis atomik kilit), görmezden geliniyor.");
        return Ok(true);
    }
    
    let _: () = conn.expire(&invite_lock_key, 30).await?;
    Ok(false)
}