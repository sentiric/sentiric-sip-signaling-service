// ========== FILE: sentiric-sip-signaling-service/src/main.rs ==========
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tokio::time::sleep;

use chrono::Utc;
use once_cell::sync::Lazy;
use regex::Regex;
use tonic::{
    transport::{Certificate, Channel, ClientTlsConfig, Identity},
    Request as TonicRequest,
};
use tracing::{error, info, instrument, warn};
use tracing_subscriber::EnvFilter;

use lapin::{
    options::*, types::FieldTable, BasicProperties, Channel as LapinChannel, Connection,
    ConnectionProperties,
};
use rand::distributions::{Alphanumeric, DistString};
use rand::Rng;
use sentiric_contracts::sentiric::{
    dialplan::v1::{dialplan_service_client::DialplanServiceClient, ResolveDialplanRequest},
    media::v1::{
        media_service_client::MediaServiceClient, AllocatePortRequest, 
        // ReleasePortRequest,
    },
};

static USER_EXTRACT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"sip:\+?(\d+)@").unwrap());
const RABBITMQ_EXCHANGE_NAME: &str = "sentiric_events";

type ActiveCalls = Arc<Mutex<HashMap<String, (u32, String, Instant)>>>;

#[derive(Clone)]
struct AppConfig {
    sip_listen_addr: SocketAddr,
    sip_public_ip: String,
    dialplan_service_url: String,
    media_service_url: String,
    user_service_url: String,
    rabbitmq_url: String,
    env: String,
}

impl fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppConfig")
            .field("sip_listen_addr", &self.sip_listen_addr)
            .field("sip_public_ip", &self.sip_public_ip)
            .field("dialplan_service_url", &self.dialplan_service_url)
            .field("media_service_url", &self.media_service_url)
            .field("user_service_url", &self.user_service_url)
            .field("rabbitmq_url", &"***REDACTED***")
            .field("env", &self.env)
            .finish()
    }
}

impl AppConfig {
    fn load_from_env() -> Result<Self, Box<dyn Error>> {
        dotenv::dotenv().ok();
        let sip_host = env::var("SIP_SIGNALING_SERVICE_LISTEN_ADDRESS").unwrap_or_else(|_| "0.0.0.0".to_string());
        let sip_port_str = env::var("SIP_SIGNALING_SERVICE_PORT").unwrap_or_else(|_| "5060".to_string());
        let sip_port = sip_port_str.parse::<u16>()?;

        Ok(AppConfig {
            sip_listen_addr: format!("{}:{}", sip_host, sip_port).parse()?,
            sip_public_ip: env::var("PUBLIC_IP")?,
            rabbitmq_url: env::var("RABBITMQ_URL")?,
            media_service_url: env::var("MEDIA_SERVICE_GRPC_URL")?,
            user_service_url: env::var("USER_SERVICE_GRPC_URL")?,
            dialplan_service_url: env::var("DIALPLAN_SERVICE_GRPC_URL")?,
            env: env::var("ENV").unwrap_or_else(|_| "production".to_string()),
        })
    }
}

async fn connect_to_rabbitmq_with_retry(url: &str) -> Arc<LapinChannel> {
    let max_retries = 10;
    for i in 0..max_retries {
        if let Ok(conn) = Connection::connect(url, ConnectionProperties::default()).await {
            if let Ok(channel) = conn.create_channel().await {
                info!("RabbitMQ bağlantısı başarıyla kuruldu.");
                return Arc::new(channel);
            }
        }
        warn!( attempt = i + 1, max_attempts = max_retries, "RabbitMQ'ya bağlanılamadı. 5sn sonra tekrar denenecek...");
        sleep(Duration::from_secs(5)).await;
    }
    panic!("Maksimum deneme sayısına ulaşıldı, RabbitMQ'ya bağlanılamadı.");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Arc::new(AppConfig::load_from_env()?);
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber_builder = tracing_subscriber::fmt().with_env_filter(env_filter);
    if config.env == "development" {
        subscriber_builder.with_target(true).with_line_number(true).init();
    } else {
        subscriber_builder.json().with_current_span(true).with_span_list(true).init();
    }
    info!(config = ?config, "Konfigürasyon yüklendi.");
    let active_calls: ActiveCalls = Arc::new(Mutex::new(HashMap::new()));
    let rabbit_channel = connect_to_rabbitmq_with_retry(&config.rabbitmq_url).await;
    rabbit_channel.exchange_declare(RABBITMQ_EXCHANGE_NAME, lapin::ExchangeKind::Fanout, ExchangeDeclareOptions { durable: true, ..Default::default() }, FieldTable::default()).await?;
    info!(exchange_name = RABBITMQ_EXCHANGE_NAME, "RabbitMQ exchange'i deklare edildi.");
    let sock = Arc::new(UdpSocket::bind(config.sip_listen_addr).await?);
    info!(address = %config.sip_listen_addr, "SIP Signaling başlatıldı.");
    let mut buf = [0; 65535];
    let cleanup_calls_clone = active_calls.clone();
    tokio::spawn(cleanup_old_transactions(cleanup_calls_clone));
    loop {
        let (len, addr) = sock.recv_from(&mut buf).await?;
        let sock_clone = Arc::clone(&sock);
        let config_clone = Arc::clone(&config);
        let rabbit_channel_clone = Arc::clone(&rabbit_channel);
        let request_bytes = buf[..len].to_vec();
        let active_calls_clone = Arc::clone(&active_calls);
        tokio::spawn(async move {
            if let Err(e) = handle_sip_request(&request_bytes, sock_clone, addr, config_clone, rabbit_channel_clone, active_calls_clone).await {
                error!(error = %e, remote_addr = %addr, "SIP isteği işlenirken akış tamamlanamadı.");
            }
        });
    }
}

#[instrument(skip_all, fields(remote_addr = %addr, call_id, trace_id))]
async fn handle_sip_request(
    request_bytes: &[u8],
    sock: Arc<UdpSocket>,
    addr: SocketAddr,
    config: Arc<AppConfig>,
    rabbit_channel: Arc<LapinChannel>,
    active_calls: ActiveCalls,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let request_str = std::str::from_utf8(request_bytes)?;
    if request_str.starts_with("INVITE") {
        handle_invite(request_str, sock, addr, config, rabbit_channel, active_calls).await
    } else if request_str.starts_with("BYE") {
        handle_bye(request_str, sock, addr, config, rabbit_channel, active_calls).await
    } else if request_str.starts_with("ACK") {
        if let Some(headers) = parse_complex_headers(request_str) {
            let call_id = headers.get("Call-ID").cloned().unwrap_or_default();
            tracing::Span::current().record("call_id", &call_id as &str);
            info!("ACK isteği alındı, SIP diyaloğu başarıyla kuruldu.");
        }
        Ok(())
    } else {
        Ok(())
    }
}

async fn create_secure_grpc_channel(url: &str, server_name: &str) -> Result<Channel, Box<dyn Error + Send + Sync>> {
    let cert_path = env::var("SIP_SIGNALING_SERVICE_CERT_PATH")?;
    let key_path = env::var("SIP_SIGNALING_SERVICE_KEY_PATH")?;
    let ca_path = env::var("GRPC_TLS_CA_PATH")?;
    let cert = tokio::fs::read(cert_path).await?;
    let key = tokio::fs::read(key_path).await?;
    let ca_cert = tokio::fs::read(ca_path).await?;
    let identity = Identity::from_pem(cert, key);
    let ca_cert = Certificate::from_pem(ca_cert);
    let tls_config = ClientTlsConfig::new().domain_name(server_name).ca_certificate(ca_cert).identity(identity);
    let endpoint = Channel::from_shared(format!("https://{}", url))?.tls_config(tls_config)?.connect_timeout(Duration::from_secs(5)).keep_alive_while_idle(true).timeout(Duration::from_secs(10));
    let channel = endpoint.connect().await?;
    Ok(channel)
}

#[instrument(skip_all, fields(remote_addr = %addr))]
async fn handle_invite(
    request_str: &str,
    sock: Arc<UdpSocket>,
    addr: SocketAddr,
    config: Arc<AppConfig>,
    rabbit_channel: Arc<LapinChannel>,
    active_calls: ActiveCalls,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut headers = parse_complex_headers(request_str).ok_or_else(|| "Geçersiz başlıklar")?;
    let call_id = headers.get("Call-ID").cloned().unwrap_or_default();
    
    {
        let mut calls_guard = active_calls.lock().await;
        if calls_guard.contains_key(&call_id) {
            warn!(%call_id, "Yinelenen INVITE isteği alındı (zaten işleniyor), görmezden gelindi.");
            let _ = sock.send_to(create_response("100 Trying", &headers, None, &config).as_bytes(), addr).await;
            return Ok(());
        }
        calls_guard.insert(call_id.clone(), (0, "pending".to_string(), Instant::now()));
    }
    
    let trace_id = format!("trace-{}", Alphanumeric.sample_string(&mut rand::thread_rng(), 12));
    tracing::Span::current().record("call_id", &call_id as &str);
    tracing::Span::current().record("trace_id", &trace_id as &str);

    sock.send_to(create_response("100 Trying", &headers, None, &config).as_bytes(), addr).await?;
    let from_uri = headers.get("From").cloned().unwrap_or_default();
    let to_uri = headers.get("To").cloned().unwrap_or_default();
    let caller_id = extract_user_from_uri(&from_uri).unwrap_or_else(|| "unknown".to_string());
    let destination_number = extract_user_from_uri(&to_uri).unwrap_or_else(|| "unknown".to_string());
    let mut dialplan_req = TonicRequest::new(ResolveDialplanRequest { caller_contact_value: caller_id, destination_number });
    dialplan_req.metadata_mut().insert("x-trace-id", trace_id.parse()?);
    let dialplan_channel = create_secure_grpc_channel(&config.dialplan_service_url, "dialplan-service").await?;
    let mut dialplan_client = DialplanServiceClient::new(dialplan_channel);
    let dialplan_res = match dialplan_client.resolve_dialplan(dialplan_req).await {
        Ok(res) => res.into_inner(),
        Err(e) => {
            error!(error = %e, "Dialplan çözümlenirken kritik hata.");
            active_calls.lock().await.remove(&call_id);
            sock.send_to(create_response("503 Service Unavailable", &headers, None, &config).as_bytes(), addr).await?;
            return Err(e.into());
        }
    };
    info!(dialplan_id = %dialplan_res.dialplan_id, "Dialplan başarıyla çözümlendi.");
    let media_channel = create_secure_grpc_channel(&config.media_service_url, "media-service").await?;
    let mut media_client = MediaServiceClient::new(media_channel);
    let mut media_req = TonicRequest::new(AllocatePortRequest { call_id: call_id.clone() });
    media_req.metadata_mut().insert("x-trace-id", trace_id.parse()?);
    let server_rtp_port = match media_client.allocate_port(media_req).await {
        Ok(res) => res.into_inner().rtp_port,
        Err(e) => {
            error!(error = %e, "Medya portu alınamadı.");
            active_calls.lock().await.remove(&call_id);
            sock.send_to(create_response("503 Service Unavailable", &headers, None, &config).as_bytes(), addr).await?;
            return Err(e.into());
        }
    };
    info!(rtp_port = server_rtp_port, "Medya portu ayrıldı.");
    if let Some(call_entry) = active_calls.lock().await.get_mut(&call_id) {
        *call_entry = (server_rtp_port, trace_id.clone(), Instant::now());
    }
    
    let sdp_body = format!("v=0\r\no=- {0} {0} IN IP4 {1}\r\ns=Sentiric\r\nc=IN IP4 {1}\r\nt=0 0\r\nm=audio {2} RTP/AVP 0\r\na=rtpmap:0 PCMU/8000\r\n", rand::thread_rng().gen::<u32>(), config.sip_public_ip, server_rtp_port);
    let to_tag = format!(";tag={}", rand::thread_rng().gen::<u32>());
    headers.entry("To".to_string()).and_modify(|v| *v = format!("{}{}", v, to_tag));
    sock.send_to(create_response("180 Ringing", &headers, None, &config).as_bytes(), addr).await?;
    sleep(Duration::from_millis(100)).await;
    let ok_response = create_response("200 OK", &headers, Some(&sdp_body), &config);
    sock.send_to(ok_response.as_bytes(), addr).await?;
    info!("Çağrı başarıyla yanıtlandı (200 OK gönderildi).");
    let event_payload = serde_json::json!({ "eventType": "call.started", "traceId": trace_id, "callId": call_id, "from": from_uri, "to": to_uri, "media": { "server_rtp_port": server_rtp_port, "caller_rtp_addr": extract_sdp_media_info(request_str).unwrap_or_default() }, "dialplan": dialplan_res, "timestamp": Utc::now().to_rfc3339() });
    let publish_result = rabbit_channel.basic_publish(RABBITMQ_EXCHANGE_NAME, "", BasicPublishOptions::default(), event_payload.to_string().as_bytes(), BasicProperties::default().with_delivery_mode(2)).await;
    if let Ok(confirmation) = publish_result {
        if confirmation.await.is_ok() { info!("'call.started' olayı yayınlandı."); } else { error!("RabbitMQ 'call.started' onayı alınamadı."); }
    } else { error!("RabbitMQ'ya 'call.started' yayınlanamadı."); }
    Ok(())
}

async fn handle_bye(
    request_str: &str,
    sock: Arc<UdpSocket>,
    addr: SocketAddr,
    config: Arc<AppConfig>,
    rabbit_channel: Arc<LapinChannel>,
    active_calls: ActiveCalls,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(headers) = parse_complex_headers(request_str) {
        let call_id = headers.get("Call-ID").cloned().unwrap_or_default();
        tracing::Span::current().record("call_id", &call_id as &str);
        info!("BYE isteği alındı.");
        
        let ok_response = create_response("200 OK", &headers, None, &config);
        sock.send_to(ok_response.as_bytes(), addr).await?;
        info!("BYE isteğine 200 OK yanıtı gönderildi.");

        let call_info = { active_calls.lock().await.remove(&call_id) };
        
        if let Some((rtp_port, trace_id, _)) = call_info {
            tracing::Span::current().record("trace_id", &trace_id as &str);
            info!(port = rtp_port, "Çağrı sonlandırılıyor, olay yayınlanacak.");

            let event_payload = serde_json::json!({ "eventType": "call.ended", "traceId": trace_id, "callId": call_id, "timestamp": Utc::now().to_rfc3339() });
            let publish_result = rabbit_channel.basic_publish(RABBITMQ_EXCHANGE_NAME, "", BasicPublishOptions::default(), event_payload.to_string().as_bytes(), BasicProperties::default().with_delivery_mode(2)).await;
            
            if let Ok(confirmation) = publish_result {
                if confirmation.await.is_ok() { 
                    info!("'call.ended' olayı başarıyla yayınlandı.");
                } else { 
                    error!("RabbitMQ 'call.ended' onayı alınamadı.");
                }
            } else { 
                error!("RabbitMQ'ya 'call.ended' yayınlanamadı.");
            }
            
            warn!(port = rtp_port, "Port, agent'ın son işlemleri için açık bırakıldı. Karantina mekanizması temizleyecek.");

        } else {
            warn!("BYE isteği alınan çağrı aktif çağrılar listesinde bulunamadı.");
        }
    }
    Ok(())
}


// ... (dosyanın geri kalanı tamamen aynı)
fn parse_complex_headers(request: &str) -> Option<HashMap<String, String>> { let mut headers = HashMap::new(); let mut via_headers = Vec::new(); let mut record_route_headers = Vec::new(); for line in request.lines() { if line.is_empty() { break; } if let Some((key, value)) = line.split_once(':') { let key_trimmed = key.trim(); let value_trimmed = value.trim().to_string(); match key_trimmed.to_lowercase().as_str() { "via" | "v" => via_headers.push(value_trimmed), "record-route" => record_route_headers.push(value_trimmed), _ => { headers.insert(key_trimmed.to_string(), value_trimmed); } } } } if !via_headers.is_empty() { headers.insert("Via".to_string(), via_headers.join(", ")); if !record_route_headers.is_empty() { headers.insert("Record-Route".to_string(), record_route_headers.join(", ")); } Some(headers) } else { warn!("Gelen SIP isteğinde Via başlığı bulunamadı."); None } }
fn create_response(status_line: &str, headers: &HashMap<String, String>, sdp: Option<&str>, config: &AppConfig) -> String { let body = sdp.unwrap_or(""); let via_lines = headers.get("Via").map_or(String::new(), |v| format!("Via: {}\r\n", v)); let empty_string = String::new(); let contact_header = format!("<sip:{}@{}:{}>", "sentiric-signal", config.sip_public_ip, config.sip_listen_addr.port()); format!( "SIP/2.0 {}\r\n{}\
        From: {}\r\n\
        To: {}\r\n\
        Call-ID: {}\r\n\
        CSeq: {}\r\n\
        Contact: {}\r\n\
        Server: Sentiric Signaling Service\r\n\
        Content-Length: {}\r\n\
        {}\r\n\
        {}", status_line, via_lines, headers.get("From").unwrap_or(&empty_string), headers.get("To").unwrap_or(&empty_string), headers.get("Call-ID").unwrap_or(&empty_string), headers.get("CSeq").unwrap_or(&empty_string), contact_header, body.len(), if sdp.is_some() { "Content-Type: application/sdp\r\n" } else { "" }, body ) }
fn extract_user_from_uri(uri: &str) -> Option<String> { USER_EXTRACT_RE.captures(uri).and_then(|caps| caps.get(1)).map(|user_part| { let original_num = user_part.as_str(); let mut num: String = original_num.chars().filter(|c| c.is_digit(10)).collect(); if num.len() == 11 && num.starts_with('0') { num = format!("90{}", &num[1..]); } else if num.len() == 10 && !num.starts_with("90") { num = format!("90{}", num); } let normalized_num = num; if original_num != normalized_num { info!(original = %original_num, normalized = %normalized_num, "Telefon numarası normalize edildi."); } normalized_num }) }
fn extract_sdp_media_info(sip_request: &str) -> Option<String> { let mut ip_addr: Option<&str> = None; let mut port: Option<&str> = None; if let Some(sdp_part) = sip_request.split("\r\n\r\n").nth(1) { for line in sdp_part.lines() { if line.starts_with("c=IN IP4 ") { ip_addr = line.split_whitespace().nth(2); } if line.starts_with("m=audio ") { port = line.split_whitespace().nth(1); } } } if let (Some(ip), Some(p)) = (ip_addr, port) { Some(format!("{}:{}", ip, p)) } else { None } }
async fn cleanup_old_transactions(transactions: ActiveCalls) { let mut interval = tokio::time::interval(Duration::from_secs(60)); loop { interval.tick().await; let mut guard = transactions.lock().await; let before_count = guard.len(); guard.retain(|_call_id, (_, _, created_at)| created_at.elapsed() < Duration::from_secs(120)); let after_count = guard.len(); if before_count > after_count { info!(cleaned_count = before_count - after_count, remaining_count = after_count, "Temizlik görevi: Eski işlemler temizlendi."); } } }