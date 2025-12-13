# Simple nginx container for pre-built Angular app
# The Angular build happens in Jenkins with the correct configuration
# This Dockerfile just packages the pre-built artifacts

FROM nginx:alpine

# Copy pre-built application (built by Jenkins with correct config)
COPY elohim-app/dist/elohim-app/browser /usr/share/nginx/html

# Copy nginx configuration for SPA routing
COPY nginx.conf /etc/nginx/conf.d/default.conf

# Expose port 80
EXPOSE 80

# Start nginx
CMD ["nginx", "-g", "daemon off;"]