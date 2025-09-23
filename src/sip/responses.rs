// sentiric-sip-signaling-service/src/sip/responses.rs
use crate::config::AppConfig;
use crate::sip::call_context::CallContext; // CallContext'i import et
use rand::Rng;
use std::collections::HashMap;
use std::net::SocketAddr;
use tracing::debug;

// DÜZELTME: Fonksiyonlar artık HashMap yerine CallContext alıyor.
pub fn build_180_ringing(context: &CallContext, config: &AppConfig) -> String {
    create_response("180 Ringing", context, None, config)
}

pub fn build_200_ok_with_sdp(context: &CallContext, rtp_port: u32, config: &AppConfig) -> String {
    let sdp_body = format!(
        "v=0\r\no=- {0} {0} IN IP4 {1}\r\ns=Sentiric\r\nc=IN IP4 {1}\r\nt=0 0\r\nm=audio {2} RTP/AVP 0\r\na=rtpmap:0 PCMU/8000\r\n",
        rand::thread_rng().gen::<u32>(),
        &config.media_service_public_ip,
        rtp_port
    );
    create_response("200 OK", context, Some(&sdp_body), config)
}

pub fn create_response(
    status_line: &str,
    context: &CallContext,
    sdp: Option<&str>,
    config: &AppConfig,
) -> String {
    let body = sdp.unwrap_or("");
    
    // DÜZELTME: Ayrı bir vektörde tutulan tüm Via başlıklarını birleştiriyoruz.
    let via_lines = context.via_headers.join("\r\n");

    let contact_header = format!("<sip:{}@{}>", "sentiric-signal", config.sip_listen_addr);
    let www_authenticate_line = context.headers.get("www-authenticate").map(|val| format!("WWW-Authenticate: {}\r\n", val)).unwrap_or_default();
    let server_header = format!("Server: Sentiric Signaling Service v{}", config.service_version);

    let response_string = format!(
        "SIP/2.0 {}\r\n{}\r\n\
        From: {}\r\n\
        To: {}\r\n\
        Call-ID: {}\r\n\
        CSeq: {}\r\n\
        {}\
        Contact: {}\r\n\
        {}\r\n\
        Content-Length: {}\r\n\
        {}\r\n\
        {}",
        status_line, 
        via_lines, // Tüm Via başlıkları burada
        context.from_header, 
        context.to_header,
        context.call_id,
        context.headers.get("cseq").cloned().unwrap_or_default(),
        www_authenticate_line, 
        contact_header, 
        server_header, 
        body.len(),
        if sdp.is_some() { "Content-Type: application/sdp\r\n" } else { "" },
        body
    );

    debug!(response_to = %context.remote_addr, response_body = %response_string.replace("\r\n", "\\r\\n"), "SIP yanıtı gönderiliyor.");
    response_string
}