---
id: "backlog-seed-provenance-anchor-gap"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Seed/HTTP-created content invisible behind require_provenance — the unifying provenance-anchor gap (two trigger populations, peer-starvation environmental)"
slug: "seed-provenance-anchor-gap"
written: "2026-06-08"
author: "cartographer"
status: "wip"
priority: "high"
relatedNodeIds:
  - "memory:project_local_stack_dht_anchor_gap"
  - "backlog-ci-genesis-stewardship-allocation-seed-truncation"
  - "backlog-qahal-collective-cid-formation-projection-gap"
  - "backlog-storage-content-tag-filter-sql-pushdown"
  - "backlog-single-node-content-provenance-visibility"
tags: [seeder, provenance, dht-anchor, a2o, content, code-domain, peer-starvation, require-provenance, cluster-degraded]
shift_objective: |
  Content with both dht_anchor_hash AND p2p_published_at NULL is hidden by the
  require_provenance=true read gate (content_diesel.rs:161-167/279-285; http.rs:3795-3812),
  returning 404/empty. TWO populations land NULL-provenance: (a) bulk-seeded affinity
  content the peer-gated publish drain never stamps, and (b) e2e-API-created content
  (POST /db/content is diesel-only, http.rs:3416 TODO(p2p-coherence)) the drain never
  stamps. The drain (p2p/mod.rs:3156-3162) bails with 0 connected peers — so when alpha
  is peer-starved (cluster-state.yaml: alpha-cluster-6peer=degraded, 10/13 CrashLooping
  since #1024; shem=offline) nothing stamps. This unifies the #1106 stewardship-tag-read
  reds AND the NEW content-lifecycle "Read own content" 404. Fix the OPEN core: the
  seed/import pipeline must write a derived anchor via CreateContentInputView.dhtAnchorHash
  (storage input field needs adding), OR the p2p drain must stamp p2p_published_at without
  requiring peers in seed/single-node mode. Settle the p2p-design-gate question first:
  should require_provenance exempt creator-scoped "Read own content" reads? Near-term
  These are NOT blocked: the write-then-read scenarios run on household-nodes (available;
  the M/J/J triad is a live multi-peer mesh — doorway-alpha /health peerCount 2). Do NOT
  tag them @requires:p2p-publish — that would hide a real household bug. The cross-node
  minority (Susan/Terrance discover each other's content) is already correctly @requires:shem.
  Done when the affinity stewardship scenarios pass on the HOUSEHOLD (seed+stamp completeness)
  AND "Read own content" passes by decoupling creator-scoped reads from DHT publish (the
  design-gate decision + single-node synchronous stamp) — i.e. fixed on the stable floor,
  not held behind a remote cap.
---

# Seed/HTTP-created content invisible behind `require_provenance` — the unifying gap

This is the **master capture** for a family of a2o reds that, across two completed root-cause
investigations, **converge on one mechanism**. It bundles related sub-captures; the now-resolved
ones are marked DONE with their commits, and the OPEN core is the provenance-anchor fix.

## The unifying mechanism (HARDENED on #1106)

The `require_provenance:true` read gate (`elohim-storage` `content_diesel.rs:161-167` / `279-285`;
`http.rs:3795-3812`) returns 404/empty for any content whose `dht_anchor_hash` **AND**
`p2p_published_at` are **both NULL**. Content reaches that NULL-provenance state when the peer-gated
publish drain (`p2p/mod.rs:3156-3162`, `DRAIN_INTERVAL_SECS=15`) — which is the only thing that
stamps `p2p_published_at` — **bails with 0 connected peers** and never stamps. That happens whenever
the alpha cluster is **peer-starved**.

`genesis/manifests/cluster-state.yaml`: `alpha-cluster-6peer=degraded` (10/13 CrashLooping,
158-168 restarts, failedSince build #1024); `shem=offline`. **Peer-starvation is the environmental
trigger** that exposed both failure populations at once.

### Two trigger populations (same gate, different un-stamped content)

1. **Bulk-seed un-stamped affinity content** — the stewardship tag/read reds.
   *Investigation 1* (scenarios "Value-scanner content has multiple stewards", "Stewardship reflects
   human affinities", "Faith content stewarded by pastoral affinity"): the affinity content **IS**
   reachable by tag and **is not** broken affinity seeding. By-tag counts over
   `genesis/data/lamad/content/*.json`: value-scanner tag=1870 / metadata.category=1865;
   public-observer tag=445 / cat=441; fct tag=218 / cat=15. The **scan windows in the failures**
   (5 value-scanner, 4 public-observer, 24 fct) **EXACTLY match** the provenance-passing
   fallback (category-None) tag-only counts (5, 4) — meaning the alpha stack returned ONLY the few
   provenance-passing rows; the bulk ~1865 affinity `scenario`-type items sat behind the provenance
   gate, un-stamped. Diagnosis: **under-seeded / un-stamped stack, not broken affinity seeding.**

2. **E2E-API-created un-drained content** — the NEW content-lifecycle "Read own content" 404.
   *Investigation 2* (`GET /db/content/e2e-<uuid>` → 404, NEW-red on #1106): **environmental**, alpha
   went peer-starved between builds. `POST /db/content` is diesel-only (NULL provenance;
   `http.rs:3416 TODO(p2p-coherence)`); the drain that would stamp `p2p_published_at` is peer-gated →
   **permanent 404 with 0 peers**. RULED OUT `f38be2635` (the concurrent "heal legacy scope shapes /
   `find_active_projections`" commit — it touches ONLY the EprRouter/project-epr path in
   `rea_commitments.rs`, never content reads; the scenario predates it, introduced 2026-02-23
   commit `9e85c1f5a`, unchanged in the changeset). The "NEW on #1106" label is environmental drift,
   **not** a code regression — so NOT a revert of `f38be2635`, NOT a content-read code fix.

## Resolved sub-captures (DONE — do not re-open)

- **"Susan" persona mapping** → **DONE `fbe3c6d70`.** stewardship-allocation.feature persona drift
  (Susan→Jessica) resolved in the same commit that fixed pagination; the feature now names Jessica.
- **Seeder idempotency partial-write / ratio-sum** → **DONE `ec5937287`** (seeder allocation
  idempotency truncation) **+ `fbe3c6d70`** (`listAllocations` pagination). "Allocation ratios sum to
  ~1.0" is GREEN on #1106. The `limit=10000` existence-read truncation that defeated the per-steward
  guard is fixed; see sibling `ci-genesis-stewardship-allocation-seed-truncation.md` (re-pointed to
  the residual provenance/under-seed gap).
- **suspend-revocation "Matthew suspends a user"** → **DONE `ff768bdb9`** (doorway identifier-keyed
  revocation). GREEN on #1106. (Listed for cross-reference; not a seed/provenance item.)

## OPEN core — the provenance-anchor fix

The durable fix has two candidate homes (the shift settles which, gated by the design question):

1. **Seed/import writes a derived anchor.** `CreateContentInputView` (`elohim-views/src/lamad.rs:108`)
   has no anchor field; the seeder never anchors. Add `dhtAnchorHash` to
   `CreateContentInputView`/`UpdateContentInputView` so the seed/import pipeline writes a
   synthetic/derived anchor (a content-derived CID is the natural value — content-addressed identity,
   not a random UUID — see the p2p-design-gate). This is the bulk-seed-population fix.

2. **p2p drain stamps without requiring peers in seed/single-node mode.** For population (2)
   (e2e/API-created), the cleaner home is making the drain mark `p2p_published_at` even at 0 peers
   when the node is operating in a seed/single-node context (flag-gated so prod still requires real
   DHT publish). This overlaps with the single-node product gap captured separately in
   `single-node-content-provenance-visibility.md`.

**p2p-design-gate question to settle FIRST:** *should `require_provenance` exempt creator-scoped
"Read own content" reads?* If a creator reading back their own just-written content is a legitimate
exemption, the read gate gains a creator-scoped bypass and population (2) closes without touching the
publish path. If not, only the anchor/drain fix closes it. This is a gate decision, not an
implementation detail — answer before writing the route logic.

## NOT blocked — this is household-provable (corrected 2026-06-08)

**An earlier draft of this item proposed tagging the write-then-read scenarios `@requires:p2p-publish`
and HELDing them as "blocked on degraded substrate." That was wrong and is retracted.** The
write-then-read scenarios run on `household-nodes` — the Matthew/Jessica/James triad, which
`cluster-state.yaml` declares `available: true`. A 3-node household is itself a multi-peer P2P mesh, so
it satisfies the drain's `≥1 connected peer` gate. Live evidence (`doorway-alpha /health`, 2026-06-08):
`"p2p":{"enabled":true,"peerCount":2}`, conductor 4/4, `discoveryComplete:true` — the mesh **is**
forming. There is no `p2p-publish` capability to declare blocked; tagging these scenarios out would
**hide a real, fixable household-substrate bug** behind a false env-gate. Do NOT add the cap.

The scope vocabulary already draws the line correctly: the only content scenarios that genuinely need
the remote multi-tenant canvas — *"Susan discovers Matthew's content"*, *"Terrance discovers Matthew's
content"* (cross-node) — are already `@requires:shem` in `content-lifecycle.feature`. Everything else
(Read own content, Create content, Discover content by tag, household-formation `@requires:household-nodes`,
stewardship) is correctly scoped to the household and SHOULD prove there. The principle (operator,
2026-06-08): **go deep on the most stable architecture (household-nodes); minimize the test surface
that needs greater complication (shem / 6-peer soak / federation) to prove.**

So the residual reds are **household-substrate/seed bugs to FIX, not env-blocks to hold**:
- **Read own content** is a *design-coupling* bug, not an env bug — see the p2p-design-gate above. A
  node reading its OWN just-written content must not require a DHT-publish round-trip; that couples the
  most-stable layer (local write) to the hardest-to-prove layer (multi-peer publish). The creator-scoped
  read exemption (or single-node synchronous stamp — `single-node-content-provenance-visibility.md`) is
  the fix, and it makes the hub-optional floor work *and* clears this household red at once.
- **stewardship affinity** reds are seed-completeness/stamp gaps on the household peer (the affinity
  content must be seeded AND its provenance stamped on the node the E2E reads from), not a peer shortage.

`fbe3c6d70` already made the stewardship failures **HONEST** ("No multi-steward allocation among the
first N items — affinity seeding looks broken") instead of misleadingly green-looking — that honest red
is now a true signal to FIX on the stable floor, not to hold.

## Latent hardening spun out (separate item)

`searchContent` is SQL-LIMIT-then-in-memory-tag-filter (`content_diesel.rs:288-306`): it applies
`.order().limit().offset()` THEN `.retain()` filters by tag in memory — a latent cliff, currently
masked because content is idempotent-by-id (~3455 rows < `limit=10000`). Hardening (push the tag
filter into SQL before LIMIT) is spun out to `storage-content-tag-filter-sql-pushdown.md` (priority
low, in-tree-testable now) so this master stays focused on the provenance gate.

## Still-uninvestigated tail (carried here for the ranked menu)

- **reach-commons "Anonymous reader can read the manifesto"** — `manifesto` content seeded with
  non-commons reach → anonymous `GET /db/content/manifesto` 403s. The storage reach gate is correct;
  the seed value (or a missed alpha re-seed after reach-enum reconciliation) is wrong. Distinct from
  the provenance gate (a reach-value seed bug, not a NULL-provenance bug). See
  `project_reach_enum_drift_reconciliation` for the enum-vocabulary context.
- **deep-link / sitemap EPR nav-context** — `elohim-host-landing` EPR doc absent on alpha →
  `GET /api/v1/epr/elohim-host-landing/nav-context` storage-level "epr not found" 404. Routing is
  healthy (verified live 2026-06-04 ~17:30Z); landing-EPR seeding belongs to the landing-pages seed
  path (`blobHash`-regex fix `d32aba767` may need a follow-up anchor/seal step for the EPR doc itself).
- **Sidenote (one glance when in-file):** `GET /api/v1/graphql` 404s at the doorway while
  placement-gaps routes — likely the manifest declares POST-only or the GET hint arm isn't
  manifest-declared; harmless today (clients POST).

## Provenance / discovery context

Discovered during shift 2026-06-04T14-52-post-merge-shakeout-e2e-greenup (iteration 1-2,
genesis#1087); hardened across #1104/#1105/#1106 (12→10→7 failure surface). Evidence chain:
`http.rs:3361`/`3795-3812` (`require_provenance=true`) → `content_diesel.rs:161-167`/`279-285`
(anchor-or-published filter) → `CreateContentInputView` (`elohim-views/src/lamad.rs:108`) has no anchor
field; seeder never anchors → `p2p/mod.rs:3156-3162` (peer-gated drain bails at 0 peers). Confirmed on
alpha CI, not just local stack. Sibling memory: `project_local_stack_dht_anchor_gap` (the
action-keyed refinement — `project-epr` and identity anchoring already work rung-3; the gap is the
unextended action gate).
