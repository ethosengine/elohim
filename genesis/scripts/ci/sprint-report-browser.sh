#!/bin/bash
# Sprint-report aggregator (Browser run).
#
# Externalized verbatim from genesis/Jenkinsfile (E2E Verification stage) to keep
# the pipeline's single CPS dispatch method under the 64KB MethodTooLargeException
# limit — see CLAUDE.md "Jenkinsfile Size Limit".
#
# Non-blocking: the trailing `|| echo` mirrors the original (no `set -e` so the
# aggregator failing never fails the stage). Behavior preserved exactly.
#
# Args:
#   $1 = run id (env.BUILD_TAG)

cd genesis/a2o
pnpm exec tsx scripts/build-sprint-report.ts \
  --run-id "$1" \
  --profile browser \
  --doorway "${E2E_DOORWAY_ALPHA:-}" \
  --cucumber reports/cucumber-report-browser.json \
  --out-json reports/sprint-report-browser.json \
  --out-md reports/sprint-report-browser.md \
  || echo 'Sprint-report aggregator (Browser) failed (non-blocking)'
