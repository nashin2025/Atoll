# syntax=docker/dockerfile:1

# ---------------------------------------------------------------------------
# Build stage: compile the atoll-node binary in a Rust container.
# ---------------------------------------------------------------------------
FROM rust:1-slim AS builder
WORKDIR /app/core

# System packages needed to compile native Rust crates.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        ca-certificates \
        build-essential \
    && rm -rf /var/lib/apt/lists/*

# Copy manifest and source files for the core crate only.
COPY core/Cargo.toml .
COPY core/src ./src
COPY core/benches ./benches

# Download dependencies and build the release binary.
RUN cargo fetch
RUN cargo build --release -p atoll-core

# ---------------------------------------------------------------------------
# Runtime stage: debian:bookworm-slim container with wget-based healthcheck.
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        wget \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -s /bin/false nonroot

COPY --from=builder /app/core/target/release/atoll-node /usr/local/bin/atoll-node
COPY config/default.config.toml /etc/atoll/config.toml

EXPOSE 8080/tcp
EXPOSE 30333/tcp

HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
    CMD ["wget", "--spider", "-q", "http://localhost:8080/health"] || exit 1

USER nonroot
ENTRYPOINT ["/usr/local/bin/atoll-node", "--config", "/etc/atoll/config.toml"]
