#!/bin/bash
# Validate Constants — type-check the seeder + run constants sync tests.
#
# Externalized verbatim from genesis/Jenkinsfile (Validate Constants stage,
# genesis/seeder dir) to keep the pipeline's single CPS dispatch method under the
# 64KB MethodTooLargeException limit — see CLAUDE.md "Jenkinsfile Size Limit".
# Original was a non-interpolated '''...''' heredoc — no args.
set -euo pipefail

echo "Type-checking seeder (compile-time enum enforcement)..."
# Filter out workspace package resolution errors (TS2307) —
# these are CI install issues, not code bugs. Fail only on
# real type errors (enum mismatches, invalid assignments, etc.)
TSC_OUTPUT=$(npx tsc --noEmit 2>&1 || true)
REAL_ERRORS=$(echo "$TSC_OUTPUT" | grep "error TS" | grep -v "TS2307" || true)
if [ -n "$REAL_ERRORS" ]; then
    echo "$REAL_ERRORS"
    echo "❌ TypeScript type errors found — fix before seeding"
    exit 1
fi
echo "✅ Type check passed"

echo "Running constants sync tests..."
npx vitest run --reporter=verbose

echo "✅ Constants validation passed"
