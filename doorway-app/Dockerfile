# Doorway Operator Dashboard
# Multi-stage build: Node for Angular, nginx for serving
#
# Build: docker build -t doorway-app .
# Run:   docker run -p 8080:80 doorway-app

# Stage 1: Build Angular app
FROM node:20-alpine AS builder

WORKDIR /app

# Copy package files first for layer caching
COPY package*.json ./

# Install dependencies
RUN npm ci --legacy-peer-deps

# Copy source and build with /threshold/ base href
COPY . .
RUN npm run build -- --configuration=production --base-href=/threshold/

# Stage 2: Serve with nginx
FROM nginx:alpine

# Copy custom nginx config
COPY nginx.conf /etc/nginx/conf.d/default.conf

# Copy built Angular app
COPY --from=builder /app/dist/doorway-app/browser /usr/share/nginx/html

# Expose port 80
EXPOSE 80

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://localhost/health || exit 1

CMD ["nginx", "-g", "daemon off;"]
