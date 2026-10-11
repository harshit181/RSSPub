FROM rust:1.92.0-slim-trixie AS builder

WORKDIR /usr/src/rsspub

RUN apt-get update && apt-get install -y pkg-config build-essential && rm -rf /var/lib/apt/lists/*

COPY . .

ARG RUSTFLAGS="-C target-cpu=x86-64-v2"
ENV RUSTFLAGS=${RUSTFLAGS}

RUN cargo build --release --features alternative-alloc

FROM oven/bun:alpine AS ui-builder

WORKDIR /project

COPY ui/package.json ui/bun.lock* ./
RUN bun install --frozen-lockfile

COPY ui .
RUN bun run build

FROM debian:trixie-slim

WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/rsspub/target/release/rsspub /usr/local/bin/rsspub

COPY --from=ui-builder /static /app/static
COPY static/cover.jpg /app/static/cover.jpg
RUN mkdir -p /app/epubs
RUN mkdir -p /app/db
EXPOSE 3000

ENV RUST_LOG=info,html5ever=error
ENV MALLOC_CONF="background_thread:true,dirty_decay_ms:1000,muzzy_decay_ms:1000"

CMD ["rsspub"]
