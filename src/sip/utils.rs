// File: src/sip/utils.rs (TAM VE GÜNCELLENMİŞ HALİ)

use crate::config::AppConfig;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::net::SocketAddr;
// DÜZELTME 1: `debug` makrosunu import ediyoruz.
use tracing::{debug, info, warn};
use rand::Rng;

// --- EKSİK OLAN KISIM BURASIYDI ---
static USER_EXTRACT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"sip:\+?(\d+)@").unwrap());
// --- DÜZELTME SONU ---

pub fn parse_complex_headers(request: &str) -> Option<HashMap<String, String>> {
    let mut headers = HashMap::new();
    let mut via_headers = Vec::new();
    let mut record_route_headers = Vec::new();

    for line in request.lines() {
        if line.is_empty() {
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            let key_trimmed = key.trim();
            let value_trimmed = value.trim().to_string();
            match key_trimmed.to_lowercase().as_str() {
                "via" | "v" => via_headers.push(value_trimmed),
                "record-route" => record_route_headers.push(value_trimmed),
                _ => {
                    headers.insert(key_trimmed.to_string(), value_trimmed);
                }
            }
        }
    }

    if !via_headers.is_empty() {
        headers.insert("Via".to_string(), via_headers.join(", "));
        if !record_route_headers.is_empty() {
            headers.insert("Record-Route".to_string(), record_route_headers.join(", "));
        }
        Some(headers)
    } else {
        warn!("Gelen SIP isteğinde Via başlığı bulunamadı.");
        None
    }
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
    
    let mut via = headers.get("Via").cloned().unwrap_or_default();
    if via.contains(";rport") && !via.contains(";received=") {
        via = format!("{};received={}", via, remote_addr.ip());
    }
    let via_line = format!("Via: {}\r\n", via);

    let from_header = headers.get("From").unwrap_or(&empty_string);
    let mut to_header = headers.get("To").unwrap_or(&empty_string).clone();

    if !to_header.contains(";tag=") && status_line != "100 Trying" {
        let to_tag = format!(";tag={}", rand::thread_rng().gen::<u32>());
        to_header.push_str(&to_tag);
    }

    let contact_header = format!(
        "<sip:{}@{}:{}>",
        "sentiric-signal",
        config.sip_public_ip,
        config.sip_listen_addr.port()
    );

    let www_authenticate_line = headers.get("WWW-Authenticate")
        .map(|val| format!("WWW-Authenticate: {}\r\n", val))
        .unwrap_or_default();

    let response_string = format!(
        "SIP/2.0 {}\r\n{}\
        From: {}\r\n\
        To: {}\r\n\
        Call-ID: {}\r\n\
        CSeq: {}\r\n\
        {}\
        Contact: {}\r\n\
        Server: Sentiric Signaling Service v1.0\r\n\
        Content-Length: {}\r\n\
        {}\r\n\
        {}",
        status_line,
        via_line,
        from_header,
        to_header,
        headers.get("Call-ID").unwrap_or(&empty_string),
        headers.get("CSeq").unwrap_or(&empty_string),
        www_authenticate_line,
        contact_header,
        body.len(),
        if sdp.is_some() { "Content-Type: application/sdp\r\n" } else { "" },
        body
    );

    debug!(
        response_to = %remote_addr,
        response_body = %response_string.replace("\r\n", "\\r\\n"),
        "SIP yanıtı gönderiliyor."
    );

    response_string
}


pub fn create_bye_request(headers: &HashMap<String, String>) -> String {
    let empty_string = String::new();
    let original_cseq = headers.get("CSeq").unwrap_or(&empty_string);
    let cseq_num = original_cseq
        .split_whitespace()
        .next()
        .unwrap_or("1")
        .parse::<u32>()
        .unwrap_or(1)
        + 1;

    format!(
        "BYE {to_uri} SIP/2.0\r\n\
        Via: {via}\r\n\
        From: {from}\r\n\
        To: {to}\r\n\
        Call-ID: {call_id}\r\n\
        CSeq: {cseq} BYE\r\n\
        Max-Forwards: 70\r\n\
        User-Agent: Sentiric Signaling Service\r\n\
        Content-Length: 0\r\n\r\n",
        to_uri = headers.get("From").unwrap_or(&empty_string).split_whitespace().nth(0).unwrap_or(""),
        via = headers.get("Via").unwrap_or(&empty_string),
        from = headers.get("To").unwrap_or(&empty_string),
        to = headers.get("From").unwrap_or(&empty_string),
        call_id = headers.get("Call-ID").unwrap_or(&empty_string),
        cseq = cseq_num
    )
}

pub fn extract_user_from_uri(uri: &str) -> Option<String> {
    USER_EXTRACT_RE
        .captures(uri)
        .and_then(|caps| caps.get(1))
        .map(|user_part| {
            let original_num = user_part.as_str();
            let mut num: String = original_num.chars().filter(|c| c.is_digit(10)).collect();
            if num.len() == 11 && num.starts_with('0') {
                num = format!("90{}", &num[1..]);
            } else if num.len() == 10 && !num.starts_with("90") {
                num = format!("90{}", num);
            }
            let normalized_num = num;
            if original_num != normalized_num {
                info!(original = %original_num, normalized = %normalized_num, "Telefon numarası normalize edildi.");
            }
            normalized_num
        })
}

pub fn extract_sdp_media_info(sip_request: &str) -> Option<String> {
    let mut ip_addr: Option<&str> = None;
    let mut port: Option<&str> = None;
    if let Some(sdp_part) = sip_request.split("\r\n\r\n").nth(1) {
        for line in sdp_part.lines() {
            if line.starts_with("c=IN IP4 ") {
                ip_addr = line.split_whitespace().nth(2);
            }
            if line.starts_with("m=audio ") {
                port = line.split_whitespace().nth(1);
            }
        }
    }
    if let (Some(ip), Some(p)) = (ip_addr, port) {
        Some(format!("{}:{}", ip, p))
    } else {
        None
    }
}