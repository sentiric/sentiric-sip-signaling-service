# 🚦 Sentiric SIP Signaling Service - Geliştirme Yol Haritası (v4.0)

Bu belge, servisin geliştirme görevlerini projenin genel fazlarına uygun olarak listeler.

---

### **FAZ 1: Stabilizasyon ve Çekirdek Çağrı Akışı**

**Amaç:** Canlı çağrı akışının çalışmasını engelleyen temel sorunları gidermek ve platformun temel çağrı kurulum/sonlandırma yeteneklerini sağlamlaştırmak.

-   [x] **Görev ID: SIG-000 - Çekirdek `INVITE`/`BYE` Akışı**
    -   **Durum:** ✅ **Tamamlandı**
    -   **Kabul Kriterleri:** Servis, gelen `INVITE` ve `BYE` isteklerini başarıyla işler, ilgili `200 OK` yanıtlarını üretir.

-   [x] **Görev ID: SIG-000B - Senkron Orkestrasyon**
    -   **Durum:** ✅ **Tamamlandı**
    -   **Kabul Kriterleri:** `user`, `dialplan` ve `media` servislerine sıralı ve güvenli (mTLS) gRPC çağrıları yaparak çağrı kurulumu için gerekli bilgileri toplar.

-   [x] **Görev ID: SIG-000C - Asenkron Olay Yayınlama**
    -   **Durum:** ✅ **Tamamlandı**
    -   **Kabul Kriterleri:** `call.started` ve `call.ended` olaylarını, `ResolveDialplanResponse`'tan gelen tüm zenginleştirilmiş verilerle birlikte RabbitMQ'ya başarılı bir şekilde yayınlar.

-   [ ] **Görev ID: SIG-004 - Fazla Konuşkan Loglamayı Düzeltme (KRİTİK & ACİL)**
    -   **Açıklama:** `src/main.rs` dosyasındaki `tracing` yapılandırmasını, `OBSERVABILITY_STANDARD.md`'ye uygun hale getirerek `INFO` seviyesindeki gereksiz `enter/exit` loglarını kaldır.
    -   **Kabul Kriterleri:**
        -   [ ] `ENV=production` veya `free` modunda, `RUST_LOG=info` ayarıyla çalışırken, loglarda artık fonksiyon giriş/çıkışlarını gösteren span olayları **görünmemelidir**.
        -   [ ] `ENV=development` modunda, `RUST_LOG=debug` ayarıyla çalışırken, bu detaylı span olayları hata ayıklama için **görünür olmalıdır**.

---

### **FAZ 2: Gelişmiş SIP Yetenekleri**

**Amaç:** Platformu, standart SIP istemcilerinin bağlanabildiği ve daha karmaşık çağrı senaryolarını yönetebilen tam teşekküllü bir SIP sunucusuna dönüştürmek.

-   [ ] **Görev ID: SIG-001 - `REGISTER` Metodu Desteği**
    -   **Açıklama:** SIP istemcilerinin (softphone, mobil uygulama) platforma kayıt olmasını (`REGISTER`) ve `user-service` üzerinden kimlik doğrulaması yapmasını sağla. Bu, platformdan dışarıya doğru arama yapmanın ilk adımıdır.
    -   **Kabul Kriterleri:**
        -   [ ] Gelen `REGISTER` isteği ayrıştırılmalı (parse edilmeli).
        -   [ ] İsteğin `Authorization` başlığındaki kimlik bilgileri `user-service`'e danışılarak doğrulanmalı.
        -   [ ] Başarılı kayıt durumunda, kullanıcının `Contact` adresi belirli bir süre (`expires`) için hafızada (örn: Redis) tutulmalı.
        -   [ ] İstemciye `200 OK` veya `401 Unauthorized` gibi uygun bir yanıt dönülmeli.

-   [ ] **Görev ID: SIG-002 - `CANCEL` Metodu Desteği**
    -   **Açıklama:** Bir `INVITE` isteği gönderildikten sonra, ancak `200 OK` yanıtı alınmadan önce çağrının iptal edilmesini sağlayan `CANCEL` isteğini işle.
    -   **Kabul Kriterleri:**
        -   [ ] Gelen `CANCEL` isteği, `Call-ID` ve `CSeq` başlıkları üzerinden bekleyen `INVITE` işlemiyle eşleştirilmeli.
        -   [ ] Eşleşen `INVITE` için ayrılan `media-service` portu derhal serbest bırakılmalı (`ReleasePort`).
        -   [ ] İstemciye `200 OK` (CANCEL için) ve ardından `487 Request Terminated` (orijinal INVITE için) yanıtları gönderilmeli.

-   [ ] **Görev ID: SIG-003 - Çağrı Transferi (`REFER`)**
    -   **Açıklama:** Aktif bir çağrıyı başka bir SIP kullanıcısına veya harici bir numaraya yönlendirmeyi sağlayan `REFER` metodunu implemente et.
    -   **Durum:** ⬜ Planlandı.