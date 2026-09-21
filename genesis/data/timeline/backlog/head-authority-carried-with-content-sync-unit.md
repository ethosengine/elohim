---
id: "backlog-head-authority-carried-with-content-sync-unit"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Head authority should be carried WITH its content sync unit — the adoption-trigger ladder is the conservative slice, not the destination"
slug: "head-authority-carried-with-content-sync-unit"
written: "2026-09-18"
author: "doorway-overnight-20260914 shift (doorway-failover pickup)"
status: "backlog"
priority: "high"
tags: [dataplane, head-adoption, trust-gradient, sync, elohim-storage, holochain]
cites:
  - elohim/elohim-storage/src/services/head_adoption_trigger.rs
  - elohim/elohim-storage/src/chain_write_gate.rs
  - genesis/a2o/reports/recovery/doorway-pickup-20260917/final/household-gossip-wedge-notes.md
---

# Head authority carried with the content sync unit

## Principle (operator, 2026-09-17)

Verification cost is paid once, up front, next to the human judgment; a head should not quiesce
as its own property separately from the content it belongs to. Target shape: the sync unit
becomes `{content, signed head record}`, verified once by the receiving peer and projected
atomically ("AdoptPeer by verified record"), with a `trust_path` label on every adoption so the
per-edge price is measurable.

## What shipped 2026-09-17 as the conservative slice

Commits `d599b8587`, `8128a1475`, `cf280f8fd`:
`elohim/elohim-storage/src/services/head_adoption_trigger.rs` — sync-triggered adoption with a
re-probe ladder at 1/2/4/8/15/30s.

Measured on the household mesh: jessica adopted at 47.7s on attempt 3, james at 58.8s on
attempt 5 — versus 4m54s / 7m25s before the trigger existed. The scenario deadline is 75s, so the
margin is thin, and the ladder being *needed at all* is the evidence that DHT walkability lags
content sync by ~45–60s. That measurement is the case for the carried-record slice below, not a
sign the ladder is sufficient on its own.

## The scenario that pins the destination (measured 2026-09-18, final binaries)

`doorway-apex-transition.feature` "A current governed version crosses withdrawal and recovery",
step `doorway "elohim.host" serves authority B's exact head…` (`apex-transition.steps.ts:1252`)
reads the surviving doorway's head **once**, immediately after the canonical author publishes
authority B — no polling, by design (`assertExactPublishedAuthority`). On a healthy fresh household
the adoption trigger adopts on attempt 1 in ~1.0–1.1 s; the single read lands inside that second
and sees authority A (`governed action diverged`, runs 20260918T230005Z and 20260918T231158Z). The
ladder cannot close a zero-lag oracle; only a head that arrives WITH the content can. This step is
the acceptance check for item (1) below.

## Open items

1. **Carried-record adoption** — the successor of the ladder: sync delivers `{content, signed
   head}` as one unit; the receiver verifies once and adopts atomically instead of polling.
2. **`ChainTopOrdering::Relaxed` for head declares** — all declares are `Strict` today; `Relaxed`
   needs a rework of the `canonical_declared_at` / chain-head assertion in the content_store
   coordinator zome (`content_store/src/lib.rs` ~2942–2965). Until then, storage serializes cell
   writes through `elohim/elohim-storage/src/chain_write_gate.rs` (commits `08c619982`,
   `0ff361417`) and retries on `HeadMoved`.
3. **Pure-iroh transport path does not raise the adoption trigger** — only the libp2p/dual path
   does.
4. **Ordered `ContentHeadDeclared` delivery observed 0-for-176**, with gossipsub
   `InsufficientPeers`. Needs a diagnostic that names which conjunct of the delivery precondition
   failed, rather than a bare count.
5. **Trigger cooldown (60s claim) and the re-probe ladder are compile-time consts**, not
   runtime-config — no lever to retune without a rebuild.
6. **Defect, 2026-09-18**: james's `adopt-before-author` CONTESTED path re-mints a canonical
   declaration for the SAME 28 content ids every ~20 minutes (11× each so far) without checking
   that its earlier contested declaration already stands — a non-converging chain-write loop.
7. **Defect, 2026-09-18**: `reanchor_backfill` re-probes the same 29–43 `held` dead candidates
   every ~70s, forever — each probe a failing `declare_canonical_head` zome call, with no backoff
   for a held candidate.
8. **Parked draft**, uncompiled, on branch `sprint/2026-09-17-candidate-head-single-call`
   (commits `5e4dbaa1e` single coordinator extern `resolve_staging_candidate_head`, `462e87498`
   candidate-byte prefetch) — collapses the candidate-head resolve from ~7 zome calls to one.

## Missing node (mintable)

chain / between "sync delivers content" → "peer adopts head" / missing node "the sync unit
carries a signed head record verified once at receipt": assertion — the receiver's adoption
decision needs no re-probe ladder because the head arrived pre-verified with the content; probe —
adoption latency collapses to sync latency (no 1/2/4/8/15/30s stepping), measured against the
47.7s/58.8s baseline above. State: **not built** — item 1 above is exactly this node.

## 2026-09-21 — item 1 attempted and held back; the scenario gained the station it was missing

Story 1.4b — open item 1 above, carried-record adoption — was attempted as `c0393c448` ("the head
arrives with the content — the signed head record rides in the doc") and **reverted off `dev` as
`c3b2583e8`** after an independent second-model review found it unsafe to ship: the receiver declared
on receipt; a replay could roll a head backwards; and adoption could land before the blob bytes,
taking a working page down. It is preserved for redesign on branch `hold/1.4b-carried-head-record`;
the review is at `genesis/a2o/reports/recovery/serving-edge-20260920/codex-review-1.4b.result.md`.

The redesign has to hold four shapes the attempt did not: verification on receipt is **read-only**;
the original election's ordering is **preserved, never re-minted** by the receiver; the work is
**bounded and memoised**; and the **previous verified version keeps being served until the new bytes
are local**.

So item 1 stays OPEN, and the household still adopts the slow way: 47–59 s on a loaded household
(measured 2026-09-18 and 2026-09-20), sometimes never.

The scenario that pins this destination was missing a node regardless, and the finish-line step was
silently absorbing it:

```
chain: doorway-apex-transition / "A current governed version crosses withdrawal and recovery"
between: A "the canonical source author publishes authority B through its storage peer"
      -> C "doorway elohim.host serves authority B's exact head, blob, addressed version,
            HTML entry, and browser bootstrap"
missing node B: the surviving doorway has RECEIVED AND ADOPTED the author's publication —
  assertion: within a small stated bound, doorway "elohim.host" answers a visitor with the
  version the author just published, in the process that was already serving;
  probe: a bounded poll of that doorway's own governed head/blob/addressed version against the
  author's receipt, attaching the observed time-to-adopt
  (`doorway-sibling-adoption-measure/v1`, `apex-transition.steps.ts`)
current state: MINTED 2026-09-21 as one station step before C, bound 10 s — a visitor's tolerance
  for waiting on a page, NOT a floor any shipped path meets today. RED, on purpose: the household
  takes 47–59 s, so the scenario now stops at the station with a measured wait instead of at C with
  an opaque "governed action diverged". C is byte-for-byte unchanged and still reads exactly once,
  immediately after the station: the waiting is the station's, the exactness is C's.
```

**Owed: the first household reading of this scenario with the station in place.** Until `just test
mesh` runs it, the expected red is reasoned, not measured, and the attached `timeToAdoptMs` is the
number the redesigned cure will be judged by. The attachment's mismatch trail names which conjunct
was late (governed action, blob, or addressed version), so a reading that is dominated by blob
propagation rather than head adoption says so instead of being read as adoption latency. Revise the
bound only from that evidence — never to make today's behaviour pass.

## The review's blockers, precisely (2026-09-21, `codex-review-1.4b.result.md`)

Read against `c0393c448` before the revert. Each is verified against the current (post-revert) tree; line
numbers below are the current tree's, which drifted slightly from the review's own citations because the
reviewed commit added lines since removed.

1. **A signed Content record is treated as authority to nominate a head, with no membership/standing check.**
   `validate_carried_record` (`elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:5606-5661`) binds the
   carried bytes to the target action hash, the author's signature, and the entry hash — three checks, all
   present and correct. It does **not** check DNA/cell membership, source-chain validity, author standing, or
   permitted reach; a separate TARGET-ID gate (`:5808-5814`) checks only that the content id matches. The
   declarer-authorization seam is a labeled, documented stand-in:
   `authorize_canonical_head_declarer` (`:3985-3988`) is `fn(_declarer: &AgentPubKey) -> ExternResult<()> { Ok(()) }`,
   commented "DEV-TIME SCAFFOLD... this gate is OPEN: any agent may declare" (`:3968-3976`). A same-id record
   correctly signed by a different DNA or an unrelated author's independent root passes every check this path
   runs.

2. **Replay renews the election timestamp, defeating rollback protection.** `declare_canonical_head` creates a
   NEW canonical-head link before electing (`content_store/src/lib.rs:5822-5826`, calling
   `create_canonical_head_link`, `:2974-3004`); that link's timestamp is `agent_info().chain_head`'s timestamp at
   the moment of THIS write (`:2990-2996`) — never the original action's own clock. A peer replaying a valid
   predecessor record therefore mints a fresh, newer election for the OLD content, which can win against a
   currently-served newer version. `canonical_move_verdict`'s ordering table
   (`elohim-storage/src/db/content_diesel.rs:1783-1809`) orders **declarations**, not content versions, so no
   later stage catches this.

3. **Serving a doc's own unverified blob under a verified head** — see the sibling standalone entry
   [declared-head-blob-serves-unauthenticated-content-under-verified-head](epr:declared-head-blob-serves-unauthenticated-content-under-verified-head)
   (`sync/mod.rs:253-266`, `http.rs:10773-10807`).

4. **Peer-triggered declarations can monopolize the source-chain writer and accumulate permanent links, with no
   rejected-record memo.** `head_adoption_trigger.rs` has no memo for a record already proven invalid — the
   4096-entry memo belongs exclusively to the producer path, so retries re-enter the carried branch and
   re-attempt the same rejected record. `chain_write_gate.rs::write_serialized` acquires the cell's writer mutex
   (`:656`, `lock_for`) and holds it across `call().await` (`:673-683`, guard held from lock acquisition through
   the call's return) — the module's own doc names the wasm body "uncancellable" (`:206`). A caller timeout
   around this does not free the lock any sooner.

5. **The cache can pair submitted bytes with the wrong winner's hash.** `head_adoption.rs:3765` (pre-revert
   numbering) stores the SUBMITTED bytes under the RETURNED winner's hash — the two need not match when a
   staging candidate loses to an earned winner, or when local retrieval succeeds while the carrier supplied
   garbage.

6. **Adoption before the blob bytes can take a working page down.** Installing a new pointer with no
   availability check or scheduled fetch lets the read path's cold-serve arm answer 503 (healing in progress) or
   404 (no peer supplies the bytes) — `http.rs:10480-10498` (the `BlobHealOutcome::NotFound`/`FinalizeFailed`
   arms), forwarded verbatim by the doorway's status passthrough
   (`doorway/doorway-service/src/routes/apps.rs:459-460`,
   `StatusCode::from_u16(status.as_u16())`). There is no "serve the previous verified version until the new
   bytes are local" guarantee anywhere in this path today.

8. **A stale record survives its own head moving.** `projector.rs:171` omits a mismatched record from what it
   projects, but `:378` only WRITES present fields — a doc whose head moves from B to C without C's record
   filling successfully in time keeps record B in the doc indefinitely, silently, because nothing clears it.

11. **Whole-record broadcast discloses fields the projection omits.** `projector.rs:359` gates disclosure on the
    SQL row's reach, not the carried entry's own reach; the full `Content` DNA entry carries fields the
    projection deliberately drops (`source_path`, `author_id`, tags, related ids —
    `content_store_integrity/src/lib.rs:490-525`), so a whole-record broadcast can leak them even when the
    projection was built to withhold them.

**The redesign has to hold four shapes** (unchanged from the prior read of this review, now cross-referenced to
the numbered blockers above): verification on receipt is **read-only** (closes 1, 2); the original election's
ordering is **preserved, never re-minted** by the receiver (closes 2); the work is **bounded and memoised**
(closes 4); and the **previous verified version keeps being served until the new bytes are local** (closes 6).
Items 3, 5, 8, 11 are independent fixes at their own sites, not resolved by those four shapes alone.

## Current decision

**Captured, not started** beyond the shipped conservative slice. Owner: next dataplane/head-
adoption shift. Items 6 and 7 are live non-converging loops on the household mesh right now and
are the cheapest wins (bounded, no design decision required) — a "does the contested declaration
already stand" check and a backoff on held reanchor candidates. Items 1–2 are the design-owning
work; item 8 is a ready-to-review draft that shortens the eventual carried-record path.
