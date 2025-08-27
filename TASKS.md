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

- [x] **Görev ID: SIG-005 - Çağrı Sonlandırma Olayını Dinleme (KRİTİK)**
    -   **Açıklama:** `call.terminate.request` olaylarını dinleyecek yeni bir RabbitMQ tüketicisi (consumer) oluştur. Bu olay geldiğinde, ilgili `call_id` için aktif SIP oturumunu bul ve istemciye bir `BYE` paketi göndererek çağrıyı sonlandır.
    -   **Kabul Kriterleri:**
        -   [ ] Servis, `sentiric.signaling.terminate` gibi kendine özel, kalıcı bir kuyruğu dinlemelidir.
        -   [ ] Gelen `call_id` için `active_calls` haritasından ilgili `SocketAddr` bilgisi bulunmalıdır.
        -   [ ] Standart bir SIP `BYE` paketi oluşturulup bu adrese gönderilmelidir.
        -   [ ] İlgili `media-service` portu serbest bırakılmalı ve `call.ended` olayı yayınlanmalıdır.
        
-   [x] **Görev ID: SIG-004 - Fazla Konuşkan Loglamayı Düzeltme (KRİTİK & ACİL)**
    -   **Açıklama:** `src/main.rs` dosyasındaki `tracing` yapılandırmasını, `OBSERVABILITY_STANDARD.md`'ye uygun hale getirerek `INFO` seviyesindeki gereksiz `enter/exit` loglarını kaldır.
    -   **Kabul Kriterleri:**
        -   [ ] `ENV=production` veya `free` modunda, `RUST_LOG=info` ayarıyla çalışırken, loglarda artık fonksiyon giriş/çıkışlarını gösteren span olayları **görünmemelidir**.
        -   [ ] `ENV=development` modunda, `RUST_LOG=debug` ayarıyla çalışırken, bu detaylı span olayları hata ayıklama için **görünür olmalıdır**.

- [ ] **Görev ID: SIG-006 - Kodun Modülerleştirilmesi (Refactoring - YÜKSEK ÖNCELİK)**
    -   **Açıklama:** `src/main.rs` dosyası, tüm mantığı tek bir yerde toplayarak hızla büyümekte ve yönetilmesi zorlaşmaktadır. Kod tabanını daha sürdürülebilir, test edilebilir ve anlaşılır hale getirmek için projenin Rust modül sistemine uygun olarak yeniden yapılandırılması gerekmektedir.
    -   **Risk:** Mevcut yapı, yeni geliştiricilerin projeye adapte olmasını zorlaştırır, hata ayıklama süresini uzatır ve yeni özelliklerin eklenmesini yavaşlatır.
    -   **Önerilen Yeni Dosya Yapısı:**
        ```
        src/
        ├── main.rs           # Sadece uygulamanın başlangıç noktası, ana döngü ve temel kurulum.
        ├── config.rs         # AppConfig struct'ı ve çevre değişkenlerinden yükleme mantığı.
        ├── sip/              # SIP ile ilgili tüm mantık
        │   ├── handler.rs    # Gelen UDP paketlerini alıp `INVITE`, `BYE` vb. için yönlendiren ana fonksiyon.
        │   ├── invite.rs     # `handle_invite` mantığı.
        │   ├── bye.rs        # `handle_bye` mantığı.
        │   ├── utils.rs      # `parse_complex_headers`, `create_response` gibi yardımcı fonksiyonlar.
        │   └── mod.rs        # sip modülünü ve alt modüllerini tanımlar.
        ├── grpc/             # gRPC istemcileriyle ilgili mantık
        │   ├── client.rs     # Güvenli gRPC kanalı oluşturma ve istemci başlatma mantığı.
        │   └── mod.rs
        ├── rabbitmq/         # RabbitMQ ile ilgili mantık
        │   ├── publisher.rs  # Olay yayınlama mantığı.
        │   ├── consumer.rs   # `listen_for_termination_requests` mantığı.
        │   └── mod.rs
        └── state.rs          # `ActiveCallInfo`, `ActiveCalls` type alias'ı ve `cleanup_old_transactions` gibi durum yönetimi.
        ```
    -   **Kabul Kriterleri:**
        -   [ ] `src/main.rs` dosyasının boyutu önemli ölçüde küçülmeli ve sadece uygulamanın ana iskeletini içermelidir.
        -   [ ] Sorumluluklar (SIP, gRPC, RabbitMQ, Config, State) ayrı modüllere ve dosyalara dağıtılmalıdır.
        -   [ ] Yeniden yapılandırma sonrasında mevcut tüm işlevsellik (`INVITE`, `BYE`, `terminate` vb.) eksiksiz olarak çalışmaya devam etmelidir.
        -   [ ] Proje `cargo build` ve `cargo clippy` komutlarından hatasız ve uyarısız geçmelidir.
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