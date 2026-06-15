#!/bin/bash
# Generate Schema Types — TypeScript enum constants from protocol JSON schemas.
#
# Externalized verbatim from genesis/Jenkinsfile (Generate Schema Types stage) to
# keep the pipeline's single CPS dispatch method under the 64KB
# MethodTooLargeException limit — see CLAUDE.md "Jenkinsfile Size Limit".
# Original was a non-interpolated '''...''' heredoc — no args.
set -euo pipefail

echo "Generating TypeScript enum constants from protocol JSON schemas..."

# JSON Schema is the single source of truth — codegen produces both TS and Rust
pnpm run schema:codegen:ts

echo "✅ Schema types generated"
