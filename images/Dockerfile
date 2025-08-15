# Multi-stage build for Angular app

# Stage 1: Build the Angular application
FROM node:18-alpine AS builder

WORKDIR /app

# Copy package files
COPY elohim-app/package*.json ./

# Install dependencies
RUN npm ci --only=production

# Copy source code
COPY elohim-app/ ./

# Build the application for production
RUN npm run build

# Stage 2: Serve with nginx
FROM nginx:alpine

# Copy built application from builder stage
COPY --from=builder /app/dist/elohim-app /usr/share/nginx/html

# Copy nginx configuration (optional - using default nginx config)
# COPY nginx.conf /etc/nginx/nginx.conf

# Expose port 80
EXPOSE 80

# Start nginx
CMD ["nginx", "-g", "daemon off;"]