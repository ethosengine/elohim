# elohim-node: Alpine-based minimal container
#
# Multi-stage build:
# 1. Build stage: Compile Rust binary
# 2. Runtime stage: Minimal Alpine with just the binary

# =============================================================================
# BUILD STAGE
# =============================================================================
FROM rust:1.75-alpine AS builder

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconfig \
    protobuf-dev

WORKDIR /build

# Cache dependencies by copying Cargo files first
COPY Cargo.toml Cargo.lock* ./

# Create dummy src to build dependencies
RUN mkdir -p src && \
    echo 'fn main() { println!("dummy"); }' > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy actual source and build
COPY src ./src
RUN touch src/main.rs && \
    cargo build --release --locked

# =============================================================================
# RUNTIME STAGE
# =============================================================================
FROM alpine:3.19 AS runtime

# Runtime dependencies (minimal)
RUN apk add --no-cache \
    ca-certificates \
    libgcc \
    tini

# Create non-root user
RUN addgroup -g 1000 elohim && \
    adduser -u 1000 -G elohim -h /home/elohim -D elohim

# Create data directory
RUN mkdir -p /var/lib/elohim && \
    chown elohim:elohim /var/lib/elohim

# Copy binary from builder
COPY --from=builder /build/target/release/elohim-node /usr/local/bin/elohim-node

# Default configuration
COPY --chown=elohim:elohim elohim-node.toml /etc/elohim/elohim-node.toml

USER elohim
WORKDIR /home/elohim

# Expose ports
# 4001 - libp2p (TCP/QUIC)
# 8080 - HTTP API
# 9090 - gRPC API
EXPOSE 4001 4001/udp 8080 9090

# Data volume
VOLUME /var/lib/elohim

# Use tini as init to handle signals properly
ENTRYPOINT ["/sbin/tini", "--"]

# Default command
CMD ["elohim-node", "--config", "/etc/elohim/elohim-node.toml"]

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget -q --spider http://localhost:8080/health || exit 1

# Labels
LABEL org.opencontainers.image.title="elohim-node"
LABEL org.opencontainers.image.description="Elohim Protocol infrastructure runtime"
LABEL org.opencontainers.image.source="https://github.com/ethosengine/elohim"
