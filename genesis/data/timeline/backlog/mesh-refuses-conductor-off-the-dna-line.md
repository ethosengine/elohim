---
id: "backlog-mesh-refuses-conductor-off-the-dna-line"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "hc-mesh.sh refuses a conductor whose line differs from the DNA's hdk line and auto-detects the pinned fork build — a stock 0.6 conductor was only warned about while the start proceeded to three dead cells"
slug: "mesh-refuses-conductor-off-the-dna-line"
written: "2026-09-07"
author: "orchestrator (second-opinion session 2026-09-06/07)"
status: "triaged"
priority: "medium"
jobs: [elohim-app]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-workspace-image-ships-stock-holochain-0-6"
  - "habit:dataplane-convergence"
tags: [local-mesh, toolchain, holochain-0.7, fail-closed]
---

**What bit (2026-09-06/07):** with the workspace image on stock 0.6.0 (sibling atom), `just mesh start` in a
fresh shell resolved `command -v holochain`, `assert_toolchain_parity` passed (hc and holochain were both 0.6.0),
and the only signal was the status banner "STOCK — alpha runs the fork, so this mesh is NOT at parity". The
hdk-0.7 hApp cannot install on a 0.6 conductor, so the Prologue failed far from the cause. The slice-1
(accountable-correction) mesh run hit this twice across restarts.

**Landed (2026-09-07, `app/elohim-app/scripts/hc-mesh.sh`):**
- `MESH_FORK_BIN_DIRS` now ends with `$MESH_TOOLS_DIR/hc-fork-<pin12>/bin`, where `<pin12>` is read from the
  `elohim/holochain-conductor` gitlink (`git ls-tree HEAD`, never a checkout — the submodule is `update = none`).
  A conductor bump changes the directory name, so a stale fork build can never be picked up silently.
- `assert_conductor_matches_dna`: parses `hdk = "=X.Y.Z"` from `elohim/holochain/dna/elohim/Cargo.toml` and
  refuses a conductor whose `X.Y` differs, naming both and the two places a matching build lives.
  `MESH_ALLOW_TOOLCHAIN_SKEW=1` overrides deliberately, as for the existing hc/holochain pair check.
- The status banner now says the start is REFUSED unless the line matches, instead of "NOT at parity".

**Verified:** `just mesh status` → `NEXT LAUNCH: …/tools/hc-fork-25dd2d0be144/bin/holochain (holochain 0.7.0)
[FORK, auto-detected]` with no `HOLOCHAIN_BIN` set; stock 0.6.0 resolves to line 0.6 ≠ DNA 0.7.

**Retire when:** the workspace image ships 0.7 AND the mesh defaults to the fork by pin — then the refusal is a
regression guard only and this atom decomposes into the mesh runbook.

## Follow-on: serving receipts identify conductor source, not running bytes

**Observed 2026-09-14:** the T2 serving-receipt validator includes the
`elohim/holochain-conductor` gitlink in `sutParts.conductor`, so a gitlink move
correctly invalidates an older report. A newly generated report can still name
that current source while the household's conductor processes continue running
a different executable. The receipt validates source equivalence and scenario
outcomes; it does not compare a conductor process's executable bytes, installed
DNA, or agent identity with the source-derived pin.

**Missing story node:**

`source gitlink → running conductor executable SHA + unchanged DNA/agent/key provenance → scenario receipt`

The planned household conductor upgrade requires this manual proof: retain the newly built
`holochain` SHA and build-source gitlink, restart through the fixture-owned
lifecycle, prove all three `/proc/<pid>/exe` hashes equal that retained SHA with
new owned process incarnations, and prove the installed DNA hashes and agent
keys are unchanged before treating the subsequent scenario receipt as evidence
for the new conductor. Until an existing receipt seam carries those facts, a
green serving report alone must be described as current-source scenario proof,
not running-conductor provenance.

**Retire this follow-on when:** the household receipt chain fail-closed joins
the source gitlink to the running conductor executable and preserves the
installed DNA/agent/key identity across the restart; the scenario report stays
the final behavioral verdict rather than being asked to imply those missing
runtime facts.

**Measured counterexample:** household run `20260914T093547Z-e328717c` passed all five serving scenarios and 102 steps while its committed conductor gitlink still named `25dd2d0be144` and all three actual executables were independently verified as fork `8591d6c20248` (SHA256 `0f583f14584cee01aa5e5d37d9616871926748933bb4f07ac8d3cfe6813f25ad`). The unchanged report bytes were moved outside top-level receipt discovery into `reports/recovery/doorway-overnight-20260914/serving-new-conductor-qualified/`, with an explicit qualification receipt. The behavior passed; final source-to-runtime delivery was not proven.

**Observation boundary:** record which process/network namespace supplied runtime evidence. On 2026-09-14, sandbox-local `/proc` and loopback checks falsely reported the persisted household stopped, while the actual workspace network returned HTTP 200 from both doorways and all three storage endpoints. Missing visibility must be recorded as unmeasured; it must not authorize recovery, identity replacement, or a restart.

**Storage build provenance counterexample:** the gated e2488e547 binary matched its retained SHA on all three peers, but `/version` reported `commit=unknown`. `elohim-compute/src/build_info.rs` reads compile-time `GIT_COMMIT_SHORT`, `GIT_COMMIT_FULL`, `BUILD_TIMESTAMP`, and `RUSTC_VERSION`; it does not discover Git itself. A local production build must supply those existing CI variables from its actual committed source and toolchain. Until then, preserve the isolated commit/build receipt and verify the running executable hash; never infer an embedded revision from the command's working directory.

**Execution-context follow-on for native memory and delegation:** the overnight delivery repeatedly retained the right intent but lost an executable constraint at handoff: an infrastructure client was used for a lamad read, a branch-derived Cargo family selected a cold pool, “production” selected a release profile instead of the established dev profile, and a retained artifact existed only in the sandbox observation namespace. A later diagnostic used an inferred route instead of the exact known `/db/content/<id>/head` route. When this tooling is revisited, consider pinning the role/client provenance, full command and profile, resolved pool path, endpoint, and artifact/process namespace in the existing task receipt, with cheap preflight assertions before expensive work. These are observed handoff failures, not evidence that a new memory index or another ledger is needed.
