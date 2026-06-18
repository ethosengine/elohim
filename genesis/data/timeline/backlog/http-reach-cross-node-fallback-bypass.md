---
id: "backlog-http-reach-cross-node-fallback-bypass"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "HTTP reach gate (Layer 1.5) is bypassed on the P2P cross-node fallback serve path; community-tier left at coarse auth"
slug: "http-reach-cross-node-fallback-bypass"
written: "2026-06-18"
author: "red-team (reach-enforcement adversarial review, post Layer-1.5 landing)"
status: "open"
priority: "high"
themes: [reach-enforcement, intimate-reach, p2p, privacy, epr-service, security]
relatedNodeIds:
  - "backlog-http-reach-enforcement-gap"
  - "genesis/a2o/features/lamad/intimate-reach-household.feature"
tags: [security, reach, cross-node, household-testable, follow-on]
---

# HTTP reach gate bypassed on the P2P cross-node fallback serve path

Follow-on to `http-reach-enforcement-gap.md`. The Layer-1.5 reach gate landed on the
**local** serve path of `GET /db/content/{id}` (`handle_db_content_by_id`, the
`authorize_reach_for_human` single-source core wired after the coarse is_public check).
An adversarial review of that change (2026-06-18) confirmed it is a genuine tightening —
deny-by-default, no new spoof vector (the `X-Agent-Cid` boundary is pre-existing and
doorway-injected from a validated JWT only), correct DNA-notarized vocabulary, dev-mode
personas disambiguated by `humans.id` — but surfaced two residuals that the local-path
fix does NOT cover. Neither makes things worse than the prior all-200 state; both are
filed here so the "single source of truth" claim isn't mistaken for full coverage.

## FINDING 1 — cross-node fallback bypasses Layer 1.5 (MEDIUM; HIGH in multi-node)

**Seams:** `elohim/elohim-storage/src/http.rs` Layer-1.5 gate (`~:4515-4584`) lives INSIDE
`if let Ok(Some(ref view)) = result`, where `result` came from
`get_content_with_tags(..., require_provenance=true)`. When the content is **not local**,
`result == Ok(None)` → the whole gate block is structurally skipped → execution falls
through to the P2P fallback (`http.rs:~4700-4786`):

- `resolve_and_fetch(content_id)` (`p2p/mod.rs:~1591`) carries **no requester identity** —
  it calls `resolve_epr(id)` which sends the **fetching node's own** key
  (`p2p/mod.rs:~1392` `agent_pubkey: self.agent_pubkey.clone()`; outgoing wire request
  `p2p/mod.rs:~2887`).
- The **serving peer** therefore runs `check_reach_authorization` against the *fetching
  node's* identity, not the HTTP requester's. On success the bytes are persisted locally
  (`http.rs:~4740`, `dht_anchor_hash: None`) and returned straight to the HTTP caller
  (`http.rs:~4775` `ContentView::from(...)` → `response::ok(&view)`) — **the requester's
  identity is never checked** for non-local content.

**Severity bound.** The serving peer gates on the fetching *node's* `agent_cid`; the
intimate/trusted/familiar arms require a human relationship/stewardship match, which a
pure node identity usually won't satisfy → fail-closed (availability loss, not
disclosure). It becomes a disclosure when the fetching node's `agent_cid` *does* satisfy
the serving peer's gate (node self_cid is also a steward/household human, or the
content is the node-operator's own intimate content fetched on behalf of an unauthorized
visitor). Latent on the single-household stack (scenarios are local-only today); activates
as cross-node fetch lands in multi-node deployments. Independent of disclosure, it is a
gate-coverage hole: the new single-source core is bypassed on an entire serving path.

**Remediation (pick one):**
1. **Preferred:** forward the resolved HTTP-requester identity into `resolve_and_fetch`
   so the serving peer gates on the right principal at the source.
2. After a successful `resolve_and_fetch`, re-run `authorize_reach_for_human` against the
   HTTP requester before returning (`http.rs:~4775`). Extract a small
   `enforce_http_reach(&req, &mut conn, reach, content_id) -> Option<Response>` helper and
   call it from BOTH the local (Layer 1.5) and fallback return sites (kills the
   duplication and makes coverage symmetric).

Until then: cross-node restricted reads are **not** reach-enforced — state this wherever
the reach gate is documented.

## FINDING 2 — community-tier left at coarse auth (MEDIUM, deliberate)

The gate fires only for `reach_level_index(view.reach) > reach_level_index("community")`
(`http.rs:~4526`), so `community`-reach content is served to **any** authenticated caller
without the `authorize_reach_for_human` "consented collective membership" check
(`epr_service.rs` community arm). This mirrors the P2P fast-path boundary ("community and
below = ambient") and is a design choice (community ≈ broadly-authenticated), not a
regression — but `community` is tier 1 (above commons/public) and is currently enforced
as public-for-the-authenticated. Compounding: the HTTP handler constructs a fresh empty
`PeerTrustCache::new()`, so the ambient fast-path can never fire on the HTTP path anyway —
if community were gated here it would always fall to the full DB membership check.

## Input-data hygiene dependency (not a defect in this change)

Any content row still carrying a **non-canonical** reach value (e.g. legacy `invited`)
maps to `reach_level_index` index 0 → gate skipped → coarse-auth only. The reach-floor
canonicalization effort (see `project_reach_enum_drift_reconciliation`) must land so no row
carries a non-canonical reach. The gate recognizes exactly the DNA-notarized enum
(`elohim/sdk/schemas/v1/enums/reach.schema.json`).

## Provenance / discovery context

Landed Layer 1.5 + the `authorize_reach_for_human` single-source core on
feat/frontend-eyes-sprint (2026-06-18), proving the local path on the household stack
(`epr_service.rs` reach unit tests: intimate-denies-unrelated, private-creator-allow/deny,
unknown-tier-deny). NOTE: live-alpha end-to-end enforcement of the steward/relationship
arms is additionally coupled to `humans.agent_pub_key` population (the
`resilience-card-self-cid-provide-loop-gate` fork) — household-provable now, live-gated on
that fork.

## REFRAME + DEFER (2026-06-18) — the delivery-filter is the INVERTED frame

Operator review flagged that this whole line of work (Layer 1.5 + this FINDING 1 fix)
treats reach as a **delivery-side deny-filter** (403 the reader by content reach tier),
which is the frame the protocol **explicitly inverted**. The canonical reach model is
**author-side earning + receiver-side pre-authorization (AttentionTending) + reach-by-consent
(invitations)** — NOT a per-message serve gate:
- `elohim/elohim-storage/src/p2p/reach_authorization.rs`: receiver-side pre-auth "decides
  which topics this node subscribes to, which Kad provider records it advertises … **It is
  NOT a per-message filter**." ("Email collapsed because anyone could publish to anyone,
  putting the cost of filtering on receivers.")
- History `2026-06-11-storage-dual-plane-design-arc.md`: "its **delivery-side reach
  filtering was inverted by author-side earning**."
- `AttentionTending` (`p2p/attention_tending.rs`, `p2p/attention-tending.schema.json`): a
  peer-private, TTL-bounded *tending of the shape of one's attention* — discernment that
  informs distribution/discovery, NOT a server gate. This is the "thinner" receiver mechanism.
- Theology/values: "Intimate belonging is never gated; only reach is" — reach governs
  spread/teaching-office, earned up front.

**The red-team confirms the frame (not just the impl) is wrong:** `EprHead.qahal.reach` is
**forgeable by any relay** (unsigned, not CID-bound; `p2p/mod.rs:1591-1620` verifies only
content-bytes-vs-blob-CID). You cannot reliably *filter* on forgeable wire data — real
enforcement belongs at the **distribution/discovery layer** (`classify_pre_authorization`:
which provider records a node advertises / which scopes it subscribes to), so restricted
content is never providable to a non-authorized peer in the first place.

**`reach-by-consent`/invitations is also design-incomplete** (doorway `"invited"` =
"authenticated only", `cache/access_control.rs`). Operator: defer until that design lands.

**DECISION:**
- **FINDING 1 cross-node delivery-filter fix → DEFERRED.** The implemented + red-team-cleared
  draft (helper `reach_gate_decision`/`enforce_http_reach` + the pre-persist fallback gate) is
  saved at `.claude/data/reach-cross-node-delivery-filter-draft-2026-06-18.patch` (recoverable).
  NOT committed. The correct fix is at the distribution/attention-tending layer, pending the
  reach-by-consent design.
- **The committed local gate (`d09928387`, Layer 1.5 + `authorize_reach_for_human`) is KEPT as
  transitional defense-in-depth** — it closes a real intimate-content leak in the *current*
  shared-projection architecture — but it is the inverted frame, not the canonical model.
  Operator decision pending: carry the stopgap, or remove it in favor of the
  earning/attention-tending mechanism when the reach design finishes.
- R1 (forgeable head reach) + R2 (cross-node `community` served to anon; fallback lacks
  Layer 1) from the red-team are recorded as why the filter is leaky-by-construction — not new
  serve-layer bugs to patch, but evidence the layer is wrong.
