#!/usr/bin/env bash
# Pre-push guard: catch MethodTooLargeException risk before CI does.
#
# JVM bytecode methods are limited to 64KB. Jenkins's CPS rewriting of
# Pipeline DSL turns each `pipeline { }` block into one giant method
# (WorkflowScript.___cps___N), so an over-stuffed Jenkinsfile fails at
# COMPILE time in Jenkins — not lint, not syntax check, runtime compile.
# That's a deterministic, source-byte-correlated condition we should
# never discover in CI.
#
# Heuristic: count source bytes inside each `pipeline { }` block.
# Compiled CPS bytecode runs roughly 1:1 with source size for declarative
# pipelines (the CPS rewriter expands tracking code but compresses literals).
# We fail at 60KB source as a 4KB safety margin under the 64KB JVM limit.
#
# To extract: move `def helperFunc()` blocks ABOVE `pipeline {` and call
# them from `script { }` bodies. CLAUDE.md says explicitly:
# "Never add inline logic to stages."
#
# Bypass: HUSKY=0 git push  (or --no-verify) — but you'll just see the
# same error in CI. Refactor instead.

set -euo pipefail

# Source-bytes thresholds. WARN signals "consider refactoring soon"; FAIL
# blocks the push.
#
# Calibration: source bytes don't map 1:1 to compiled CPS bytecode. The
# 64KB JVM limit is on compiled bytecode. Empirical data points:
#   • elohim/holochain/Jenkinsfile @ 62812 source — compiles fine (#1210)
#   • genesis/orchestrator/Jenkinsfile @ 74732 source — FAILED with
#     MethodTooLargeException (#922)
# So the danger zone in source-byte terms starts somewhere between 63KB
# and 75KB. We set HARD at 65000 as a conservative threshold (just above
# the largest known-good size) and WARN at 55000 to flag impending issues
# while there's still margin to refactor calmly.
WARN=55000
HARD=65000

# Jenkinsfiles in this repo. Maintain alongside genesis/orchestrator/Jenkinsfile's
# PIPELINES map — any new pipeline's Jenkinsfile should be added here.
JFILES=(
  "Jenkinsfile"
  "genesis/Jenkinsfile"
  "genesis/orchestrator/Jenkinsfile"
  "elohim/holochain/Jenkinsfile"
  "elohim/holochain/dna/Jenkinsfile"
  "steward/device/Jenkinsfile"
  "sophia/Jenkinsfile"
  "elohim/epr/Jenkinsfile"
  "app/elohim-library/Jenkinsfile"
)

# Repo root anchoring — script can be invoked from any cwd.
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo .)"
cd "$REPO_ROOT"

failed=0
warned=0
checked=0

for f in "${JFILES[@]}"; do
  [ -f "$f" ] || continue
  checked=$((checked+1))

  # Extract bytes inside the top-level `pipeline { ... }` block. awk
  # tracks brace depth; at depth 0 again we exit. The `inpipe` flag
  # ensures we only count from `pipeline {` onward, not preceding helpers.
  size=$(awk '
    BEGIN { depth = 0; inpipe = 0 }
    /^pipeline \{/ { inpipe = 1 }
    inpipe {
      print
      n = gsub(/\{/, "{")
      depth += n
      n = gsub(/\}/, "}")
      depth -= n
      if (depth == 0 && inpipe) { exit }
    }
  ' "$f" | wc -c)

  if [ "$size" -eq 0 ]; then
    # No `pipeline {` block found — file is a library / shared module.
    continue
  fi

  printf "  pipeline{} block: %6d bytes  %s" "$size" "$f"
  if [ "$size" -ge "$HARD" ]; then
    printf "  ❌ FAIL (>= %d)\n" "$HARD"
    failed=$((failed+1))
  elif [ "$size" -ge "$WARN" ]; then
    printf "  ⚠️  WARN (>= %d)\n" "$WARN"
    warned=$((warned+1))
  else
    printf "  ✅\n"
  fi
done

echo
echo "checked $checked Jenkinsfiles ($warned warned, $failed failed)"

if [ "$failed" -gt 0 ]; then
  cat <<EOF

ERROR: At least one Jenkinsfile's pipeline { } block is too large.
Jenkins compiles each pipeline block into one CPS method capped at 64KB
JVM bytecode. The compile failure surfaces as:

   groovyjarjarasm.asm.MethodTooLargeException: Method too large:
   WorkflowScript.___cps___N ()Lcom/cloudbees/groovy/cps/impl/CpsFunction;

Refactor pattern (see root Jenkinsfile + CLAUDE.md):

   def myStageHelper(arg1, arg2) {
       // logic that used to live inline
   }

   pipeline {
       stages {
           stage('My Stage') {
               steps {
                   container('builder') {
                       script {
                           myStageHelper(env.X, params.Y)
                       }
                   }
               }
           }
       }
   }

Top extraction targets in priority order:
  1. post.always blocks (often the largest single CPS method)
  2. CI-summary / build-graph map literals (lots of inline structure)
  3. Stage logic with nested loops or per-pipeline branching

EOF
  exit 1
fi

exit 0
