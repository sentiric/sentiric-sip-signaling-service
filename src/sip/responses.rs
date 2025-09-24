// src/sip/responses.rs
use crate::config::AppConfig;
use crate::sip::call_context::CallContext;
use rand::Rng;
use std::collections::HashMap;
use std::net::SocketAddr;
use tracing::debug;

pub fn build_180_ringing(
    headers: &HashMap<String, String>,
    via_headers: &[String],
    config: &AppConfig,
    remote_addr: SocketAddr,
) -> String {
    create_response_from_parts("180 Ringing", headers, via_headers, None, config, remote_addr)
}

pub fn build_200_ok_with_sdp(
    headers: &HashMap<String, String>,
    via_headers: &[String],
    rtp_port: u32,
    config: &AppConfig,
    remote_addr: SocketAddr,
) -> String {
    // Görevimiz sadece SDP'yi doğru public IP ile oluşturmak.
    // Geri kalan işi create_response_from_parts halledecek.
    let sdp_body = format!(
        "v=0\r\no=- {0} {0} IN IP4 {1}\r\ns=Sentiric\r\nc=IN IP4 {1}\r\nt=0 0\r\nm=audio {2} RTP/AVP 0\r\na=rtpmap:0 PCMU/8000\r\n",
        rand::thread_rng().gen::<u32>(),
        &config.sip_public_ip, // Doğrudan doğru public IP'yi kullanıyoruz.
        rtp_port
    );
    create_response_from_parts("200 OK", headers, via_headers, Some(&sdp_body), config, remote_addr)
}

pub fn create_response(
    status_line: &str,
    context: &CallContext,
    sdp: Option<&str>,
    config: &AppConfig,
) -> String {
    create_response_from_parts(status_line, &context.headers, &context.via_headers, sdp, config, context.remote_addr)
}

// BU FONKSİYON ARTIK TEK DOĞRULUK KAYNAĞI
pub fn create_response_from_parts(
    status_line: &str,
    headers: &HashMap<String, String>,
    via_headers: &[String],
    body: Option<&str>, // Adını sdp'den 'body'ye değiştirdik, daha genel.
    config: &AppConfig,
    remote_addr: SocketAddr,
) -> String {
    let response_body = body.unwrap_or("");
    let empty_string = String::new();
    
    let mut via_lines_vec = Vec::new();
    for via in via_headers {
        let mut temp_via = via.clone();
        if temp_via.contains(";rport") && !temp_via.contains(";received=") {
            temp_via = format!("{};received={}", temp_via, remote_addr.ip());
        }
        via_lines_vec.push(temp_via);
    }
    let via_lines = via_lines_vec.join("\r\n");

    // Contact başlığını config'den okuduğumuz public IP ve port ile oluşturuyoruz.
    let contact_header = format!("<sip:sentiric-signal@{}:{}>", config.sip_public_ip, config.sip_listen_addr.port());

    let www_authenticate_line = headers.get("www-authenticate").map(|val| format!("WWW-Authenticate: {}\r\n", val)).unwrap_or_default();
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
        via_lines,
        headers.get("from").unwrap_or(&empty_string),
        headers.get("to").unwrap_or(&empty_string),
        headers.get("call-id").unwrap_or(&empty_string),
        headers.get("cseq").unwrap_or(&empty_string),
        www_authenticate_line, 
        contact_header, 
        server_header, 
        response_body.len(),
        if body.is_some() { "Content-Type: application/sdp\r\n" } else { "" },
        response_body
    );

    debug!(response_to = %remote_addr, response_body = %response_string.replace("\r\n", "\\r\\n"), "SIP yanıtı gönderiliyor.");
    response_string
}