# Build stage
FROM rust:1.83-slim-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./

# Create dummy src to build dependencies
RUN mkdir -p src && \
    echo 'fn main() { println!("placeholder"); }' > src/main.rs && \
    echo 'pub fn placeholder() {}' > src/lib.rs

# Build dependencies (this layer gets cached)
RUN cargo build --release && rm -rf src target/release/doorway*

# Copy actual source
COPY src ./src

# Build the release binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 doorway

# Copy binary from builder
COPY --from=builder /app/target/release/doorway /usr/local/bin/doorway

# Copy default config
COPY .env.example /app/.env.example

# Set ownership
RUN chown -R doorway:doorway /app

USER doorway

# Default port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run doorway
ENTRYPOINT ["doorway"]
CMD ["--listen", "0.0.0.0:8080"]
