# Custom Edge Node with Elohim conductor config
#
# Based on Holo Edge Node, adds:
# - Custom conductor config for elohim bootstrap server
# - Pre-installed hApp

FROM ghcr.io/holo-host/edgenode:v0.0.8-alpha31-hc0.6.0-go-pion-custom

# Copy custom conductor config
COPY conductor-config.yaml /etc/holochain/conductor-config.yaml

# Copy hApp bundle
COPY elohim.happ /opt/holochain/

# Expose WebSocket ports
EXPOSE 4444 4445

ENV CONDUCTOR_MODE=true

# Default command from base image starts conductor
