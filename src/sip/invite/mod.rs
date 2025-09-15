// File: src/sip/invite/mod.rs
// Bu modül, bir INVITE isteğinin işlenmesiyle ilgili tüm mantığı içerir.

pub mod handler;
pub mod orchestrator;
// response_builder modülü artık gereksiz olduğu için kaldırıldı.

// Ana `sip` modülünün kolayca erişebilmesi için `handler` fonksiyonunu public yapıyoruz.
pub use handler::handle;