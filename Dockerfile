# syntax=docker/dockerfile:1

# ---- Build both workspace binaries in one shared stage ----
FROM rust:1-bookworm AS builder
WORKDIR /app
COPY . .
# Only the two runnable services are built; the legacy SeaORM crates are skipped.
RUN cargo build --release -p it-backend -p it-bot

# ---- Minimal runtime base (TLS roots for Postgres/Anthropic/Discord) ----
FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# ---- Backend service ----
FROM runtime AS backend
COPY --from=builder /app/target/release/it-backend /usr/local/bin/it-backend
ENV BIND_ADDR=0.0.0.0:80
EXPOSE 80
CMD ["it-backend"]

# ---- Bot service ----
FROM runtime AS bot
COPY --from=builder /app/target/release/it-bot /usr/local/bin/it-bot
CMD ["it-bot"]
