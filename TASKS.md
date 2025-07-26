# Sentiric SIP Signaling Service - Görev Listesi ve Yol Haritası

Bu belge, servisin geliştirme önceliklerini ve gelecekte eklenecek SIP özelliklerini takip eder.

---

### Faz 1: Çekirdek Çağrı Kurulumu (Core Call Setup)

Bu fazın amacı, platformun temel gelen çağrı (`INVITE`) akışını uçtan uca, sağlam bir şekilde yönetmektir.

-   [x] **Temel `INVITE` İşleme:** Gelen SIP `INVITE` isteklerini alma ve yanıtlama.
-   [x] **gRPC Orkestrasyonu:** `user`, `dialplan` ve `media` servislerine gRPC ile danışma.
-   [x] **Asenkron Olay Yayınlama:** Başarılı çağrı kurulumunda `call.started` olayını RabbitMQ'ya gönderme.
-   [ ] **Tam NAT Desteği:** `Record-Route` ve `Via` header'larını doğru işleyerek farklı ağ topolojilerinde kararlı çalışma.

---

### Faz 2: Kullanıcı Kaydı ve Durum Yönetimi (Registration & Presence)

Bu faz, platformu statik bir çağrı alıcısından, kullanıcıların bağlanabildiği dinamik bir SIP sunucusuna dönüştürecektir.

-   [ ] **`REGISTER` Metodu Desteği**
    -   **Görev ID:** `sip-task-001`
    -   **Açıklama:** SIP istemcilerinin (softphone, mobil uygulama) platforma kayıt olmasını (`REGISTER`) ve kimlik doğrulaması yapmasını sağla. Bu, `user-service` ile entegre çalışacaktır.
    -   **Durum:** ⬜ Planlandı.

-   [ ] **Durum Yönetimi (`SUBSCRIBE`/`NOTIFY`)**
    -   **Görev ID:** `sip-task-002`
    -   **Açıklama:** Kullanıcıların "meşgul", "müsait" gibi durumlarını (presence) yönet ve diğer kullanıcılara bildir.
    -   **Durum:** ⬜ Planlandı.

---

### Faz 3: Gelişmiş Çağrı Kontrolü (Advanced Call Control)

-   [ ] **Çağrı Sonlandırma (`BYE`) ve İptal (`CANCEL`)**
    -   **Görev ID:** `sip-task-003`
    -   **Açıklama:** Aktif bir çağrının sonlandırılması veya kurulum aşamasında iptal edilmesi senaryolarını yönet. İlgili `call.ended` olayını RabbitMQ'ya yayınla.
    -   **Durum:** ⬜ Planlandı.

-   [ ] **Çağrı Transferi (`REFER`)**
    -   **Görev ID:** `sip-task-004`
    -   **Açıklama:** Bir çağrıyı başka bir SIP kullanıcısına veya harici bir numaraya yönlendirme yeteneği ekle.
    -   **Durum:** ⬜ Planlandı.