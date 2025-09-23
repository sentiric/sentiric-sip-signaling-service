// File: src/sip/responses.rs
use crate::config::AppConfig;
use rand::Rng;
use std::collections::HashMap;
use std::net::SocketAddr;
use tracing::debug;

pub fn build_180_ringing(
    response_headers: &HashMap<String, String>,
    config: &AppConfig,
    remote_addr: SocketAddr,
) -> String {
    create_response("180 Ringing", response_headers, None, config, remote_addr)
}

pub fn build_200_ok_with_sdp(
    response_headers: &HashMap<String, String>,
    rtp_port: u32,
    config: &AppConfig,
    remote_addr: SocketAddr,
) -> String {
    let sdp_body = format!(
        "v=0\r\no=- {0} {0} IN IP4 {1}\r\ns=Sentiric\r\nc=IN IP4 {1}\r\nt=0 0\r\nm=audio {2} RTP/AVP 0\r\na=rtpmap:0 PCMU/8000\r\n",
        rand::thread_rng().gen::<u32>(),
        &config.media_service_public_ip,
        rtp_port
    );
    create_response("200 OK", response_headers, Some(&sdp_body), config, remote_addr)
}

pub fn create_response(
    status_line: &str,
    headers: &HashMap<String, String>,
    sdp: Option<&str>,
    config: &AppConfig,
    remote_addr: SocketAddr,
) -> String {
    let body = sdp.unwrap_or("");
    let empty_string = String::new();

    // DÜZELTME: HashMap'teki anahtarlar küçük harfli olduğu için, get() metodunu küçük harfli anahtarlarla çağırıyoruz.
    let mut via = headers.get("via").cloned().unwrap_or_default();
    if via.contains(";rport") && !via.contains(";received=") {
        via = format!("{};received={}", via, remote_addr.ip());
    }
    
    // NOT: Yanıt oluştururken SIP başlıklarının standart büyük harfli formatını koruyoruz ("Via:", "From:" vb.).
    let via_line = format!("Via: {}\r\n", via);
    let from_header = headers.get("from").unwrap_or(&empty_string);
    let to_header = headers.get("to").unwrap_or(&empty_string);
    
    let contact_header = format!("<sip:{}@{}>", "sentiric-signal", config.sip_listen_addr);

    let www_authenticate_line = headers.get("www-authenticate").map(|val| format!("WWW-Authenticate: {}\r\n", val)).unwrap_or_default();
    
    let server_header = format!("Server: Sentiric Signaling Service v{}", config.service_version);

    let response_string = format!(
        "SIP/2.0 {}\r\n{}\
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
        via_line, 
        from_header, 
        to_header,
        headers.get("call-id").unwrap_or(&empty_string),
        headers.get("cseq").unwrap_or(&empty_string),
        www_authenticate_line, 
        contact_header, 
        server_header, 
        body.len(),
        if sdp.is_some() { "Content-Type: application/sdp\r\n" } else { "" },
        body
    );

    debug!(response_to = %remote_addr, response_body = %response_string.replace("\r\n", "\\r\\n"), "SIP yanıtı gönderiliyor.");
    response_string
}