// sentiric-sip-signaling-service/src/sip/responses.rs
use crate::config::AppConfig;
use crate::sip::call_context::CallContext;
use rand::Rng;
use std::collections::HashMap;
use std::net::SocketAddr;
use tracing::debug;

pub fn build_180_ringing(context: &CallContext, config: &AppConfig) -> String {
    create_response(
        "180 Ringing",
        &context.via_headers,
        &context.headers,
        None,
        config,
        context.remote_addr,
    )
}

pub fn build_200_ok_with_sdp(
    context: &CallContext,
    rtp_port: u32,
    config: &AppConfig,
) -> String {
    let sdp_body = format!(
        "v=0\r\no=- {0} {0} IN IP4 {1}\r\ns=Sentiric\r\nc=IN IP4 {1}\r\nt=0 0\r\nm=audio {2} RTP/AVP 0\r\na=rtpmap:0 PCMU/8000\r\n",
        rand::thread_rng().gen::<u32>(),
        &config.media_service_public_ip,
        rtp_port
    );
    create_response("200 OK", &context.via_headers, &context.headers, Some(&sdp_body), config, context.remote_addr)
}

// --- YENİ VE DAHA ESNEK FONKSİYON İMZASI ---
pub fn create_response(
    status_line: &str,
    via_headers: &[String],
    headers: &HashMap<String, String>,
    sdp: Option<&str>,
    config: &AppConfig,
    remote_addr: SocketAddr, // `remote_addr` artık doğrudan alınıyor
) -> String {
    let body = sdp.unwrap_or("");
    
    // Argüman olarak verilen Via başlıklarını birleştir
    let via_lines = via_headers.join("\r\n");

    let contact_header = format!("<sip:{}@{}>", "sentiric-signal", config.sip_listen_addr);
    let server_header = format!("Server: Sentiric Signaling Service v{}", config.service_version);
    
    // Gerekli başlıkları `headers` map'inden al
    let from = headers.get("from").cloned().unwrap_or_default();
    let to = headers.get("to").cloned().unwrap_or_default();
    let call_id = headers.get("call-id").cloned().unwrap_or_default();
    let cseq = headers.get("cseq").cloned().unwrap_or_default();
    let www_authenticate_line = headers.get("www-authenticate").map(|val| format!("WWW-Authenticate: {}\r\n", val)).unwrap_or_default();


    let response_string = format!(
        "SIP/2.0 {}\r\n{}\r\nFrom: {}\r\nTo: {}\r\nCall-ID: {}\r\nCSeq: {}\r\n{}{}\r\n{}\r\nContent-Length: {}\r\n{}\r\n{}",
        status_line, 
        via_lines,
        from, 
        to,
        call_id,
        cseq,
        www_authenticate_line, 
        format!("Contact: {}", contact_header),
        server_header, 
        body.len(),
        if sdp.is_some() { "Content-Type: application/sdp\r\n" } else { "" },
        body
    );

    debug!(response_to = %remote_addr, response_body = %response_string.replace("\r\n", "\\r\\n"), "SIP yanıtı gönderiliyor.");
    response_string
}