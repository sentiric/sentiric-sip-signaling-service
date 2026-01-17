// sentiric-sip-signaling-service/src/sip/responses.rs
use crate::config::AppConfig;
use crate::sip::call_context::CallContext;
use rand::Rng;
use std::collections::HashMap;
use std::net::SocketAddr;
// use tracing::debug; // <-- KALDIRILDI

// 180 Ringing
pub fn build_180_ringing(
    headers: &HashMap<String, String>,
    via_headers: &[String],
    config: &AppConfig,
    remote_addr: SocketAddr,
) -> String {
    create_response_from_parts("180 Ringing", headers, via_headers, None, config, remote_addr)
}

// 200 OK + SDP
pub fn build_200_ok_with_sdp(
    headers: &HashMap<String, String>,
    via_headers: &[String],
    rtp_port: u32,
    config: &AppConfig,
    remote_addr: SocketAddr,
) -> String {
    let sdp_body = format!(
        "v=0\r\n\
        o=- {0} {0} IN IP4 {1}\r\n\
        s=Sentiric\r\n\
        c=IN IP4 {1}\r\n\
        t=0 0\r\n\
        m=audio {2} RTP/AVP 0 8 18 101\r\n\
        a=rtpmap:0 PCMU/8000\r\n\
        a=rtpmap:8 PCMA/8000\r\n\
        a=rtpmap:18 G729/8000\r\n\
        a=rtpmap:101 telephone-event/8000\r\n\
        a=fmtp:101 0-16\r\n\
        a=sendrecv\r\n",
        rand::thread_rng().gen::<u32>(),
        &config.sip_public_ip, 
        rtp_port
    );
    
    create_response_from_parts("200 OK", headers, via_headers, Some(&sdp_body), config, remote_addr)
}

pub fn create_response(
    status_line: &str,
    context: &CallContext,
    body: Option<&str>,
    config: &AppConfig,
) -> String {
    create_response_from_parts(status_line, &context.headers, &context.via_headers, body, config, context.remote_addr)
}

pub fn create_response_from_parts(
    status_line: &str,
    headers: &HashMap<String, String>,
    via_headers: &[String],
    body: Option<&str>,
    config: &AppConfig,
    remote_addr: SocketAddr,
) -> String {
    let response_body = body.unwrap_or("");
    let empty_string = String::new();
    
    let mut via_lines_vec = Vec::new();
    for via in via_headers {
        let mut temp_via = via.clone();
        if !temp_via.contains(";received=") {
             temp_via = format!("{};received={}", temp_via, remote_addr.ip());
        }
        via_lines_vec.push(temp_via);
    }
    let via_lines = via_lines_vec.join("\r\n");

    let contact_header = format!("<sip:sentiric@{}:{}>", config.sip_public_ip, config.sip_listen_addr.port());

    let server_header = format!("Server: Sentiric Signaling v{}", config.service_version);
    let content_type = if body.is_some() { "Content-Type: application/sdp\r\n" } else { "" };
    let www_auth = headers.get("www-authenticate").map(|v| format!("WWW-Authenticate: {}\r\n", v)).unwrap_or_default();

    // Debug yerine info kullanabiliriz veya hiç loglamayabiliriz. 
    // Performans için şimdilik loglamayı kaldırıyorum.
    // debug!(response_to = %remote_addr, "SIP yanıtı oluşturuluyor.");

    format!(
        "SIP/2.0 {}\r\n\
        {}\r\n\
        From: {}\r\n\
        To: {}\r\n\
        Call-ID: {}\r\n\
        CSeq: {}\r\n\
        {}\
        Contact: {}\r\n\
        {}\
        {}\
        Content-Length: {}\r\n\
        \r\n\
        {}",
        status_line,
        via_lines,
        headers.get("from").unwrap_or(&empty_string),
        headers.get("to").unwrap_or(&empty_string),
        headers.get("call-id").unwrap_or(&empty_string),
        headers.get("cseq").unwrap_or(&empty_string),
        www_auth,
        contact_header,
        server_header,
        content_type,
        response_body.len(),
        response_body
    )
}