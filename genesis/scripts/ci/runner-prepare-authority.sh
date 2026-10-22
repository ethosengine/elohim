#!/usr/bin/env bash
# Fresh diagnostic pods do not inherit the parent Genesis container's jq install.
# Called before binding ADMIN_KEY; mirror Verify Target Health's package contract.
set -euo pipefail
if ! command -v jq >/dev/null 2>&1; then
  echo 'Installing jq for native runner registration...'
  apt-get update -qq && apt-get install -y -qq jq 2>/dev/null || apk add --no-cache jq
fi
jq --version >/dev/null
bash "$(dirname "$0")/install-substrate-runner.sh"
