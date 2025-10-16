# 🚦 Sentiric SIP Signaling Service

[![Status](https://img.shields.io/badge/status-active-success.svg)]()
[![Language](https://img.shields.io/badge/language-Rust-orange.svg)]()
[![Protocol](https://img.shields.io/badge/protocol-SIP,_gRPC,_AMQP-green.svg)]()

**Sentiric SIP Signaling Service**, Sentiric platformunun **iç çağrı orkestrasyon beynidir**. Görevi, **yalnızca `sip-gateway-service`'ten gelen** temizlenmiş ve güvenli SIP isteklerini alıp, bir çağrıyı hayata geçirmek için gereken adımları koordine etmektir.

Bu servis, dış dünyanın karmaşık SIP protokol detaylarından (NAT, çoklu `Via` başlıkları vb.) **kasıtlı olarak izole edilmiştir**. Bu sorumluluk `sip-gateway`'e aittir.

## 🎯 Temel Sorumluluklar

*   **Dayanıklı Başlangıç ve Durum Yönetimi:** Servis, başlar başlamaz SIP isteklerini dinlemeye alır ancak arka planda kritik bağımlılıkları (gRPC servisleri, Redis) hazır olana kadar bekler. Bu süreçte gelen çağrılara `503 Service Unavailable` yanıtı vererek sistemin meşgul olduğunu bildirir.

*   **Senkron Orkestrasyon:** Tam işlevsel moda geçtiğinde, bir çağrıyı kurmak için **gRPC** üzerinden sıralı olarak diğer uzman servisleri çağırır:
    1.  `user-service`: Arayanı doğrulamak için.
    2.  `dialplan-service`: Çağrının ne yapması gerektiğini öğrenmek için.
    3.  `media-service`: Gerçek zamanlı ses (RTP) kanalı için bir port ayırmak.

*   **Asenkron Devir:** Çağrı başarıyla kurulduktan sonra, uzun sürecek olan AI diyalog mantığını `agent-service`'e devreder. Bunu, `call.started` ve `call.answered` olaylarını **RabbitMQ**'ya yayınlayarak yapar.

*   **Çağrı Sonlandırma:** `BYE` isteği veya dahili sonlandırma komutu aldığında, ilgili medya portunu `media-service`'e serbest bıraktırır ve `call.ended` olayını RabbitMQ'ya yayınlar.

## 🛠️ Teknoloji Yığını

*   **Dil:** Rust
*   **Asenkron Runtime:** Tokio
*   **Servisler Arası İletişim:**
    *   **gRPC (Tonic ile):** Senkron, tip-güvenli komutlar için.
    *   **AMQP (Lapin ile):** Asenkron olay yayınlama için (RabbitMQ).
*   **Durum Yönetimi:** Redis (Kayıtlar ve atomik kilitler için).
*   **Gözlemlenebilirlik:** `tracing` ile yapılandırılmış loglama.
*   **Kimlik Doğrulama:** `SIP Digest Authentication` (MD5)

### **Önemli Kavram: SIP Realm**

Platformumuzda, `SIP_SIGNALING_SERVICE_REALM` ortam değişkeni (örn: `sentiric_demo`), kimlik doğrulama işlemlerinde kullanılan mantıksal "bölgeyi" tanımlar. Bu, SIP standardındaki `realm` parametresine karşılık gelir.

MicroSIP gibi bazı SIP istemcileri, bu değeri "Domain" olarak adlandırılan bir alana girmenizi isteyebilir. Ancak bu, paketin gönderileceği "SIP Sunucusu" (bizim `sip-gateway` IP adresimiz) ile karıştırılmamalıdır. Bizim mimarimizde bu iki kavram nettir:
*   **SIP Sunucusu:** `sip-gateway`'in genel IP adresi.
*   **SIP Realm/Domain:** `SIP_SIGNALING_SERVICE_REALM` değişkeni ile tanımlanan kimlik doğrulama alanı.

## 🚀 Yerel Geliştirme

1.  **Bağımlılıkları Yükleyin:**
2.  **Ortam Değişkenlerini Ayarlayın:** `.env.example` dosyasını `.env` olarak kopyalayın ve gerekli değişkenleri doldurun.
3.  **Servisi Çalıştırın:**

---
## 🏛️ Anayasal Konum

Bu servis, [Sentiric Anayasası'nın](https://github.com/sentiric/sentiric-governance) **Zeka & Orkestrasyon Katmanı**'nda yer alan merkezi bir bileşendir.