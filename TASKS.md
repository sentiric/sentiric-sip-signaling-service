### **`sentiric-sip-signaling-service/TASKS.md` (Kapsamlı Revizyon v5.0)**

# 🚦 Sentiric SIP Signaling Service - Geliştirme Yol Haritası (v5.0)

Bu belge, `sip-signaling-service`'in, Sentiric Anayasası'nda tanımlanan **"Senkron Çağrı Kurulum Orkestratörü"** rolünü eksiksiz yerine getirmesi için gereken tüm görevleri fazlara ayrılmış şekilde listeler.

---

### **FAZ 1: Stabilizasyon ve Çekirdek Orkestrasyon (Tamamlandı)**

**Amaç:** Platformun temel çağrı kurulum/sonlandırma yeteneklerini sağlamlaştırmak, dayanıklı hale getirmek ve gözlemlenebilirlik standartlarına uydurmak.

-   [x] **Görev ID: SIG-001 - Çekirdek `INVITE`/`BYE` Akışı**
    -   **Açıklama:** Gelen `INVITE` isteklerini kabul edip `200 OK` ile yanıtlama ve `BYE` ile çağrıyı sonlandırma temel mantığı oluşturuldu.
    -   **Durum:** ✅ **Tamamlandı**

-   [x] **Görev ID: SIG-002 - Senkron Orkestrasyon Mantığı**
    -   **Açıklama:** Bir `INVITE` geldiğinde `user-service`, `dialplan-service` ve `media-service`'e sıralı gRPC çağrıları yaparak çağrı kurulumunu koordine etme yeteneği eklendi.
    -   **Durum:** ✅ **Tamamlandı**

-   [x] **Görev ID: SIG-003 - Asenkron Olay Yayınlama**
    -   **Açıklama:** `call.started` ve `call.ended` gibi kritik yaşam döngüsü olaylarını, platformun asenkron beyni olan `agent-service`'in tüketmesi için RabbitMQ'ya (Topic Exchange) yayınlama yeteneği eklendi.
    -   **Durum:** ✅ **Tamamlandı**

-   [x] **Görev ID: SIG-004 - Gözlemlenebilirlik Standardı Uyumu**
    -   **Açıklama:** `tracing` yapılandırması, `OBSERVABILITY_STANDARD.md` ile tam uyumlu hale getirildi. Gereksiz `INFO` logları `DEBUG` seviyesine çekilerek logların okunabilirliği artırıldı.
    -   **Durum:** ✅ **Tamamlandı**

-   [x] **Görev ID: SIG-005 - Uzaktan Çağrı Sonlandırma**
    -   **Açıklama:** `call.terminate.request` olaylarını dinleyerek, `agent-service` gibi diğer servislerden gelen komutlarla çağrıları proaktif olarak sonlandırma yeteneği eklendi.
    -   **Durum:** ✅ **Tamamlandı**

-   [x] **Görev ID: SIG-006 - Kodun Modülerleştirilmesi**
    -   **Açıklama:** `src/main.rs` dosyası, sorumlulukların `sip`, `grpc`, `rabbitmq` gibi ayrı modüllere taşınmasıyla yeniden yapılandırıldı.
    -   **Durum:** ✅ **Tamamlandı**

-   [x] **Görev ID: SIG-007 - Yinelenen `INVITE` Yönetimi**
    -   **Açıklama:** Aynı `Call-ID`'ye sahip yinelenen `INVITE` isteklerinin, platformda birden fazla çağrı süreci başlatmasını engelleyen bir kilit mekanizması eklendi.
    -   **Durum:** ✅ **Tamamlandı**

-   [x] **Görev ID: SIG-008 - `REGISTER` Metodu ile Kimlik Doğrulama**
    -   **Açıklama:** SIP istemcilerinin platforma kayıt (`REGISTER`) olmasını ve `user-service` üzerinden Digest Authentication ile kimlik doğrulaması yapmasını sağlayan mantık implemente edildi.
    -   **Durum:** ✅ **Tamamlandı**

---

### **FAZ 2: Platform İçi İletişim (Peer-to-Peer) Yetenekleri (Sıradaki Öncelik)**

**Amaç:** Platformu, sadece dış hatlarla konuşan bir sistem olmaktan çıkarıp, kendi içindeki kayıtlı kullanıcıların birbirleriyle doğrudan iletişim kurabildiği tam teşekküllü bir SIP sunucusuna dönüştürmek.

-   [ ] **Görev ID: SIG-009 - P2P Çağrı Yönlendirme (SIP Proxy Mantığı)**
    -   **Durum:** ⬜ **Planlandı**
    -   **Açıklama:** `handle_invite` içinde `TODO` olarak işaretlenen, bir SIP kullanıcısından (`1001`) diğerine (`2001`) gelen çağrıları, `dialplan-service`'e gitmek yerine, hedef kullanıcının Redis'teki adresine doğrudan yönlendiren (proxy) mantığı implemente et.
    -   **Kabul Kriterleri:**
        -   [ ] Aranan URI'nin bir telefon numarası mı yoksa bir SIP kullanıcısı mı olduğu doğru bir şekilde tespit edilmeli.
        -   [ ] Hedef kullanıcının kayıtlı `contact` adresi Redis'ten okunmalı.
        -   [ ] Gelen `INVITE` paketi, `Request-URI` hedef kullanıcının `contact` adresi olacak şekilde modifiye edilmeli.
        -   [ ] Yanıtların doğru yoldan geri dönebilmesi için `Via` ve `Record-Route` başlıkları standartlara uygun olarak yönetilmeli.
        -   [ ] Uçtan uca test: Bir softphone'dan (`1001`) başka bir softphone'a (`2001`) yapılan arama başarıyla kurulmalı ve iki taraf arasında sesli iletişim sağlanmalıdır.

-   [ ] **Görev ID: SIG-010 - Kullanıcı Durum Yönetimi (Presence)**
    -   **Durum:** ⬜ **Planlandı**
    -   **Açıklama:** SIP istemcilerinden gelen `PUBLISH` isteklerini işleyerek kullanıcıların "online", "busy", "away" gibi durumlarını yönet ve `SUBSCRIBE`/`NOTIFY` ile bu bilgiyi diğer kullanıcılara ilet.
    -   **Kabul Kriterleri:**
        -   [ ] `handle_sip_request`, `PUBLISH` metodunu tanımalı ve işlemeli.
        -   [ ] Kullanıcı durumları (presence state) Redis'te bir TTL ile saklanmalı.
        -   [ ] Bir kullanıcı başka bir kullanıcının durumuna `SUBSCRIBE` olduğunda, durumu değiştiğinde `NOTIFY` mesajı gönderilmelidir.
        -   [ ] **İlişkili Görev:** `sentiric-web-agent-ui`'da diğer ajanların durumunu (yeşil/kırmızı ışık) gösterecek altyapı bu mekanizmaya dayanacaktır.

---

### **FAZ 3: Gelişmiş Çağrı Kontrolü ve Dayanıklılık**

**Amaç:** Platformun çağrı akışları üzerindeki kontrolünü artırmak ve daha karmaşık telekomünikasyon senaryolarını yönetebilmesini sağlamak.

-   [ ] **Görev ID: SIG-011 - `CANCEL` Metodu Desteği**
    -   **Durum:** ⬜ **Planlandı**
    -   **Açıklama:** Bir `INVITE` isteği gönderildikten sonra, ancak `200 OK` yanıtı alınmadan önce çağrının arayan tarafından iptal edilmesini sağlayan `CANCEL` isteğini doğru bir şekilde işle.
    -   **Kabul Kriterleri:**
        -   [ ] `CANCEL` isteği alındığında, ilgili `INVITE` işlemi (gRPC çağrıları vb.) durdurulmalı.
        -   [ ] `media-service`'ten tahsis edilen port varsa derhal serbest bırakılmalı.
        -   [ ] Hem `CANCEL`'a hem de orijinal `INVITE`'a standartlara uygun yanıtlar (`200 OK` ve `487 Request Terminated`) gönderilmelidir.

-   [ ] **Görev ID: SIG-012 - Temel Çağrı Transferi (`REFER`)**
    -   **Durum:** ⬜ **Planlandı**
    -   **Açıklama:** Aktif bir çağrıyı başka bir SIP kullanıcısına veya harici bir numaraya yönlendirmeyi sağlayan `REFER` metodunu implemente et.
    -   **Kabul Kriterleri:**
        -   [ ] Çağrı sırasında `REFER` isteği alındığında, transfer hedefi parse edilmeli.
        -   [ ] Platform, hedefe yeni bir `INVITE` göndererek transferi başlatmalı.
        -   [ ] Transferin durumu (`100 Trying`, `200 OK`) `NOTIFY` mesajları ile `REFER`'ı başlatan tarafa bildirilmelidir.
        -   **İlişkili Görev:** Bu, `agent-service`'in bir çağrıyı insan bir operatöre ("kör transfer") devretmesinin temelini oluşturur.

-   [ ] **Görev ID: SIG-013 - Gelişmiş Kimlik Doğrulama Mantığı**
    -   **Durum:** ⬜ **Planlandı**
    -   **Açıklama:** `user-service` ile olan kimlik doğrulama akışını, HA1 hash hesaplama sorumluluğunu tamamen `user-service`'e devredecek şekilde yeniden yapılandır.
    -   **Bağımlılık:** `sentiric-user-service`'de `VerifySipPassword(username, realm, nonce, response)` gibi yeni bir RPC'nin oluşturulmasını gerektirir (`USER-007`).
    -   **Kabul Kriterleri:**
        -   [ ] `sip-signaling-service` artık MD5 hesaplaması yapmamalı.
        -   [ ] `REGISTER` isteğindeki `Authorization` başlığının içeriği, olduğu gibi yeni `user-service` RPC'sine gönderilmeli.
        -   [ ] `user-service`'den gelen `true/false` yanıtına göre kayıt işlemi devam etmeli veya reddedilmeli.

---

Bu yol haritası, `sip-signaling-service`'in mevcut stabil durumundan, tam teşekküllü ve akıllı bir SIP iletişim merkezine nasıl evrileceğini net bir şekilde tanımlar.

