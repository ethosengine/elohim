---
id: three-lane-dataplane-performance-contract
title: Three-lane dataplane performance contract — enforce reach, then make each lane measurable, then optimize
status: Draft
class: protocol-canonical
topic: [dataplane, performance, reach, replication, notary, doorway, habits, measurement]
domain: D5
sprint: three-lane-performance
cites:
  - "content-reconcile-gap-rca-handoff | Content-reconcile gap plateau | sha256:78b831ddfe2b825f | path: genesis/docs/superpowers/plans/2026-08-19-content-reconcile-gap-rca-handoff.md"
  - "elohim-seam-map-concern-routing | The Elohim Seam Map | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - "trust-as-efficiency-signal | Trust is an Efficiency Signal | sha256:40b8e3d166c935a7 | path: genesis/docs/content/elohim-protocol/architecture/trust-as-efficiency-signal.md"
  - "head-plane-trust-gradient-program-plan | Head-Plane Trust-Gradient Program | sha256:aee96a34080d4efa | path: genesis/docs/superpowers/plans/2026-08-08-head-plane-trust-gradient-program-plan.md"
  - genesis/manifests/habits.yaml
  - elohim/elohim-storage/src/http.rs
  - elohim/elohim-storage/src/epr_service.rs
  - elohim/elohim-storage/src/trust/adoption_pricing.rs
  - elohim/elohim-storage/src/p2p/reconcile_rails.rs
  - scripts/ci/fleet-quiesce-gate.sh
---

# Three-lane dataplane performance contract

**Written 2026-08-20.** Every claim is marked **PROVEN** (read at the cited line),
**MEASURED** (live probe or Prometheus this session), **INFERRED**, or **UNKNOWN**.
The prior investigation's documented failure mode was inferring past what the
instruments could see, so an unlabelled claim is a defect in this document.

## 1. The model this serves

Performance is optimized **per participation track**, using the atlas's own names
— not a new taxonomy. Three lanes, three different concerns, three latency
budgets that must not be inherited from one another:

| lane | track | job | performance concern |
|---|---|---|---|
| A | **T2** substrate | iroh/SQLite peer sync: blobs present, addressable, deliverable | must be **fast** |
| B | **T1** DHT-notary floor | supply the notarized trust signal | **allowed to lag**, bounded and visible |
| C | **T4** doorway projection | the federated CDN edge | cache effectiveness, served latency |

Two constraints shape every decision below.

**The scale asymmetry is the leverage.** Fewer than 20 peers, who know each other
and have agreed up-front to replicate most of what each other hold, against 4000+
EPRs. Per-item negotiation among consenting peers is the wrong shape; bulk set
reconciliation is.

**Custody is not readability.** A replica holder need not decrypt what it stores.
This dissolves the standing objection to set-level digests — that reach is earned
per-node, so two honest peers hold different visible sets and a shared digest
would lie. If custody is universal and reach governs decryption and serving
rather than holding, the replication set is identical across peers and the digest
cannot lie. **The digest therefore belongs on the replication plane, not the head
plane.** The invariant that survives: a peer holding ciphertext it cannot read
gains **no** reach; serving still requires authorization.

## 2. How much of the model is already true

**Claim: the DHT is the notary and may lag. ALREADY TRUE — no work here.**
PROVEN: the serving floor is one predicate, `content_diesel.rs` `MinTrust`; every
external HTTP boundary passes `Amber`, and `MinTrust::Green` has **zero**
production callers. No read path requires notarization. MEASURED: storage-A
reports `drain {total: 4495, published: 4495, pending: 0}` — every row satisfies
the Amber floor via `p2p_published_at` alone.

The one place it is **not** true is **CI**, not the protocol: the quiesce gate
blocks all measurement on notary-plane terms. That is a shell-script change.

**Claim: nothing should limit the peer sync. PARTIALLY FIGHTS — and differently
than expected.** Two real couplings (PROVEN): a `sync_paused` `AtomicBool` gates
byte replication on the *head-publish* backlog, and replicated rows land with all
provenance markers NULL so arrived bytes 404 until a per-item Kademlia publish
stamps them. MEASURED: neither is firing on alpha today (`drain.pending == 0`) —
they are latent, with unbounded worst cases, not live latency sources.

**One correction worth absorbing before spending: there is no iroh peer-sync to
optimize.** PROVEN: the iroh plane is serve-only — a blob store and protocol
handler with single-hash fetch, no proactive replication driver, and the default
transport backend is libp2p. The bytes move over libp2p `/elohim/shard/1.0.0`
and `/elohim/blob/1.0.0`. **Performance work lands on libp2p. Do not spend a
shift making iroh faster at a job it does not yet do.**

**Claim: bulk set reconciliation, not per-item. ACTIVELY FIGHTS — the prize.**
PROVEN: the replication cycle sends a 1000-item `ListContent` page to every
eligible peer on an unconditional 60s tick, then one `GetContent` per gap, then
one blob `Get` per gap. Cold sync of 4000 items against one peer is ~8000 round
trips; converged steady state re-enumerates ~24,000 items/min/peer **forever**,
because the shard plane has no digest to short-circuit on. The Automerge doc
plane already received exactly this cure and the fold helper is shared one module
over. MEASURED: `inventory_page` mean is **639 ms** across 3,509 observations —
per-item negotiation across 4,000 EPRs is ~43 minutes for a single peer pair
before any bytes move. That number is the case for the digest.

**Claim: custody is not readability. HALF TRUE — the dangerous half.** PROVEN:
the holding half is already the wire behavior; the shard service reads at
`MinTrust::Invisible` so peers can cover pre-drain content, and the blob
responder does one path-traversal check and serves. Universal reach-blind custody
is tonight's wire, not a future design. The **encryption** half does not exist:
`services/private_replica.rs` implements the model exactly — per-content data
key, sealed envelopes, a passing "custodian cannot read" test — and has **zero
production callers**.

## 3. What that made urgent, and what landed

Universal custody is safe only if the **serve** boundary holds. It did not.

MEASURED live on doorway-alpha, verified twice: an anonymous
`GET /db/content?limit=1000&offset=1000` returned 90 rows, all commons. The
identical request carrying the literal header `Authorization: Bearer bogus`
returned **1000 rows — familiar 906, commons 90, private 3, intimate 1 — every
one with its content body.** PROVEN cause: the listing decided reach from header
*presence*, never validation, and `X-Agent-Id` (self-asserted) opened it equally.

That is not a lenient posture. It is the absence of one, which is why it ran on a
production-shaped fleet without anyone having chosen it. **Landed** (`41f7780aa`):
the listing resolves the requester and authorizes per row, deny-by-default when
unresolvable; the single-item route's equivalent check is cured; and
`reach_level_index`'s `_ => 0` fail-open — an *unrecognized* tier read as the most
permissive, making the authorizer's own `_ => Err` unreachable — now fails closed.

Scope held honestly. MEASURED: the doorway strips client-supplied identity
headers (`X-Agent-Cid: matthew` returns the anonymous result), so the cure closes
the bypass for doorway-fronted traffic. It does **not** make identity
unforgeable — `extract_agent_cid` returns the header verbatim — so a caller
reaching storage directly in-cluster can still assert an identity. That residual
belongs to `identity-cross-signed`.

## 4. The lane table

| | T2 substrate | T1 notary | T4 doorway |
|---|---|---|---|
| the one metric | time-to-present p95 | head notarization lag p95 | request duration p95 by route class + cache status |
| proposed SLO | ≤60 s for a single blob into a connected peer set; ≤30 min cold-peer full-corpus backfill | p95 ≤30 min, p99 ≤4 h, hard ceiling 24 h | cache-hit p95 ≤200 ms; cold p95 ≤800 ms; blob cache-hit ratio ≥90% |
| state today | **no series exists** | **unmeasurable** | **no series exists** |

The SLOs are anchored, not guessed. T2's floor comes from the 639 ms
`inventory_page` mean. T1's comes from `adopt_declare` measuring 22.3 s per leg
plus the documented ~20-min conductor restart churn, which makes anything tighter
than ~20 min unachievable across a deploy. T4's is 50× the one real observation
that exists anywhere in the tree.

MEASURED, and it is the practical gate on everything: `p2p_iroh/` contains **zero
metrics call sites** across 25 files; the shard-fetch and put-record atoms show
only their registration pre-touch, i.e. zero real observations fleet-wide; no
doorway request-duration series exists; and **only one doorway is scraped** — the
PodMonitor selects `app: elohim-doorway` while doorway-B's pods are labelled
`app: elohim-doorway-b`. Lane C is half blind while the quiesce gate hard-gates
on doorway-B answering 200.

Correction to an earlier reading in this session: T1's micro-legs **are** timed
(`adopt_declare`, `head_record_verify`). The T2 and T4 gaps stand.

## 5. The couplings to cut, ranked by latency actually removed

1. **No set digest on the replication plane.** Steady state ~24,000 enumerated
   items/min/peer → ~200 bytes/min/peer; one-item delta ~8000 RTT → ~4. Days,
   additive, no migration. PROVEN.
2. **No batch `GetContent`.** Cold start 4000 RTT → ~63 at 64/request; the batch
   precedent already exists on the atom protocol. PROVEN.
3. **Doorway strips cache headers on the blob-miss path** while storage already
   answers `immutable` with an ETag — and the pantry-hit path sets them
   correctly, so the code knows the right answer and applies it only where it
   does not matter. ~10 lines. PROVEN.
4. **Every HTML document is `no-store`**, because a per-request `authenticated`
   flag is spliced into the bundle. Move it to a small uncached fetch. PROVEN.
5. **Arrived bytes 404 until a per-item Kad publish.** Zero cost on alpha today,
   unbounded on a cold peer, infinite at zero connected peers. PROVEN + MEASURED.

**Do not fund:** a reach gate on custody (none exists, and none may be added);
conductor gating of byte replication (none exists); erasure coding (the live
corpus bands at `encoding: none`; with ~20 peers each holding a whole replica,
parity buys nothing); making iroh faster (no replication driver yet).

## 6. The trust lever, measured

MEASURED across all seven pods: `elohim_trust_priced_adoptions_total` reads
`accept_with_provenance = 0` **and** `delta_verify = 0`; all 81 priced adoptions
fleet-wide landed on `full_chain`. The pricer is deployed, running, and has never
cheapened a verification.

PROVEN cause, and it is structural rather than a tuning problem: the cheap corner
requires `row.declared_head_action_hash.is_none()` — the row must declare
*nothing*. The population that dominates post-restart cost is the *declared*
divergent one. **The lever is allowed to price only the population that is not
expensive.** Identity-trust-weighted verification therefore cannot accelerate
quiesce today even if cross-signed bindings landed. Content-inference — receiver
re-derived, needing no trust primitive — does not have this problem.

## 7. Habit deltas

The register is at **12 of 12** habits and **2 of 2** active. Nothing is added and
nothing retired; the lanes already have homes: `blob-durability` (T2),
`notary-authority` (T1), `doorway-failover` (T4, red).

Every lane habit is a **conjunction** — a performance property *and* the trust
invariant it must not spend to reach it. A habit asserting only throughput is
inadmissible, because speed bought by widening unearned reach is the failure the
habit exists to prevent.

**Wire the trust invariant first.** All three lane habits state their trust half
as a *relative* property (placement carries the row's own reach; the cache does
not widen what the origin serves), and a relative property is satisfied for free
by a permissive absolute. Until one habit asserts the absolute — *an unauthorized
requester does not obtain the bytes* — all three can go green over a corpus where
nothing is refused anywhere.

**Landed this session:** `reach-enforced-everywhere` **unwired → red**, bound to
`@concern:reach-enforced-http`. Red on measured violation is a stronger position
than unwired, which was an absent check. The habit's original first clause ("one
reach vocabulary") moved out of the checkable statement: it is a reconciliation
goal, not an enforcement property, and canonizing a vocabulary is forbidden while
the drift is open — which is exactly why the habit sat unwired for so long. The
check asserts a **relation** instead and names no tier.

**Refused this session:** flipping `notary-authority` green → red on a claim that
271/300 EPRs resolve different canonical heads across doorways. MEASURED across
both doorways: `blobCid`, `blobHash`, `serverBlobHash`, `contentBody` and
`validationStatus` agree 20/20; only `dhtAnchorHash` and `updatedAt` differ. The
anchor is a **per-peer ActionHash and is supposed to differ** — the canonical EPR
hash excludes `cid`/`proof`/`supersededBy`, so two peers signing identical content
get the same CID. The salvageable half stands: that habit went green on a feature
naming exactly one EPR, so corpus-scoping its check is worth doing — on a
canonical field, not the anchor.

## 8. Sequencing, and the falsifier

**Instrument, then optimize.** Every optimization proposed before the lanes are
separately measurable is unfalsifiable. The first increment is the smallest set
of series that makes T2, T1 and T4 independently readable, plus two fixes whose
falsifiers are independent of the quiesce curve: the replication-cycle counters,
the doorway PodMonitor label, and cache-header forwarding.

**Falsifier for the batch-sizing fix**, stated before building: `BATCH_SIZE_FLOOR`
is 8 against a measured ~2,666 ms per id and a 12 s extern budget, so the
controller's *floor* asks for ~8 ids where the conductor completes ~4.5; the
remainder returns `unattempted{budget_exhausted}`, which AIMD reads as pressure
and shrinks toward a floor it can never satisfy — a control loop closed over a
signal outside its own reachable range. If `unattempted` goes to zero and
per-sweep verdicts do **not** rise, batch sizing was not the binding cost and the
next lever is the conductor error class.

**The honest ceiling.** No verification-cost reduction converges anything.
`healed` is still 0 and `caughtUp` still false. This work makes the fleet reach
the *gate* faster; it does not make peers agree. Say that in any habit delta, so
a faster gate never reads as agreement.

## 8a. Named risk from the reach cure — fail-closed has a cost

Closing `reach_level_index`'s fail-open was correct, and it carries a consequence
that must not be discovered later by a confused author.

`content.reach` has **no write-time validation**, and the reach vocabulary is in
declared multi-vocabulary drift. Before the cure, a row carrying a non-canonical
reach string read as the most permissive tier — wrongly public. After it, that
row reads as maximally restricted: **invisible to everyone, including its own
owner**, with no backfill path and no error at the point of authorship. The cure
converted a confidentiality failure into an availability one. That is the right
direction — a leak is worse than an outage, and an outage is visible — but it is
a trade, not a free win.

Two follow-ups this implies, neither landed:

1. **Validate `reach` at write time** against the protocol enum, so a
   non-canonical value is refused at authorship rather than silently swallowing
   the row. Today the only feedback is content that stops appearing.
2. **A sweep for already-written non-canonical values**, which are now dark.
   UNKNOWN how many exist; nothing counts them today, which is itself the
   finding — the same missing-instrument shape as §4.

Related, and the reason this is a *named* risk rather than a footnote: a second
fail-open in the same function was found by review after the cure landed.
`reach_level_index` is read in two directions — as a RESTRICTION on content, and
as a PRIVILEGE via `ctx.reach_ceiling` in the ambient trust fast path, where the
ceiling arrives verbatim off the wire from a peer and is cached unvalidated. A
value that fails closed in the first direction fails **open** in the second:
`u8::MAX` satisfies `ceiling_idx >= reach_idx` for every tier, so a peer with a
typo'd or forked vocabulary would buy ambient community access with the
participation check skipped. The lesson generalizes past this function: **a
sentinel is only fail-closed with respect to a direction**, and any ordinal read
in both directions needs an `Option`-returning form rather than a magic value.

## 9. Open decisions for the architect

1. **Ciphertext CID vs convergent encryption.** If the address is the CID of the
   ciphertext, two peers encrypting the same plaintext under different keys
   produce different CIDs — dedup dies and edge caching fragments per recipient,
   which also costs T4 most of its leverage. Convergent encryption (key derived
   from the plaintext hash) preserves both, at the price of a confirmation
   attack: anyone holding a candidate plaintext can test whether the network
   holds it. Harmless for a commons corpus, real for private reach.
2. **Serve-policy posture's home.** Reach enforcement is unconditional; what a
   declared dev stage may cheapen is verification *depth*. That belongs on the
   existing declared-stakes axis (`trust/stage.rs`) rather than a new flag beside
   it — the repo has already paid once for a signal living in two homes that
   drifted. Confirm before building.
3. **Whether the quiesce gate should assert per lane.** Its own header records
   predicates being deleted to unwedge CI. Splitting it is the cut; what the
   notary leg was protecting must be named before it stops blocking.
