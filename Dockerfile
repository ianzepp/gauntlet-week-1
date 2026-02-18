# Stage 1: Build React/Vite client
FROM oven/bun:1 AS client-builder
WORKDIR /app/client
COPY client/package.json client/bun.lock ./
RUN bun install --frozen-lockfile
COPY client/ ./
RUN bun run build

# Stage 2: Build Leptos SSR server + WASM frontend
FROM rust:1.89-slim AS server-builder
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
RUN rustup target add wasm32-unknown-unknown
RUN cargo install cargo-leptos

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY server/ server/
COPY canvas/ canvas/
COPY client-rust/ client-rust/
RUN cargo leptos build --release

# Stage 3: Runtime
FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

# Server binary from cargo-leptos build
COPY --from=server-builder /app/target/release/gauntlet-week-1 /usr/local/bin/gauntlet-week-1

# Leptos site assets (WASM + CSS + static)
COPY --from=server-builder /app/target/site /app/site

# React static files
COPY --from=client-builder /app/client/dist /app/client/dist

ENV HOST=0.0.0.0
ENV PORT=3000
ENV LEPTOS_PORT=3001
ENV STATIC_DIR=/app/client/dist
ENV LEPTOS_SITE_ROOT=/app/site
EXPOSE 3000 3001
CMD ["sh", "-c", "gauntlet-week-1 --migrate-only && gauntlet-week-1"]
