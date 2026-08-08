# syntax=docker/dockerfile:1.7

FROM rust:1.97.1-slim-bookworm AS builder

RUN apt-get update \
    && apt-get install --yes --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
    && printf 'fn main() {}\n' > src/main.rs \
    && touch src/lib.rs \
    && cargo build --locked --release --all-features \
    && rm -rf src

COPY . .
ENV SQLX_OFFLINE=true
RUN touch src/main.rs \
    && cargo build --locked --release --all-features

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates \
    && groupadd --system --gid 10001 kaspa \
    && useradd --system --uid 10001 --gid kaspa --home-dir /nonexistent --shell /usr/sbin/nologin kaspa \
    && install -d --owner=kaspa --group=kaspa --mode=0750 /var/lib/kaspa-pulse \
    && rm -rf /var/lib/apt/lists/*

LABEL org.opencontainers.image.source="https://github.com/KaspaPulse/kaspa-telegram-notify" \
      org.opencontainers.image.licenses="MIT"

WORKDIR /app
COPY --from=builder --chown=kaspa:kaspa /app/target/release/kaspa-pulse /usr/local/bin/kaspa-pulse

ENV PANIC_EVENT_MARKER_PATH=/var/lib/kaspa-pulse/panic_event_pending.json

USER 10001:10001
ENTRYPOINT ["/usr/local/bin/kaspa-pulse"]
