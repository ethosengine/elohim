# Custom Edge Node with Elohim conductor config
#
# Based on Holo Edge Node, adds:
# - Custom conductor config for elohim bootstrap server
# - Pre-installed hApp (when available)

FROM ghcr.io/holo-host/edgenode:latest

# Copy custom conductor config
COPY conductor-config.yaml /etc/holochain/conductor-config.yaml

# Copy hApp if present (optional - can be installed at runtime)
COPY *.happ /opt/holochain/ 2>/dev/null || true

# Expose WebSocket ports
EXPOSE 4444 4445

ENV CONDUCTOR_MODE=true

# Default command from base image starts conductor
