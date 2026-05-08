# Simple nginx container for pre-built Angular app
# The Angular build happens in Jenkins with the correct configuration
# This Dockerfile just packages the pre-built artifacts

FROM nginx:alpine

# Copy pre-built application (built by Jenkins with correct config)
COPY elohim-app/dist/elohim-app/browser /usr/share/nginx/html

# Copy nginx configuration for SPA routing
COPY nginx.conf /etc/nginx/conf.d/default.conf

# Build-time verification — fail loudly if dist was empty at COPY time.
# Without this guard, an empty dist silently ships an image that serves the
# nginx:alpine default "Welcome to nginx!" page (since /usr/share/nginx/html
# from the base image stays in place when the COPY contributes nothing).
# That's exactly the regression on alpha that the doorway-ssr-deliver shift
# (2026-05-08) discovered: stale image, no SPA, last-modified a month ago.
# This RUN aborts the build with a clear error if index.html or main.*.js
# (Angular 19 hashed bundle) are missing.
RUN test -f /usr/share/nginx/html/index.html || { \
        echo "FATAL: /usr/share/nginx/html/index.html missing — Angular dist was empty at build time"; \
        echo "Check that 'pnpm exec ng build --configuration=<env>' ran successfully BEFORE docker build"; \
        ls -la /usr/share/nginx/html/ || true; \
        exit 1; \
    } && \
    if ! ls /usr/share/nginx/html/main-*.js >/dev/null 2>&1 && ! ls /usr/share/nginx/html/main.*.js >/dev/null 2>&1; then \
        echo "FATAL: no Angular main bundle found in /usr/share/nginx/html/"; \
        echo "Expected main-<hash>.js or main.<hash>.js — directory listing:"; \
        ls -la /usr/share/nginx/html/; \
        exit 1; \
    fi

# Expose port 80
EXPOSE 80

# Start nginx
CMD ["nginx", "-g", "daemon off;"]