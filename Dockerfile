# syntax=docker/dockerfile:1

# ---------------------------------------------------------------------------
# Build stage: compile the atoll-node binary in a Rust container.
# ---------------------------------------------------------------------------
FROM rust:1-slim AS builder
WORKDIR /app

# System packages needed to compile native Rust crates.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        ca-certificates \
        build-essential \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests first so dependency downloads are cached in a
# separate layer and only re-run when Cargo.lock changes.
COPY Cargo.lock Cargo.toml ./
COPY core/Cargo.toml core/Cargo.toml

# Docker build only needs the core crate; strip GUI/CLI workspace members
# from Cargo.toml so cargo fetch doesn't try to parse their manifests.
RUN sed -i '/wallet\//d' Cargo.toml

# Download and cache dependencies separately from source compilation.
RUN cargo fetch --locked

# Copy sources and build the release binary.
COPY core/src core/src
COPY core/benches core/benches
RUN cargo build --release --locked -p atoll-core

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

COPY --from=builder /app/target/release/atoll-core /usr/local/bin/atoll-node
COPY config/default.config.toml /etc/atoll/config.toml

EXPOSE 8080/tcp
EXPOSE 30333/tcp

HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
    CMD ["wget", "--spider", "-q", "http://localhost:8080/health"] || exit 1

USER nonroot
ENTRYPOINT ["/usr/local/bin/atoll-node", "--config", "/etc/atoll/config.toml"]
