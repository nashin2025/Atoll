# Multi-stage Dockerfile for Atoll Node
# Stage 1: Build the Rust binary
FROM rust:1.75 AS builder

WORKDIR /app

# Copy Cargo files first for dependency caching
COPY Cargo.toml Cargo.lock ./

# Create dummy source to build dependencies
RUN mkdir src && echo 'fn main() {}' > src/main.rs && cargo build --release

# Remove dummy source and copy real source
RUN rm -rf src
COPY src ./src
COPY Cargo.toml Cargo.lock ./

# Build the release binary
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/atoll-node /usr/local/bin/atoll-node

# Create non-root user
RUN useradd -m -s /bin/bash atoll
USER atoll

# Expose ports
EXPOSE 7777 8080 8081 8082 9090

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1

ENTRYPOINT ["/usr/local/bin/atoll-node"]
