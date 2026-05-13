FROM rust:1.95-bookworm as builder

WORKDIR /app

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

COPY . .

RUN cargo build --target x86_64-unknown-linux-musl --release -p hse-document-processor

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    poppler-utils \
    ffmpeg \
    libreoffice-writer \
    libreoffice-impress \
    libreoffice-common \
    && rm -rf /var/lib/apt/lists/*

RUN curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux -o /usr/local/bin/yt-dlp \
    && chmod a+rx /usr/local/bin/yt-dlp

COPY --from=builder /app/target/release/hse-document-processor ./hse-document-processor

EXPOSE 8000
ENTRYPOINT ["./hse-document-processor"]