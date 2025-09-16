# 🚦 SIP Signaling Service - Görev Listesi (v2.1 - Strateji B+ Mimarisi)

Bu belge, `sip-signaling-service`'in geliştirme yol haritasını, tamamlanan kritik kilometre taşlarını ve gelecekteki hedeflerini tanımlar.

---

### **FAZ 2: Strateji B+ ve Mimari Sağlamlaştırma (Tamamlandı)**

-   [x] **MIMARI-02 - Strateji B+ ile Sorumlulukların Ayrıştırılması**
-   [x] **MIMARI-01 - Dayanıklı ve Anında Yanıt Veren Başlangıç Mimarisi**
-   [x] **SIG-BUG-02 - Yinelenen INVITE İsteklerine Karşı Dayanıklılık**

---

### **FAZ 3: Zenginleştirilmiş Olaylar ve Temizlik (Mevcut Odak)**

**Amaç:** Platformun geri kalanına daha zengin ve temiz veri sağlayarak asenkron iş akışlarının doğru çalışmasını garanti altına almak.

-   **Görev ID: SIG-FEAT-01 - `call.started` Olayını Kullanıcı Bilgileriyle Zenginleştirme**
    -   **Durum:** ⬜ **Yapılacak (Öncelik 1 - KRİTİK)**
    -   **Bağımlılık:** `sentiric-contracts`'teki `CT-FEAT-01` görevinin tamamlanmış olması.
    -   **Açıklama:** Loglarda görülen veri bütünlüğü sorununu çözmek için, `dialplan-service`'ten alınan `ResolveDialplanResponse` nesnesinin tamamını, yeni kontratlara uygun olarak `call.started` olayının `dialplan_resolution` alanına eklemek. Bu, `agent-service`'in arayanı doğru bir şekilde tanımasını sağlayacaktır.
    -   **Kabul Kriterleri:**
        -   [ ] `sip/invite/orchestrator.rs` içindeki `publish_call_event` fonksiyonu, `dialplan_res` parametresini almalı ve `serde_json` kullanarak `event_payload`'a eklemelidir.
        -   [ ] Yapılan bir test aramasında, RabbitMQ'ya giden `call.started` mesajının içinde `dialplan` anahtarının ve altında `matchedUser` bilgilerinin olduğu doğrulanmalıdır.

-   **Görev ID: SIG-CLEANUP-01 - Gereksiz `call.answered` Olayını Kaldırma**
    -   **Durum:** ⬜ **Yapılacak (Öncelik 2)**
    -   **Açıklama:** Loglarda `agent-service`'in `call.answered` olayını `Bilinmeyen olay türü, görmezden geliniyor.` mesajıyla işlediği görülmektedir. Bu olay gereksizdir ve sistemdeki gürültüyü azaltmak için kaldırılmalıdır.
    -   **Kabul Kriterleri:**
        -   [ ] `sip/invite/orchestrator.rs` içindeki `setup_and_finalize_call` fonksiyonundan `call.answered` olayını yayınlayan kod satırı kaldırılmalıdır.