# relay-addr-beacon — multi-stage build.
#
# Self-contained: build context is THIS directory only (no repo-root context, no
# internal path-deps). crates.io deps are routed through the Nexus mirror via an
# inline cargo config so image builds match local builds.
#
# Build (from this directory):
#   docker build -t harbor.ethosengine.com/ethosengine/relay-addr-beacon:dev-latest .

# ---- builder ---------------------------------------------------------------
FROM rust:1-slim-bookworm AS builder

WORKDIR /build

# Route crates.io through the Nexus mirror (matches the repo .cargo/config.toml).
RUN mkdir -p /usr/local/cargo && cat > /usr/local/cargo/config.toml <<'EOF'
[source.crates-io]
replace-with = "elohim-mirror"

[source.elohim-mirror]
registry = "sparse+https://nexus.ethosengine.com/repository/cargo/"
EOF

# rustls needs no OpenSSL; reqwest uses rustls-tls. Nothing else to install.
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Native build — clear the Holochain WASM getrandom flag if it leaks in.
ENV RUSTFLAGS=""
RUN cargo build --release && strip target/release/relay-addr-beacon

# ---- runtime ---------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# ca-certificates for TLS to the echo endpoints / Cloudflare / pkarr relay.
# procps provides `pkill`, which the coturn sink's on-change-exec
# (`pkill -TERM -x turnserver`) needs to signal coturn.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates procps \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/relay-addr-beacon /usr/local/bin/relay-addr-beacon

# State + pkarr key live under a writable dir the operator should mount.
VOLUME ["/var/lib/relay-addr-beacon"]

ENTRYPOINT ["/usr/local/bin/relay-addr-beacon"]
