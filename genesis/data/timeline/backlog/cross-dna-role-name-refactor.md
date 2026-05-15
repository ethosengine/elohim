---
id: "backlog-cross-dna-role-name-refactor"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Cross-DNA role-name refactor — introduce role-target constant, untangle lamad/elohim DNA names"
slug: "cross-dna-role-name-refactor"
written: "2026-05-15"
author: "investigation (Recovery M4 Task 3 spillover)"
status: "envisioned"
priority: "medium"
relatedNodeIds:
  - project_attestation_consolidation_sprint_state
  - feedback_no_sovereignty_stewardship_over_ownership
tags: [holochain, dna, cross-dna-bridge, hygiene, refactor, naming]
shift_objective: |
  Resolve the production-vs-test role-name mismatch that lets imagodei/mishpat/infrastructure
  bridge calls compile and pass SweetTest while pointing at a role that does not exist in
  the packaged hApp. Introduce a shared `ELOHIM_DNA_ROLE` Rust constant so every cross-DNA
  call site is reachable by the compiler; either rename the production happ.yaml role to
  match the bridge target OR split protocol-core entries (Content, Attestation,
  GovernanceAction) into a properly named `elohim` DNA. Also reconcile the divergent
  `steward/device/workdir/happ.yaml` (single-role `elohim` bundling a non-existent
  `elohim.dna` artifact) with the central manifest.
---

## Finding

The imagodei zome (and mishpat, infrastructure) bridge cross-DNA calls hard-code the role
name `"elohim"` as a string literal. The production happ.yaml does **not** declare a role
named `elohim`; it declares `lamad`. The SweetTest harness silently masks this by loading
`lamad.dna` and re-mounting it under the role name `"elohim"`.

This is **Possibility B** from the investigation framing — production-vs-test mismatch,
not name-only confusion — with **Possibility C compounding** (directory naming is also
confusing: `dna/elohim/` compiles a DNA whose `dna.yaml` declares `name: lamad` and the
build emits `lamad.dna`).

**Evidence:**

- `elohim/holochain/dna/elohim/workdir/happ.yaml:24` declares the role as `name: lamad`
  with `bundled: lamad.dna`. There is no role named `elohim` in this manifest.
- `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs:689,723,757,794` (and
  mishpat:1663,1695, infrastructure:1091) issue:
  `CallTargetCell::OtherRole("elohim".into())`
- `elohim/holochain/tests/sweettest/src/tests/recovery_m3.rs:199,205` does the masking:
  ```rust
  let elohim_dna = load_dna("lamad", ...).await?;   // loads lamad.dna
  // ...
  ("elohim".into(), elohim_dna),                    // mounts it under "elohim" role
  ```
  The test even documents the workaround on lines 195–196 ("install them under explicit
  role names matching the imagodei coordinator's `CallTargetCell::OtherRole(\"elohim\")`
  target") — i.e. the test was deliberately authored to work around the production gap.
- `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:866` already demonstrates
  the correct pattern in the opposite direction: `const IMAGODEI_ROLE: &str = "imagodei";`
  used at `:904, :940, :976`. The constant matches the production happ.yaml role name,
  so those calls are correct.

**Second-order finding (parallel inconsistency in steward Tauri shell):**
`steward/device/workdir/happ.yaml:6-11` declares a *single* role `elohim` bundling
`elohim.dna`. No `elohim.dna` artifact is produced by any build pipeline — the
`hc dna pack` invocations in `elohim/holochain/dna/Jenkinsfile:443` and
`elohim/holochain/dna/elohim/build.sh:21` both emit `lamad.dna`. This Tauri happ.yaml
appears stale and would fail to install regardless of the bridge issue. Fixing should be
coupled with this refactor.

## Risk

In production (packaged elohim.happ installed via `hc app install`), every Recovery M4
cross-DNA call from imagodei into the content_store zome will fail with a Holochain
"role not found" error at runtime — Gate 1 of intimate witness, key rotation revocation
checks, recovery request reads, all four `call_elohim_*` helpers. SweetTest passes
because the test harness re-mounts the DNA under the role name the bridge expects.

**Test gap that would catch this:** an integration test that installs the *production*
happ bundle (not custom-mounted DNAs) and exercises one cross-DNA bridge call end-to-end.
None of the current SweetTests do this — they all construct test-only role maps. A
production-bundle smoke test belongs alongside `manifest-hygiene/` (which validates the
manifest itself but doesn't install it).

The Recovery M4 Task 3 implementer flagged this correctly. The bridge code's *intent* is
right (attestations target the elohim/protocol-core DNA); the role-name string is wrong
relative to the only production happ.yaml that declares it.

## Proposed refactor

Two reconciliation options — either resolves the runtime defect:

1. **Rename role to match bridge target** (minimal change):
   Edit `elohim/holochain/dna/elohim/workdir/happ.yaml:24` to `name: elohim` (keep
   `bundled: lamad.dna` initially, or rename artifact to `elohim.dna` in build
   pipelines). Update SweetTest fixtures to install under the production name.
   Update `steward/device/workdir/happ.yaml` once artifact naming converges.

2. **Split DNA along semantic boundary** (proper, larger):
   Today's `lamad.dna` carries both LMS content (lamad-specific: paths, mastery,
   practice) and protocol-core entries (Content, Attestation, GovernanceAction).
   Split into two DNAs: a `lamad` DNA for LMS-specific zomes, and an `elohim` DNA
   for protocol-core. This matches the conceptual model (the elohim pillar owns
   cross-pillar protocol primitives) and removes the directory-vs-DNA-name
   confusion that `dna/elohim/` compiling `name: lamad` creates today.

**Required for either option — introduce a Rust constant:**

```rust
// elohim/holochain/dna/_shared/ or per-DNA src/cross_dna.rs
pub const ELOHIM_DNA_ROLE: &str = "elohim";   // or "lamad" until rename
pub const IMAGODEI_DNA_ROLE: &str = "imagodei";
pub const MISHPAT_DNA_ROLE: &str = "mishpat";
```

Replace every `OtherRole("elohim".into())` / `OtherRole("imagodei".into())` literal
with the constant. Future role renames then become a compiler error at every call
site, not a silent runtime regression that only manifests in production.

The constant placement question (shared crate vs duplicated per-DNA) is a design call
during the refactor — Holochain zomes have historically avoided shared dev-deps to keep
WASM builds clean, so per-zome `const` with a documented convention may be the path of
least friction.

## Scope estimate

**Small-to-medium** if Option 1 (rename role): single PR — happ.yaml edit, constant
introduction, ~10 bridge sites updated, SweetTest fixture cleanup, manifest-hygiene test
to install the production bundle. One day of focused work.

**Medium-to-large** if Option 2 (DNA split): multi-day. New DNA crate, entry-type
re-homing, network_seed decisions, migration story for any existing alpha-network state,
manifest changes across happ.yaml + steward Tauri happ.yaml + CI build wiring. Best
done as a dedicated sprint.

Regardless of option, the constant-extraction sub-task is small and worth landing first
on its own (turns a silent string-literal problem into a compiler-visible one).

## Linked sprints / contexts

- Recovery M4 sprint (current): `genesis/docs/plans/2026-05-15-recovery-m4-completion-shamir-optional-kickoff-prompt.md`
  — Task 3 surfaced this concern; the recovery_m3.rs SweetTest is the masking harness.
- Attestation consolidation memory: `.claude/memory/project_attestation_consolidation_sprint_state.md`
  — the attestation issue path that introduced cross-DNA bridge helpers in the first place.
- Stewardship/naming reference: `.claude/memory/project_no_sovereignty_stewardship_over_ownership.md`
  — relevant when choosing role names for any DNA split.

## NOT a blocker for this sprint

Recovery M4 can and should continue as-is. The bridge calls are **semantically correct**
— they target the right DNA conceptually (the one that owns Content, Attestation,
GovernanceAction entries). Only the role-name string used to address that DNA in
production happ.yaml is off. SweetTest provides functional coverage of the bridge logic
under a test-mounted role map.

This refactor is hygiene + future-proofing, to be picked up when there is bandwidth for
a focused PR (Option 1) or a dedicated sprint (Option 2). The integration-test gap
should be addressed at the same time so this class of mismatch cannot recur silently.
