# 🚦 SIP Signaling Service - Görev Listesi (v2.2 - Veri Bütünlüğü Odaklı)

Bu belge, `sip-signaling-service`'in geliştirme yol haritasını, tamamlanan kritik kilometre taşlarını ve gelecekteki hedeflerini tanımlar.

---

### **FAZ 1: Uçtan Uca Veri Akışı Düzeltmesi (Mevcut Odak)**

**Amaç:** Platformdaki en kritik veri akışı kopukluğunu gidermek ve `agent-service`'in çağrıyı yapan kullanıcıyı tanımasını sağlamak için gerekli olan zenginleştirilmiş olayı yayınlamak.

-   **Görev ID: SIG-FIX-01 - `call.started` Olayını Kullanıcı Bilgileriyle Zenginleştirme**
    -   **Durum:** x **Yapılacak (Öncelik 1 - KRİTİK)**
    -   **Bağımlılık:** `sentiric-contracts` deposunda `CT-FIX-01` görevinin tamamlanmış ve yeni bir sürümün yayınlanmış olması.
    -   **Problem:** `call.started` olayı, `agent-service`'in kullanıcıyı tanıması için gereken `dialplan` ve `user` bilgilerini içermemektedir.
    -   **Çözüm:**
        -   [x] `Cargo.toml` dosyasındaki `sentiric-contracts` bağımlılığı, `CT-FIX-01` görevini içeren en son sürüme güncellenmelidir.
        -   [x] `src/sip/invite/orchestrator.rs` içindeki `publish_call_event` fonksiyonu, parametre olarak `ResolveDialplanResponse` nesnesini almalıdır.
        -   [x] `call.started` olayı oluşturulurken, bu `ResolveDialplanResponse` nesnesi, yeni kontratlardaki `dialplan_resolution` alanına atanmalıdır.

-   **Görev ID: SIG-CLEANUP-01 - Gereksiz `call.answered` Olayını Kaldırma**
    -   **Durum:** ⬜ **Yapılacak (Öncelik 2 - DÜŞÜK)**
    -   **Problem:** `agent-service` tarafından işlenmeyen, gereksiz bir `call.answered` olayı yayınlanıyor.
    -   **Çözüm:**
        -   [ ] `src/sip/invite/orchestrator.rs` içindeki `setup_and_finalize_call` fonksiyonundan `call.answered` olayını yayınlayan kod satırı kaldırılmalıdır.