---
title: Meta / Facebook — substrate-native social graph + feed
tier: architecture
status: Composition draft (primitives mapped; full walkthrough pending)
created: 2026-05-24
authors: Matthew Dowell + Opus 4.7
pillar coupling: imagodei (identity, presence, relationships), qahal (community / group dynamics), shefa (creator monetization in commons-reach), elohim (substrate)
realizes:
  - genesis/docs/content/elohim-protocol/social_medium/epic.md (the old web extracts; the new web contributes — speech free, reach earned)
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-15-dna-signal-as-epr-envelope.md (FeedbackSignals as the social-move surface)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-23-doorway-access-tier-patterns.md (web2 projection)
informs:
  - app/elohim-app/src/app/imagodei/ + qahal pillar for social surfaces
  - elohim/sdk/domains/imagodei/manifest.json + qahal/manifest.json (signal_kinds: comment, endorse, react, follow, friend)
defers:
  - Recommendation / feed-ranking algorithm details (substrate provides reach + standing; ranking is app-layer using those signals)
  - Cross-platform federation (covered by doorway ATProto/ActivityPub bridge specs)
---

## The grandma test

A contributor opens the app. They see a feed of posts from friends and the Collectives they participate in — ranked by the standing the authors have earned, the density of considered signals each post has accumulated, and recency. No "you might also like" infinite scroll, no engagement-optimization. They author a post; the post propagates to people who have earned reach to receive it (friends, then friends-of-friends through amplification, then their Collectives — never globally by default). They endorse, comment, react — each a notarized social act with the author's standing staked on it. They join a neighborhood gardening Collective; the Collective stewards its own space. If they share something a Correction lands against, their reach quiets; their elohim agent surfaces the moment for honest repair. Facebook-shape — but speech is free, reach is earned, and the medium itself encodes attention as sacred.

## Primitive composition

| What you see | Primitive | Notes |
|---|---|---|
| Profile | imagodei Human + Presence EPR (`content_type: "presence"`) | the contributor's substrate-native identity surface |
| Post | EPR (`content_type: "post"`) | optional child media-asset EPRs via `parent_epr_cid`; reach scope household / community / collective / commons |
| Photo / video post | Post EPR with iroh `media_cid` ref | shape parallels Google-Photos archetype; blob lives in iroh, not gossiped |
| Comment | FeedbackSignal (`signal_kind: "comment"`, `signal_class: "care"`) | threaded via `parent_signal_cid`; comment body lives in a child Post EPR, the signal links them |
| Like / endorse | FeedbackSignal (`signal_kind: "endorse"`, `signal_class: "care"`, `standing_impact: "credit-soft"`) | accumulates into the post's earned-reach signal density |
| Reaction (😂, 🔥, ❤️, ...) | FeedbackSignal (`signal_kind: "react"`, `signal_class: "care"`) | reaction-emoji in metadata; advisory standing impact |
| Share / amplify | Event (`action: "amplify"`) | re-broadcasts under sharer's reach; provenance back to original via `parent_epr_cid` |
| Symmetric friend | imagodei `AgentToRelationship` link (D.15 D-2 resolved) | mutual reach grant; lives in imagodei DNA, not elohim DNA |
| Asymmetric follow | Collective EPR Membership (D.15 D-2 resolved) | receiver doesn't reciprocate; follower joins the followee's profile-collective |
| Block / mute | FeedbackSignal (`signal_kind: "mute"`, `signal_class: "care"`) | locally private; never propagates to the muted agent |
| Group / community | Collective EPR (`content_type: "qahal"`) | members are Memberships; Collective stewards its own reach scope |
| Page (org) | Collective EPR (`content_type: "organization"`) | followers via Membership |
| Feed | derived SQL view over reach-scoped Posts | ranked by author standing + recency + signal density — never by predicted attention |
| Notification | local projection event from neighborhood gossip | app-layer surface; not a substrate primitive |
| Report | FeedbackSignal (`signal_kind: "report"`, `signal_class: "governance"`, `standing_impact: "debit-soft"`) | escalates via D.15 cross-DNA bridge call to mishpat / qahal mediation |
| Correction | FeedbackSignal (`signal_kind: "correction"`, evidence_cid → Correction EPR) | substrate-floor evidence requirement; triggers D.9 reach-mutation |
| Vouch (repair) | FeedbackSignal (`signal_kind: "vouch"`, `signal_class: "trust"`, `standing_impact: "credit-firm"`) | the relational-repair witnesses Bob's restitution |

Eight primitives, ~14 `signal_kind` discriminators (care + governance + trust split per D.18), no new DHT entry types — Posts compose as EPRs, every social act lands as a FeedbackSignal whose vocabulary is manifest-declared.

## How one post flows (and how Mira's share that didn't land flows)

A contributor authors a Post on a topic her network has been working through:

```
1. Author drafts in elohim-app; her elohim-agent ("attention-tending elohim")
   surfaces context: "this article connects to a pattern your network has been
   working through. Want to look first?"  (carrot-before-stick per social_medium)

2. She publishes. Coordinator zome creates:
       EPR { content_type: "post",
             reach_scope: "community",
             metadata_json: { body, media_cid?, ... },
             author_pubkey: <her ed25519 pk> }

3. DHT write — single ~2 KB entry; validator quorum confirms author identity,
   reach within author's max-grantable reach (D.9 invariant).

4. post_commit signal → ElohimContentDispatcher → PostProjector
       libp2p EPR-atom plane fans out to predecessor + reach-neighborhood peers
       iroh gossip plane broadcasts on BLAKE3(community_reach_scope) topic

5. Receiving peers' SQL projections upsert the post with dht_anchor_hash;
   feed-ranking view recomputes the slice that includes the new post.

6. Each peer's elohim agent ("epistemic elohim") may run a lightweight
   recognition pass — does this match a known harm-class pattern? — and
   author a low-cost `react` or `comment` FeedbackSignal, or queue a
   `correction` if evidence is available.

7. (Hours later) The elohim agents of three independent households recognize
   the post matches a class already tracked as harmful. Each authors a
   `correction` FeedbackSignal with evidence_cid pointing to a Correction EPR
   (citations, prior debunkings, observation_refs to the source pattern).

8. D.9 reach-mutation Event fires:
       Event { action: "revoke-reach",
               subject_cid: <post_cid>,
               metadata_json: { target_reach: "household", prior_reach: "community",
                                rationale: "correction-quorum reached" } }
   Authority chain validates: corrections came from agents whose governance
   standing met the threshold per the post's Collective's policy.

9. D.20 restitution flow: a small Global Commons allocation Event fires —
   credit toward the labor those receiving households spent unwinding the
   misinformation. From the author's accumulated reach, not her bank account.
   Mass-conservation discipline holds (fees + receiver = total).

10. Submerge: per D.3 canonical signal, the post moves from hot to shelved
    in the author's library. Her elohim retains access; so do the elohims
    of the people she shared with. The post is still part of her trajectory
    — just no longer hot. Centralized censorship never enters the loop.

11. (Weeks later) The author's elohim surfaces the pattern gently: "the
    pattern of these recent shares connects to a thread your trajectory has
    been working on. Would you like to look at it together?"  Restoration
    prioritizes over punishment.
```

The whole flow is substrate-native: no platform-side moderation queue, no central rate-limiter, no engagement-optimization signal. The mechanism is reach-coupling + corrections + submerge — visible, accountable, repairable.

## Storage footprint per household

| Item | Count | Size | Total |
|---|---|---|---|
| EPRs (Profile + Posts × 5 yr at ~5/wk) | ~1,500 | 2 KB | 3 MB |
| FeedbackSignals (~500/day × 5 yr) | ~900k | 300 B | ~270 MB |
| Media-asset child EPRs (~1/post × 30%) | ~500 | 1 KB | 500 KB |
| iroh blob references (media bytes themselves) | ~500 | (held outside SQL) | ~5 GB cold |
| Membership EPRs (~10 Collectives) | ~10 | 2 KB | 20 KB |
| imagodei `HumanRelationship` entries (~150 friends) | ~150 | 1 KB | 150 KB |
| Standing-score derived view (per signal_class × per agent in reach) | ~5k rows | — | ~1 MB |
| **Total local SQL projection (5-year, signal-aggregate-subordinated)** | | | **~50 MB hot + ~225 MB cold-aggregated** |
| Cold archive (signals >30d aggregate-subordinated per D.12) | — | — | ~225 MB residue (custody-quilt) |

After D.12 aggregate-subordinate Commitments roll up endorses (>30 days) and comments (>90 days) per the manifest-declared policy, the hot SQL working set stays in the low-tens of megabytes even after years of heavy participation. **Fits on a phone, fits in memory.**

## Network bandwidth profile

- Per post: ~2 KB DHT entry + ~20 KB neighborhood gossip = ~22 KB total peer load
- Per FeedbackSignal: ~300 B DHT entry + ~3 KB neighborhood gossip = ~3.3 KB
- Per amplify Event: ~500 B DHT entry + ~5 KB neighborhood gossip
- Active contributor daily: 5 posts + 500 signals + 20 amplifies ≈ ~1.8 MB/day inbound + outbound aggregate
- iroh-blob media pull: lazy; only when the photo / video tile renders
- **Per household: ~50 MB/month** for full Facebook-shape participation (excluding media bytes; media follows Photos-archetype profile)

## DHT entry impact

- 500 FeedbackSignals/day/contributor × 8B contributors = 4×10¹² signals/day globally if everything went global
- But: per A.7, signal `target_cid` reach is scoped — only signals on commons-reach EPRs federate to global DHT neighborhoods; friend / community signals stay in the author's reach cluster
- Per-peer DHT visibility: signals within own reach-cluster (~5k people × 500/day = ~2.5M/day inbound, but only ~10% retained after standing-curve aggregate-subordination kicks in at 30 days)
- Typical household peer holds ~50k FeedbackSignal entries hot + custody-quilt-shelved aggregates older than 30 days — comfortably inside the ~3000-entry-per-peer DHT budget (per A.7, signal-density crosses budget pressure after ~6 months without subordination; D.12 aggregate-subordinate Commitment IS the release valve)
- Global commons-attested content (a viral Correction EPR, an apex-elohim council attestation) federates to all peers — but only after passing council-arbitration per D.9

## Why the feed renders fast

The home-feed query is local SQL over the contributor's reach-scoped projection. Per A.7's worked example (records-lifecycle §1133):

```sql
-- Ranked post feed: posts in reach scope, ordered by earned standing + signal density
SELECT
    e.id, e.title, e.created_at,
    COUNT(DISTINCT fs.id) AS signal_density,
    SUM(CASE WHEN fs.standing_impact = 'debit-firm' THEN -3
             WHEN fs.standing_impact = 'debit-soft' THEN -1
             ELSE 1 END)  AS net_standing_weight,
    ss.standing_score AS author_standing
FROM content e
JOIN standing_scores ss
  ON ss.agent_pubkey = e.author_pubkey
  AND ss.signal_class = 'care'             -- D.18 class isolation
LEFT JOIN feedback_signals fs ON fs.target_cid = e.cid
  AND fs.signal_kind IN ('endorse', 'react', 'vouch')
WHERE e.content_type = 'post'
  AND e.reach_scope IN ('community', 'commons')
  AND e.lifecycle_state = 'active'         -- D.7: closed/submerged excluded
GROUP BY e.id
ORDER BY (author_standing * 0.4
          + net_standing_weight * 0.4
          + signal_density * 0.2) DESC,
         e.created_at DESC
LIMIT 100;
```

- Indexed on `(reach_scope, content_type, created_at)` and `(target_cid, signal_kind)`
- Standing-curve view materialized incrementally (D.14) — never re-derived per query
- Aggregate-subordinated signal windows (D.12) read the Commitment's `aggregate_metrics` instead of summing individual signals
- The ranking weights `(0.4, 0.4, 0.2)` are manifest-declared per Collective (D.14 standing-curve policy); a poetry circle weights signal-density higher; a science community weights standing higher
- **Indexed local SQL, <100 ms** for a feed of 100 posts even after years of history

## Why social-graph traversal doesn't melt the network

Friends-of-friends queries are the classic relational-DB pain point. The substrate handles them via **reach-cluster federation, not graph replication**:

- Each contributor's local SQL holds: their immediate `HumanRelationship` graph (~150 friends per imagodei) + Collective Memberships (~10 × ~200 members)
- A second-degree query ("posts from my friends' friends in topic X") issues parallel libp2p RPCs to the contributor's immediate-friend peers
- Each friend's peer answers from their own local SQL projection (subject to their reach grants to the asker)
- Aggregated locally; **no centralized social-graph index**
- Per-query bandwidth: ~150 friends × ~1 KB metadata response ≈ ~150 KB; sub-second latency on healthy local mesh
- The Dunbar friction Sacha-Baron-Cohen's "reach is earned" depends on lives here: each hop costs something to the peer answering, so coordinated graph-scraping attacks face per-peer cost gates

## DHT-budget release valve — D.12 aggregate-subordinate in practice

A viral Post EPR accumulates signals at velocity. Without intervention, a single post with 50k endorses overruns the per-peer DHT-entry budget. D.12 is the canonical release valve:

```
1. signal_aggregate_service watches per-post signal counts
2. When `endorse` signals on a post cross the manifest-declared threshold
   (imagodei manifest: trigger_age_days=30 OR trigger_min_count=100) AND
   the standing-curve window has crystallized:

   Commitment { action: "aggregate-subordinate",
                subject_cid: <post_cid>,
                resource_classified_as: ["aggregation:feedback-signal",
                                          {"signal_kind": "endorse",
                                           "shelf_destination": "peer-cellar://..."}],
                metadata_json: { signal_count_aggregated: 50000,
                                 aggregate_metrics: { total_endorse_count, distinct_authors,
                                                      standing_impact_sum },
                                 merkle_root_of_aggregated_signals: <hash> } }

3. ReconcileController fans out:
   - memory-lifecycle submerge for the 50k individual signals
   - tiered-quilt: signals demoted to custody-quilt (tier_floor=shelved)
   - standing-curve view updated to use aggregate metrics, not individual signals
4. Per-peer DHT budget reclaims ~10 MB; the post still has a fully verifiable
   signal record (merkle root + K-of-N quilt recovery)
```

`report` signals are exempt — manifest declares `aggregate_subordinate_policy: null` for them; governance-evidence stays queryable indefinitely (per D.18 governance-class invariant).

## Community moderation — qahal-mediated, not platform-mediated

A Collective EPR (`content_type: "qahal"`) is the moderation surface. When a `report` FeedbackSignal lands on a post within the Collective's reach:

- The signal's `signal_class: "governance"` routes it to the Collective's mishpat-pipeline via D.15 cross-DNA bridge call (`call(CallTargetCell::OtherRole("mishpat"), ...)`)
- The Collective's stewards (Memberships with steward-attestation) plus the Collective's co-steward elohim agent (per `project_commons_elohim_co_steward`) participate in mediation
- Outcomes: a `correction` FeedbackSignal (if evidence supports), a `revoke-reach` Event (D.9), or no action (the report itself is recorded; the reporter's standing in governance-class is staked on report quality)
- **No platform-owned moderation queue.** The Collective owns its space; its mishpat-pipeline owns its policy
- Cross-Collective: a Post that spans multiple Collectives (the contributor cross-posted into three groups) gets one mediation per Collective; mediation outcomes are per-Collective reach-mutations, not global

## Dissolution in practice

- Contributor deletes their account → `Event(action: "close-account", subject: profile_cid)` → Profile EPR transitions to `closed` (D.7)
- Their authored Posts remain queryable but their author-profile resolves to a tombstone (`closed`-state agent CID) — provenance preserved, identity withdrawn
- Per-Post deletion → `Event(action: "dispose", subject: post_cid)` → Post transitions to `closed`; future amplify/endorse Events targeting it fail validation (substrate-floor invariant)
- Per the social_medium epic's constitutional-revealability commitment: provenance chains remain queryable through governance handshake even after closure (Cain's question — "who told you" — must remain answerable through the right process)
- Collective dissolution → the Collective's stewards author a `dispose` Event on the Collective EPR; Memberships transition to inactive; member's authored Posts within the Collective lose their Collective-reach scope but retain authorship

## Where agentic intelligence carries the load

- **attention-tending elohim** (per social_medium Part II) — sits beside the contributor at the moment of authorial agency; surfaces context ("this matches a pattern your network has been working through"); offers narrow-scope / add-context / share-anyway as choices; never blocks. Pushes accountability *left* into the cheap, humane moment.
- **epistemic elohim** (per A.7) — evaluates `correction` evidence chains, advises mishpat on debit-firm escalation, runs lightweight per-peer harm-class recognition. Without this, contributors would have to read every citation themselves; nobody does.
- **standing-stewardship elohim** (per A.7) — re-derives per-signal_class standing-scores from signal aggregates; detects Sybil-shaped vouch clusters; proposes D.9 reach-mutations. Manipulation patterns at 500 signals/contributor/day × hundreds of millions of contributors are not human-monitorable.
- **commons-stewardship elohim** (per `project_commons_elohim_co_steward`) — represents the Collective's commons-interest in mediations; can't be silenced; speaks in the Collective's councils.
- **rights-stewardship elohim** (per A.7) — receives `signal_kind: "forget-request"`; evaluates against the subject's standing + mishpat constitutional constraints; authors or withholds an `attestation:forget-decision` EPR.

**The value-prop unlock**: a social medium where reach is earned and manipulation is mechanically expensive — not because a platform enforces it centrally, but because the substrate's social nervous system is elohim-mediated at scale, with humans always able to inspect, contest, and repair.

## What the cross-Collective view shows (federated query, not graph replication)

A contributor in three Collectives (neighborhood gardening, a research circle, a creative writing group) sees a unified feed:

```
Dashboard query → federation-hub elohim-node → parallel libp2p RPCs to:
   - the contributor's own SQL (immediate-reach Posts)
   - each Collective hub elohim-node (Collective-reach Posts)
→ each hub answers from its own SQL projection (Posts within that Collective's reach)
→ federation-hub merges, applies per-Collective standing-curve weights,
  returns top-K ranked
```

Each Collective hub holds only Posts within its Collective's reach scope; no Collective replicates another's data. Privacy is structural: when a contributor leaves a Collective, their data stops flowing immediately because the Collective's hub never held their out-of-Collective content. **Cash-out is structural** — leaving a Collective doesn't require platform-side data-deletion ceremony.

## Bridges (legacy interop / cash-out)

- **bridges/facebook/** (Takeout import only) — historical Posts batch-graduated under `stewardship-elohim` signature; preserves timestamps, attachments, friend lists (as `HumanRelationship` reconciliation candidates). The contributor sees their old life imported; future authoring is substrate-native. Cash-in path only — substrate doesn't publish back to Facebook (a hostile incumbent surface).
- **doorway-projection (atproto)** — per `project_doorway_is_federation_surface_atproto` + 2026-05-23 doorway-access-tier-patterns. Substrate-native Posts project as ATProto records via doorway; Mastodon and Bluesky federate as bridge-collective EPRs. **Doorway as bridge — not as platform.** Per the operator's bridge-as-collective reframe (D.8), a federation partner like Bluesky-the-org would itself be a Collective EPR; their per-transaction fee (if any) ratchets into Bridge Commons + Global Commons per D.20.
- **bridges/activitypub/** — analogous projection for the Fediverse; same shape; substrate-native social moves project to ActivityPub `Like` / `Announce` / `Note` activities.
- **Bridge-collective example**: Meta Inc. itself, if it ever wanted to participate substrate-natively, would be a Collective EPR whose per-transaction fee flows into Bridge Commons + Global Commons (D.20). The substrate doesn't refuse the incumbent — it offers terms the incumbent's old shape can't economically reproduce.
- **Cash-out**: every Post, FeedbackSignal, Membership, friend-relationship exports as machine-readable record; portable to any ActivityPub instance or another peer's substrate node. Per the social_medium epic: nothing important is lost; nothing is held against the contributor's will.

## Code anchors

| Surface | Path |
|---|---|
| Imagodei pillar Angular services | `app/elohim-app/src/app/imagodei/` (HumanRelationship, Presence, Identity) |
| Qahal pillar Angular services | `app/elohim-app/src/app/qahal/` (Collective, Governance, Signal-Accumulation) |
| FeedbackSignal entry type + integrity floors | `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/feedback_signal.rs` |
| FeedbackSignal coordinator | `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` |
| Standing-curve service (planned per D.14) | `elohim/elohim-storage/src/services/standing_curve_service.rs` |
| Signal-aggregate service (planned per D.12) | `elohim/elohim-storage/src/services/signal_aggregate_service.rs` |
| Imagodei HumanRelationship link types | `elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs` (AgentToRelationship, etc.) |
| Qahal Collective + Membership | `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` (Collective EPRs via `content_type: "qahal"`) |
| Reach-mutation Event handlers | `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` (D.9 — `grant-reach` / `revoke-reach` / `reclassify-reach`) |
| Doorway ATProto projection (planned) | `doorway/doorway-service/src/handlers/atproto/` |
| Imagodei + qahal signal_kinds manifest | `elohim/sdk/domains/imagodei/manifest.json`, `elohim/sdk/domains/qahal/manifest.json` (signal_kinds: endorse/comment/react/report/vouch/correction/mute) |

## What this proves about the substrate

A skeptical systems architect should walk away from this archetype able to say:

- The FeedbackSignal primitive (one entry type, manifest-declared `signal_kind` vocabulary) carries the entire social-move surface of a Facebook-shape application without inventing new DHT entry types — every like, comment, react, report, vouch, correction is the same primitive with different discriminators
- The standing-curve view + per-signal_class isolation (D.14 + D.18) gives feed ranking a substrate-native ordering that is anti-extractive by construction — there is no place in the pipeline where engagement-optimization could be wired in
- Reach-coupling at friend-of-friend distances works via federated libp2p RPC across reach-clusters — no centralized social-graph index, no platform-side data replication of the global graph
- The D.12 aggregate-subordinate Commitment is a load-bearing release valve for signal-dense content — a viral Post can accumulate 50k+ signals without overrunning peer DHT budgets, and the merkle-rooted aggregate preserves verifiable truth
- The D.9 reach-mutation Event + D.20 Global Commons restitution flow gives a fully substrate-native answer to misinformation that is neither centralized censorship nor accountability-free amplification — the Mira's-share-that-didn't-land flow demonstrates the full loop (recognition → correction → reach-adjust → restitution → submerge → eventual repair)
