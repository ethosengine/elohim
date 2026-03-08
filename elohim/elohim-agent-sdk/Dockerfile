# Elohim Agent SDK - Constitutional AI sidecar
# Multi-stage build: install deps, compile TypeScript, run minimal production image
#
# Build: docker build -t elohim-agent-sdk .
# Run:   docker run -p 8095:8095 -e ANTHROPIC_API_KEY=... elohim-agent-sdk

# Stage 1: Install and build
FROM node:20-alpine AS builder

RUN corepack enable && corepack prepare pnpm@10.30.3 --activate

WORKDIR /app

COPY package.json ./
RUN pnpm install --frozen-lockfile

COPY tsconfig.json ./
COPY src ./src
RUN pnpm build

# Stage 2: Production
FROM node:20-alpine

WORKDIR /app

COPY package.json ./
RUN corepack enable && corepack prepare pnpm@10.30.3 --activate \
    && pnpm install --frozen-lockfile --prod

COPY --from=builder /app/dist ./dist

EXPOSE 8095

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://localhost:8095/health || exit 1

USER node

CMD ["node", "dist/server.js"]
