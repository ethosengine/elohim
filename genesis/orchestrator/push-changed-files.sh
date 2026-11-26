#!/usr/bin/env bash
# Print the paths changed by one pushed ref, in Git's native order.
#
# Usage: push-changed-files.sh <destination> <tip> [integration-ref]
# Exit 0: destination..tip was read directly.
# Exit 3: the integration-ref...tip merge-base range was used (new ref or fallback).
# Exit 4: neither applicable range was readable; stdout is empty.

destination=${1-}
tip=${2-}
integration_ref=${3-origin/dev}
zero_sha=0000000000000000000000000000000000000000

# An empty tip names nothing to read: refuse. An empty destination (a malformed pre-push line) is read
# as the hook's inline block read it before this script existed: the direct diff cannot be taken, so
# the merge-base range is used, exactly as for a new ref.
if [ -z "$tip" ]; then
  exit 4
fi

if [ -n "$destination" ] && [ "$destination" != "$zero_sha" ]; then
  if changed=$(git diff --name-only "$destination" "$tip" 2>/dev/null); then
    [ -z "$changed" ] || printf '%s\n' "$changed"
    exit 0
  fi
fi

if changed=$(git diff --name-only "$integration_ref...$tip" 2>/dev/null); then
  [ -z "$changed" ] || printf '%s\n' "$changed"
  exit 3
fi

exit 4
