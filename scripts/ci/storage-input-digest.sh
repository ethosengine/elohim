#!/usr/bin/env bash
# Existing storage digest entrypoint; derivation has one owner.
set -euo pipefail
exec bash "$(dirname "$0")/closure-input-digest.sh" build-storage "$@"
