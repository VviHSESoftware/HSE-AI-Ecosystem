FROM rust:1.95-bookworm as builder

RUN apt-get update && apt-get install -y musl-tools clang && rm -rf /var/lib/apt/lists/*

WORKDIR /app

RUN rustup target add x86_64-unknown-linux-musl

COPY . .

RUN cargo build --target x86_64-unknown-linux-musl --release -p hse-task-processor

FROM scratch

WORKDIR /app

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/hse-task-processor ./hse-task-processor

COPY --from=builder /app/ai-task-processor/prompts ./prompts
COPY --from=builder /app/ai-task-processor/migrations ./migrations

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

EXPOSE 8000
ENTRYPOINT ["./hse-task-processor"]