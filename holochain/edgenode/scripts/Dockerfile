# hApp Installer Container
# Installs the hApp on conductor startup as a sidecar

# Use slim instead of alpine for better ESM module compatibility
# (libsodium-wrappers has ESM resolution issues on Alpine)
FROM node:20-slim

WORKDIR /app

# Copy package files and install dependencies
COPY package.json ./
RUN npm install --production

# Copy installer script
COPY install-happ.mjs ./

# Copy hApp bundle (passed in during build via --build-arg or COPY)
# Default location matches script's default HAPP_PATH
RUN mkdir -p /opt/holochain
COPY elohim.happ /opt/holochain/elohim.happ

# Run installer, then sleep to keep container alive
# (Kubernetes restarts containers that exit)
CMD ["sh", "-c", "node install-happ.mjs && echo 'Sleeping to keep container alive...' && sleep infinity"]
