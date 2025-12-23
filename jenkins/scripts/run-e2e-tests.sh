#!/bin/bash
set -euo pipefail

BASE_URL=$1
ENV=$2
GIT_COMMIT_HASH=$3

export CYPRESS_baseUrl=$BASE_URL
export CYPRESS_ENV=$ENV
export CYPRESS_EXPECTED_GIT_HASH=$GIT_COMMIT_HASH
export NO_COLOR=1
export DISPLAY=:99

# Start Xvfb
Xvfb :99 -screen 0 1024x768x24 -ac > /dev/null 2>&1 &
XVFB_PID=$!
sleep 2

# Verify Cypress
npx cypress verify > /dev/null
mkdir -p cypress/reports

# Run tests
npx cypress run \
    --headless \
    --browser chromium \
    --spec "cypress/e2e/staging-validation.feature"

# Cleanup
kill $XVFB_PID 2>/dev/null || true
