FROM oven/bun:1 AS client-builder
WORKDIR /app/client
COPY client/package.json client/bun.lock ./
RUN bun install --frozen-lockfile
COPY client/ ./
RUN bun run build

FROM rust:1.85-slim AS server-builder
WORKDIR /app/server
COPY server/Cargo.toml server/Cargo.lock ./
COPY server/src ./src
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=server-builder /app/server/target/release/gauntlet-week-1 /usr/local/bin/gauntlet-week-1
COPY --from=client-builder /app/client/dist /app/client/dist
ENV HOST=0.0.0.0
ENV PORT=3000
ENV STATIC_DIR=/app/client/dist
EXPOSE 3000
CMD ["gauntlet-week-1"]
