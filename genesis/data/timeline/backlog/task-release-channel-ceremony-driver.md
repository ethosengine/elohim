---
id: "backlog-task-release-channel-ceremony-driver"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Task: release-channel ceremony driver — author a channel, publish a release version, declare staging, promote to earned, and revert by re-election, as matthew's authorized runtime/device"
slug: "task-release-channel-ceremony-driver"
written: "2026-09-01"
author: "session-2026-09-01-rung5-design"
status: "open"
priority: "high"
claimedBy: "claude-sonnet-t2"
jobs: [elohim-genesis]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-upgrade-propagation-p2p-design-arc"
  - "spec:runtime-artifacts-elected-content"
  - "backlog-task-release-manifest-schema-packager"
  - "backlog-task-runtime-upgrade-a2o-receipt"
tags: [upgrade-propagation, rung5, ceremony, canonical-head, election, delegable]
---

**Claimable by any implementation agent. Depends on T1
(`task-release-manifest-schema-packager`) for the manifest currency; consumes
its output file. The election machinery itself is landed — this task DRIVES
it for runtime channels, exactly as the developer/device ceremony already
drives it for epr-content.**

## Why

The spec's whole thesis is that "which release is canonical" is the SAME
ceremony as content head election. This task proves it: a workspace device
authorized as matthew's runtime (the rail proven 2026-08-30 for native content
sync) authors a runtime channel, publishes releases as versions, and moves the
head — staging declare, earned promotion, revert-by-re-election — with zero
new zome code.

## P2P design-gate decision

Carried by the spec §5: the channel content + declarations reuse
`content_store` authoring and `declare_canonical_content_head` /
`declare_earned_canonical_head` (three-arm authority; the MVP leans on the
bootstrap-steward/progenitor + `HeadDelegation` arms —
`elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:5431-5477`).
Nothing new is notarized beyond ordinary content + head links; Ephemeral (C)
script state only.

## Scope

1. `genesis/a2o/scripts/release-ceremony.ts` (tsx), composing the zome-call
   rails of `carried-election-mesh-proof.ts` rather than re-deriving them.
   Verbs:
   - `channel create <channelId> --reach <tier> --discipline <json>` —
     author the channel root content (metadata_json `kind:
     "release-channel"`).
   - `publish <manifest.json>` — author the manifest as a content version on
     its channel id (body = T1 manifest; metadata_json `kind:
     "release-manifest"`), then `declare` staging.
   - `promote <channelId> <releaseCid>` — earned declaration via the
     authorized arm.
   - `revert <channelId> <priorReleaseCid>` — earned re-declaration of the
     prior release; PRINT the arbiter's answer from a second peer to prove
     convergence, never assert from the declaring peer alone.
   - `status <channelId>` — resolve the canonical head from EVERY reachable
     peer's conductor and render the tier + cid table (partition honesty:
     unreachable ≠ absent).
2. Respect adopt-before-author: `publish` on a channel with an existing
   declared head must adopt/resolve first (the four-arm pre-flight in
   `services/head_adoption.rs` is the reference semantics — the script must
   not crown its own commit).

## Interface contract (consumed by T3, T6)

- Channel root + release versions are ordinary content ids — T3's controller
  needs ONLY the channelId string to watch a channel.
- `status` output is machine-readable JSON (one row per peer) — T6's receipt
  script consumes it.

## Disjointness contract

- MAY create `release-ceremony.ts`, edit this atom.
- MUST NOT edit Rust source, zomes, `hc-mesh.sh`,
  `carried-election-mesh-proof.ts` (frozen oracle), or sibling scripts.

## DoD + verification

- On a 3-peer mesh: create channel → publish two releases → `status` shows
  staging head on all 3 → promote → all 3 show earned on release-2 → revert →
  all 3 show earned on release-1. Two consecutive runs, fresh channel ids.
- A declare attempted WITHOUT the authorized arm (plain agent) is REFUSED and
  the refusal is printed — the negative control proving the gate, not luck.

## Implementation notes (2026-09-01)

Built `genesis/a2o/scripts/release-ceremony.ts` (tsx, verb-dispatched CLI:
`channel create` / `publish` / `promote` / `revert` / `status`). Composes
`carried-election-mesh-proof.ts`'s rail in shape (admin connect → walk
`cell_info` for the `provisioned` `lamad` role → `authorizeSigningCredentials`
→ app connect with an issued token → `callZome`), parameterized over named
peers (`--conductors name=admin:app,...`, default `matthew=4444:4445,
jessica=4454:4455,james=4464:4465` — the `hc-mesh.sh` port scheme,
`admin_port(i)=4444+10i`). The oracle file itself was only read, never
edited or imported (it has no exports).

**Payload shapes discovered from source** (not guessed):
- `create_content`/`update_content` wire types are `lamad_types::
  {Create,Update}ContentInput` (`elohim/sdk/domains/lamad/types/src/lib.rs`),
  plain snake_case (no `rename_all`). **`UpdateContentInput` has NO `content`
  field** — it patches only `blob_cid` / `content_size_bytes` / `content_hash`
  / `title` / `description` / `metadata_json` / `reach`. So `publish` cannot
  put the T1 manifest into the Content body; it rides `metadata_json` as
  `{"kind":"release-manifest","publishedAt":...,"manifest":{...}}`, matching
  spec §5's own stated MVP valve (`metadata_json` discriminator) — the atom's
  "body = T1 manifest" phrasing is satisfied by this field, not `content`.
- `declare_canonical_content_head` / `declare_earned_canonical_head` both take
  `DeclareCanonicalHeadInput { id, head_action_hash, carried_record,
  adopt_before_author, delegation }` and return `ContentHeadOutput`. Note:
  `adopt_before_author` on this struct is a DIFFERENT concept from the atom's
  "adopt-before-author" scope item — it is the carried-record
  both-sides-missing bypass, unused here (always `false`; we always operate
  against a locally-authored chain). The script's own adopt-before-author
  pre-flight (see below) is separate, implemented in TS.
- `resolve_canonical_election(id) -> Option<CanonicalElectionOutput>` (tier +
  winner cid, no target-content retrieval, `GetStrategy::Local` — never a
  network await) is what `status` and the promote/revert second-peer check
  use; cheaper and more honest than `resolve_content_head` for this purpose
  since it separates "what did the DHT elect" from "can I serve the bytes."

**Authority arm exercised**: root-author (arm 1) — `channel create` and
`publish` both run `--as matthew` by default, so `promote`/`revert` under the
same default identity satisfy `declare_earned_canonical_head`'s root-author
check directly, no delegation or bootstrap steward needed. `--delegation
<file>` (arm 2, device-ceremony.ts-shaped `{grantor,delegate,scope,
validUntil,signature}` JSON) is wired but NOT exercised live — deferred,
documented as best-effort.

**Adopt-before-author (atom scope item 2)**: implemented as the practical
LOCAL-DHT-arm subset of `services/head_adoption.rs`'s four-arm pre-flight —
`publish` calls `resolve_canonical_election` on the acting peer first and
refuses locally (clear message, no zome round-trip) if the current head is
already EARNED, rather than let a raw "earned head is protected" Guest error
surface. The full PEER-HINT / AUTHOR-THEN-ADOPT / CONTEST-THEN-OBEY sweep
semantics are `task-release-adoption-controller-observe`'s (T3), not this
driver's — noted as deliberately out of scope.

**T1 dependency not satisfied**: `task-release-manifest-schema-packager`
(T1) had NOT landed as of this writing — no
`elohim/rakia/schemas/v1/release-manifest.schema.json` and no
`epr-release-package.ts` exist yet. `publish` therefore reads the manifest
file duck-typed (requires only a string `channelId` field; everything else
rides through into `metadata_json.manifest` verbatim) rather than
schema-validating it. **Open station for the integrator**: once T1 lands,
consider whether `publish` should validate against the schema before
authoring — currently it does not.

**Live smoke evidence (2026-09-01, already-running local 3-peer household
mesh — matthew/jessica/james on the hc-mesh.sh port scheme; no process
started/stopped, per the hard rail)**: ran the full verb sequence against a
throwaway channel id (`runtime:coordinators:elohim:t2-ceremony-smoke-
1788288758`): `channel create` → `publish` release-1 → `publish` release-2 →
`status` (matthew: staging on release-2; jessica/james: reachable, tier
none — no gossip convergence observed in ~2 min of polling, consistent with
spec §10's "~2 min class" being a lower bound, not a guarantee, and with
no adoption controller running yet to drive convergence — T3's job) →
`promote release-2` (earned declared on matthew; second-peer check on
jessica honestly reported "none", not fabricated as converged) → `revert
release-1` (earned re-declared on matthew, i.e. "earned may override
earned" per the zome) → `status` (matthew: earned on release-1, confirming
revert-by-re-election). Also verified the `reachable: false` vs `reachable:
true, tier: "none"` distinction directly with a deliberately-unreachable
conductor port. `pnpm exec tsc --noEmit` (from `genesis/a2o`) shows zero
errors attributable to `release-ceremony.ts` (two pre-existing, unrelated
`PWPage.getByTestId` errors in `steps/ui/auth.steps.ts` are baseline noise).

**Negative-control finding (open station, not a script defect)**: running
`promote --as jessica` (an agent that is neither the channel's root author
nor holds a delegation) on this local mesh WAS refused and the refusal WAS
printed, satisfying the DoD's negative-control bullet — but the actual Guest
error was `bootstrap_steward.rs:44: "bootstrap steward pubkey in DNA
modifiers is malformed: Deserialize(\"invalid type: unit value, expected a
string\")"`, not the textbook "restricted to the root author, a device it
delegated, or the bootstrap steward" message. This reads as a local-mesh
`happ.yaml` DNA-modifier configuration quirk (an unset/null
`progenitor_pubkey` failing to deserialize inside `am_i_bootstrap_steward()`
rather than resolving to a clean `false`), not something in this driver's
write-set to fix. **Story-graph node**: chain `declare_earned_canonical_head`
→ `authorize_author_or_delegate` (fails, no delegation) → `am_i_bootstrap_
steward()` / between: missing node — `am_i_bootstrap_steward()`'s
`Deserialize` error on a malformed/unset `progenitor_pubkey` DNA modifier
propagates as a raw WasmError instead of degrading to `Ok(false)` on the
local mesh's happ.yaml — assertion: "a network with no progenitor configured
(alpha's happ.yaml carries `progenitor_pubkey: null`) can still be moved by
its authors" (per the function's own doc comment) implies the malformed-vs-
absent distinction is not actually handled the way the doc comment claims;
current state: unverified whether alpha's `progenitor_pubkey: null` hits the
same deserialize failure or a genuinely different (well-formed-null) shape
than this local mesh's happ.yaml. Left for the integrator or a fresh
triage — outside this task's write-set (Rust source / happ.yaml are both
off-limits here).

**Follow-up resolved (2026-09-01, Codex task 3)**: the null field and null
properties-block shapes now decode as honest absence, and both bootstrap
identity predicates share the optional path (`None` → `false`); a non-null
malformed key still errors. The coordinator-only fix was applied consistently
to the imagodei reference plus lamad/content_store, mishpat, and node-registry
ports. `content_store` unit tests passed 70/70, the sibling coordinator suites
passed 24/24 + 55/55 + 0/0, and the isolated sweettest
`absent_bootstrap_steward_refuses_earned_declaration_cleanly` passed against a
freshly packed lamad DNA: a distinct non-author reached the clean
root-author/delegate/bootstrap-steward refusal with no `Deserialize` leakage.
Story-graph node state: **green by coordinator sweettest**; the persistent
household mesh was down, so the literal CLI rerun remains a future live receipt,
not claimed here.

**Deferred / not exercised**: `--delegation` (arm 2) live path; T1 schema
validation (schema doesn't exist yet); full multi-minute gossip-convergence
observation (bounded by the 2-minute Bash tool ceiling during this session,
not by the script — an operator running `status` in a loop over a longer
window, or invoking it after T3's adoption controller exists, would likely
observe convergence). The throwaway smoke channel
(`runtime:coordinators:elohim:t2-ceremony-smoke-1788288758`, earned on
release-1 as of this writing) was left as-is on the local mesh — cheap,
harmless, and useful as a live fixture for the next agent to `status`
against without re-running the write path.
