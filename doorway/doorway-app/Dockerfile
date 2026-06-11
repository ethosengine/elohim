# Doorway Operator Dashboard
# THIN image: the Angular build happens OUTSIDE this Dockerfile, in the pnpm
# workspace. doorway-app depends on @elohim/identity via workspace:* plus
# tsconfig paths into ../../app/elohim-library — both unreachable from this
# build context, so an in-image standalone `pnpm install` cannot resolve them
# (edge #1057: ERR_PNPM_WORKSPACE_PKG_NOT_FOUND). CI builds in the
# 'Build Doorway App' stage of elohim/holochain/Jenkinsfile.
#
# Build: pnpm install --frozen-lockfile --filter doorway-app... \
#        && pnpm --filter doorway-app build \
#        && docker build -t doorway-app .
# Run:   docker run -p 8080:80 doorway-app

FROM nginx:alpine

# Copy custom nginx config
COPY nginx.conf /etc/nginx/conf.d/default.conf

# Copy built Angular app (baseHref=/threshold/ is set in angular.json
# production config; path matches angular.json "outputPath" + /browser)
COPY dist/doorway-app/browser /usr/share/nginx/html

# Expose port 80
EXPOSE 80

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://localhost/health || exit 1

CMD ["nginx", "-g", "daemon off;"]
