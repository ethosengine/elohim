#!/bin/bash
# Sprint-report aggregator (API run).
#
# Externalized verbatim from genesis/Jenkinsfile (E2E Verification stage) to keep
# the pipeline's single CPS dispatch method under the 64KB MethodTooLargeException
# limit — see CLAUDE.md "Jenkinsfile Size Limit".
#
# Non-blocking: the trailing `|| echo` mirrors the original (no `set -e` so the
# aggregator failing never fails the stage). Reads the cucumber-report-api.json
# snapshot so the API report reflects the API run, not the browser run's
# clobbered JSON. Behavior preserved exactly.
#
# Args:
#   $1 = run id (env.BUILD_TAG)

cd genesis/a2o
pnpm exec tsx scripts/build-sprint-report.ts \
  --run-id "$1" \
  --profile "${CUCUMBER_PROFILE:-alpha}" \
  --doorway "${E2E_DOORWAY_ALPHA:-}" \
  --cucumber reports/cucumber-report-api.json \
  || echo 'Sprint-report aggregator (API) failed (non-blocking)'
