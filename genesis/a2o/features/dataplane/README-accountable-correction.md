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
