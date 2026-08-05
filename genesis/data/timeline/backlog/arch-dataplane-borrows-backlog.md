---
id: "backlog-arch-dataplane-borrows-backlog"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Dataplane borrows backlog — survey-sourced transport/replication/blob mechanisms (Holepunch, SSB, p2panda)"
slug: "arch-dataplane-borrows-backlog"
written: "2026-08-04"
author: "claude (research mint pass, operator-directed clustering)"
status: "backlog"
priority: "medium"
tags: [architecture, dataplane, p2p, transport, replication, blobs, research-derived, cross-pollination]
cites:
  - genesis/research/holepunch-p2p-dataplane-cross-pollination-2026-06-24.md
  - genesis/research/ssb-scuttlebutt-ancestor-retrospective-2026-08-03.md
  - genesis/research/p2panda-cross-pollination-2026-08-04.md
  - genesis/data/timeline/backlog/arch-dataplane-refactor-backlog.md
---

# Dataplane borrows backlog (research mint pass, 2026-08-04)

Externally-sourced *mechanisms* for the dataplane, harvested from the cross-pollination surveys and
previously stranded in survey prose. Sibling of [arch-dataplane-refactor](epr:arch-dataplane-refactor-backlog)
(internal reshaping — this cluster is external borrows; compose, don't duplicate). Every item carries
its survey cite and the survey's seam/class adjudication; each needs a p2p-design-gated brainstorm/spec
before code. **Fold new survey-sourced dataplane borrows here — do not mint siblings.**

| # | Borrow | Source + what it fixes | Gate/blocker | Owner shape |
|---|--------|------------------------|--------------|-------------|
| 1 | **Distributed-introducer signaling** | [Holepunch](epr:holepunch-p2p-dataplane-cross-pollination-2026-06-24) TOP-3 #1 — "every DHT node is a potential introducer" retires the single SBD signal-relay SPOF; harvest any connected peer's AutoNAT observation as the DCUtR rendezvous. Targets the WAN-NAT Gap A (relay/DCUtR built-but-unwired). Class C, no new entry type. | p2p-design-gated brainstorm; WAN-NAT backlog owns substrate context | rust-architect shift |
| 2 | **Per-block verified streaming + byte-range fetch** | Holepunch TOP-3 #2 — `race_fetch` verifies whole-blob sha256 only; lift per-block Merkle proofs + `{byteOffset, blockLength}` addressing for the chunked/RS path. Home: `blob_fetch.rs` + `sharding.rs`. Class C. | p2p-design-gated spec; sequence with `elohim-blob` extraction ([workspace-discipline](epr:arch-workspace-discipline-backlog) #5) | rust-architect shift |
| 3 | **EBT bandwidth disciplines** | [SSB](epr:ssb-scuttlebutt-ancestor-retrospective-2026-08-03) take #2 — request skipping (persist remote's last vector clock; omit current heads) + clock partitioning (one peer per head, timeout to alternates) for our head-announcement gossip, which has no request-skipping analog. Same problem Freenet's 53.7%-anti-entropy finding names. | small spec; composes with [anti-entropy-egress-baseline](epr:2026-07-27-anti-entropy-egress-baseline) | rust-architect shift (small) |
| 4 | **Blob want/have flood-fill (hop-bounded)** | SSB take #5 — wants at −1/−2 forwarded, −3 dropped: content discovery lighter than a Kad lookup for the T3-spoke/household tier; consonant with replication-follows-relationship. | p2p-design-gate (routing class); household-nodes testable | rust-architect shift (small) |
| 5 | **Sneakernet / offline export-import bundle** | SSB take #4 — we have zero offline-transfer story (grep-verified) despite the household-nodes floor doctrine. Storage-layer bundle: heads + bytes + provenance; testable entirely on household-nodes. | held/backlog candidate per survey; needs a2o scenario first | content-pipeline + rust-architect |
| 6 | **PSI confidential topic discovery** | [p2panda](epr:p2panda-cross-pollination-2026-08-04) study #9 — discovery that never leaks topic identity to unrelated peers; the private end of the locate-token space (our inventory gossip ships bare hashes). Sits beside the Holepunch three-way credential split in the confidentiality cluster. | study-then-spec; pairs with [confidentiality-plane](epr:arch-confidentiality-plane-backlog) #3 | brainstorm first |
| 7 | **Actor supervision for swarm event loops** | p2panda study #10 — ractor-style per-subsystem restart trees vs our monolithic `p2p/mod.rs` select! loop. Explicitly sequenced AFTER the refactor cluster's #10→#12→#15 decomposition chain hollows the loop. | blocked on refactor chain | backlog-only until then |
| 8 | **Two-tier consistency idiom (named)** | p2panda adopt #6 — ephemeral gossip for presence, durable sync for content, one topic API (Reflection's proven shape). Documentation-only: name the idiom in the `automerge-sync` skill + dataplane docs so new features declare which plane they ride. | none | librarian/docs pass |

**Below the line (dies honestly in the surveys unless resurfaced):** SSB vocabulary imports ("free
listening", "near moderation") — adopt opportunistically in prose, no work item; Holepunch UDX
(DEFER likely-permanent — only if measured iroh-QUIC underperforms post-cutover); Autobase
linearization (redundant vs Automerge); channel-binding via handshake-hash (already owned by the
`agent-peer-binding-cross-signed-proof` backlog entry).
