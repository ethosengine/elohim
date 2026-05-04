# Static nginx container for Storybook.
# Storybook build happens in Jenkins via `pnpm --filter elohim-library run build-storybook`,
# producing app/elohim-library/dist/storybook/. This Dockerfile packages the pre-built
# static assets — no Node, no build tools at runtime.

FROM nginx:alpine

# Copy pre-built storybook static bundle
COPY elohim-library/dist/storybook /usr/share/nginx/html

# Custom nginx config for gzip + sane cache headers on hashed assets
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
