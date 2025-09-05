# 🚦 Sentiric SIP Signaling Service

[![Status](https://img.shields.io/badge/status-active-success.svg)]()
[![Language](https://img.shields.io/badge/language-Rust-orange.svg)]()
[![Protocol](https://img.shields.io/badge/protocol-SIP,_gRPC,_AMQP-green.svg)]()

**Sentiric SIP Signaling Service**, Sentiric platformunun **dayanıklı ve akıllı çağrı kurulum orkestratörüdür**. Yüksek performans, bellek güvenliği ve düşük seviye ağ kontrolü için **Rust** ile yazılmıştır. Görevi, `sip-gateway`'den gelen SIP isteklerini alıp, bir çağrıyı hayata geçirmek için gereken adımları koordine etmek ve bu süreç boyunca sistemin kararlılığını ve telekom standartlarıyla uyumluluğunu garanti etmektir.

## 🎯 Temel Sorumluluklar

*   **Dayanıklı Başlangıç ve Durum Yönetimi:** Servis, başlar başlamaz SIP isteklerini dinlemeye alır. Arka planda kritik bağımlılıklarının (gRPC servisleri, Redis) hazır olmasını bekler. Bu süreçte gelen çağrıları yanıtsız bırakmak yerine, anında `503 Service Unavailable` yanıtı vererek sistemin meşgul olduğunu bildirir. Sadece tüm sistem işlevsel olduğunda çağrıları kabul eder.

*   **SIP Uyumluluğu ve Temizleme:** Telekom operatörlerinden gelen, standart dışı veya hatalı SIP başlıklarını (`Record-Route` başlığındaki `trasport` yazım hatası gibi) akıllıca temizler ve düzeltir. Giden isteklerin, maksimum uyumluluk için RFC standartlarına uygun olmasını sağlar.

*   **Senkron Orkestrasyon:** Tam işlevsel moda geçtiğinde, bir çağrıyı kurmak için **gRPC** üzerinden sıralı olarak diğer uzman servisleri çağırır:
    1.  `user-service`: Arayanı doğrulamak için.
    2.  `dialplan-service`: Çağrının ne yapması gerektiğini öğrenmek için.
    3.  `media-service`: Gerçek zamanlı ses (RTP) kanalı için bir port ayırmak.

*   **Asenkron Devir:** Çağrı başarıyla kurulduktan sonra, uzun sürecek olan AI diyalog mantığını platformun asenkron beyni olan `agent-service`'e devreder. Bunu, `call.started` ve `call.answered` olaylarını **RabbitMQ**'ya yayınlayarak yapar.

*   **Çağrı Sonlandırma:** `BYE` isteği veya dahili sonlandırma komutu aldığında, ilgili medya portunu `media-service`'e serbest bıraktırır ve `call.ended` olayını RabbitMQ'ya yayınlar.

## 🛠️ Teknoloji Yığını

*   **Dil:** Rust
*   **Asenkron Runtime:** Tokio
*   **Servisler Arası İletişim:**
    *   **gRPC (Tonic ile):** Senkron, tip-güvenli komutlar için.
    *   **AMQP (Lapin ile):** Asenkron olay yayınlama için (RabbitMQ).
*   **Durum Yönetimi:** Redis (Kayıtlar ve atomik kilitler için).
*   **Gözlemlenebilirlik:** `tracing` ile yapılandırılmış, ortama duyarlı loglama.

## 🔌 API Etkileşimleri

*   **Gelen (Protokol):**
    *   `sentiric-sip-gateway-service` (SIP/UDP): Temizlenmiş SIP isteklerini alır.
    *   `RabbitMQ` (AMQP): Harici sonlandırma isteklerini (`call.terminate.request`) dinler.
*   **Giden (İstemci):**
    *   `sentiric-user-service` (gRPC): Kullanıcı doğrulaması.
    *   `sentiric-dialplan-service` (gRPC): Yönlendirme kararı.
    *   `sentiric-media-service` (gRPC): Medya portu yönetimi.
    *   `RabbitMQ` (AMQP): `call.started` ve `call.ended` gibi olayları yayınlama.

## 🚀 Yerel Geliştirme

**Önemli:** Bu servis, bir mikroservis mimarisinin merkezi bir parçasıdır ve tek başına tam işlevsel olarak **çalışmaz**. `user-service`, `dialplan-service` gibi kritik bağımlılıkları ayakta olmadan başlatıldığında, kendini korumak için 10 denemeden sonra otomatik olarak kapanacaktır.

Bu nedenle, yerel geliştirme için önerilen ve en kolay yöntem **Docker Compose** kullanmaktır.

1.  **Sistemi Docker Compose ile Başlatın:**
    Projenin ana dizininde (`docker-compose.service.yml` dosyasının olduğu yerde) aşağıdaki komutu çalıştırın. Bu komut, tüm bağımlılıkları başlatacak ve kodunuzdaki son değişikliklerle `sip-signaling-service`'i yeniden derleyecektir:
    ```bash
    docker-compose -f docker-compose.service.yml up --build sip-signaling-service
    ```

### İleri Düzey: `cargo run` ile Çalıştırma

Eğer tüm platform (`user-service`, `postgres`, `redis` vb.) zaten Docker Compose veya başka bir yöntemle arkaplanda çalışıyorsa ve sadece bu servisi `cargo run` ile yerel olarak test etmek istiyorsanız:

1.  **`.env` Dosyasını Oluşturun:** Projenin kök dizininde, Docker dışı ortamınıza uygun bir `.env` dosyası oluşturun. Diğer servislerin `localhost` üzerinde çalıştığını varsayan bir örnek:
    ```dotenv
    # Yerel Windows/Linux Geliştirme Ortamı İçin .env
    ENV=development
    RUST_LOG=info,sentiric_sip_signaling_service=debug

    RABBITMQ_URL="amqp://sentiric:sentiric_pass@127.0.0.1:5672/%2f"
    REDIS_URL="redis://127.0.0.1:6379/0"

    MEDIA_SERVICE_GRPC_URL="127.0.0.1:50052"
    USER_SERVICE_GRPC_URL="127.0.0.1:50053"
    DIALPLAN_SERVICE_GRPC_URL="127.0.0.1:50054"

    SIP_SIGNALING_SERVICE_LISTEN_ADDRESS=0.0.0.0
    SIP_SIGNALING_SERVICE_PORT=5061 # Gateway'den farklı bir port kullanın
    PUBLIC_IP=127.0.0.1
    SIP_REALM=sentiric_demo

    # Sertifika yollarını projenize göre güncelleyin
    GRPC_TLS_CA_PATH=../sentiric-config/tls/certs/ca.crt
    SIP_SIGNALING_SERVICE_CERT_PATH=../sentiric-config/tls/certs/sip-signaling-chain.crt
    SIP_SIGNALING_SERVICE_KEY_PATH=../sentiric-config/tls/certs/sip-signaling.key
    ```
2.  **Servisi Çalıştırın:** `cargo run --release`

## 🤝 Katkıda Bulunma

Katkılarınızı bekliyoruz! Lütfen projenin ana [Sentiric Governance](https://github.com/sentiric/sentiric-governance) reposundaki kodlama standartlarına ve katkıda bulunma rehberine göz atın.

---
## 🏛️ Anayasal Konum

Bu servis, [Sentiric Anayasası'nın (v11.0)](https://github.com/sentiric/sentiric-governance/blob/main/docs/blueprint/Architecture-Overview.md) **Zeka & Orkestrasyon Katmanı**'nda yer alan merkezi bir bileşendir.