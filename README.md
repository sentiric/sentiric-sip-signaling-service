# 🚦 Sentiric SIP Signaling Service

[![Status](https://img.shields.io/badge/status-active-success.svg)]()
[![Language](https://img.shields.io/badge/language-Rust-orange.svg)]()
[![Protocol](https://img.shields.io/badge/protocol-SIP,_gRPC,_RabbitMQ-green.svg)]()

**Sentiric SIP Signaling Service**, Sentiric platformunun **senkron çağrı kurulum orkestratörüdür**. Yüksek performans, bellek güvenliği ve düşük seviye ağ kontrolü için **Rust** ile yazılmıştır. Görevi, `sip-gateway`'den gelen temizlenmiş SIP isteklerini alıp, bir çağrıyı hayata geçirmek için gereken tüm adımları anlık olarak koordine etmektir.

## 🎯 Temel Sorumluluklar

*   **SIP Mesaj İşleme:** `INVITE`, `BYE` gibi temel SIP metotlarını işler ve standartlara uygun yanıtlar (`100 Trying`, `200 OK`) üretir.
*   **Senkron Orkestrasyon:** Bir çağrıyı kurmak için **gRPC** üzerinden sıralı olarak diğer uzman servisleri çağırır:
    1.  `user-service`: Arayanı doğrulamak için.
    2.  `dialplan-service`: Çağrının ne yapması gerektiğini öğrenmek için.
    3.  `media-service`: Gerçek zamanlı ses (RTP) kanalı için bir port ayırmak.
*   **Asenkron Devir:** Çağrı başarıyla kurulduktan sonra, uzun sürecek olan AI diyalog mantığını platformun asenkron beyni olan `agent-service`'e devreder. Bunu, `call.started` olayını **RabbitMQ**'ya yayınlayarak yapar.
*   **Çağrı Sonlandırma:** `BYE` isteği aldığında, ilgili medya portunu `media-service`'e serbest bıraktırır ve `call.ended` olayını RabbitMQ'ya yayınlar.

## 🛠️ Teknoloji Yığını

*   **Dil:** Rust
*   **Asenkron Runtime:** Tokio
*   **Servisler Arası İletişim:**
    *   **gRPC (Tonic ile):** Senkron, tip-güvenli komutlar için.
    *   **AMQP (Lapin ile):** Asenkron olay yayınlama için (RabbitMQ).
*   **Gözlemlenebilirlik:** `tracing` ile yapılandırılmış, ortama duyarlı loglama.

## 🔌 API Etkileşimleri

*   **Gelen (Protokol):**
    *   `sentiric-sip-gateway-service` (SIP/UDP): Temizlenmiş SIP isteklerini alır.
*   **Giden (İstemci):**
    *   `sentiric-user-service` (gRPC): Kullanıcı doğrulaması.
    *   `sentiric-dialplan-service` (gRPC): Yönlendirme kararı.
    *   `sentiric-media-service` (gRPC): Medya portu yönetimi.
    *   `RabbitMQ` (AMQP): `call.started` ve `call.ended` olaylarını yayınlama.

## 🚀 Yerel Geliştirme

1.  **Bağımlılıkları Yükleyin:** `cargo build`
2.  **`.env` Dosyasını Oluşturun:** `sentiric-agent-service/.env.docker` dosyasını referans alarak gerekli servis URL'lerini ve sertifika yollarını tanımlayın.
3.  **Servisi Çalıştırın:** `cargo run --release`

## 🤝 Katkıda Bulunma

Katkılarınızı bekliyoruz! Lütfen projenin ana [Sentiric Governance](https://github.com/sentiric/sentiric-governance) reposundaki kodlama standartlarına ve katkıda bulunma rehberine göz atın.

---
## 🏛️ Anayasal Konum

Bu servis, [Sentiric Anayasası'nın (v11.0)](https://github.com/sentiric/sentiric-governance/blob/main/docs/blueprint/Architecture-Overview.md) **Zeka & Orkestrasyon Katmanı**'nda yer alan merkezi bir bileşendir.