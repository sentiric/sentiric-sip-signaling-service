# --- AŞAMA 1: Derleme (Builder - Statik Derleme için MUSL Hedefi) ---
FROM rust:1.88-bullseye AS builder

# MUSL target'ını ve gerekli derleme araçlarını kuruyoruz (buf dahil).
RUN rustup target add x86_64-unknown-linux-musl
RUN apt-get update && \
    apt-get install -y musl-tools protobuf-compiler git curl && \
    curl -sSL https://github.com/bufbuild/buf/releases/latest/download/buf-Linux-x86_64 -o /usr/local/bin/buf && \
    chmod +x /usr/local/bin/buf && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Bağımlılıkları önbelleğe al
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release --target x86_64-unknown-linux-musl

# Kaynak kodunu kopyala ve asıl derlemeyi yap
COPY src ./src
RUN cargo build --release --target x86_64-unknown-linux-musl

# --- AŞAMA 2: Çalıştırma (Runtime) ---
# Bu servis TLS sertifikalarını okuduğu için ca-certificates'e ihtiyacı var.
FROM alpine:latest
RUN apk add --no-cache ca-certificates

ARG SERVICE_NAME
WORKDIR /app
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/${SERVICE_NAME} .

USER 10001

ENTRYPOINT ["./sentiric-sip-signaling-service"]