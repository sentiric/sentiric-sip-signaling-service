// File: sentiric-sip-signaling-service/src/sip/handler.rs

use super::{bye::handle_bye, invite::handle_invite, register::handle_register};
use crate::config::AppConfig;
use crate::state::ActiveCalls; // <-- AÇIKLAMA: Registrations import'u kaldırıldı.
use lapin::Channel as LapinChannel;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{debug, error, info, instrument};

#[instrument(skip_all, fields(remote_addr = %addr, call_id, trace_id))]
pub async fn handle_sip_request(
    request_bytes: Vec<u8>,
    sock: Arc<UdpSocket>,
    addr: SocketAddr,
    config: Arc<AppConfig>,
    rabbit_channel: Arc<LapinChannel>,
    active_calls: ActiveCalls,
    redis_client: Arc<redis::Client>, // <-- AÇIKLAMA: Artık Registrations yerine redis::Client alıyor.
) {
    let request_str = match std::str::from_utf8(&request_bytes) {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "Geçersiz UTF-8 dizisi alındı.");
            return;
        }
    };

    let result = if request_str.starts_with("INVITE") {
        handle_invite(request_str, sock, addr, config, rabbit_channel, active_calls).await
    } else if request_str.starts_with("REGISTER") {
        // AÇIKLAMA: handle_register'a redis_client'i paslıyoruz.
        handle_register(request_str, sock, addr, config, redis_client).await
    } else if request_str.starts_with("BYE") {
        handle_bye(request_str, sock, addr, config, rabbit_channel, active_calls).await
    } else if request_str.starts_with("ACK") {
        info!("ACK isteği alındı, SIP diyaloğu başarıyla kuruldu.");
        Ok(())
    } else {
        debug!(
            request_preview = &request_str[..20.min(request_str.len())],
            "Desteklenmeyen SIP metodu, görmezden geliniyor."
        );
        Ok(())
    };

    if let Err(e) = result {
        error!(error = %e, "SIP isteği işlenirken hata oluştu.");
    }
}