# 🚦 Sentiric SIP Signaling Service - Görev Listesi

Bu belge, servisin geliştirme önceliklerini ve gelecekte eklenecek SIP özelliklerini takip eder.

---

### Faz 1: Çekirdek Çağrı Kurulum ve Sonlandırma (Mevcut Durum)

Bu fazın amacı, platformun temel gelen çağrı (`INVITE`) ve sonlandırma (`BYE`) akışını uçtan uca, sağlam bir şekilde yönetmektir.

-   [x] **`INVITE` İşleme:** Gelen SIP `INVITE` isteklerini alma ve `100 Trying`, `180 Ringing`, `200 OK` yanıtlarını üretme.
-   [x] **gRPC Orkestrasyonu:** `user`, `dialplan` ve `media` servislerine sıralı ve güvenli (mTLS) gRPC çağrıları yapma.
-   [x] **Asenkron Olay Yayınlama:** `call.started` ve `call.ended` olaylarını RabbitMQ'ya gönderme.
-   [x] **`BYE` İşleme:** Aktif bir çağrıyı sonlandırma, ilgili medya portunu serbest bıraktırma ve `call.ended` olayını yayınlama.
-   [x] **Aktif Çağrı Takibi:** Devam eden çağrıları ve ilgili port/trace ID'lerini hafızada tutma.

---

### Faz 2: Gelişmiş Çağrı Kontrolü ve Kullanıcı Kaydı (Sıradaki Öncelik)

Bu faz, platformu statik bir çağrı alıcısından, kullanıcıların bağlanabildiği dinamik bir SIP sunucusuna dönüştürecektir.

-   [ ] **Görev ID: SIG-001 - `REGISTER` Metodu Desteği**
    -   **Açıklama:** SIP istemcilerinin (softphone, mobil uygulama) platforma kayıt olmasını (`REGISTER`) ve `user-service` üzerinden kimlik doğrulaması yapmasını sağla. Bu, platformdan dışarıya doğru arama yapmanın ilk adımıdır.
    -   **Durum:** ⬜ Planlandı.

-   [ ] **Görev ID: SIG-002 - `CANCEL` Metodu Desteği**
    -   **Açıklama:** Bir `INVITE` isteği gönderildikten sonra, ancak `200 OK` yanıtı alınmadan önce çağrının iptal edilmesini sağlayan `CANCEL` isteğini işle. İlgili medya portunu serbest bırak.
    -   **Durum:** ⬜ Planlandı.

-   [ ] **Görev ID: SIG-003 - Çağrı Transferi (`REFER`)**
    -   **Açıklama:** Bir agent'ın veya AI'ın, aktif bir çağrıyı başka bir SIP kullanıcısına veya harici bir numaraya yönlendirmesini sağlayan `REFER` metodunu implemente et.
    -   **Durum:** ⬜ Planlandı.