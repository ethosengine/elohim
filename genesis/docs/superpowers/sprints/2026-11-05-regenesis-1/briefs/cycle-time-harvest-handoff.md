---
title: Cycle-time harvest from re-genesis 1 — handoff for a fresh session
date: 2026-11-06
from: re-genesis 1 orchestrator session 184beb01 (Claude Fable 5.1), /btw fork
status: proposal; nothing below is implemented
durable copy: /projects/elohim/genesis/local-dev/regenesis-1/cycle-time-harvest-handoff.md
---

# Why

Re-genesis 1 (the address crossing) ran the whole next-generation integration flow end to end:
closure-keyed gates, reuse attestations, attested pins, serving receipts, leases, gate-live, tag-steered
CI. The flow paid in **correctness and coordination** (refusals found the seeder proof-shape bug, the a2o
ESM load trap, the custody-over-iroh regression, a stale DNA wasm) but not yet in **cycle time**: a
crossing invalidates every closure, so nearly every gate ran cold. Four savings are harvestable before the
artifact-store sprint. Each was weighed for integrity loss; the verdict is on each item.

Evidence base: `genesis/docs/superpowers/sprints/2026-11-05-regenesis-1/STATE.md` (journal),
`genesis/local-dev/regenesis-1/push-landing-r1.log` (the landing push's hook output),
`genesis/local-dev/regenesis-1/household-r1b-summary.log` (recast refusal + lanes),
memory `project_brit_worktree_gaps_2026_11.md`.

# Item 1 — enable gate reuse for the projects that ran cold on every landing

**Observed.** The landing push (139b1be02 → dev) printed:
`[gate] elohim-placement not reused: no attestation for this closure yet`, same for `elohim-epr` and
`k8s-bridge`; and `[gate] schema-dna not reused: project not enabled for reuse`, same for `epr-ts` and
`genesis`. Over the sprint these ran on every closure (39 cold gate runs in the overnight pass alone).

**Change.** Add the not-enabled projects to `genesis/agentic/pool-policy.json` `gate_reuse`. Leave eprfs
and elohim-storage as they are (their verdicts already cross from shem via `gate-verdict@1`).

**Integrity.** None lost by construction: reuse stands in only when the full input closure is unchanged
(declared inputs, manifest, recipe, `run-local-gate.sh`, project tree, `rustc -vV`, gitlinks, every repo
path the gate reads) and the attestation is this workspace's own green one.
**Precondition:** run `genesis/orchestrator/gate-input-audit.mjs` on each project first. A read the gate
cannot name keys the whole commit tree, so reuse never fires for it (safe, just useless); a genuine walk
declares itself with `// gate-reads: <path>`.

**Expected saving.** Minutes per closure per project now; the real dividend is sprint 2, whose closures are
narrow.

# Item 2 — serving receipt: bind to the import closure, NOT to serving-path parts alone

**Observed.** `genesis/orchestrator/scripts/serving-receipt.mjs` binds the receipt to the whole a2o tree.
A one-line eslint fix in `genesis/a2o/src/framework/byte-address.ts` forced a fresh 20–40 min household
run before L2c could push.

**Operator's concern (shared): the naive narrowing loses integrity.** The lint fix was in framework code the
deliverability steps import, so the whole-tree binding was *right* to refuse. A receipt that can be stale is
worse than a slow one.

**Only acceptable form.** Bind to: the serving-path SUT parts the leg already enumerates (a2o, elohim/sdk,
scripts/ci, app/scripts, pnpm-lock, hc-mesh.sh, storage, doorway, render, conductor pin, dna, dnaHash,
fixture) **plus** the deliverability feature **plus the transitive import closure of its step files**
(framework, shared steps, support). Exclude only a2o files provably outside that closure (other features,
unrelated scripts, reports).
**If the import closure cannot be computed mechanically, do not take this item.** Keep the whole-tree
binding and pay the run.

# Item 3 — mesh recast: rebuild a stale DNA wasm instead of only refusing (opt-in)

**Observed.** `MESH_RESET=1 just mesh start` REFUSED at preflight:
`node_registry coordinator wasm (node_registry_integrity.wasm) is STALE: built 11:41Z, newest tracked
source … modified 16:10Z (HEAD 2fb2f93c5) — rebuild it (cd elohim/holochain/dna/node-registry && just
build) or set MESH_ALLOW_STALE_HAPP=1`. The cure was exactly the command the refusal printed; one recast
cycle was lost to it (and my chain ran lanes against a dead mesh because it did not stop on the refusal —
fixed in `genesis/local-dev/regenesis-1/r1b-household.sh`, which stops on the first red).

**Change.** A `MESH_REBUILD_STALE_HAPP=1` arm in `hc-mesh.sh` that runs the named zome `just build`
(plain cargo, in-tree target — DNA workspaces are exempt from the pool), then lets the existing
wasm → dna → happ repack cascade proceed, then re-runs the freshness check.

**Integrity.** None lost: it measures the coordinator the refusal wanted measured, built from tracked
source at HEAD. **Guard:** an *integrity*-zome rebuild moves the DNA hash, so the arm must only fire under
an explicit `MESH_RESET=1` (a recast) or when the staleness is coordinator-only; on an ordinary resume it
would turn a refusal into a silent recast of the household's identities. Keep it opt-in like the other
`MESH_ALLOW_*` arms; the refusal stays the default.

# Item 4 — stop paying the pre-push hook twice: push with git from the start

**Observed.** On both devices, every landing ran the full hook (30–60 min) under `brit push`, died with
`send-pack: sending on a closed channel` after the green hook, and re-ran under
`GATE_EVIDENCE=reuse git push origin HEAD:dev`. brit 0.1.12 (pin 1c6ae5568); other brit gaps this sprint
(commit refuses worktree `core.hooksPath`, status panics `too few index slots`, pull refuses the shallow
main checkout, `add` takes `-- <paths>` only) are in memory `project_brit_worktree_gaps_2026_11.md`.

**Change.** Until the brit pin carries a push fix (T4 lands brit-cli 0.1.13; re-check then), land with git
from the start, keep brit for `add`/inspection, and keep the `# Brit fallback: <reason>` line so the
adoption experiment keeps its slip evidence.

**Integrity.** No gate integrity lost: the pre-push hook is git's hook (`.husky/pre-push.bash`) and ran
identically under brit. **Two things to verify before adopting:**
1. Whether `brit push` would have transported `refs/notes/brit/*` (validation attestations the
   serving-receipt leg reads) and git's `HEAD:dev` refspec does not — if so, add the notes refs to the
   push (`git push origin HEAD:dev 'refs/notes/brit/*:refs/notes/brit/*'` or the equivalent).
2. That each fallback stays recorded (journal + memory already do), so the brit experiment is not
   silently abandoned.

# Not worth harvesting now

- **CI rust-style baseline lockout** (#2220–#2232): cured in `genesis/orchestrator/rust-style-ci.sh`
  (baseline moves forward to the method bootstrap); recurs only if the discipline method moves again.
- **`gate-source-state` epr-closure trap** (push refused "epr is required before any gate" when the pool
  family was wiped): fires only when a family is wiped mid-sprint; the bin-store candidate
  (`/projects/.claude-config/tools/bin-store/epr/<cid>/epr`) is the standing cure, recorded in memory
  `project_regenesis1_landing_traps_receipt_and_epr_closure.md`.

# Order and size

1 (audit + policy lines; ~1 h) → 4 (verify notes transport; ~30 min) → 3 (hc-mesh.sh arm + test;
~2 h) → 2 only if the import closure is mechanical (~half a day, else skip).
None of these touch the artifact-store sprint's write-set.
