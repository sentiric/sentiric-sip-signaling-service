# --- AŞAMA 1: Derleme (Builder) ---
FROM rust:1.88-slim-bookworm AS builder

# Gerekli derleme araçlarını ve YENİ olarak buf CLI'ı kuruyoruz.
RUN apt-get update && \
    apt-get install -y protobuf-compiler git curl && \
    curl -sSL https://github.com/bufbuild/buf/releases/latest/download/buf-Linux-x86_64 -o /usr/local/bin/buf && \
    chmod +x /usr/local/bin/buf && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY . .

RUN cargo build --release

# --- AŞAMA 2: Çalıştırma (Runtime) ---
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y netcat-openbsd ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/sentiric-sip-signaling-service .

ENTRYPOINT ["./sentiric-sip-signaling-service"]