# Accountable correction — slice 1 environment notes

An **accountable correction** is a feedback signal one peer files against another peer's
published content record — a factual complaint ("this is wrong"), not an edit. Filing one is an
allegation and costs the filer nothing; only the record's own **root author** (whoever's `Create`
action started the record's lineage) can turn it into something settled, by explicitly **accepting**
it and then publishing a **successor** (the amended content). Nobody else's say-so moves the
record. This is **slice 1** of a staged reimplementation — the first bounded increment; later
slices are expected to add capabilities (e.g. delegated acceptance) this one deliberately excludes.
The feature file organizes its eight scenarios as **stations**, one per numbered contract section.

Companion to `accountable-correction.feature` and the contract it is bound to:
`genesis/docs/superpowers/specs/2026-09-06-accountable-correction-contract.md` (read that first —
every section below cites it, e.g. "contract §3", by number). This file is appended to by
whichever agent lands the matching Rust surface, in either direction (a2o first, Rust first) —
never overwritten. Each section names who wrote it and when.

The three coordinator calls (functions storage exposes for a conductor to invoke) this feature
exercises: `create_feedback_signal` files a correction; `create_vouch` (with
`vouch_kind = accept-correction`) is the root author's own act of accepting one — a "vouch" here
means one signed record standing behind another; `amend_content` is the root author publishing the
corrected content, naming the exact prior record ("predecessor") it replaces.

## `ELOHIM_FEEDBACK_NOTIFY` (a2o, 2026-09-06)

Storage learns about a correction through two independent paths: a **durable discovery scan**
(contract §3) — every peer periodically re-scans the records it stewards or has been pointed at,
so a correction is found even if nobody told it — and a **direct p2p notification** (contract §4),
an accelerant that lets a peer learn immediately instead of waiting for its next scan. Station 1
needs a way to prove the scan alone is sufficient, with the notification switched off. No such
flag exists today. (The contract's own text calls this mechanism a "sweep" — this document and the
feature file say "scan" throughout for one consistent term; they name the same thing.)

The a2o story names the flag it needs as **`ELOHIM_FEEDBACK_NOTIFY`** on the storage peer that
_files_ the correction (the author's own peer — in the feature, James's):

- `ELOHIM_FEEDBACK_NOTIFY=0` — suppress sending the direct p2p `feedback-signal` notification
  (contract §4) for a correction this peer authors. The correction is still committed to the
  author's own source chain and still discoverable by every other peer's durable discovery scan
  (contract §3); only the accelerant is withheld.
- unset, or `ELOHIM_FEEDBACK_NOTIFY=1` — today's behavior: notify as well as commit.

This is a **test-only escape hatch** on the notification send path, not a protocol behavior — a
production peer always notifies when it can. The step definitions in
`steps/dataplane/accountable-correction.steps.ts` set this env var on the target peer's process
environment before the correction-filing call and expect the storage process to have read it at
call time, not only at boot. That constraint is load-bearing: `hc-mesh.sh` (the local mesh
harness) never restarts a peer's process between scenarios in one run, so a boot-time-only flag
could not be flipped per scenario.

**Owed by the Rust surface:** reading this env var at the point `p2p/mod.rs` would otherwise send
the `feedback-signal` message (contract §4), and treating it as an author-local, per-call
suppression, not a global peer-startup flag.

## Peer roles this feature assumes (a2o, 2026-09-06)

The Background registers all three household-mesh peers — fixture identities named `matthew`,
`jessica`, `james`, each running its own storage node — but the stations assign them fixed roles
so a reader does not have to re-derive who is who per scenario:

- **Jessica** — root author of the target content record in every station. Only she may accept a
  correction (`create_vouch` with `vouch_kind = accept-correction`) or publish its successor
  (`amend_content`).
- **James** — files every correction (`create_feedback_signal`) against Jessica's record. Station
  3 also has James attempt acceptance, refused because he is not the root author.
- **Matthew** — the third peer: never a root author, correction author, or acceptor in these
  stations. He is the durable-discovery witness — the peer whose scan (contract §3) is what
  proves discovery does not depend on notification.

## The Rust surface as it landed (rust-architect, 2026-09-06)

Written after the coordinator + storage slice landed on `sprint/accountable-correction`. Bind
step definitions to what is written here; where this contradicts an earlier expectation above,
this section is what the code does.

### `ELOHIM_FEEDBACK_NOTIFY` — landed as a RUNTIME-CONFIG setting, not a raw env read

The a2o note above asks for a flag readable "at call time, not only at boot", because
`hc-mesh.sh` never restarts a peer between scenarios. A process cannot have its environment
changed from outside after it starts, so a literal `std::env::var` at the send site could not
have satisfied that — it would only ever see the boot value.

What landed instead is a registered setting in this crate's existing **watched runtime-config
registry** (`elohim/elohim-storage/src/runtime_config.rs`, `Key::FeedbackNotify`), which is the
mechanism already armed on every mesh peer (`ELOHIM_RUNTIME_CONFIG_PATH=<mesh>/<peer>/runtime-config.toml`,
set from boot by `hc-mesh.sh`). It is hot: the send path reads the registry per act.

A scenario flips it on a RUNNING peer, two equivalent ways:

- write `ELOHIM_FEEDBACK_NOTIFY=0` into that peer's `runtime-config.toml` and wait for the 10s
  poller, or force it immediately with `POST /admin/runtime-config/reload` on that peer;
- read back the effective value and its provenance (`boot-env` vs `runtime-config`) from
  `GET /admin/runtime-config` — use this to ASSERT the flip took, rather than assuming it did.

Boot-time `ELOHIM_FEEDBACK_NOTIFY=0` in the process environment also works and remains the
default source; the file overrides it while present, and removing the key restores the boot value.

Semantics as implemented (`services::back_prop::direct_notify_enabled`): at `0`, `back_prop_one_hop`
returns "no predecessors targeted" without sending. The act is still committed to the author's own
source chain and still discoverable by every other peer's durable scan. Default (unset) is `1`.

### Coordinator functions (elohim DNA, `content_store` zome — coordinator-only, DNA hash unmoved)

| Function | Input | Returns |
|---|---|---|
| `create_feedback_signal` | `{ target_action_hash, signal_kind, evidence_action_hash?, standing_impact }` | `ActionHash` |
| `create_vouch` | `{ target_action_hash, vouch_kind, standing_impact }` | `ActionHash` |
| `amend_content` | `{ predecessor_action_hash, content: { title?, description?, content?, metadata_json?, blob_cid?, content_size_bytes?, content_hash?, reach? } }` | `ContentOutput` |
| `get_content_lineage` | `{ action_hash, local? }` | `ContentLineageOutput` |
| `get_feedback_signal_record` | `ActionHash` | `Option<Record>` |
| `get_feedback_signal_refs_for_target` | `{ target_action_hash, resolve? }` | `FeedbackSignalRefs` |
| `list_feedback_signal_refs_by_signer` | `{ signer_pubkey, resolve? }` | `FeedbackSignalRefs` |

`ContentLineageOutput` carries `root_action_hash`, `root_author`, `content_id`, `candidates[]`
(each `{ action_hash, predecessor, author, timestamp, fetch_outcome, in_root }`),
`head_action_hash`, `contested`, `contested_predecessors[]`, and the honest counters
`link_count` / `duplicate_links` / `invalid_link_targets` / `other_root_candidates` /
`unfetchable_candidates` / `truncated`.

**Correction admission changed (contract §8).** `create_feedback_signal` with
`signal_kind: "correction"` now REFUSES unless `evidence_action_hash` resolves to a Content
record that is (a) public — `reach` one of `public` / `commons` — and (b) carries this exact
object in its `metadata_json`:

```json
{ "correctionRequest": {
    "operationId": "<the operation id>",
    "targetActionHash": "<same value as target_action_hash>",
    "signalKind": "correction",
    "standingImpact": "<same value as standing_impact>" } }
```

Any mismatch is a refusal, not a retry. A station that files a correction must author its
evidence in that shape — or use the outbox below, which does it for you.

**Refusal substrings** worth asserting on: `"is not the author"` (a non-root-author
`amend_content`), `"cross-root canonical head already stands"` (an amendment that could never
become the served head), `"correction evidence"` (every admission refusal).

### Storage HTTP

| Route | Purpose |
|---|---|
| `POST /api/v1/feedback/operations` | The submission outbox (§8). Body: `{ operationId, targetActionHash, signalKind, standingImpact, vouchKind?, body? }`. Runs BOTH phases: authors the Correction EPR with Content id `correction:<operationId>` and the embedded request, then files the feedback citing it. |
| `GET /api/v1/feedback/operations/{operationId}` | `{ operationId, phase, status, evidenceActionHash, feedbackActionHash, requestBytesCid, lastError, … }`. |

Status codes: `201` resolved · `202` accepted-but-not-resolved (read `status`: `pending` or
`unresolved`) · `409` the operation id was reused with DIFFERENT request bytes (the request an
operation pins is immutable) · `503` this node has no content cell.

`status: "unresolved"` is a terminal-for-automation state: an uncertain call whose recovery
enumeration found ZERO matches is NEVER auto-resubmitted. A scenario that wants to retry POSTs
the same `operationId` again, which reuses the same evidence action and therefore the same
group — a second act would be a group MEMBER, never a second contribution.

### What a station can and cannot observe today

- **Can:** the coordinator refusals above; `get_content_lineage` naming the exact root and
  reporting `contested`; the outbox's phase/status; `GET /admin/runtime-config` proving the
  notify flip took.
- **Cannot yet:** there is no HTTP read for the per-generation application rows or the
  `rebuilding` generation state. Standing is still read through the existing
  `GET /api/v1/standing/{agent_cid}`, whose semantics CHANGED at this cutover: a correction alone
  now contributes zero and leaves the subject's aggregate ABSENT (Unknown), where before it
  debited the signal's SIGNER immediately. A station asserting "an allegation costs the filer
  nothing" should assert Unknown/absent for the TARGET's root author, not a zero score.
