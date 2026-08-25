---
id: "backlog-security-doorway-blob-pantry-ungated"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Doorway blob pantry caches and re-serves any 200 blob body with no reach or Authorization re-check — one authorized fetch of a private blob makes it anonymous-readable until eviction"
slug: "security-doorway-blob-pantry-ungated"
written: "2026-08-23"
author: "fable-5 (red-team finding folded from the swarm-curve + blind-custody design pass)"
status: "refined"
priority: "high"
area: "doorway/serving-path"
domain: "protocol"
jobs: [elohim-holochain]
relatedNodeIds:
  - "habit:reach-enforced-everywhere"
  - "habit:blob-durability"
cites:
  - "swarm-curve-and-blind-custody-design | The swarm curve and blind custody | sha256:ef23b30ec9b8145c | path: genesis/docs/superpowers/specs/2026-08-23-swarm-curve-and-blind-custody-design.md"
  - genesis/data/timeline/backlog/arch-confidentiality-plane-backlog.md
tags: [security, doorway, blob, reach, cache, bounded-feature, codex-claimable, agent-agnostic]
---

# Doorway blob pantry is ungated

**Finding (CONFIRMED 2026-08-23, red-team lens).** `doorway/doorway-service/src/routes/storage_proxy.rs`
`forward_blob_to_storage` buffers the upstream body (~:905) and stocks the pantry on the bare
condition `status == 200 && status != 206 && len <= blob_pantry_max_bytes()` (~:911-917) — **no
reach check, no `Authorization` check**. The hit path (~:775-790) serves the cached bytes to any
later requester for that hash without re-evaluating `ctx.agent_cid`. The freshness pantry next door
already does this right: `routes/freshness.rs` `should_stock` (~:210) / `reach_is_stockable`
(~:243) refuses `Authorization`-bearing and non-public/commons bodies.

**Why it matters now.** Any private-reach blob (or, once blind custody lands, a key ring) fetched
once through a doorway by an authorized reader becomes anonymous-readable from that doorway until
eviction. This is a live `reach-enforced-everywhere` hole on the serve path — exactly the boundary
that habit's `first_move` names.

## Scope (doorway-service only)
1. Apply a `reach_is_stockable`-equivalent predicate to the blob pantry's stock decision: never
   stock a body whose request carried `Authorization`, and never stock a body whose reach (from
   the storage response headers / the EPR lookup the forward already does) is not public/commons.
2. Hit path: if a cached entry exists but the *current* request would not have been stockable
   (authenticated request for a non-public hash), bypass the pantry and forward.
3. Counter: `doorway_blob_pantry_skipped_total{reason="authz"|"reach"}`.

## DoD / verification
- Unit test: a 200 body for a private-reach hash with `Authorization` is NOT stocked; a commons
  body is; a second anonymous request for the private hash forwards (miss) rather than hits.
- `just gate doorway-service` → `GATE_EXIT=0` echoed on its own line.
- Household lane: `just test mesh features/dataplane/blob-replication.feature` and the
  `reach-enforced-everywhere` scenarios show no new red.
- Commit path-limited (`git commit -- <paths>`); never `--amend`.

## Disjointness
Write-set is `storage_proxy.rs` (+ its tests). Does not touch `freshness.rs`, elohim-storage, or
the blob swarm rows. Independent of every other row in the spec's §9.

---

## RESOLUTION + CORRECTION (2026-08-24, verified on the two-peer local mesh)

**The finding as originally filed OVERSTATED the pantry's role, and the live repro corrected it.**
The claim was that "one authorized fetch makes a private blob anonymous-readable *until eviction*"
— i.e. the pantry widens the audience. It does not. Measured against `community`-reach content
`community-garden-club` (blob CID `bafkreibg27hns7…`), fully anonymous (no `Authorization`, no
`X-Agent-Cid`):

| request | result |
|---|---|
| `GET /db/content/community-garden-club` | **403** `{"requiredReach":"community"}` |
| `GET /blob/<cid>` doorway A | **200** — the gated content, verbatim |
| `GET /blob/<cid>` doorway B (COLD — never cached there) | **200** |
| `GET /blob/<cid>` storage :8090 / :8091 directly (no doorway) | **200** |

Doorway B served it on a **cold miss**. The bytes are already anonymous-readable with or without
the pantry, because the real hole was one hop up: **`GET /blob/{hash}` in elohim-storage enforced
no reach at all** (its only gate was a device-policy filter that fired solely when an agent was
present, with `reach_level` hardcoded `None`). Full-corpus scale: 38 reach-gated blob-bearing rows,
all 38 leaking their CID anonymously via `/epr-head`, **36/38 bytes anonymously retrievable**. This
is the byte-route half of `http-reach-enforcement-gap` and the live instance of
`arch-confidentiality-plane-backlog` #3 ("holding the locate-token implies fetch rights").

**What shipped (all locally-verified, uncommitted at time of writing):**

1. **Storage-side reach gate (the real cure)** — `elohim/elohim-storage/src/blob_reach.rs`
   (`blob_serve_verdict`, pure, 9 unit tests) + `HttpServer::blob_reach_refusal` wired into
   `handle_get_blob` BEFORE both the iroh and legacy backends. Rule: *a blob serves to a caller iff
   some referencing content row does* ("some" not "all" — content addressing dedups; 12 shared CIDs
   in-corpus). Candidate-matching validated against a copy of the real mesh DB: 38/38 gated rows
   resolve to a gated verdict; public + unreferenced rows still Serve (no false refusal). Registered
   in `elohim/elohim-storage/seam-registry.yaml` (`blob_serve_verdict`, census-clean).
2. **Doorway pantry predicate (defense-in-depth, requirement 1)** — `blob_should_stock` in
   `storage_proxy.rs`: never stock a **credentialed** fetch (Authorization header OR resolved
   `agent_cid`) under the bearer-blind hash key. Soundness of stocking an un-credentialed 200 now
   RESTS on the storage gate above (the two are one cure). Counter
   `doorway_blob_pantry_skipped_credentialed`. Registered as `blob_should_stock`, census-clean.
3. **`X-Content-Type-Options: nosniff`** on the pantry hit and both blob refusals — the stored
   Content-Type is caller-supplied upstream and there was ZERO sniff defense in either tree.

**Requirement 2 (hit-path bypass) is intentionally a no-op** and should NOT be implemented: the
invariant "only stockable bodies are ever IN the pantry" makes a per-hash stockable tag dead code.
The credential-blind key means a later anonymous hit can only draw bytes that were themselves
stocked from an un-credentialed (public) fetch.

**Corrections to this row's original text:** the `~:911-917` / `~:775-790` line anchors were stale
(the real stock decision is `storage_proxy.rs` ~929; hit path ~791). The DoD's gate name
`just gate doorway-service` is wrong — the gate project is `doorway` (dir `doorway/doorway-service`).
The Disjointness claim ("storage_proxy.rs only") no longer holds: the real cure is storage-side, and
the seed-path co-tenant writer (`routes/seed.rs`) shares the same `ContentCache` keyspace — see the
separate `security-doorway-devmode-auth-bypass` row.

**Sequencing note:** the storage-side change requires an elohim-storage build/deploy and was
deliberately built in an ISOLATED cargo-pool slot (not the live-mesh binary) so it does not
contaminate the in-flight transport-recovery before/after matrix; land its deploy after that matrix.

**Residuals (named, not silent):** the storage gate fails OPEN on a db-pool/query error (an
unreadable projection serves rather than darking every public blob); the above-`community` Authorize
path fails CLOSED pending per-reader HTTP authz (rides `http-reach-enforcement-gap`'s
`humans.agent_pub_key` unblock — no above-community blob-bearing row exists in any seeded corpus
today). `/epr-head` still leaks the blob CID + declared reach of gated content anonymously (metadata,
not bytes) — the byte gate breaks the exploit chain regardless; the metadata leak is its own follow-on.

Status: **triaged** — cure implemented + locally verified; flip to done on the deploy that carries it.

---

## ADVERSARIAL RE-REVIEW (red-team, 2026-08-24): the byte-route fix closes ONE of several egresses

A red-team pass over the whole serving surface found the reach-gate hole is not one route but a
**family of egresses for the same reach-gated bytes**, plus one address-form bypass of the new gate
itself. Reconciled findings, most-severe first. Two were verified live/against the real corpus this
pass; the rest are read-confirmed and carry their file anchors for the follow-on.

### FIXED THIS PASS (in addition to the byte-route gate)

- **Address-form bypass of the new `/blob` gate (was CRITICAL).** The gate built its content-row
  candidate set only from the request's OWN form. The community fixture stores its digest as a
  `bafkrei…` CID, so a request phrased as `sha256-<same-digest>` matched no row and served as
  "honest absence". FIXED: `blob_reach_refusal` now resolves the request to its canonical sha256
  digest and enumerates BOTH renderings — `sha256-<hex>` AND the raw-codec `bafkrei…` CID (via the
  canonical `BlobStore::hash_to_cid`, round-trip-tested) — plus blake3 aliases. Re-validated against
  a copy of the real mesh DB: the bypass request now resolves to the community row; 38/38 gated rows
  resolve in BOTH forms; 0/20 public rows falsely refused. Regression test
  `the_two_renderings_of_one_digest_are_distinct_and_both_needed`.

### STILL OPEN — same C7 reach-plane hole, other routes (operator/architect triage)

The root is a **reach-vs-replication plane conflation**: several surfaces gate on
`reach_is_distribution_safe` / `DISTRIBUTION_SAFE_REACH` (`sync/projector.rs:56`,
`db/content_diesel.rs:1895`), which treat `community` as broadcast-safe — while the CONTENT route
(`/db/content/{id}`) refuses `community` anonymously. The content route is the read-audience
authority; distribution-safety is a CUSTODY/replication predicate and must not gate a READ-audience
decision. Either community is anonymously readable (then the content route + this fix over-refuse)
or it is not (then these surfaces leak) — the two cannot both ship. **This is an architectural
reach-vocabulary call the operator/architect owns; CLAUDE.md forbids canonizing it unilaterally.**

1. **`GET /db/content/{id}/head-record` — CRITICAL, open web, VERIFIED LEAKING.** Returns 200 anon
   for `community-garden-club` with the full DHT `record` (headActionHash + serialized Record). Gated
   by `is_distribution_safe_reach`, which passes community. (`/sync/v1/{ns}/docs/{id}/changes` shares
   the predicate — projects `body`/`blobHash`/`blobCid` for community rows, `sync/projector.rs:115` —
   but did not reproduce a body leak in a quick probe; verify with the right ns/doc-id before closing.)
2. **Locate-token leaks (HIGH, open web).** `GET /epr-head/{id}` → the blob CID (`epr_head.rs:151`;
   this is the enabler — 38/38 gated CIDs anonymously readable, and the a2o step itself uses it);
   `GET /db/content/{id}/head` → `blobHash` at `MinTrust::Invisible`; `GET /api/v1/resilience/{id}` →
   ordered shard-hash list + holder peers (`api/resilience.rs:163`); `GET /api/v1/epr/{cid}/payload`
   → `reach_visible_to` accepts ANY non-empty `Authorization` up through `self`/`private`
   (`api/epr.rs:281`) — presence-only, worse than the blob gate's community rule.
3. **`GET /apps/{content-address}/{file}` — HIGH, open web, LATENT.** The `is_cid` branch
   (`http.rs:8271`) serves via `get_blob_or_heal` with NO reach check across `:8211-8600`; only
   `MinTrust::Amber` (a convergence floor, not reach). Bounded to ZIP-shaped blobs; 0 gated
   `html5-app`/`spa-bundle` rows today, so leaks nothing measured — gate before restricted app
   bundles ship.
4. **Direct-to-storage byte family (HIGH, NOT open-web — Tauri sidecar / LAN / in-cluster only).**
   `GET /shard/{blob_hash}`, `GET /ipfs/{blob_cid}`, `GET /manifest/{hash}` all bypass the `/blob`
   gate against `localhost:8090`. For the `"none"` encoding band (3542/3544 manifests) the shard
   hash IS the blob hash, so `/shard/{blob_hash}` returns the whole gated blob. Excluded from
   `is_service_path`, so not reachable through the ingress — but every direct-to-storage posture
   (Tauri `ACAO:*` sidecar, mesh peers, in-cluster lateral) is exposed.
5. **`GET /db/content/{id}` P2P-fallback branch** returns `ContentView` at `http.rs:7701` WITHOUT
   re-running Layer 1/1.5 — relies on the remote peer's gate. Residual.
6. **Integrity (not confidentiality): `PUT /epr-head/{id}` and `PUT /api/v1/epr/{cid}`** take no
   auth/ownership check — an attacker can rewrite a row's declared head or ingest atoms at any reach.
   Separate class; file under a governance/integrity row, not this confidentiality one.

**The dev_mode JWT forgery (`security-doorway-devmode-auth-bypass`) sits UNDER all of the above:**
while `dev_mode` lets an attacker mint any `X-Agent-Cid`/`Admin`, even the routes that DO gate
(content route, the new `/blob` gate for community) fall to a forged identity. That row's Axis-1 fix
is the precondition for every reach gate on the fleet to mean anything.

**Honest scope statement:** the filed concern was the blob byte route; that is fixed and proven
(byte gate + address-form bypass, live). It closes one egress. The community tier still walks out via
`/head-record` (and the locate-tokens arm reconstruction elsewhere) until the plane-conflation
decision above is made and the sibling routes are gated. This row should NOT be read as "the reach
hole is closed" — it closes the `/blob` leg, and only on the fleet once the dev_mode JWT path is also
fixed.

---

## REACH-EGRESS RE-SCOPE (2026-08-25) — plane-aware correction of the egress inventory

A prior red-team pass listed "8 egresses leak community." Verifying each in code shows that was
OVER-ATTRIBUTED. Several are notary/head/replication-plane surfaces that CORRECTLY use
`is_distribution_safe_reach` (the REPLICATION predicate, set `["community","public","commons"]` at
content_diesel.rs:1895) — re-gating them with the body-serve rule would make the HTTP surface
STRICTER than the P2P surface and could break head adoption. reach ≠ head ≠ replication must not be
conflated in EITHER direction (see [[project_head_reach_freshness_semantics]]).

**LEAVE — notary/head/replication plane, the distribution predicate is the correct gate:**
- `GET .../head-record` (http.rs:7332) serves the head ACTION HASH + notary record, explicitly
  mirroring the P2P responder `p2p::view_federation::build_content_head_record_payload`. For a
  distribution-safe reach the notary proof is ALREADY on the public DHT. Not the content body.
- `GET /db/content/{id}/head` — ContentHeadView (head plane). Any blobHash it exposes is now inert as
  a leak vector because `/blob/{hash}` is reach-gated (this doc's resolution). Metadata for
  distribution-safe content is DHT-public.
- `GET /epr-head/{id}` — EPR head plane, same class.
- `/sync` handlers — the REPLICATION stream; the projector's distribution predicate is the CORRECT
  gate for peer replication. The only real concern is doorway-exposure to unauthenticated open-web
  callers (it is in `is_service_path`): if warranted the fix is to ensure `/sync` is not
  open-web-reachable, NOT to narrow the replication predicate.

**FIX — body-serve plane, must reach-gate (subtleties noted; NOT a mechanical gate reuse):**
- `GET /apps/{id}/{file}` (http.rs:8227, `handle_app_request`) serves app-bundle BODY bytes by blob,
  bypassing the `/blob` gate; doorway-proxied (open web). SUBTLETY: correct gating needs slug-vs-CID
  reach resolution — `blob_reach_refusal` keys on the blob hash, so the CID path gates but a
  SLUG-addressed app resolves via `slug_index` and would slip through unless the content-row reach is
  resolved BY IDENTIFIER. Requires threading `agent_id`/`req` into the handler (currently
  `handle_app_request(&self, path, query)` — no req) + a content-row reach lookup by identifier.
- `GET /shard/{h}`, `GET /ipfs/{cid}`, `GET /dag/{cid}` — byte/block plane, no reach check, but
  DIRECT-TO-STORAGE only (not doorway-proxied → not open web). Severity = the trusted-network posture
  (same as the pre-existing direct-to-storage `X-Agent-Cid` trust). SUBTLETY: shard hashes are RS
  fragments that do NOT map to content-row blob columns, so a naive `blob_reach_refusal(shard_hash)`
  finds no referencing row and serves ungated — shard-level reach needs the parent blob's reach, a
  separate resolution.

**Net:** beyond `/blob` (fixed), the genuine open-web body-serve leak is `/apps` (slug-path gating is
the real work); the byte routes are trusted-network; the head/notary/sync surfaces are correctly
gated and must be LEFT. This re-scope prevents a harmful "gate everything" fix and is the precise
remaining work — an architectural reach-vocabulary/plane call the operator/architect owns, since
CLAUDE.md forbids canonizing a single reach vocabulary while the multi-way drift is open.
