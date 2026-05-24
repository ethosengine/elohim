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

A user opens the app. They see: a feed of posts from friends and pages they follow, ranked by what's earned reach in their network (not by engagement-optimization). They post; the post propagates to people who've earned reach to receive it. They like, comment, react — these are reach-coupled social moves, not extraction events. They join a community group; the group's collective owns its space. Facebook-shape — but speech is free and reach is earned.

## Primitive composition

| What you see | Primitive | Notes |
|---|---|---|
| Profile | imagodei Human + Presence EPR (`content_type: "presence"`) | the user's substrate-native identity |
| Post | EPR (`content_type: "post"`) | optional media-asset children; reach scoped to friends / community / commons |
| Photo / video post | Post EPR with `media_cid` ref | shape parallels Google-Photos archetype |
| Comment | FeedbackSignal (`signal_kind: "comment"`) | threaded via parent_signal_cid |
| Like / endorse | FeedbackSignal (`signal_kind: "endorse"`) | contributes to post's earned reach |
| Reaction (😂, 🔥, ❤️, ...) | FeedbackSignal (`signal_kind: "react"`) with reaction-type | |
| Share / repost | Event (`action: "amplify"`) | re-broadcasts under sharer's reach; provenance back to original |
| Friend | bidirectional Membership / Relationship link | mutual reach grant |
| Follow | unidirectional `grant-reach` Event | receiver doesn't reciprocate to be visible |
| Group / community | Collective EPR (`content_type: "qahal"`) | members are Memberships; group has its own reach scope |
| Page (org) | Collective EPR (`content_type: "organization"`) | followers via Membership |
| Feed | derived view over reach-coupled Posts | ranked by standing + recency + signal-density (not engagement) |
| Notification | local-projection event from neighborhood gossip | not a substrate primitive — app-layer surface |
| Report | FeedbackSignal (`signal_kind: "report"`) | escalates to qahal governance via mishpat |

## Stress points the substrate handles

- **Graph traversal at social scale**: friends-of-friends queries federate through reach-cluster, not by replicating the whole graph; each user's local SQL has their immediate graph + reach-attested second-degree
- **Feed ranking without engagement-optimization**: substrate provides earned-reach signal (FeedbackSignal accumulation) + standing-curve (per friction-gradient memory); app ranks by these, not by predicted-attention
- **Reach-coupling across distances**: a friend-of-friend's post is visible because the friend amplified it (grant-reach Event chain), not because an algorithm guessed you'd engage
- **Community moderation**: qahal-Collective owns its space; member-attestations gate participation; moderation = reach-revocation Events authored by community-stewards
- **Misinformation handling**: per living_memory epic — `consolidation event` adjusts reach + offers restitution + submerges (vs. centralized censorship)
- **Creator monetization**: tips and patronage Events flow viewer → creator using the same shefa primitive (Patreon archetype shares this surface)

## Scale answer

- Per-user graph: ~1k friends + ~10 communities × few hundred members ≈ ~5k people in immediate reach
- Per-user feed working set: posts from immediate reach (~1k posts/week × 1 KB EPR + maybe 50 with media) ≈ ~10 MB SQL/week
- 6-month history: ~250 MB SQL + cold archive after that
- Per-post gossip cost: ~5 KB DHT entry; reach-coupling propagates to friends' projections via libp2p sync
- 8B users × 5k immediate-reach footprint = bounded local working set per peer; no global feed-replication

## Bridges to legacy

- **bridges/facebook/** (import) — Facebook Takeout → batch-graduated Post EPRs preserving timestamps and engagement counts (as Attestations for historical receipt)
- **doorway-projection (atproto)** — ATProto / ActivityPub federation lets substrate-native posts appear on Mastodon / Bluesky and vice versa
- **Cash-out**: every post + relationship + community-membership exports as machine-readable record; portable to ActivityPub instances

## Code anchors

| Surface | Path |
|---|---|
| Imagodei pillar (presence, relationships) | `app/elohim-app/src/app/imagodei/` |
| Qahal pillar (communities, governance) | `app/elohim-app/src/app/qahal/` |
| FeedbackSignal handling | `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/feedback_signal.rs` |
| Doorway ATProto projection | `doorway/doorway-service/src/handlers/atproto/` (planned per 2026-05-01-atproto-lexicon-projection spec) |

*Full draft pending.*
