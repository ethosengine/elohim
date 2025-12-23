#!/bin/bash
set -euo pipefail

IMAGE_TAG=$1

AUTH_HEADER="Authorization: Basic $(echo -n "$HARBOR_USERNAME:$HARBOR_PASSWORD" | base64)"

# Trigger scan
wget --post-data="" \
  --header="accept: application/json" \
  --header="Content-Type: application/json" \
  --header="$AUTH_HEADER" \
  -S -O- \
  "https://harbor.ethosengine.com/api/v2.0/projects/ethosengine/repositories/elohim-site/artifacts/${IMAGE_TAG}/scan" || \
echo "Scan request failed"

# Poll for completion
MAX_ATTEMPTS=24
ATTEMPT=1

while [ $ATTEMPT -le $MAX_ATTEMPTS ]; do
    VULN_DATA=$(wget -q -O- \
      --header="accept: application/json" \
      --header="$AUTH_HEADER" \
      "https://harbor.ethosengine.com/api/v2.0/projects/ethosengine/repositories/elohim-site/artifacts/${IMAGE_TAG}/additions/vulnerabilities" 2>/dev/null || echo "")

    if [ ! -z "$VULN_DATA" ] && echo "$VULN_DATA" | grep -q '"scanner"'; then
        echo "✅ Scan completed"
        break
    fi

    [ $((ATTEMPT % 5)) -eq 0 ] && echo "Waiting for scan (attempt $ATTEMPT/$MAX_ATTEMPTS)..."
    sleep 10
    ATTEMPT=$((ATTEMPT + 1))
done
