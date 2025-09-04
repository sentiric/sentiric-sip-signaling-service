// File: src/sip/utils.rs
use crate::config::AppConfig;
use crate::state::ActiveCallInfo;
use once_cell::sync::Lazy;
use rand::Rng;
use regex::Regex;
use std::collections::HashMap;
use std::net::SocketAddr;
use tracing::{debug, info, warn};

static USER_EXTRACT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"sip:\+?(\d+)@").unwrap());

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
    let to_header = headers.get("To").unwrap_or(&empty_string);

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

// =========================================================================
//   NİHAİ VE DOĞRU `BYE` OLUŞTURMA MANTIĞI
// =========================================================================
pub fn create_bye_request(call_info: &ActiveCallInfo) -> String {
    let cseq_line = call_info.headers.get("CSeq").cloned().unwrap_or_default();
    let cseq_num = cseq_line.split_whitespace().next().unwrap_or("1").parse::<u32>().unwrap_or(1) + 1;

    let mut lines = Vec::new();
    
    // Request-URI, diyaloğun karşı tarafının INVITE'ta belirttiği Contact adresidir.
    lines.push(format!("BYE {} SIP/2.0", call_info.contact_header));

    // Via başlığı, bu isteği oluşturan olarak KENDİ adresimizi (signaling) içermelidir.
    // Gateway'in adresi `call_info.remote_addr`'dir.
    let branch: String = rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(16).map(char::from).collect();
    lines.push(format!("Via: SIP/2.0/UDP {};branch=z9hG4bK.{}", call_info.remote_addr, branch));
    
    lines.push(format!("Max-Forwards: 70"));
    
    // ÖNEMLİ: `sip-signaling` Route başlığı EKLEMEMELİDİR. Bu, gateway'in görevidir.
    // Gateway, bizim gönderdiğimiz bu "temiz" paketi alıp, kendi hafızasındaki
    // düzeltilmiş `Record-Route`'u kullanarak doğru `Route` başlığını ekleyecektir.
    
    // From ve To başlıkları INVITE'takinin tersidir.
    lines.push(format!("From: {};tag={}", call_info.to_header, call_info.to_tag));
    lines.push(format!("To: {}", call_info.from_header));
    
    lines.push(format!("Call-ID: {}", call_info.call_id));
    lines.push(format!("CSeq: {} BYE", cseq_num));
    lines.push(format!("User-Agent: Sentiric Signaling Service"));
    lines.push(format!("Content-Length: 0"));

    lines.join("\r\n") + "\r\n\r\n"
}

pub fn extract_user_from_uri(uri: &str) -> Option<String> {
    USER_EXTRACT_RE.captures(uri).and_then(|caps| caps.get(1)).map(|user_part| {
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

pub fn extract_sdp_media_info_from_body(sip_body: &str) -> Option<String> {
    let mut ip_addr: Option<&str> = None;
    let mut port: Option<&str> = None;
    for line in sip_body.lines() {
        if line.starts_with("c=IN IP4 ") {
            ip_addr = line.split_whitespace().nth(2);
        }
        if line.starts_with("m=audio ") {
            port = line.split_whitespace().nth(1);
        }
    }
    if let (Some(ip), Some(p)) = (ip_addr, port) {
        Some(format!("{}:{}", ip, p))
    } else {
        None
    }
}