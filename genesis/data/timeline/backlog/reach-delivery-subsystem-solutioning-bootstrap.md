---
id: "backlog-reach-delivery-subsystem-solutioning-bootstrap"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Reach & delivery as one subsystem — the full defect ledger, enforcement census and open questions that bootstrap the solutioning session"
slug: "reach-delivery-subsystem-solutioning-bootstrap"
written: "2026-09-29"
author: "claude-opus-5-5 (reach census for the operator's reach design session)"
status: "open"
priority: "high"
area: "cross-cutting: DNA integrity · elohim-storage · doorway · dataplane (libp2p/iroh/Automerge) · steward · Angular"
domain: "protocol"
jobs: [elohim-holochain, elohim-edge, elohim-app]
relatedNodeIds:
  - "habit:reach-enforced-everywhere"
  - "habit:dataplane-convergence"
cites:
  - genesis/docs/superpowers/specs/2026-07-22-reach-ontology-vocabulary-split-spec.md
  - genesis/research/ontology-systems-survey-reach-reconciliation-2026-07-22.md
  - genesis/research/owl2-graduation-floor-ceiling-ontology-2026-07-23.md
  - genesis/docs/superpowers/specs/2026-06-10-deterministic-reach-archetype-floor-design.md
  - genesis/docs/superpowers/specs/2026-06-19-reach-projection-facing-lens-design.md
  - genesis/docs/superpowers/specs/2026-05-29-epr-reachability-economics.md
  - genesis/docs/superpowers/plans/2026-07-23-ontology-keel-slice1-verdict-spine-plan.md
  - genesis/docs/content/elohim-protocol/architecture/2026-09-06-holons-are-spaces-how-we-use-holochain.md
  - genesis/docs/content/elohim-protocol/architecture/2026-09-06-ai-stewarded-commons-reimplementation-plan.md
  - genesis/docs/content/elohim-protocol/architecture/2026-08-25-doorway-auth-posture-declared-stage.md
  - genesis/docs/content/elohim-protocol/architecture/social-reach-nervous-system.md
  - .epr-meta/reach-enforced-everywhere.habit.md
  - genesis/data/timeline/backlog/arch-authority-in-integrity-backlog.md
  - genesis/data/timeline/backlog/arch-confidentiality-plane-backlog.md
  - genesis/data/timeline/backlog/arch-dataplane-borrows-backlog.md
  - genesis/data/timeline/backlog/commons-holonic-stewardship-backlog.md
  - genesis/data/timeline/backlog/security-earned-election-tier-unauthenticated.md
  - genesis/data/timeline/backlog/lamad-teacher-authoring-backlog.md
tags: [reach, authorization, delivery, confidentiality, custody, holons, security, design-session-input]
---

# Reach & delivery — the solutioning session's starting file

**What this is.** This is not a design. It gathers every reach bug, every divergence and every open
question found so far into one place, so the design session starts from the evidence instead of
rediscovering it. The session's charter, from the operator on 2026-09-29: design reach and delivery
with the same care Google gave Zanzibar, without adopting Zanzibar. Every component should then know
exactly how reach is declared, derived, verified and delivered.

**Accepted before the session (operator, 2026-09-29):**
- **commons / public:** one verdict for everyone, served statically.
- **community / familiar / trusted:** relationship tuples plus an indexed check.
- **intimate / self / private:** access means holding the key, not passing a rule.
- **Every authorization claim is backed by a holon with standing to make it:** private/self, intimate,
  community, or authoritative.

**Habit served.** `reach-enforced-everywhere` (red). Its `retire-when` is the session's finish line:
*"every egress plane derives authorization from one shared verifier it cannot bypass by construction."*

**Evidence grade.** §2 comes from commits and filed backlog items. §3–§5 come from a static code
census on 2026-09-29 (three read-only agents). The three most serious claims were confirmed against
source (doorway `Restricted` arm, the shard plane's private-only gate, and `lamad.rs:137`
defaulting a missing reach to commons). Nothing else in §3–§5 was confirmed by running code. Treat
each line as a claim to re-check before building on it.

---

## 1. The problem in one paragraph

Reach is decided in about 20 places across 8 components, using at least 6 vocabularies (4 of them
hand-typed). Everywhere it is a **mutable string column evaluated at serve time**, not a verdict
derived from facts. Only one function, `authorize_reach_for_human`
(`elohim-storage/src/epr_service.rs:473`), actually evaluates the middle bands against the viewer.
Every other plane collapses them to one of three answers:

- deny: the EPR atom plane
- "any authenticated": the doorway
- "anyone": the shard plane and gossip

Beyond that:
- **No claim is backed by a cryptographic proof.** Memberships and relationships are checked only
  in coordinators, which a direct chain write can bypass. Countersigning is used nowhere.
- **Holding content means being able to read it.** Private and intimate content is stored,
  replicated and published to the DHT in plaintext.
- **Nothing orders a revocation ahead of the content it protects.**

Each security fix so far (listed below) has closed one egress path. None has changed the shape of
the problem, so new paths keep turning up.

## 2. Defect ledger: every reach bug so far

### 2a. Found and fixed (fixed means cured on the recorded path, not that the class is closed)

| When | Defect | Cure |
|---|---|---|
| 2026-08-20 | The HTTP listing decided reach from whether a header was *present*. `Authorization: Bearer bogus` on doorway-alpha returned 906 familiar, 3 private and 1 intimate row, with bodies. | `41f7780aa`: resolve the requester and authorize each row. The index `_ => 0` fail-open now fails closed to `u8::MAX`. |
| 2026-08-25 | `GET /blob/{hash}` served restricted bytes anonymously. The `sha256-` address form bypassed the first fix (found by red-team). | `blob_reach`: a blob serves only if some content row referencing it serves. Both address forms give the same verdict. |
| 2026-08-25 | The doorway seed/admin-cache routes were open to an anonymous remote PUT, because every deployed doorway runs `DEV_MODE=true`. | Now requires dev_mode **and** a loopback peer. CI had been uploading unauthenticated through this hole (`47fb60f58`). |
| 2026-08-27 | The conductor admin socket was reachable from the open internet: anonymous `/hc/admin` gave install/uninstall/revoke. | `13ed1721e`, on the fleet in edge #1386. |
| 2026-09-03 | A private row could be served on the shard plane to anyone, and kept by any receiver. | `497f1398a`, `502ff2765`: the shard plane has a custody-scoped gate, for `private` only. |
| 2026-09-12 | Cached-serve paths did not re-ask reach. | `b47b440b5`: reach is re-asked at serve time on every cached-serve path. |
| 2026-09-24 | The search index declared its own reach as `self`, not its serving reach (R-S8), and search checked reach per page, not per candidate. | `7c737650c`, `c66d60ea4`, `ba1ca98ff`. |
| 2026-09-25 | A creator could not find their own private row under the doorway's second namespace. | `9f7529be5`. The fix treats a symptom of the identity-spelling split (§4, D10). |
| 2026-09-27 | Reading an atom's edges was not reach-gated. | `7920095ed`: every read of an atom and its edges is reach-gated. |
| 2026-09-28/29 | An **earned** head election was judged by link-tag prefix alone, so any peer could forge one. | `4687e4111`, `1e9368935` (dev, local): coordinator-only authentication of earned-link authors. The integrity-layer tag gate is still pending, because it moves the DNA hash. |
| 2026-09-28 | **F19**: a peer adopting an earned head widened the row's reach and edges (`1869b8fab`). | Reverted (`b31100358`). Being re-landed with safeguards (leg 3, local, in progress). |

### 2b. F19: the four holes the 2026-09-29 adversarial review found

These show the underlying problem more clearly than any other entry, because each exists only
because reach is a row column that head adoption overwrites:

1. **A newer private version is served under an older public reach.** Adoption rejects the change
   to reach, but still applies the new body and metadata, so the content-reach gate reads the stale
   public column.
2. **A retained private blob becomes readable.** An earned public head with inline text and no
   `blob_cid` keeps the old private blob pointer. Anonymous `/blob/P` then serves it, because a
   referencing row is now public.
3. **An old earned head reopens content the author narrowed.** The author's narrowing goes through
   `upsert_with_anchor` `HeadElection::Declare`, which clears `canonical_declared_at` and
   `canonical_earned`, so an older earned election then wins.
4. **Private titles and descriptions survive a public replacement.** Local adoption does not carry
   title or description, so the old values remain under the widened reach.

In Zanzibar terms these are: reach not tied to the version it governs (1), a child object inheriting
its parent's reach without its own relation (2, 4), and the "new enemy" problem (3). F19 leg 3 fixes
them in storage. The solutioning session should treat them as fixtures that any new design must
refuse by construction.

### 2c. Open, filed

| Backlog item | Defect | Status |
|---|---|---|
| `security-storage-direct-caller-unauthenticated` | Storage on :8090 trusts any direct caller. `X-Agent-Cid` can be forged, and `PATCH /db/content/{id}` re-signs through the node's conductor. | refined |
| `security-doorway-devmode-auth-bypass` | `DEV_MODE=true` on every deployed doorway: JWTs are forgeable with a secret visible in the source, and anonymous callers are granted Authenticated. This defeats every fleet reach gate. | open (operator posture) |
| `security-doorway-blob-pantry-ungated` | The pantry re-serves any 200 blob without re-checking reach. One authorized fetch makes a private blob anonymously readable. | refined |
| `ssr-render-cache-credential-blind-key` | The SSR cache key ignores the credential, so a credentialed render is served to other principals. | wip |
| `security-content-head-route-bypasses-reach-gate` | `GET /db/content/{id}/head` answers anonymously for intimate content: it reveals the row exists, plus its anchor, blobHash and updatedAt. | partial |
| `security-earned-election-tier-unauthenticated` | The earned tier is forgeable (coordinator mitigated; integrity gate pending). | open |
| `security-declare-carries-record-carried-evidence-bounds` | The declare-carries-Record gates prove self-consistency, not that the record exists on the DHT. | open |
| `http-reach-cross-node-fallback-bypass` | Layer 1.5 is bypassed on the P2P cross-node fallback path. Community is left at coarse auth. | open |
| `http-reach-enforcement-gap` | HTTP served intimate content to any authenticated caller. Partly superseded by the 2026-08-20 cure; re-check. | refined |
| `household-love-map-unreadable-by-its-couple` | Matthew and Jessica cannot read their own love map: the intimate check is against *stewards*, and the intimate path has none. | open |
| `alpha-manifesto-content-403` | 11 landing destinations 403 anonymously: seeds say commons, the live rows do not (reach drift in data). | refined |
| `2026-08-21-seed-doorway-unauthored-reach-default` | The two seeders disagree on an ungraded row's reach (doorway seed: public; inverted burden: private). | open |
| `2026-08-10-familiar-reach-origin-archaeology` | 1987 lamad rows carry `familiar` from an unknown origin; the repair surface is undefined. | backlog |
| `reach-vocabulary-frontend-strand` | The TS geographic vocabulary is still treated as reach in lamad (`ContentReach = LocalityLevel`). | backlog |
| `search-reach-vocabulary-seam` | Search casts schema-8 into geographic-8 with `as unknown as`. | backlog |
| `doorway-agent-cid-resolver-namespace-mismatch` | `resolve_agent_cid_from_request` returns a `human_id`, not an agent CID. | backlog |
| `humans-household-id-vocabulary-normalization` | `humans.household_id` holds two vocabularies (slug and raw collective CID). | open |
| `arch-confidentiality-plane-backlog` | The encryption layer is unbuilt (clustered). | backlog |
| `storage-island-harvest-residue` | Encryption-at-rest vision, and an identity serving stub. | backlog |
| `commons-holonic-stewardship-backlog` | Custody vs ownership, nested ceilings, credential-as-lens. | backlog |
| `epr-head-chrome-version-aware-optin-canary-governance` | Reach-gated opt-in to competing heads. | open |
| `seed-provenance-anchor-gap` | Seed/HTTP-created content is invisible behind `require_provenance`. | wip |

### 2d. Neighbouring clusters: already ranked, fold in and don't duplicate

- **[arch-authority-in-integrity-backlog](epr:arch-authority-in-integrity-backlog)** says authority
  is enforced in hot-swappable coordinators instead of integrity. Four rows are the holon-authority
  problem of §6 in DNA form:
  - row 1: any agent may create any link on any base;
  - row 3: `StewardshipGrant` is never bound to `action.author()`;
  - row 4: qahal `Membership{role: Steward}` needs only `sponsor_cid.is_some()`;
  - row 9: no membrane anywhere.

  Every cure moves the DNA hash, so they batch per DNA. The solutioning session's tuple-authority
  rules (§10 Q4) land *through* these crossings, not beside them.
- **[arch-confidentiality-plane-backlog](epr:arch-confidentiality-plane-backlog)** holds the unbuilt
  encryption layer: fail-closed classifier, `KeyEnvelope`, X25519 substrate, ciphertext relay. This
  is the build list for §10 Q5.
- **[arch-dataplane-borrows-backlog](epr:arch-dataplane-borrows-backlog) row 12** covers reach
  enforced at replication, in three grains:
  - DHT private entries plus capability grants for private/self/intimate (no production entry
    declares `EntryVisibility::Private`, and no zome calls `create_cap_grant`);
  - iroh-docs read capabilities, one namespace per ring per holon;
  - Meadowcap sub-range capabilities;
  - plus a `viewer` parameter in `reach_earning::evaluate`.

  This is the field survey behind §10 Q5 and Q7.
- **[commons-holonic-stewardship-backlog](epr:commons-holonic-stewardship-backlog)** covers custody ≠
  ownership, and credential-as-lens. Row 26 (custodial account class for wards) is the
  supported-decision case §10 Q5's key lifecycle must serve.

## 3. Enforcement census: where reach is decided (static reading, 2026-09-29)

`S` = `elohim/elohim-storage/src`, `D` = `doorway/doorway-service/src`.

| Plane | Where | What it reads | Rule |
|---|---|---|---|
| DNA integrity | `content_store_integrity/src/healing.rs:73` | the entry's `reach` | checks the value is in the enum. **No rule on changing reach, and no monotonicity.** `Content` is a **public** DHT entry, including private and intimate rows. |
| DNA cache hint | `content_store_integrity/src/lib.rs:544` | reach | only `commons` counts as public; `public` does not |
| DNA defaults | `content_store/src/providers.rs:204` | – | `community` (storage defaults to `public`, `S/db/content_diesel.rs:401`) |
| Storage HTTP gate | `S/api/content_reach_gate.rs:269-314` | the stored reach column and the `X-Agent-Cid` header | L1: public/commons are anonymous. L1.5: the authorizer runs only above community. **Community = any identified caller.** |
| Storage authorizer | `S/epr_service.rs:473-600` | Human; stewards (`stewardship_allocations → contributor_presences`); local `human_relationships`; `collective_participations` | community = any consented membership in any collective. familiar = shares a collective with a steward. trusted = relationship ≥ trusted, **consent not checked**. intimate = mutual intimate with a **steward, not the author**. self/private = creator. |
| Storage P2P | `S/epr_service.rs:368` | agent key; peer-supplied `reach_ceiling` | fast path for ≤ community; community requires membership here (not on HTTP) |
| Tier index | `S/epr_service.rs:845/861` | – | 8 tiers collapsed to 6 (public = commons, self = private) |
| EPR-head view | `S/views_convert/lamad.rs:137` | the head's `qahal.reach` | **missing → `commons`** (confirmed) |
| P2P body fallback | `S/http.rs:10163` → `content_reach_gate.rs:320` | the resolved head's reach | missing → refused (the opposite of the row above). The bytes are fetched before the gate runs. |
| Blob bytes | `S/blob_reach.rs:119` | reach of every referencing row | the most open referencing row wins. Unreferenced blobs serve. `PUT /blob` stamps the manifest `commons`. |
| EPR atom plane | `S/p2p/mod.rs:10284` | caller vs signer | open/commons are open; **every middle tier is author-only** |
| Shard serve/receive | `S/private_reach.rs:33-39`, `p2p/private_receive.rs` | reach; custody commitments | **only `private` is gated (confirmed, deliberate).** self, intimate, trusted, familiar and unknown tiers serve to any peer. |
| Author earning | `S/p2p/reach_authorization.rs:78-101` | signer is a "known agent" | Stage 1 only: any `peer_identity_bindings` row passes. **Fails open on a DB error.** |
| Compose earning | `S/services/reach_earning.rs:207` | standing plus manifest thresholds | on the author side; **no viewer parameter**; the sole caller is `epr_compose.rs` |
| Widen/narrow | `S/db/content_diesel.rs` 825/899/1094/1150/2320; `head_adoption.rs:2128`; `courier_obey.rs:368,404` | stored reach vs head reach | adoption only ever widens; heal refuses to narrow; an own commit narrows |
| Edge reach | `S/db/authored_edges.rs:79,98` | reach of the source atom | edge ≤ source |
| Fanout | `S/p2p/fanout.rs:72-94` | reach | private/self/intimate are DirectOnly (currently no push at all). trusted/familiar/community get CID-only gossip on `topic_for(pillar, reach, None)`, **one topic shared by every collective**. |
| Automerge / view federation | `S/sync/projector.rs:264`, `view_federation.rs:662` | reach | community/public/commons rows ship to **every peer** |
| Doorway fold | `D/services/serve_eligibility.rs:147,535-575` | projection reach; whether the JWT authenticated; `MembershipPrerequisite` gate hints | commons/public serve. private/invited refuse everyone. **Every other tier, including `self` and `intimate`, is served to any authenticated requester when the projection has no audience hints (confirmed).** Undeclared refuses. |
| Doorway legacy | `D/cache/access_control.rs:14,44`; `reach_aware_serving.rs:50` | – | geographic ladder; `should_serve_response` is called only from tests |
| Doorway caching | `D/routes/freshness.rs:243`, `projection/cache_refresh.rs:97`, `cache/rules.rs:45`, `ssr.rs:261` | the response's reach | only public, commons or absent is stocked. **Absent is stocked.** |
| Steward | `steward/node/src/storage/reach.rs` | – | a 6-value locality enum (dormant) |
| epr crate | `elohim/epr/src/reach.rs`; `verdict.rs` | – | canonical openness 1-8 (distinguishes private < self and public < commons). `Verdict{Permit,Refuse,Refer}` exists, but **`ReachVerdict::into_verdict` has no production caller**. |
| gate-client | `elohim/elohim-agent/gate-client/src/dag/reach_aggregation.rs` | attestation counts | a separate vocabulary (protocol/public/community/self-reach) |
| facings | `elohim/elohim-facings/src/viewer_lens.rs:55,73` | intimacy tier | viewer-relative, but for facets, not content |
| Angular lamad | `content.service.ts:91,270`; `trust-badge.service.ts:439-440`; `content-viewer.component.ts:1347` | `ContentReach = LocalityLevel` | `indexOf` returns −1 for canonical words, so **everything is granted**. `getContentWithAccessCheck` has no callers. |

**Hand-typed reach lists, not generated:** `republish_epr.rs:74`, mishpat `commitments.rs:1476`,
imagodei `:1195`, `S/epr_service.rs:845`, `S/p2p/mod.rs:10289`, `S/api/epr.rs:565`,
`S/api/signal_emit.rs:524`, `S/services/epr_kind.rs:118`, `S/bounds_validator.rs:332`,
`S/distribution_view.rs:303`, `S/epr_nav_context_view.rs:34`, `S/sync/projector.rs:56`,
`S/db/content_diesel.rs:2805`, `D/cache/access_control.rs:14`, `D/services/serve_eligibility.rs:147`,
`D/projection/cache_refresh.rs:97`, `viewer_lens.rs:73`, the lamad TS lists, and the steward enum.
`check-reach-drift.mjs` scans only seed JSON, and is wired into the genesis build only, not pre-push.

## 4. Divergences: the same question, different answers

| # | Question | Answers today |
|---|---|---|
| D1 | Who reads **community**? | HTTP: anyone identified. P2P: any consented membership, in *any* collective (not the content's own). Doorway: authenticated plus gate hints. Automerge/gossip: every peer. |
| D2 | Who reads **intimate / self**? | Storage: mutual intimate consent with a steward, or the creator. Doorway: anyone authenticated (without hints). EPR atom plane: author only. Shard plane: anyone. |
| D3 | Is **public = commons**? | DNA: no (only commons is public). Storage, doorway, blob: yes. epr crate: 7 vs 8. |
| D4 | Is **private = self**? | epr crate: 1 vs 2. Storage: 5 = 5. facings: T4 vs T5. Doorway: private is beneficiary-only, self is merely restricted. |
| D5 | What does **missing reach** mean? | Refused: `content_reach_gate:320`, doorway fold. Commons: `lamad.rs:137`, doorway pantry. |
| D6 | What does an **unknown tier** mean? | Fail closed: HTTP, blob. Serve: shard plane. Grant: Angular `indexOf`. |
| D7 | What is the **default** reach? | DNA: community. Storage: public. Blob manifest: always commons. Doorway seeder: public. Inverted-burden spec: private. |
| D8 | What is the **intimacy vocabulary**? | imagodei: intimate/trusted/familiar/acquainted/public. Storage `models.rs:733`: recognition/connection/trusted/intimate. |
| D9 | Where do **relationships** live? | imagodei `HumanRelationship` on the DHT (single-author, **never read by any gate**; storage drops `RelationshipCommitted`, `reconcile/holochain_app_signal.rs:347`). Storage `human_relationships` in SQLite (written only via HTTP POST; **the only one the gate reads**). Nothing projects one into the other. |
| D10 | Who is the **viewer**? | Doorway injects `X-Agent-Id` (agent key) and `X-Agent-Cid` (**a `human_id`**). Storage resolves either spelling. P2P uses peer id → `peer_identity_bindings` → agent. The storage hop is not authenticated. |
| D11 | Whose relationship does **intimate** need? | Storage: the content's *stewards*, not its author. The author can fail their own intimate content (unverified; the love-map item shows the effect). |

## 5. Structural gaps (the ones the session exists to close)

| # | Gap | Evidence |
|---|---|---|
| G1 | **No per-viewer verdict.** Reach is a stored column checked at each egress, not `verdict(content@version, viewer, announcement, freshness)`. | spec §2 "Measured 2026-07-23: the viewer term is not built"; `Verdict` has no viewer, freshness or explain field |
| G2 | **Reach is not tied to a version.** Adoption, heal and own-commit all mutate one row column. | §2b holes 1–4 |
| G3 | **Child objects inherit reach implicitly.** Blobs take the most-open referencing row; edges take the source; title and description ride along with the row. | `blob_reach.rs:119`; §2b holes 2, 4 |
| G4 | **No relationship tuples for the middle bands**, and no holon backing (see §6). | `reach_authorization.rs:90-101` collapses to "known agent" |
| G5 | **Holding content means being able to read it.** No key envelope in production. Custodians of private content hold plaintext. `Content` is a public DHT entry at every tier. | `private_replica.rs` is a proof with no callers; `HumanRelationship.shared_encryption_key_id` is always `None`; the commons reimplementation plan §5.5 |
| G6 | **No revocation ordering or freshness term.** Membership withdrawal and relationship deletion take effect after projection lag. Verification memos expire by TTL only (event-driven invalidation is "until T19 lands, nothing calls this", `epr_service.rs:792-801`). `MinTrust` and `canonical_declared_at` order head elections, not revocations. | spec §4 (unbuilt) |
| G7 | **Every plane has its own gate.** A new plane is unenforced until someone remembers to gate it. That is how the `/head`, `/head-record`, `/epr-head`, `/apps/{cid}`, direct `/shard` and `/ipfs` paths leaked. | habit `reach-enforced-everywhere` evidence 2026-08-25 |
| G8 | **The identity hop is unauthenticated.** Storage trusts the doorway-injected header, with no proof the hop came from the doorway. `DEV_MODE` makes the doorway's own JWTs forgeable. | §2c |
| G9 | **Reach vs replication are conflated.** `reach_is_distribution_safe` treats community as safe to broadcast. Gossip topics carry no collective scope. | `sync/projector.rs`, `fanout.rs` |
| G10 | **Vocabulary drift.** Six vocabularies; ~20 hand-typed lists; the drift check covers seed JSON only. | §3 |
| G11 | **Explanation is ad hoc.** Doorway refusals speak well (`serve_eligibility.rs`). Storage says 403. No metered explain API; no witness tree. | spec §7 item 3 |
| G12 | **No offline verdict fixture harness.** The spec requires one to ship *with* the reconciliation ("given these tuples/commitments, agent A sees exactly {…}"). | spec §7 item 4 |

## 6. Holons as the authority behind claims: what exists

| Holon tier | What would back a claim | Today |
|---|---|---|
| **self / private** | key possession | Creator match on the `created_by` column. No encryption. The shard plane gates `private` on custody standing (custody *is* read standing). |
| **intimate** (couple, household) | a bilateral, countersigned relationship, plus a key envelope | imagodei `HumanRelationship` (`imagodei_integrity/src/lib.rs:377`) is single-author. The validator ignores the action author (`:1238`). `consent_b` is forced false and consented separately. **Countersigning is used nowhere in any DNA.** The gate reads SQLite instead. |
| **community** (governed collective) | membership ratified under the collective's own terms | Qahal `Collective`/`Membership`/`CollabAgreement` (`imagodei_integrity/src/qahal.rs:24/37/52`). Integrity checks field shape only; authority lives in the coordinator (`issue_household_invite` Steward-only, `affirm_membership` signed token). **A direct chain write can forge a Membership.** Projected into `collective_participations`. |
| **authoritative** (governance) | a Mishpat `Commitment`, collectively ratified | `mishpat_integrity/src/lib.rs:275` is single-author. It grants reach standing only for `private` custody (`custody-spool` / `custody-blob`). `acknowledges-reach-change` is named only in comments. All `genesis_self_check`s are permissive stubs (holons doc `:958-963`). |
| delegation | UCAN attenuation (narrowing only), rooted in Mishpat | `StewardshipGrant` (`stewardship.rs:133`) is field shape only; `verified_by` is a free string. Affiliations landed locally only (`.eprfs/status/affiliations.jsonl`, 2026-09-25); no DHT write. |

**The missing design, stated plainly.** No document says *which holon may assert which relation, at
which tier, with what proof*. In Zanzibar the operator writes the tuples. Here the tuple writer must
itself be governed by the holon the tuple is about. Spec §8 names federation of independently evolved
vocabularies as unsolved field-wide; this is the same problem at the holon level.

## 7. Already decided: compose, don't re-litigate

From `2026-07-22-reach-ontology-vocabulary-split-spec.md` (Draft) unless noted:

- **Declared floor vs derived verdict.** The floor is small, closed, DNA-validated and has no viewer
  term. The verdict is derived and never stored as truth (`:46-51`).
- **Narrow, never widen; deny overrides.** Floor and key envelope are sovereign (`:52`).
- **Schema-8 is the only declared vocabulary.** The geographic vocabulary becomes locality; Part-V
  becomes custody (`:34-44`).
- **Serving cost varies by level:** static, then tuples, then key possession (`:91-96`).
- **Freshness is part of every verdict.** A revocation takes effect before the content it protects
  is served (`:98-100`).
- **Explain is an opt-in, metered API**, plus a fixture harness (`:118-119`).
- **Ceiling marker.** `Refer` ("not mine to decide") is a first-class value, never a fallthrough.
  Authoring may fail open; serving and validation may not (`:55-73`).
- **The Zanzibar borrow:** tuples, userset rewrite (union, intersection and exclusion), the Expand
  witness tree. `user` resolves to `agent_cid`, never a pubkey (`:142`).
- **UCAN attenuation**, with the root in a collectively ratified `Mishpat::Commitment`, not a
  keypair (`:147`).
- **Bound constraints:** no general negation, stratified exclusion only, at most one bounded closure,
  a witness on every operator, default-deny, zero RDF (`:150`).
- **Inverted burden.** Default `private`; unknown means most restrictive
  (`2026-06-10-deterministic-reach-archetype-floor-design.md:80-102`).
- **Reach ≠ delivery.** Delivery is economics; reach is enforced peer-side by standing
  (`2026-05-29-epr-reachability-economics.md:22-33`).
- **Custody, readership, membership and authority are separate relationships**, each with its own
  authorizing fact. Encrypt before any non-reader holds the bytes, with the envelope bound to
  object, version, reader policy and key epoch
  (`2026-09-06-ai-stewarded-commons-reimplementation-plan.md` §5.5, status proposed).

**Status of what these decisions produced:**
- Slice 2 (locality rename) landed.
- Slice 1 (canonical enum) and keel slice 1 (verdict spine: `elohim/epr/src/verdict.rs`) are in code
  but unverified.
- Slice 3 is partial (`trust-badge.service.ts:439` `getNextReachLevel` remains).
- Floor foundation is largely present.
- Not started: per-viewer verdict, freshness, explain API, fixture harness, announcement slot,
  composition-law scenarios.

## 8. Tensions the session must settle

1. **Is reach one axis or several?** The spec defines reach as disclosure. The holons doc (`:246`)
   says reach cannot stand in for audience, propagation entitlement, holding and governance standing
   together. social-reach-nervous-system (`:30`) says reach is "not who can see what". Decide which
   named axes exist and which one the verdict answers.
2. **Earned widening vs narrow-never-widen.** F19 widens stored reach on earned adoption. Is earned
   reach a change to the *declared floor* (a new version's own declaration, which needs no widening),
   or a *derived* term? If derived, it contradicts "derived layers only narrow".
3. **The verdict's signature.** The spec reserves `viewer?` and `freshness`. The OWL2 doc (`:193`)
   refuses speculative `as_of`/`viewer` fields and "any unification of the three deciders". The keel
   then built "one spine, many gradients".
4. **Exclusion.** Refused at `owl2…:193`; adopted as Zanzibar exclusion at `:191` and spec `:142`.
5. **Where enforcement lives.** Economics says peer-side standing. The holons doc §10.2 first calls
   read-time enforcement "over the line", then settles on holder ≠ reader. The key envelope reconciles
   these; say so explicitly.
6. **Holons vs spaces.** Is a community a membrane or clone-space (the D6 candidate, holons
   `:374-376`), or a tuple namespace inside a shared DNA? This decides whether community content
   ever reaches non-member DHT authorities.
7. **Public vs commons, private vs self.** Keep 8 distinct serving classes, or declare the collapses
   canonical? Today each component chooses differently (D3, D4).

## 9. Test and scenario coverage today

| Scenario | State |
|---|---|
| `dataplane/reach-enforced-http.feature` (`@concern:reach-enforced-http`) | 3 live (unverifiable bearer, self-asserted `X-Agent-Cid`, self-asserted `X-Agent-Id` each return the anonymous listing). The byte-route scenario is @wip. |
| `auth/reach-commons.feature` | live |
| `content/content-search.feature` (`@concern:recall-reaches-authority`) | live: a private note excluded from another person's results |
| `lms/attention-witnessed-privately.feature`, `dataplane/served-under-standing.feature` | live |
| `resilience/death-witness.feature` | station 3b (stranger refused) live; 3b-ii (replication plane) @wip |
| `lms/intimate-reach-household.feature` | **all 7 @wip**; encrypted shards and cross-household are @envisioned. "James holds backups without read" is @wip. Its header ("HTTP is coarse") is out of date. |
| `auth/visitor-boundaries.feature`, `elohim/content-reach-negotiation.feature`, `lms/love-map-negotiation.feature` (@privacy, @revocation), `qahal/household-formation.feature` (2) | @wip |
| Unit | `tests/api/reach_vocabulary_contract.rs`; `private_reach` `every_other_reach_serves_unchanged` (this *pins* the shard-plane gap) |

No scenario or unit test yet asserts: the composition law (narrow-never-widen, deny-overrides);
that a revocation takes effect before serving; community scoped to the content's own collective; or
that holding content does not mean being able to read it.

## 10. Agenda: questions the solutioning session must answer

1. **The verdict.** Signature, home crate (the `elohim/epr` `Verdict` spine?), and the single
   verifier every egress plane calls. List the planes: HTTP content, `/head` family, blob, EPR atom,
   shard serve/receive, Automerge, view federation, gossip fanout, doorway fold and caches, SSR.
2. **Versioned reach.** Is declared reach part of the content version (CID-bound), so adoption never
   rewrites it? What happens to title, description, blob and edges: do they carry their own relation
   or inherit explicitly?
3. **The tuple model.** Relations (reader, member, custodian, steward, delegate, …), each relation's
   authorizing holon and proof (self-signed, countersigned, governance-ratified), and which DNA holds
   them. Reconcile imagodei `HumanRelationship` with storage `human_relationships` (D9).
4. **Holon authority rules.** Who may write which tuple. Integrity-layer validation versus the
   coordinator-only checks that direct writes bypass today. Where countersigning is required.
5. **Key possession.** Envelope lifecycle for intimate/self/private: issue, rotate, revoke, recover,
   and the custodian who is not a reader. Make `private_replica.rs` production. Keep private/intimate
   bodies off the public DHT `Content` entry.
6. **Freshness without a global clock.** A causal "zookie": the verdict pins the head CID plus the
   tuple and revocation entries it read. Grading by stakes (tie to the freshness-graded-by-stakes
   decision). What amber means for a restricted tier.
7. **Delivery vs reach.** Which planes may move bytes to non-readers (custody, encrypted only), and
   per-collective gossip topics.
8. **Identity binding.** One viewer identity (`agent_cid`) end to end. The doorway→storage hop
   proven, not trusted. `DEV_MODE` posture.
9. **Vocabulary closure.** One generated list; every hand-typed site either deleted or asserted
   against the schema; drift check extended to `.rs`/`.ts` and wired into pre-push.
10. **Explain and the fixture harness.** The witness-tree shape; the harness seeded with the §2b
    holes, the 2026-08-20 bogus-bearer leak, the blob `sha256-` bypass, the pantry and SSR-cache
    leaks, and D1–D11 as invariants.
11. **Migration.** Existing rows (the 1987 `familiar` rows, seeder defaults, alpha drift) moved
    data-aware. SpiceDB's lesson: a vocabulary migration must account for live data.

## 11. Definition of done for the solutioning session

The session is done when it has produced:
- **(a)** an amendment to `2026-07-22-reach-ontology-vocabulary-split-spec.md`, not a new spec,
  that answers §10 and settles §8;
- **(b)** the fixture harness specified, with every §2 defect and every D1–D11 divergence as a named
  case;
- **(c)** a plane-by-plane migration order, each step citing the §3 row it retires;
- **(d)** a2o composition-law scenarios, with a `@concern:` tag the `reach-enforced-everywhere`
  habit's `checks:` can name;
- **(e)** a re-evaluation of the F19 leg-3 storage safeguards: keep as an interim measure, or
  retire once reach is versioned.

## 12. Pre-read (in this order)

1. This item: §2b and §4.
2. `genesis/docs/superpowers/specs/2026-07-22-reach-ontology-vocabulary-split-spec.md`.
3. `genesis/research/ontology-systems-survey-reach-reconciliation-2026-07-22.md` §2 and §4.
4. `genesis/docs/content/elohim-protocol/architecture/2026-09-06-ai-stewarded-commons-reimplementation-plan.md` §5.5.
5. `genesis/docs/content/elohim-protocol/architecture/2026-09-06-holons-are-spaces-how-we-use-holochain.md`
   (§Social reach, §10.2, the membership sections).
6. `.epr-meta/reach-enforced-everywhere.habit.md` (evidence list).
7. `elohim/elohim-storage/src/epr_service.rs:368-600` and
   `doorway/doorway-service/src/services/serve_eligibility.rs`: the two deciders that most need to
   agree.
