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

RUN cargo build --release --target x86_64-unknown-linux-musl --bin hse-kb-mcp

FROM scratch
WORKDIR /app

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/hse-kb-mcp ./hse-kb-mcp

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

EXPOSE 8000
ENTRYPOINT ["./hse-kb-mcp"]