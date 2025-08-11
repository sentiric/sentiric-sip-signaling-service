# --- AŞAMA 1: Derleme (Builder) ---
# En güncel ve 'slim' bir temel imaj kullanarak başlıyoruz.
# Bu, hem en son derleyici özelliklerini almamızı sağlar hem de imaj boyutunu küçültür.
FROM rust:1.88-slim-bookworm AS builder

# Gerekli derleme araçlarını kuruyoruz.
RUN apt-get update && apt-get install -y protobuf-compiler clang libclang-dev pkg-config

WORKDIR /app

# Bağımlılıkları önbelleğe almak için önce sadece Cargo dosyalarını kopyala
COPY Cargo.toml Cargo.lock ./
# Sahte bir src dizini oluşturarak sadece bağımlılıkları derle
RUN mkdir src && echo "fn main() {}" > src/main.rs
# --release olmadan derlemek, sadece bağımlılıkları çekmeyi hızlandırır.
RUN cargo build

# Kaynak kodunu kopyala ve asıl derlemeyi yap
COPY src ./src
# Artık production için optimize edilmiş tam bir build yapıyoruz.
RUN cargo build --release

# --- AŞAMA 2: Çalıştırma (Runtime) ---
# Mümkün olan en küçük ve en güvenli imajlardan birini kullanıyoruz.
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y netcat-openbsd ca-certificates && rm -rf /var/lib/apt/lists/*

ARG SERVICE_NAME
WORKDIR /app
COPY --from=builder /app/target/release/${SERVICE_NAME} .

ENTRYPOINT ["./sentiric-sip-signaling-service"]