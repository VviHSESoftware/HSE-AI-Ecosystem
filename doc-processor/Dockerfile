FROM lukemathwalker/cargo-chef:latest-rust-trixie AS planner
WORKDIR /app

COPY . .

RUN cargo chef prepare --recipe-path recipe.json

FROM lukemathwalker/cargo-chef:latest-rust-trixie AS cacher
WORKDIR /app
RUN apt-get update && apt-get install -y musl-tools clang && rm -rf /var/lib/apt/lists/*
RUN rustup target add x86_64-unknown-linux-musl

COPY --from=planner /app/recipe.json recipe.json

RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json

FROM cacher AS builder

COPY . .

RUN cargo build --release --target x86_64-unknown-linux-musl --bin hse-document-processor


FROM debian:trixie-slim

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

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/hse-document-processor ./hse-document-processor

EXPOSE 8000
ENTRYPOINT ["./hse-document-processor"]