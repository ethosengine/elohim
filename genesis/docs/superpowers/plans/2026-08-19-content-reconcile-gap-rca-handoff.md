---
id: content-reconcile-gap-rca-handoff
title: Content-reconcile gap plateau — what is cured, what is inferred, and every gap still needing RCA
status: Draft
class: protocol-canonical
topic: [dataplane, projection-reconcile, content-arm, reach, quiesce, head-plane, measurement]
domain: D5
sprint: content-reconcile-gap-rca
cites:
  - genesis/data/timeline/backlog/content-gap-limit-cycle-blocks-convergence.md
  - genesis/data/timeline/backlog/quiesce-gate-measurement-availability.md
  - genesis/data/timeline/backlog/spin-divergent-undeclared-rows-block-a-convergence.md
  - "substrate-trust-contract-runbook | The per-red decision tree this handoff routes G4 into — adam/elohim.host shedding catching-up is a dataplane red whose triage order (CFS throttle and breaker before the identity plane) the runbook already fixes | sha256:e47d962ca7259c79 | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md"
  - elohim/elohim-storage/src/p2p/projection_reconcile.rs
  - elohim/elohim-storage/src/db/content_diesel.rs
---

# Content-reconcile gap plateau — RCA handoff

**Written 2026-08-19** by the session that traced the alpha content-arm stall.
Everything below is marked **PROVEN**, **INFERRED**, or **UNKNOWN**. Do not
promote an INFERRED line to a decision without running the check attached to it.
One prior investigation was derailed by exactly that (see §7 H3).

---

## 1. One-paragraph model of what is going on

alpha's content projection has never converged, and the reason is **not** one
defect. It is a stack: a **classifier conflation** manufactured a large permanent
"gap" population that can never resolve (cured this session, fleet-unconfirmed);
that population **saturates the conductor batch budget**, so the genuinely
healable work is starved behind impossible work; the resulting gap floor keeps
`divergent_actionable`/`caughtUp` from ever settling, which makes the
**fleet-quiesce gate unmeasurable**, which is why saga chapters read red or
unmeasured. Orthogonally, the **B-side node (adam / elohim.host) is in a
permanent `catching-up` shed** that fails three saga chapters on its own, and a
**single poison row** disables the fleet-wide corpus-digest shortcut. Each layer
was invisible behind the one above it.

---

## 2. What landed this session

Commit `53c3a04cc` (pushed to dev, edge #1370), plus `698c56063` held unpushed.

`classify_content_gap` asked its two questions with mismatched predicates:

| question | source | filter |
|---|---|---|
| is it present? | `content_ids_present` | `h_app_id` only — reach-agnostic, anchor-agnostic |
| is it anchored? | `list_content_anchor_inventory` | `dht_anchor_hash IS NOT NULL` **AND** `reach IN (community, public, commons)` |

So `present && !local_anchors.contains(id)` was read as *"un-anchored"* when it
actually means *"not advertisable"*. A present, **anchored** row held at a scoped
reach satisfies the first and fails the second → classified `AnchorGap` forever,
because it can leave that set by neither available route:

- heal stamps **anchors** and never widens reach, so healing cannot resolve it;
- `local_total` counts distribution-safe rows only, so it cannot age out.

Cure: `anchored_content_ids_any_reach` (reach-agnostic, **local-only** — the
advertised inventory keeps its distribution-safe filter, which is a requirement
not an optimization), `ContentGap::ReachScoped` (counted, resolved in the
`MissLedger`, never enqueued), and `elohim_projection_reconcile_reach_scoped`.

Verified locally: classifier test red→green (and a companion assertion that a
genuinely NULL-anchor row is **still** `AnchorGap`, so scenario-2 heal is not
silenced); a SQL-level test pinning all three predicates against real SQLite; a
render test proving the new gauge actually reaches `/metrics`; full
`just gate elohim-storage` green (fmt, clippy `-D warnings`, 2806 lib tests, all
integration binaries); pre-push gate ALL CLEAR.

**This cure is fleet-UNCONFIRMED.** §3 is the falsifier.

---

## 3. The falsifier — run this first

After edge #1370 rolls alpha, read on matthew:

```promql
elohim_projection_reconcile_reach_scoped{stream="content"}
elohim_projection_reconcile_gaps{stream="content"}
elohim_projection_reconcile_divergent{stream="content"}
elohim_projection_reconcile_local_total{stream="content"}
```

- **CONFIRMS** — `reach_scoped` ≈ 1500–2000, `gaps` collapses toward `divergent`
  (~600–1000), `caughtUp` becomes reachable. Then §4 G2 becomes the next
  question (*why* are those rows scoped on two pods only).
- **FALSIFIES** — `reach_scoped` ≈ 0 and `gaps` stays ~1900–2900. Then the
  population is something else entirely, the cure is a latent-bug fix only, and
  §4 G2 reopens from scratch with the reach theory dead.

The change is safe either way: it can only reclassify rows that are **provably
anchored**, so the downside is "no improvement", never a regression.

The single decisive query nobody has run (operator-side; needs pod access):

```sql
SELECT reach, dht_anchor_hash IS NULL AS unanchored, count(*)
FROM content WHERE h_app_id='lamad' GROUP BY 1, 2;
```

Run it on **matthew and susan** and diff. That settles G1 and G2 together in one
shot and makes everything below cheaper.

---

## 4. The gap inventory — every distinct thing still needing RCA

### G1 — classifier conflation · **CURED, fleet-unconfirmed**
See §2/§3. Owner: whoever reads the falsifier.

### G2 — why are matthew and jessica the only two short? · **UNKNOWN, highest value**

`local_total{content}` (anchored ∧ distribution-safe), 2026-08-19:

| pod | local_total | gaps | divergent | exhausted |
|---|---|---|---|---|
| **matthew** | **2466** | 1901 | 554 | 318 |
| **jessica** | **2601** | 1135 | 694 | 2 |
| james | 4489 | 737 | 812 | 75 |
| susan | 4489 | 1809 | 1969 | 160 |
| eve | 4485 | 1899 | 2014 | 120 |
| gertrude | 4481 | 2268 | 2443 | 175 |
| adam | 4364 | 585 | 580 | 4 |

matthew `/db/stats` `contentCount` = **4495**, so the rows are *present*; only
2466 are anchored-and-distribution-safe. matthew's `local_total` moved
2445 → 2466 in **7 days**.

**PROVEN:** the rows exist locally; matthew has exactly **1** NULL-anchor lamad
row (`provideLoop.reanchorPending`, and `witness_bootstrap` runs unconditionally
at the tail of every `run_heal`, so the value is fresh).

**INFERRED:** therefore ~2000 of matthew's rows are anchored but at a
non-distribution-safe reach.

**UNKNOWN — this is the real open question:** *why matthew and jessica and not
the other five?* Candidate lines, none checked:

1. **Seed provenance** — were these two seeded by a different path or era than
   the other five? Check `created_at` clustering of the scoped rows and correlate
   with deploy/seed history.
2. **Legitimate earned reach** — reach is earned and per-node. If matthew simply
   has not earned distribution-safe reach for those rows, there is *nothing to
   fix* and the correct outcome is that `reach_scoped` is a large steady-state
   number forever. **This possibility must be ruled in or out before anyone
   "fixes" the data** — widening reach that was never earned is a governance
   violation, not a repair.
3. **A write path that sets a narrower reach on one code path** — RC-4 falsified
   `apply_content_patch_fields` (no reach branch on UPDATE). Other write paths
   (import, seed, `create_content`, patch-apply, salvage placement) are
   unchecked.
4. **Doorway-facing pods differ** — matthew is behind doorway-A. Is there a
   doorway-side write path that lands rows at a scoped reach?

### G3 — the single poison NULL-anchor row · **PROVEN symptom, UNKNOWN identity**

`provideLoop` on matthew: `reanchorPending: 1`, `reanchorCompleted: 0`
(cumulative), `reanchorFailed: 0`. Neither completed nor failed ⇒ it is being
**skipped**, which in `reanchor_backfill::run_once` means one of exactly two
guards: non-canonical `reach` (not in `CORE_REACH_LEVELS`) or non-canonical
`content_type`.

Impact is out of all proportion to one row:

- pins `reanchorCaughtUp: false` permanently;
- pins `head_corpus_digest_readiness` in **Amber** (the predicate is
  `pending > 0` over the *whole* distribution-safe corpus), which suppresses the
  **T5 corpus-digest shortcut fleet-wide** even though
  `ELOHIM_HEAD_CORPUS_DIGEST=true` is set on every human.

**Checks:** find the row (`SELECT id, reach, content_type FROM content WHERE
h_app_id='lamad' AND dht_anchor_hash IS NULL`), decide correct values, fix the
seed data (`check-reach-drift.mjs` is the guard that should have caught it).
Then ask the design question: **should one unfixable row be able to disable a
fleet-wide lever?** Amber is currently whole-corpus and binary — see G8.

### G4 — adam / elohim.host permanent `catching-up` shed · **PROVEN, UNDIAGNOSED**

`https://elohim.host/` → 503 `{"status":"catching-up"}`. Also 503 on
`/db/content/...`, `/api/v1/resilience/...`, and even `/p2p/status` — it sheds
its own diagnostic probe. `/db/content/elohim-host-landing/head` answers 200 and
**agrees byte-for-byte with alpha-A** (same `headActionHash`, same `blobHash`,
`trust: notarized`), so this is not head divergence.

This alone fails **three saga chapters** (ch04 doorway-serves, ch06
heads-converge, ch10 card-tells-truth) — all four failing scenarios in edge #1366
are this same 503. adam's own reconcile numbers are unremarkable (`local_total`
4364, gaps 585, exhausted 4), which makes the shed the more interesting question.

**Checks:**
1. Which predicate sets `catching-up` on the doorway
   (`doorway/doorway-service/src/routes/catching_up.rs`) and which upstream
   signal feeds it — projector lag, admission shed, or storage health?
2. `/p2p/status` being shed contradicts `is_diagnostic_probe()` listing it as
   exempt (`catching_up.rs:132`). **Either the exemption is broken or the shed is
   coming from a different layer.** This is a concrete, cheap, high-value check.
3. Is adam's storage or its conductor the source? Per the trust-contract runbook,
   check CFS throttle / breaker before the identity plane.

### G5 — conductor batch throughput · **PROVEN numbers, UNKNOWN cost centre**

matthew, `resolve_content_heads_local`, 6h: 349 calls answered, 183
`call_failed` (**34%**), 5242 ids `unattempted` (all `budget_exhausted`).
Heal outcomes: `healed` **0**, `missing` 184, `missing_deferred` 143,
`refused_declared` 149, `refreshed` 82, `call_failed` 2596, `unattempted` 5228.

That is **~1.6 ids resolved per 12-second batch call** (~7.5s per id). Admission
is *not* the constraint (`capacity 5`, `in_flight 2`). Mean
`head_batch_queue_wait` ≈ 1.9s on matthew.

Much of the volume is downstream of G1 (impossible work crowding the budget), but
the **per-id cost inside the extern is its own defect** and G1 does not explain
it. Note the module doc records a prior incident with this exact signature
(healed 0/hr) cured by raising the budget 4s→12s; we are seeing it again at 12s,
which suggests the budget is not the lever.

**Checks:** where the 12s goes inside `resolve_content_heads_local` (DB permit
queue-wait vs wasm body); what the 183 `call_failed`s actually are (WS error?
decode? conductor restart?); whether AIMD is pinned at the floor and why.
Re-measure **after** G1 lands — the population change may move all of these.

### G6 — `healed=0` vs `healedTotal` climbing · **RESOLVED as an instrument trap, but the SPIN population is real**

`/p2p/status` `healedTotal` accumulates the GapTracker's `completed`, which
counts rows brought to a *settled* state — `refreshed` and `refused_declared`
settle **without converging**. `Refreshed` is documented in-code as "real work
but NOT convergence". So a climbing `healedTotal` against a flat `local_total`
is the **SPIN signature**, not progress. The convergence measure is
`elohim_projection_heal_outcomes_total{outcome="healed"}`, which is 0.

Sustained `refreshed` against non-zero `divergent_anchor` means two peers hold
genuinely different roots and only a canonical channel can converge them. Ties
to `spin-divergent-undeclared-rows-block-a-convergence.md` (still open;
`known_divergent` = 13/15/13 on matthew/jessica/james matches that entry exactly).

### G7 — fleet-quiesce gate unmeasurable · **downstream, but has its own cure**

Already filed (`quiesce-gate-measurement-availability.md`). #1367–#1369 all ended
DID-NOT-MEASURE. Largely downstream of G1+G4, but the gate-policy half stands on
its own: a sustained-window predicate that tolerates bounded steady-state
oscillation (time-weighted `actionable<=tol`, not instantaneous), or a
measure-under-churn mode that runs the suite and **labels** the churn state
instead of refusing. Re-evaluate after G1 lands.

### G8 — corpus digest is whole-corpus and binary · **DESIGN GAP, unbuilt**

The only place the topology (7 peers vs 4489 EPRs) is exploited is a single
`sha` over all `id=anchor` lines. Two structural failure modes:

- **Amber is whole-corpus** — one NULL-anchor row anywhere abstains the whole
  shortcut (G3 is currently exercising this).
- **The verdict is binary** — one differing row out of 4489 forces full
  enumeration. It tells you *that* you differ, never *where*.

So it pays out **only when there is nothing to do**.

Proposal (unbuilt): ~64 range-bucket digests (bucket by id-hash prefix). Per peer
per sweep ~2KB of hashes instead of a ~400KB page; drill only into mismatching
buckets. Buys: divergence located in **one** round trip instead of waiting for
the rotating window (kills the latency floor in G9); **per-bucket amber** so
un-witnessed rows disable only their own bucket; a non-binary verdict; and
"compare hashes with all peers, enumerate from one".

### G9 — discovery is O(corpus × peers) with a 15-minute latency floor · **PROVEN**

Each sweep asks each peer for a 2000-row page (`PROJECTION_INVENTORY_CAP`), then
rotates the window one page. A 4489-row corpus needs **3 sweeps ≈ 15 minutes**
per peer before a given divergence is even visible — against a quiesce deadline
of 2700s. `peersAsked` is 4–6, so most of that wire traffic is redundant. Cured
by G8; recorded separately because the latency floor, not the bytes, is what
costs the gate.

### G10 — the gauges oscillate; single readings lie · **instrument trap**

`gaps` is a **per-sweep recount over a rotating page**, so it swings hard.
Observed on matthew within ~2h: `pending` 1644 → 2909 → 2353 → 1901;
`divergentAnchor` 533 → 1610 → 1018 → 554. **Never RCA from one instant.** Use
ranges, and prefer `local_total` (a real corpus count) and
`heal_outcomes_total{outcome="healed"}` (a real counter) as the ground-truth
pair — those two are what exposed the stall when the oscillating gauges hid it.

### G11 — `dataplane-convergence` stayed green through a 7-day non-convergence · **coverage gap**

The habit's checks (a2o concerns + `sync_libp2p_convergence`) all pass while the
fleet demonstrably does not converge. A DELTA is recorded on the habit; the
status was deliberately **not** flipped (no fleet evidence yet). The real
question for whoever picks this up: **what runnable check would have gone red?**
Candidate: a bounded assertion that `local_total` on every pod reaches the fleet
max within N sweeps — i.e. convergence as a *rate*, not a *state*.

### G12 — sibling arms are NOT affected · **PROVEN, recorded so nobody re-checks**

- **REA arm** — `rea_commitments::inventory_for_reconcile` has **no reach
  filter** and includes un-anchored rows as `""`. No conflation.
- **Collectives arm** — keyed by `collective_cid`, NULL-cid rows excluded from
  both sides symmetrically. No conflation.

The bug is content-arm-specific.

---

## 5. Instruments and what each one lies about

| instrument | trap |
|---|---|
| `/db/stats` | returns only `contentCount` + `uniqueTags`. **Cannot see anchor state.** This is what derailed H3. |
| `/db/content?limit&offset` | trust-gate filters **after** offset, so page sizes are nonsense (991, 90, 0, 19, 26). Never count with it. |
| `/p2p/status` `healedTotal` | counts *settled*, not *converged* (see G6). |
| `gaps`/`divergent` gauges | per-sweep recount over a rotating page — oscillate ~2× (G10). |
| `reanchorPending` | trustworthy; `witness_bootstrap` runs unconditionally each heal tick. |
| Loki | returned 502 throughout this session. Zeros from it are untrustworthy. |
| `jenkins-sync.sh` | defaults to `lastSuccessfulBuild`, which may be a validate-only run that archived **no** report — the fetch 404s and it silently keeps the stale local file. It served a 5-day-old report (#1349) this session, understating the saga as 6/11 when #1366 said 8/11. **Always pass `JENKINS_BUILD=<a build that measured>`.** |
| `provideLoop.active: false` | meaning unknown; not investigated. |

---

## 6. Suggested order

1. **Read the §3 falsifier** the moment #1370 rolls. Everything branches here.
2. **Run the §3 SQL** (operator). Settles G1 and G2 together.
3. **G4** — independent of everything else and worth 3 saga chapters. The
   `/p2p/status`-is-shed-despite-being-exempt check is cheap and pointed.
4. **G3** — one row, big blast radius.
5. **G5** — re-measure only *after* G1 lands; the population change may move it.
6. **G8/G9** — the real scaling work, once the fleet is converging at all.

Do not start G5 or G8 before G1's falsifier reads out. Both of their measurements
are polluted by the impossible-work population.

---

## 7. Correction to prior art — read before trusting the backlog

`content-gap-limit-cycle-blocks-convergence.md` carried H3: *"~2000 rows present
but `dht_anchor_hash` NULL on matthew/jessica"*, marked **CONFIRMED via
`/db/stats` parity**. That confirmation does not hold. `/db/stats` returns only
`contentCount` and `uniqueTags` — it establishes row-count parity (which
correctly killed the H2 corpus-split hypothesis) but **cannot see anchor state**.
The "~2000 NULL" was a subtraction (`contentCount` − anchored-distribution-safe)
that silently assumes the only reason a row is absent from the advertised
inventory is a missing anchor — the exact conflation that caused the bug.

Separately, RC-4 falsified *reconcile narrowing reach* (`apply_content_patch_fields`
has no reach branch on UPDATE). That is true and stands, and it is a **different
claim** from *rows seeded at a scoped reach*, which remains live (G2).

Both prior findings survive as stated. Neither supports the H3 reading. RCA #2 is
appended to that backlog entry; a `run:correction` note is recorded on
`projection_reconcile.rs`.

**The general lesson worth carrying:** every wrong turn in this investigation —
H3, and my own first reading that matthew was *missing* ~2000 rows — came from
the same subtraction against an instrument that could not see the distinction
being inferred. When a number is derived by subtracting two differently-filtered
counts, find the instrument that measures the thing directly before building on
it.
