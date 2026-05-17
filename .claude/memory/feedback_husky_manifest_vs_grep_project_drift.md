---
name: husky-manifest-vs-grep-project-drift
description: "husky pre-push has two project-detection paths (manifest-driven + grep-fallback) that emit DIFFERENT project names; when adding a manifest sub-project gate, audit the run_gate fallback case statement"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: d5ebc70b-b1ff-43c0-9172-9d14847a28ec
---

`.husky/pre-push` has TWO project-detection paths that BOTH feed `run_gate`:

1. **Manifest-driven** (lines 138-158, used when `node` is available): runs `genesis/orchestrator/graph-walker.mjs` against the changed-files list. Emits project names from `build-manifest.json`'s `gate.projects` map (the **fine-grained** sub-project names, e.g. `epr-ts`, `elohim-storage`, `elohim-app`).
2. **Grep-fallback** (lines 161+, runs when manifest detection is unavailable or empty): hand-rolled `if echo "$CHANGED" | grep -qE …; then PROJECTS="$PROJECTS <name>"` blocks. Emits **coarser-grained** names (e.g. `elohim-epr` combining both the Rust crate AND the epr-ts SDK, `epr-storage` for EPR-specific gates).

The `run_gate` fallback case statement (the `case "$PROJECT_NAME" in …` block) was written against the grep-fallback names and DID NOT include the fine-grained manifest names. Result: when manifest detection emitted `epr-ts` (a sub-project gate declared in `elohim/epr/build-manifest.json`), the case statement hit the default `*) echo "Unknown project: $PROJECT_NAME"; rc=1` and aborted the push.

**Why:** Surfaced 2026-05-17 by the `rca-orchestrator-963-graph-failure` shift — the prep commit regenerating `elohim/sdk/epr-ts/src/generated/EprKind.ts` blocked the actual hardening push for ~88 minutes (the pre-push ran every other matching gate first, then died on the `epr-ts` "Unknown project" case). Fix in `15aef755c` added a single `epr-ts) pnpm test ;;` case; smoke-tested at 10/10 vitest in 618 ms.

**How to apply:** When ADDING a new sub-project to any `build-manifest.json`'s `gate.projects` map (the entries that graph-walker turns into project names), audit `.husky/pre-push`'s `run_gate` fallback case statement for the new name. If absent, add a one-line gate case. The PROJECT_DIR is auto-passed from the manifest's `dir` field, so the case body just needs the gate command. The pre-push hook will run as `bash .husky/pre-push` if node is available (manifest path) AND as the grep path if not, so both name forms need coverage; the alternative is to make the grep fallback emit the same fine-grained names manifest-driven detection does, but that's a bigger refactor.

See also: [[cargo-target-dir-for-native-builds]] for adjacent pre-push hook concerns; [[orchestrator-predictive-vision]] for graph-walker's broader role.
