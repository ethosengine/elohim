---
title: "Attestation consolidation — residual tails (codegen $ref subtype registration + legacy HTTP cleanup)"
id: attestation-consolidation-residual-tails-plan
status: Draft
class: protocol-canonical
domain: D7
sprint: tiered-quilt-wave-minus-1
cites:
  - attestation-consolidation-design | the canonical design this completes — the consolidation primitive (Content attestation:<subtype>) it builds the residual tails on | sha256:220c0a2a68c2a805 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-attestation-consolidation-design.md
  - tiered-quilt-wave-0-substrate-cleanup | the wave-0 plan whose Stage-A attestation-dedupe this supersedes/completes; tiered-quilt unblock chain | sha256:206f564eada640f8 | path: genesis/docs/superpowers/plans/2026-05-11-tiered-quilt-wave-0-substrate-cleanup.md
  - attestation-consolidation-phase2a-dedup | the history record proving Phase-2a (commit 34fcf1070) landed Stage A→G — this plan is ONLY the residual tails it left | sha256:b5bb7b0f18a4ac8e | path: genesis/docs/content/elohim-protocol/history/2026-06-02-attestation-consolidation-phase2a-dedup.md
# The big consolidation LANDED in Phase-2a (commit 34fcf1070, Stage A→G). This plan is ONLY the
# residual tails the scoping pass found. NO doc-level requires_env (mixed): Slice 2 (HTTP dead-code) is
# native/household-nodes; Slice 1 touches the INTEGRITY zome → DNA-hash-changing → its LIVE reinstall is
# operator-gated (the in-repo proof is a sweettest, which IS dev-buildable). @requires tagged per-slice.
---

# Attestation Consolidation — Residual Tails Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the two residual tails left after the attestation-consolidation Phase-2a sprint (commit `34fcf1070`, which landed Stage A→G): (1) fix the codegen `$ref` bug so lamad's four `attestation:*` subtypes register and become mint-able (today they fail the F1 validator at both issuance and commit); (2) [**SUPERSEDED → backlog**] what was assumed to be dead legacy HTTP arms turned out to be a live-consumer incoherence — `content_attestations` is dropped but still queried by 8 files incl. EPR-head reads; re-scoped to a separate ~8-file migration (see Task 2 banner + backlog `content-attestations-table-dropped-but-still-consumed.md`). Slice 1 alone is the prerequisite tail that unblocks **new** `attestation:*` subtypes — including tiered-quilt's own tier attestations.

**Architecture:** The consolidation primitive is already live — a `Content` entry on elohim DNA with `content_type: "attestation:<subtype>"`, gated by the F1 validator (`content_store_integrity/src/attestation_validator.rs:164`) which fail-closed rejects any kind not in the generated allow-list `content_store_integrity/src/generated_attestation_kinds.rs`. That allow-list is produced by `elohim/sdk/schemas/scripts/codegen-rs.mjs` from the pillar manifests. The bug: lamad declares its attestations via a `$ref` (`lamad/manifest.json:45` → `./manifest/attestations.json`), but the codegen does a flat `Object.keys(manifest['attestations'])` (`codegen-rs.mjs:186`) that never resolves the `$ref` — so it registers a phantom `"$ref"` kind and drops lamad's four real subtypes. Fix the resolver; the four subtypes flow through; the DNA rebuild makes the validator accept them.

**Tech Stack:** Node (codegen-rs.mjs), Rust (elohim DNA `content_store_integrity` WASM zome — `pnpm run schema:codegen:rs` + `hc dna pack`), the sweettest harness, elohim-storage HTTP (native).

## Global Constraints

- **Build env:** DNA/WASM workspaces (`elohim/holochain/dna/*`) use **plain cargo** — do NOT redirect `target/` (`hc dna pack` canonicalizes `./target`). The integrity-zome rebuild uses the WASM toolchain (`RUSTFLAGS='--cfg getrandom_backend="custom"'`). Storage (Slice 2) is **native** (`RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/owv`). Sweettest runs **serial** (`--test-threads=1`); never parallel. `cargo-pool prune --stale-incrementals --yes` between heavy DNA builds (PVC).
- **⚠ DNA-hash safety (Slice 1 is integrity-zome → DNA-hash-changing):** regenerating `generated_attestation_kinds.rs` changes the `content_store_integrity` WASM → changes the **DNA hash**. Per root CLAUDE.md DNA gotchas: a new hash does NOT reach running conductors on a normal redeploy (install stale-check is role-structure-only), and forcing a reinstall mints a new agent key (prod needs migration/lineage, the alpha genesis pair must both get `ALLOW_DNA_REINSTALL`). **The live reinstall is OPERATOR-GATED — this plan's Slice 1 deliverable is the in-repo proof (codegen correct + DNA builds + a sweettest mints the 4 subtypes), NOT a live deploy.** Flag the reinstall decision for the operator; do not deploy.
- **Codegen is source-of-truth-driven:** never hand-edit `generated_attestation_kinds.rs` — fix the codegen and regenerate (root CLAUDE.md schema rule).
- **Branch `feat/frontend-eyes-sprint`, commit-only** (integrator pushes; never `git push`). A concurrent session commits here — stage ONLY your files.

## Non-goals (verified out of scope by the scoping pass)

- The big consolidation itself — **already landed** (Phase-2a, `34fcf1070`): coordinator fns, unified `attestations`/`governance_actions` projection + tally, legacy-table drop, Shamir transport, unified routes. Do NOT re-do.
- The 3 **retained** entry types `ContentAttestation` / `ContentSuccession` / `CustodianCommitment` — kept deliberately (live callers: ~14 for CustodianCommitment shard-replication, versioning callers for ContentSuccession). Retiring them is a separate follow-on (captured as backlog), NOT this plan. ⚠ Note: lamad subtypes `content-succession`/`custodian-commitment` map onto these retained types — registering the subtypes (Slice 1) does NOT remove the entry types; that double-migration is the deferred follow-on.
- imagodei **Stage-G** remnants (`KeyStewardship`/`StewardshipGrant`/`StewardshipAppeal`/`HumanityWitness`/`RecoveryVote`) — owned by `genesis/docs/superpowers/plans/2026-05-15-recovery-m4-completion-shamir-optional-plan.md`.
- The `lamad_event_type → elohim_event_type` rename — that's tiered-quilt **wave-0 Stage B** (gated on EPR-Phase-4 merge), a separate plan.
- The tier substrate (`quilt_tier_state`, pledge `tier_floor`, temperature classes, tier attestations) — tiered-quilt **Wave 1+**, the real feature, a separate plan that builds on this fix.

## File Structure

- **Modify** `elohim/sdk/schemas/scripts/codegen-rs.mjs` (~:182-190) — resolve `$ref` in the `attestations` (and `governance-actions`) manifest blocks before `Object.keys`.
- **Regenerate** `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/generated_attestation_kinds.rs` (via `pnpm run schema:codegen:rs`) — never hand-edit.
- **Add** a sweettest proving the 4 lamad subtypes mint + the phantom `"$ref"` kind is gone (`elohim/holochain/tests/sweettest/` — find the attestation sweettest home).
- **Modify** `elohim/elohim-storage/src/api/attestations.rs` (:105-186) — delete the legacy `content_attestations`-backed arms; **Modify** `http.rs` if it routes them.

---

### Task 1: Slice 1 — codegen `$ref` resolution (register lamad's 4 attestation subtypes)

**Files:**
- Modify: `elohim/sdk/schemas/scripts/codegen-rs.mjs` (~:182-190 the attestations loop; the governance-actions loop similarly)
- Regenerate (do not hand-edit): `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/generated_attestation_kinds.rs`

**Interfaces:**
- Consumes: `lamad/manifest.json:45` `"attestations": { "$ref": "./manifest/attestations.json" }` and the referenced `lamad/manifest/attestations.json` (the 4 subtypes: `attestation:mastery`, `attestation:content-quality`, `attestation:content-succession`, `attestation:custodian-commitment`).
- Produces: a `generated_attestation_kinds.rs` whose `ATTESTATION_KINDS` includes the 4 lamad subtypes mapped to `"lamad"`, with NO `"$ref"` phantom key.

- [ ] **Step 1: Reproduce the bug (characterize before fixing).** Run `grep -n '"\$ref"' elohim/holochain/dna/elohim/zomes/content_store_integrity/src/generated_attestation_kinds.rs` — confirm the phantom kind (`:8`, `:48`). Confirm the 4 lamad subtypes are ABSENT. This is the regression anchor.

- [ ] **Step 2: Fix the codegen `$ref` resolver.** In `codegen-rs.mjs`, where it reads `const attestations = manifest['attestations'] || {}`, detect a `$ref` and load+merge the referenced JSON relative to the manifest dir before `Object.keys`. Representative:

```js
function resolveRefBlock(block, manifestPath) {
  if (block && typeof block === 'object' && block['$ref']) {
    const refPath = path.resolve(path.dirname(manifestPath), block['$ref']);
    return JSON.parse(fs.readFileSync(refPath, 'utf8'));
  }
  return block || {};
}
// ...
const attestations = resolveRefBlock(manifest['attestations'], manifestPath);
for (const kind of Object.keys(attestations)) { attestationKinds.set(kind, pillar); }
// apply the same to the governance-actions block
```
Apply the same `$ref` resolution to the governance-actions loop (defensive — even if lamad only $ref's attestations today, the bug class is shared).

- [ ] **Step 3: Regenerate + verify the allow-list.** Run `pnpm run schema:codegen:rs`. Then assert: `grep -c '"\$ref"' …/generated_attestation_kinds.rs` is **0**, and `grep -E 'attestation:(mastery|content-quality|content-succession|custodian-commitment)' …/generated_attestation_kinds.rs` shows all **4**. (Codegen may be non-idempotent on unrelated ordering — stage only the genuinely-changed lines / the regenerated file.)

- [ ] **Step 4: DNA rebuild + sweettest (the in-repo proof; live reinstall is operator-gated).** Rebuild the elohim DNA (plain cargo + `hc dna pack`, WASM flag, serial). Add/extend a sweettest that calls `issue_attestation` for `attestation:mastery` (a lamad subtype) and asserts it COMMITS (F1 accepts) — and that an unknown kind still fails-closed. Run serial: `cargo test … --test-threads=1`. Expected: the 4 subtypes mint; unknown rejected.

- [ ] **Step 5: Commit** (codegen-rs.mjs + the regenerated generated_attestation_kinds.rs + the sweettest). Message: `fix(attestation): resolve $ref in attestation-kind codegen — register lamad's 4 subtypes (consolidation residual tail)`. Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>. **In the commit body + the report, FLAG the operator DNA-reinstall decision** (this changed the integrity-zome hash; live conductors need an operator-gated reinstall to pick it up).

---

> **⚠ Task 2 SUPERSEDED → BACKLOG (2026-06-20).** The STOP-guard fired: `content_attestations` is dropped
> (migration `100300`, applied via `embed_migrations`) but still declared in `diesel_schema:742` and queried
> by **8 live files including EPR-head reads** (`epr_head`/`epr_service`/`http.rs`) — NOT isolated dead code.
> A blind api-arm deletion would leave the EPR-head path querying a dropped table. Re-scoped to a coherent
> ~8-file migration onto the unified `attestations` projection: backlog
> `content-attestations-table-dropped-but-still-consumed.md` (commit `b95aa3c7f`). The steps below are
> retained for reference only — **do NOT execute them as-is.**

### Task 2: Slice 2 — delete the dead legacy `content_attestations` HTTP arms (SUPERSEDED — see banner above)

**Files:**
- Modify: `elohim/elohim-storage/src/api/attestations.rs` (:105-186 — the legacy arms)
- Modify: `elohim/elohim-storage/src/http.rs` (if it routes the legacy arms)

**Interfaces:**
- Consumes: nothing (removal). The unified routes (`http.rs:10930-10996`, `api/attestations.rs` + `api/governance_actions.rs`) are the live surface and stay untouched.
- Produces: removal of the dead arms that call `content_attestations::*` against the table dropped by `migrations/2026-05-12-100300_drop_legacy_attestation_tables`.

- [ ] **Step 1: Confirm they're dead.** `grep -rn "content_attestations" elohim/elohim-storage/src` — confirm the only callers are the legacy arms (`api/attestations.rs:105-186`) and that the table is dropped (the migration). Confirm no live route the app depends on hits them (the unified routes replaced them per Phase-2a). If any live consumer is found, STOP and report (it would be a real regression, not dead code).

- [ ] **Step 2: Write the failing routing assertion.** A unit test asserting the legacy path now returns not-found / is absent from the dispatch (mirror the storage routing-test pattern), OR — if there's no routing-test harness — assert at the narrowest level and note the limitation.

- [ ] **Step 3: Delete the legacy arms** in `api/attestations.rs:105-186` + their `http.rs` routing (if any). Keep the unified routes intact.

- [ ] **Step 4: Build + test.** `RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/owv cargo test --manifest-path elohim/elohim-storage/Cargo.toml --lib attestations` — storage compiles, no reference to the dropped table remains.

- [ ] **Step 5: Commit** (`refactor(storage): delete dead legacy content_attestations HTTP arms (consolidation residual tail)` + Co-Authored-By).

---

## Follow-on (captured, NOT this plan)

- **Retire the 3 retained entry types** (`ContentAttestation`/`ContentSuccession`/`CustodianCommitment`) by migrating their live callers onto the `attestation:content-quality`/`content-succession`/`custodian-commitment` subtypes — a backlog item (genesis/data/timeline/backlog/), DNA-hash-changing, gated on the callers' migration.
- **The tier-substrate waves** (the real tiered-quilt feature) — a separate plan, now buildable on the landed consolidation primitive + this $ref fix.

## Self-Review

- **Scope coverage:** the scoping pass's two genuine remaining gaps — codegen $ref (Task 1, issuance-blocking) + dead HTTP arms (Task 2) — are each a task; the 3 retained types + Stage-G + the rename + the tier waves are explicit non-goals/follow-ons. ✓
- **No re-doing landed work:** Phase-2a (`34fcf1070`) is verified landed; this plan touches only the residual tails. ✓
- **DNA safety encoded:** Task 1 flags the integrity-zome DNA-hash change + the operator-gated live reinstall; the dev deliverable is the sweettest proof. ✓
- **Type/path consistency:** the 4 lamad subtypes (`attestation:mastery|content-quality|content-succession|custodian-commitment`) match `lamad/manifest/attestations.json`; the codegen fix targets `codegen-rs.mjs:~186`; the dead arms at `api/attestations.rs:105-186`. ✓
