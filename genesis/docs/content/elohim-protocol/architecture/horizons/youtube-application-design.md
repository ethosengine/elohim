---
title: YouTube-shape digital media platform on the substrate
tier: architecture
status: Horizon (coherent pattern, not on active subsumption path)
created: 2026-05-24
authors: Matthew Dowell + Opus 4.7
realizes:
  - genesis/docs/content/elohim-protocol/social_medium/epic.md (the public-facing platform alternative)
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md (the primitives this composes)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-15-dna-signal-as-epr-envelope.md (FeedbackSignal surface)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md (iroh-blob for video bytes)
defers:
  - Active implementation — Patreon (creator monetization) + Photos (media at scale) prove the substrate first; YouTube falls out of those once they're shipping
---

## Why this is a horizon, not active

YouTube-shape video distribution is architecturally clean on the substrate (the primitives compose), but the path to subsumption runs through **Patreon** (creator economy) and **Google Photos** (media at scale) first. Both prove the same substrate moves — content addressing, iroh-blob asymmetric pull, FeedbackSignal-gated discovery — without taking on YouTube's specific scale + recommendation-system complexity. Once those are real, a YouTube-shape becomes assembly, not invention.

## Primitive composition

| What you see | Primitive | Notes |
|---|---|---|
| Channel | EPR (`content_type: "channel"`) | root vessel for a creator's body of work |
| Video | EPR (`content_type: "video"`) | `media_cid → iroh-blob`; bytes live in iroh, metadata on DHT |
| Playlist | EPR (`content_type: "playlist"`) | references Video EPRs |
| Subscribe | FeedbackSignal (`signal_kind: "subscribe"`) | DHT-notarized; contributes to reach earning |
| Comment | FeedbackSignal (`signal_kind: "comment"`) | |
| Like | FeedbackSignal (`signal_kind: "endorse"`) | |
| Report | FeedbackSignal (`signal_kind: "report"`) | |
| View | Observation (`observation_kind: "media:played"`) | libp2p only; graduates to summary Event per view-window |
| View count | Derived view over graduated summary Events | not stored; computed locally |
| Ad / sponsorship | Event (`action: "transfer"`) advertiser → creator | shefa-pillar economic flow |
| Tip | Event (`action: "transfer"`) viewer → creator | same primitive as Patreon B.6 |
| Age-gate / rating | Attestation (`content_type: "attestation:content-rating"`) | |

## Stress points the substrate handles

- **Massive blob storage**: bytes in iroh-blob, popular videos replicate via BitTorrent-like demand, long-tail in quilt with K-of-N erasure recovery. Doesn't melt peers.
- **Asymmetric access** (one author, billions of viewers): DHT carries metadata only; bytes flow peer-to-peer; viewer count never becomes 8B-individual DHT entries because views are Observations graduated 1000:1 to summary Events.
- **Reach-gated discovery**: Channels and Videos earn reach via FeedbackSignal accumulation; the substrate's reach-as-nervous-system surfaces what earned attention rather than what an algorithm optimized for engagement.
- **Monetization**: REA Events flow advertiser→creator and viewer-tip→creator using the same shefa primitive Mint/Monarch uses for any transfer.

## Why this is deferred

YouTube has heavy network effects on its current platform — solving "build the substrate version" is easy; solving "get creators to migrate" requires the creator-economy substrate to already be the obvious choice. **Patreon B.6 builds the creator-economy substrate; YouTube becomes a special case once Patreon proves it.**
