# hApp Installer Container
# Installs the hApp on conductor startup as a sidecar
#
# NOTE: hApp is NOT baked into this image. It is fetched at runtime
# by an init container and mounted via a shared volume at /opt/holochain.
# See: holochain/manifests/edgenode-*.yaml

# Use slim instead of alpine for better ESM module compatibility
# (libsodium-wrappers has ESM resolution issues on Alpine)
FROM node:20-slim

WORKDIR /app

# Copy package files and install dependencies
COPY package.json ./
RUN npm install --production

# Copy installer script
COPY install-happ.cjs ./

# hApp will be mounted at /opt/holochain via Kubernetes volume
# (fetched by init container from elohim-holochain pipeline artifacts)
RUN mkdir -p /opt/holochain

# Run installer, then sleep to keep container alive
# (Kubernetes restarts containers that exit)
CMD ["sh", "-c", "node install-happ.cjs && echo 'Sleeping to keep container alive...' && sleep infinity"]
