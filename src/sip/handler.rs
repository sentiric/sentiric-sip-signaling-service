// File: src/sip/handler.rs (TAM VE DÜZELTİLMİŞ NİHAİ HALİ)

use super::{bye::handle_bye, invite::handle_invite, register::handle_register};
use crate::config::AppConfig;
use crate::state::ActiveCalls;
use lapin::Channel as LapinChannel;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{debug, error, info, instrument};

use crate::redis;

#[instrument(skip_all, fields(remote_addr = %addr, call_id, trace_id))]
pub async fn handle_sip_request(
    request_bytes: Vec<u8>,
    sock: Arc<UdpSocket>,
    addr: SocketAddr,
    config: Arc<AppConfig>,
    rabbit_channel: Arc<LapinChannel>,
    active_calls: ActiveCalls,
    redis_client: Arc<redis::Client>,
) {
    let request_str = match std::str::from_utf8(&request_bytes) {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "Geçersiz UTF-8 dizisi alındı.");
            return;
        }
    };

    debug!(
        request_from = %addr,
        request_body = %request_str.replace("\r\n", "\\r\\n"),
        "Gelen ham SIP isteği."
    );

    let result = if request_str.starts_with("REGISTER") {
        info!("REGISTER isteği işleniyor...");
        handle_register(request_str, sock, addr, config, redis_client).await
    } else if request_str.starts_with("INVITE") {
        info!("INVITE isteği işleniyor...");
        handle_invite(request_str, sock, addr, config, rabbit_channel, active_calls, redis_client).await
    } else if request_str.starts_with("BYE") {
        info!("BYE isteği işleniyor...");
        // --- DÜZELTME BURADA: Fazladan `rabbit_channel` argümanı kaldırıldı ---
        handle_bye(request_str, sock, addr, config, active_calls).await
    } else if request_str.starts_with("ACK") {
        debug!("ACK isteği alındı, görmezden geliniyor.");
        Ok(())
    } else {
        debug!(
            method = &request_str[..request_str.find(' ').unwrap_or(10)],
            "Desteklenmeyen veya ilgisiz SIP metodu, görmezden geliniyor."
        );
        Ok(())
    };

    if let Err(e) = result {
        error!(error = %e, "SIP isteği işlenirken hata oluştu.");
    }
}