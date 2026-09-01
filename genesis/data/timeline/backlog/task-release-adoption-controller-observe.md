---
id: "backlog-task-release-adoption-controller-observe"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Task: adoption controller (observe mode) — watch followed release channels through the own conductor, fetch + verify releases, report typed verdicts on /admin/adoption; NO apply"
slug: "task-release-adoption-controller-observe"
written: "2026-09-01"
author: "session-2026-09-01-rung5-design"
claimedBy: "claude-opus-t3"
status: "open"
priority: "high"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-upgrade-propagation-p2p-design-arc"
  - "spec:runtime-artifacts-elected-content"
  - "backlog-task-release-manifest-schema-packager"
  - "backlog-task-release-apply-vehicles"
tags: [upgrade-propagation, rung5, adoption-controller, reconciliation, elohim-storage, delegable]
---

**Claimable by any implementation agent. Depends on T1's schema (the manifest
currency); T4 (`task-release-apply-vehicles`) builds on this task's module and
MUST NOT start until this lands. The one genuinely new component of the rung-5
spec (§6), in its safe half: observe-only.**

## Why

The spec's adoption semantics — "bytes hash right" is transport, adoption is
consent — need a home: a reconciliation loop (P1: DHT is the manifest, the
controller eagerly reconciles) that can SEE and JUDGE releases before anything
is allowed to act. Observe-mode-first mirrors how every prior arm landed
(dry-run before apply on `/admin/coordinators/sync`).

## P2P design-gate decision

Carried by the spec §5: AdoptionState is Ephemeral (C) — reconstructable,
node-local, surfaced on an admin route, never notarized or gossiped as
authority. The controller adds NO entity, NO route in `build_manifest()` (the
admin surface is node-local exactly like `/admin/coordinators/sync`,
`http.rs:4931`'s exclusion pattern). Concern canon: this task must land C6a
(bounded work per sweep + finite backoff), C6b (idempotent on (channel,
releaseCid)), C8 (typed reason on every arm + per-decision metrics), C4
(honest absence: no earned head → idle, `tier: none`, never a guess) — and
register the verdict predicate in the crate's `seam-registry.yaml` at birth.

## Scope

1. New module `elohim/elohim-storage/src/services/release_adoption/`
   (`mod.rs`, `watch.rs`, `verify.rs`, `state.rs`; `apply.rs` is T4's — leave
   a `pub trait ApplyVehicle { fn apply(&self, v: &VerifiedRelease) ->
   Result<AppliedReceipt, AdoptionRefusal>; }` seam with NO implementations).
2. **Watch**: followed channels come from the rung-4 runtime-config surface —
   add `releaseChannels: [{channelId, mode: "observe"}]` to the watched
   config (only `observe` is legal until T4). Resolve each channel's
   canonical head through THIS node's conductor (I1; reuse the head-resolve
   rails `services/head_adoption.rs` uses — compose, don't re-derive).
3. **Fetch**: manifest content + artifact blobs by CID via the existing blob
   fetch path (`p2p/blob_fetch.rs` evidence-ordered candidates).
4. **Verify** (`verify.rs`, the floor — spec §6.3): schema-validate the
   manifest (T1's schema, vendored or re-exported); blob CID match; envelope
   check against the runtime passport's installed reality
   (`runtime_passport.rs` per-role dna_hash + coordinator_wasm_hashes — the
   same per-role refusal `happ_manager.rs::lineage_mismatch_error` enforces,
   moved to verify time); lineage parent verified against the channel's L2
   version chain, body field a hint that must match; attestation threshold
   per the manifest's `adoptionDiscipline` (read via T5's
   `count_qualifying_attestations` if landed, else a `threshold_unchecked`
   typed verdict — NOT a pass).
5. **Report**: `GET /admin/adoption` (node-local, NOT in `build_manifest()`)
   → per channel: `{channelId, mode, resolvedHead: {cid, tier} | null,
   verdict: {ok} | {refusal: <typed reason>}, lastCheckedAt}`. Metrics:
   `elohim_release_adoption_decisions_total{arm, reason}` following the
   `elohim_content_election_*` pattern (`metrics.rs`).

## Interface contract (consumed by T4, T6)

- `VerifiedRelease { channel_id, release_cid, manifest: ReleaseManifest,
  artifact_paths: Vec<PathBuf> }` and `AdoptionRefusal` (typed enum) are the
  currency T4 implements `ApplyVehicle` against — names normative.
- `/admin/adoption` JSON is T6's receipt input — extend only additively.

## Disjointness contract

- MAY create the module, add the runtime-config key, the admin route, metric
  names, the seam-registry rows, unit/contract tests, and edit this atom.
- MUST NOT implement any apply (no conductor mutation, no config write, no
  exec/slot touch), edit `happ_manager.rs` / `head_adoption.rs` /
  `hc-mesh.sh` / zomes / sibling scripts. Conductor calls are read-only
  resolves, bounded per sweep (the uncancellable-call rule: size work before
  calling).

## DoD + verification

- `cargo test` green for the module (verify-arm contract tests: envelope
  mismatch, lineage-hint mismatch, threshold-unchecked, honest-absence —
  each a distinct typed reason).
- On the mesh with T1+T2 outputs: a followed channel with a staging head
  shows `verdict: ok` (or the precise refusal) on `/admin/adoption` on every
  peer, within a bounded number of sweeps; `mode: observe` provably applies
  nothing (conductor PIDs + coordinator hashes unchanged).
- `seam-registry.yaml` row present; `placement-audit.py --epr-meta` clean for
  the crate.

## Implementation notes (2026-09-01)

Landed by `claude-opus-t3` against the T1/T2/T5 outputs as they actually exist,
which differs from this atom's prose in three places worth naming.

### Where the manifest actually lives (differs from Scope §3)

The atom says "fetch manifest content + artifact blobs by CID." T2's ceremony
driver does **not** publish the manifest as a content body: `release-ceremony.ts
publish` patches the channel's own `metadata_json` to
`{"kind":"release-manifest","publishedAt":…,"manifest":{…}}` and declares that
version canonical. So the manifest arrives **inside the head resolve** — one
`resolve_content_head` call yields head + tier + `supersedes` + the manifest,
and the only bytes still to fetch are the artifact blobs. Three consequences
baked into `watch.rs::extract_release_body`:

- a channel ROOT (`kind: "release-channel"`) is `Idle`, not malformed;
- an envelope that *claims* `release-manifest` and is unreadable is
  `manifest_undecodable` — reading it as "no release here" would let a corrupt
  publish look like an empty channel;
- the **L2 lineage evidence is free**: the head declaration's own `supersedes`
  IS the version chain, so the lineage check needs no second call.

### Config key shape

The rung-4 registry (`runtime_config.rs`) is a lock-free `AtomicU64` array —
`Kind::{Bool, Seconds}` only, and it cannot hold a list. Rather than widen
`Kind` (a branch on every hot read site for a shape almost nothing uses), a
small parallel **text-setting family** was added with identical semantics
(file overrides, absent key restores boot-env, provenance visible):

```toml
ELOHIM_RELEASE_CHANNELS = "runtime:coordinators:elohim:canary-a=observe, runtime:config:elohim:commons"
```

Comma/semicolon/newline separated; a bare id defaults to `observe`. An unknown
mode is **refused and reported** on `/admin/adoption` (`configRefusals`), never
downgraded to `observe` — a peer told to `apply` that quietly observes looks
compliant while doing something else. Text settings render under
`textSettings` on `GET /admin/runtime-config`.

### Verdict / refusal vocabulary

`AdoptionRefusal` is a data-bearing struct (`reason` label, `detail`, `arm`,
`transient`) over a `Copy` **`RefusalReason`** enum that implements
`seam_contracts::ReasonLabel` — so duplicate/unstable labels are a failing test,
not a silently merged metric series. 20 variants, pinned by
`refusal_reason_labels_are_stable`. Two axes hang off the reason rather than the
call site:

- `RefusalReason::arm() -> DecisionArm` — the metric's `arm` label is a property
  of the reason, so it cannot be mislabelled where it is emitted;
- `RefusalReason::is_transient()` — the retry axis. `dna_lineage_mismatch` is
  terminal (only a new release reopens it) and parks at the ladder ceiling on
  the first sweep; `artifact_unavailable` / `threshold_unchecked` /
  `installed_reality_unknown` are transient and climb the finite ladder.

`Verdict` is a three-way sum — `Idle | Ok | Refused` — so C4's honest absence is
structurally distinct from both success and failure.

### Schema vendoring decision

**Vendored as typed Rust in `mod.rs`, deliberately OPEN** (no
`deny_unknown_fields`, every optional field `serde(default)`) because T1's schema
is open by design for the mixed-version additive floor. Serde alone is not
enough — the schema pins `pattern`s a `String` field happily accepts — so
`verify::verify_shape` enforces them by hand (no `regex` dependency added; the
crate's hand-written-scanner convention). Pinned to T1 by
`release_manifest_mirror_agrees_with_the_rakia_schema`, which loads
`elohim/rakia/schemas/v1/release-manifest.schema.json` and all five committed
fixtures **from disk** and runs a real JSON-Schema validator alongside the Rust
mirror — two independent sources, so neither measures the other.

### seam-registry rows (3, registered at birth)

- `release_adoption::verify::verify` — `verdict-fn`, the floor. C1/C4/C5/C6a/
  C6b/C8/C9/C10/C12/C13/C14 answered; C2/C7/C11 n-a.
- `release_adoption::RefusalReason` — `reason-outcome-enum`. C3/C4/C5/C8/C10/
  C14 answered.
- `release_adoption::state::ChannelAdoptionState::record` — `state-transition`,
  the backoff ladder. C0/C3/C4/C6a/C6b/C8/C12/C13/C14 answered; **C11 partial**
  (per-sweep byte budget defers, but ram-guard / PVC / quiesce state is not read
  — the one honest gap, recorded as `partial` with its reason).

### What did NOT land, and why

- **No apply, anywhere.** `ApplyVehicle` is declared with the atom's normative
  signature plus an additive `handles()` defaulting to `&[]`; there is no impl,
  and `AdoptionController` deliberately has no field that could hold one.
  `DecisionArm::Apply` is in the vocabulary (T4 shares it) but is **not
  pre-touched** at metric registration — this build compiles no vehicle, so a
  measured zero there would claim an arm with no code.
- **Peer blob pull is behind the `ArtifactSource` trait, not wired.**
  `BlobStoreArtifactSource` reads only what is already local; a missing blob is
  the transient `artifact_unavailable`. Wiring `p2p::blob_fetch::race_fetch`
  needs the swarm command channel + inventory candidates threaded into the
  controller — an integrator decision, not a side effect of an observe sweep.
- **`conductor_writes` was not edited.** The head resolve reuses that module's
  `ContentHeadWire` decode mirror but issues the call as
  `AdmissionClass::Background` (a sweep must not take the lane a person is
  standing in). Residual: `conductor_writes` wants a `call_resolve_content_head_classed`
  the way its declare path already has one — another lane's file.
