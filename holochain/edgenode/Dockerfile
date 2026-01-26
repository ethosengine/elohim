# Custom Edge Node with Elohim conductor config
#
# Based on Holo Edge Node, adds:
# - Custom conductor config for elohim bootstrap server
#
# NOTE: hApp is NOT baked into this image. It is fetched at runtime
# by an init container and mounted via a shared volume.
# See: holochain/manifests/edgenode-*.yaml

FROM ghcr.io/holo-host/edgenode:v0.0.8-alpha31-hc0.6.0-go-pion-custom

# Copy custom conductor config
COPY conductor-config.yaml /etc/holochain/conductor-config.yaml

# hApp will be mounted at /opt/holochain via Kubernetes volume
# (fetched by init container from elohim-holochain pipeline artifacts)

# Expose WebSocket ports
EXPOSE 4444 4445

ENV CONDUCTOR_MODE=true

# Default command from base image starts conductor
