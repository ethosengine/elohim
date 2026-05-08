# Simple nginx container for pre-built Angular app
# The Angular build happens in Jenkins with the correct configuration
# This Dockerfile just packages the pre-built artifacts

FROM nginx:alpine

# Clear nginx:alpine's baked-in welcome page BEFORE COPY so a sparse dist
# can't be silently masked by the base-image content. This was the exact
# regression the doorway-ssr-deliver shift (2026-05-08) discovered: when
# Angular 19 began emitting `index.csr.html` instead of `index.html` after
# the SSR merge (764d8787), the base image's stock `index.html` survived
# the merge-COPY and was served at every alpha request. Now: empty dist
# → 404, full dist → SPA. No more silent nginx-default fallthrough.
RUN rm -rf /usr/share/nginx/html/* /usr/share/nginx/html/.[!.]* 2>/dev/null || true

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
RUN if ! test -f /usr/share/nginx/html/index.html && ! test -f /usr/share/nginx/html/index.csr.html; then \
        echo "FATAL: neither /usr/share/nginx/html/index.html nor /index.csr.html present — Angular dist was empty at build time"; \
        echo "Check that 'pnpm exec ng build --configuration=<env>' ran successfully BEFORE docker build"; \
        ls -la /usr/share/nginx/html/ || true; \
        exit 1; \
    fi && \
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