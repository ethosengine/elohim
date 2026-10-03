---
id: "backlog-arch-dataplane-borrows-backlog"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Dataplane borrows backlog — survey-sourced transport/replication/blob mechanisms (Holepunch, SSB, p2panda, sedimentree)"
slug: "arch-dataplane-borrows-backlog"
written: "2026-08-04"
author: "claude (research mint pass, operator-directed clustering)"
status: "backlog"
priority: "medium"
tags: [architecture, dataplane, p2p, transport, replication, blobs, research-derived, cross-pollination, performance, perf-queue, perf-latency, friction-mechanical, custody-subset, plane-custody]
cites:
  - genesis/research/holepunch-p2p-dataplane-cross-pollination-2026-06-24.md
  - genesis/research/ssb-scuttlebutt-ancestor-retrospective-2026-08-03.md
  - genesis/research/p2panda-cross-pollination-2026-08-04.md
  - genesis/research/serialization-canonicality-cross-pollination-2026-08-11.md
  - genesis/research/backstitch-realtime-scm-cross-pollination-2026-08-15.md
  - genesis/data/timeline/backlog/governance-native-dna-upgrade-path.md
  - genesis/data/timeline/backlog/arch-dataplane-refactor-backlog.md
  - "version-dag-lives-at-l2-not-in-crdt-doc | History/ADR: The version DAG lives at L2 (the DHT), not inside the Automerge doc | sha256:3fd5a2fffdc19377 | path: genesis/docs/content/elohim-protocol/history/2026-08-07-version-dag-lives-at-l2-not-in-the-crdt-doc.md"
  - "admission-receiver-granted-lanes-design | Story 3.3 design input | sha256:54611915e41292b9 | path: genesis/docs/superpowers/specs/2026-09-23-admission-receiver-granted-lanes-design.md"
  - genesis/data/timeline/backlog/arch-scale-risk-backlog.md
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
| 9 | **Hash-trailing-zeros boundary selection + levelled strata** | [Sedimentree](https://github.com/inkandswitch/keyhive/blob/main/design/sedimentree.md) (Ink&Switch Beelay, Apache-2.0) — read a hash as a numeral in base *b*, count trailing zeros; *n* zeros marks a level-*n* boundary, ~1 per *bⁿ*. Peers who never negotiate still **agree on chunk boundaries**, because the boundary is a property of the content. Two applications: (a) the **L2 version-lineage DAG** (`Mishpat::Commitment` + `version_parent`) has exactly the unbounded-walk problem levelled strata + a support relation solve — *likely the higher-value leg, since it is our own growth problem, not a borrowed one*; (b) blob-dataplane range dedup without a negotiation round trip. **A technique, not a dependency** — no crate linked; Beelay itself is disqualified (pre-alpha, unaudited, "DO NOT use in production"). | p2p-design-gate for leg (a) — touches an existing DHT entry type's read path; leg (b) is Class C. **Leg (a) has a START-HERE handoff:** [version-dag-strata-compaction-handoff](epr:version-dag-strata-compaction-handoff) — grounds the four instances with file:line, and gates the work behind a measurement step | rust-architect brainstorm → spec |
| 10 | **Schema-as-content-addressed-EPR (bind bytes to a schema identity)** | [serialization-canonicality](epr:serialization-canonicality-cross-pollination-2026-08-11) — dag-cbor repeats every map key as a string in every value (a `Quantity` costs 116 bytes for four numbers, two enums and a string), which is a head-plane-scale cost. Schema-relative encoding (Borsh, SSZ) drops the key names, but then bytes are only interpretable *relative to a schema*, so the CID silently becomes a hash of "bytes + implied schema". To stay honest the schema must itself be content-addressed and referenced in the envelope. **The hook already exists and is unused for encoding:** `Envelope.schema_ref: Cid` + `schema_key` (`elohim/epr/src/envelope.rs:23-29`). Two mature precedents for binding bytes to a schema identity: **Avro**'s Parsing Canonical Form + SHA-256 schema fingerprint (shipped ~15 years), and **SSZ**'s fork-versioned schemas agreed out of band across many independently-written Ethereum clients — the production answer to *peer diversity*. **The row needs a second half those don't supply: schema *transformation*, not just identity.** [Cambria](https://github.com/inkandswitch/cambria-project) (Ink & Switch) is the model — a directed graph of schema versions with bidirectional edges (`rename`/`add`/`remove`/`convert`/`wrap`/`head`/`in`/`hoist`/`plunge`), composed along the shortest path. **Adopt the model, not the dependency** (TypeScript-only vs our Rust truth layer; last push 2024-06-14; self-declared "not yet ready for production"). Two constraints it forces: (a) **translation is a read-time projection, never a re-notarization** — a translated doc has different bytes → a different CID signed by nobody, so the original CID stays identity and the transform is a derived view (`DerivationKind::Projection`); minting a new notarized EPR is a *supersession*, a governance act, not a decode step. (b) **Do not call it a "lens"** — `elohim-storage/src/db/lenses.rs::find_lenses_governing_epr` already owns that word for Mishpat governance standing on a different plane; say schema *migration*/*transform*. **Convergence worth noticing before designing a third one:** row 9's levelled strata for the L2 version DAG, [lens-version-dag-policy-dependency](epr:lens-version-dag-policy-dependency)'s `version_parent` DAG, and Cambria's schema-version graph are the same shape. **This is the same question as DNA-hash skew, one layer up:** a peer that can't decode a newer peer's payload and a peer on a stale DNA hash are both schema skew across a diverse peer population — so this row and [governance-native-dna-upgrade-path](epr:governance-native-dna-upgrade-path) must be designed together, not separately. **A substrate redesign, not a library swap** — every CID and every signature changes. schemaboi itself is disqualified as a dependency (experimental, JS/TS only, canonicality unspecified). | **p2p-design-gate mandatory** (touches EPR identity + envelope). Vision-deferred: do NOT start until the DNA-upgrade-path design pass runs — the two share one migration story. Nothing in flight locks it in (measure vocabulary is plain data; derives are annotations) | rust-architect brainstorm → spec, jointly with the governance upgrade path |
| 11 | **Branch-as-doc collaborative authoring plane (backstitch-shaped)** | [backstitch survey](epr:backstitch-realtime-scm-cross-pollination-2026-08-15) TAKE #1 — the shipped mechanics of realtime version control over structured artifacts: a branch IS an Automerge doc, `HistoryRef = {branch_doc, heads}`, structural auto-merge inside the branch, a mandatory Merge Preview → Confirm ceremony at the branch boundary, revert as forward inverse-commit (append-only, never rewritten). Applied here: a **collaborative authoring plane for structured content** (lamad content nodes, specs/plans, graphos stories) riding the EXISTING sync plane (`elohim-storage/src/sync/` — same automerge 0.10 generation as backstitch ✅), NOT the SQL-authoritative projection path; the amber/green authority discipline and producer-side reach filter carry over unchanged (CRDT stays transport+collaboration, notarization stays authority). The Confirm gate maps onto ratification-at-dev-merge / brit's covenantal joining — ceremony relocates, never disappears. Their plaintext-username identity hole is our export: actor-plane claims + steward slots + brit `AgentKey` ceiling attach at the ceremony gate. Watch item riding along: **samod** (their extracted sync engine) as prior art when the `sync-scale-honesty` standing red (`p2p/mod.rs:7779` full-enumeration round opener) is next worked. | **p2p-design-gate mandatory** (births a doc-namespace entity + a ceremony act; head-plane cost of branch docs must be priced). Sequence after the authoring surface that wants it names itself — this is a mechanism awaiting a driver, minted so the driver finds it. **First named driver candidate (operator question, 2026-08-15): rakia's awareness half** — `rakia affected`/`plan` today fold over quantized commits (gix tree reads, `brit-cli`/`brit-graph`); over live doc heads the same pure fold yields a continuous cross-session affected-set (the coordination signal the shared-worktree collisions lack), while `baseline`/attestations stay anchored to ceremony-quantized AgentKey-signed commits — amber heads inform, green commits bind (the p2panda two-tier idiom one altitude up; C5 at the build plane). The doc substrate itself stays dataplane (brit-graph's no-IO purity is load-bearing); ceremony stays brit; hosting stays lvi | rust-architect brainstorm → spec |
| 12 | **Reach enforced at replication — three grains in order: DHT private entries + capability grants for Private/SelfScope/Intimate; iroh-docs read capabilities, one namespace per ring per holon, so a replica cannot *request* out-of-ring bytes; Meadowcap (Willow) sub-range capabilities as the grain reference for sub-atom reach; plus a `viewer` parameter in `reach_earning::evaluate`** | Minted from [local-first → council](epr:local-first-to-council-memory-search-seams-2026-09-12) §2.2 via [the three-seams spec](epr:memory-search-scale-three-seams-design) §4. Finding: reach is enforced at authoring (`services/reach_earning.rs:207-213`, no viewer) and at serving (`doorway-service/src/cache/reach_aware_serving.rs`) and never at replication; no production entry in any DNA declares `EntryVisibility::Private` and no zome calls `create_cap_grant`. Field: Keyhive/Beelay, Jazz/CoJSON and p2panda-encryption converge on group-membership-as-CRDT gating a key before any read; Meadowcap is the only published spec gating sub-document ranges at sync. Patterns under the DHT and iroh, never a substrate swap. | **p2p-design-gate mandatory**; composes with confidentiality rows #3/#5 (KeyEnvelope, storage-minted reader key); a2o: a household node requests a Trusted-ring record it holds no grant for and is refused before bytes move. | rust-architect (DNA integrity + storage + iroh plane) |
| 13 | **Hedged requests per arc, with the replication factor as the tail-latency property (Dean & Barroso, "The Tail at Scale", 2013)** | [pools §3a](epr:commons-data-pools-hot-path-and-external-tooling-2026-09-11) row 6; [spec](epr:memory-search-scale-three-seams-design) §5. A request hedged to a second custodian after a short delay, first answer wins; hedged work is counted twice so the requester's bound prices it; which custodian answered prints on the result. A straight take: latency is a property of replication, not of the engine. | Row 12; stewardship row 28 (the pool fold). | rust-architect (the scatter step) |
| 14 | **Native lexical + semantic providers — SQLite FTS5 (BM25, zero new crates) and sqlite-vec with a pinned local embedding model (bytes CID'd in the `IndexMeasure`), in the storage service's own database with the reach ceiling as a query predicate, behind the discovery spec's `Provider` trait with `ranking_known: true`** | [research](epr:local-first-to-council-memory-search-seams-2026-09-12) §1.3–1.4; [spec](epr:memory-search-scale-three-seams-design) §3. SuperLocalMemory 4.0's canonical store (SQLite + FTS5 + vec0, derived projections behind staged verification) independently converged on this shape; sqlite-vec MIT/Apache-2.0, llama.cpp MIT. Fixes: no FTS or vector crate exists anywhere in the tree; the recall contract's `semantic_provider` reads `mempalace`. Done when it no longer does and removing the palace loses only the second opinion. **L0 landed 2026-09-24** in the epr-cli recall executor (bundled `rusqlite` + FTS5, brute-force cosine in Rust, a pinned Python embedding procedure under the bounded envelope, the model as a Manifest EPR with a CID pin): both done-conditions hold. **L2 remains**: sqlite-vec once fetchable (ranking query only), the mirror in elohim-storage's own database with the reach ceiling as a query predicate, and a native embedding runtime (a procedure version change). | Governed-discovery plan station 1; measure-family row 26; research Q4 (embedding model: license + size pass before pinning). | rust-architect (elohim-storage); epr-cli recall executor |
| 15 | **Bi-temporal `as_of` on `EpistemicStanding` and graph edges — demotion with a timestamp, never deletion; DROP (revoke-reach-everywhere) lands as attested demotion across every fold that reaches** | Zep/Graphiti's temporal edge validity (MIT) and SuperLocalMemory's three-date model with `as_of` threaded through every surface, "point-in-time demotion, not snapshot deletion"; [research](epr:local-first-to-council-memory-search-seams-2026-09-12) §1.5, §3.5; [spec](epr:memory-search-scale-three-seams-design) §2, §7. The Living Memory epic's forgetting made mechanical. | Measure-family row 26; the digital-memory standing ontology spec. | rust-architect |
| 16 | **Receiver-granted doorway→storage blob forwarding — no blind retry, bounded large-transfer concurrency** (Homa: receiver-driven grants, limited overcommitment, incast) | [story 3.3 design input](epr:admission-receiver-granted-lanes-design) §2, §5 change (2). **Incident 2026-09-23 (alpha):** one doorway re-sent the same 3–12 MB `PUT /blob` ~100× in 3 h. Storage's PUT awaited one advisory Node Registry zome call per shard (`elohim-storage/src/http.rs:3417-3445`) behind a conductor pinned at its CPU limit. **Why the sender kept sending:** the forward re-offers only on a *declared* shed (`doorway-service/src/routes/seed.rs:535-555`, clamped 1–5 s, 3 attempts), so a timeout is terminal and the caller (`stage-spa-blob.sh`) re-PUTs. Each re-PUT that hits the doorway cache re-forwards the whole payload (`seed.rs:259-264`), and nothing single-flights a forward of a hash already in flight. Storage admits a 12 MB body on the same 64-permit write pool as a small PATCH and holds the permit for the whole transfer (`http.rs:254-260`, `:1760-1790`). **Shape:** storage grants a bulk slot (a per-peer bulk lane of k=1–2 concurrent large bodies, apart from the write pool) *before* bytes move — `Expect: 100-continue` or a body-less reservation answered 503 + `Retry-After` + `X-Available-Permits`. The doorway single-flights each hash and treats a timeout as "unknown — ask" (`HEAD /blob/{hash}`), never "re-send". **Smallest next step:** doorway-only — single-flight per hash plus HEAD-before-resend on the cache-hit branch, proven on the household with a deliberately slowed storage PUT; the storage bulk lane follows. Sibling: [blob-forward-confirmation-status-only-no-body-check](epr:blob-forward-confirmation-status-only-no-body-check) (same forward, read-back leg). | Sequence after change (1), which moves shard registration into the Background admission lane (in flight on another branch). p2p-design-gate n/a: no entity; the blob stays content-addressed. | rust-architect shift (doorway first, then storage) |
| 17 | **Conductor publish backpressure — surface publish rate + pending receipts, then receipt-aware republish backoff and per-peer outbound caps (holochain/kitsune2 fork)** (TCP: Jacobson's congestion avoidance; Homa: receiver-paced sending, ACKs never queued behind data) | [story 3.3 design input](epr:admission-receiver-granted-lanes-design) §1–2, §5 change (4). **Incident 2026-09-23 (alpha):** after repeated crashes matthew's conductor republished ~1.13 M DHT ops per 15 min in ~24 k-op rounds (~10× james/jessica, 18× adam). Every alpha conductor sat at its CPU limit, SQLite median was 1.5 s (max 210 s), and doorway zome calls timed out at 10 s (`doorway-service/src/services/zome_caller.rs:99`). Yet storage offered its conductor only ~0.2 calls/s per pod, so the load was conductor-internal and invisible to our telemetry. **Mechanism (fork code):** `get_ops_to_publish` selects every authored op that lacks `receipts_complete` and was last published before now − `min_publish_interval`, with no `LIMIT` (`holochain-conductor` `crates/holochain_data/src/dht/inner/chain_op_publish.rs:115-150`). The interval defaults to 300 s (`crates/holochain_conductor_api/src/config/conductor.rs:702-704`). Completion needs `required_validations`, or 5 receipts by default (`crates/holochain/src/core/workflow/publish_dht_ops_workflow.rs:28`, `crates/holochain/src/conductor/cell.rs:606-651`). kitsune2 queues outgoing publishes on a 16 384-slot channel and skips only peers already marked unresponsive, with no per-peer rate or back-off (`kitsune2` `crates/core/src/factories/core_publish.rs:215-217`, `:293-340`). **Working theory, unconfirmed:** receipts starve behind saturation, so ops stay incomplete and are republished every 5 min, which sustains the saturation. **Steps, in order:** (a) *telemetry first* — per cell, ops pending receipts and ops published per round, exported where storage's `/db/p2p/conductor-diagnostics` can read them; this confirms or refutes the theory; (b) receipt-aware backoff — a per-op exponential republish interval in place of the fixed 300 s; (c) a per-peer outbound publish cap in `core_publish`. **Smallest next step:** (a), read-only; nothing is tuned before it is measured. Raising `min_publish_interval` in conductor tuning is an available operator stopgap, but it is a limit raise — a design signal, not the cure. | Graduates [scale-risk](epr:arch-scale-risk-backlog) row 8 mitigation (1); sibling of [conductor-publish-livelock-fk787](epr:conductor-publish-livelock-fk787). The fork patch rides the conductor submodule pin (no DNA-hash move). p2p-design-gate n/a. | rust-architect (fork), telemetry slice first |
| 19 | **Research pass: open-source exemplars of plane separation — what each keeps apart, and what we would borrow** | Operator request, 2026-10-02, after the gradient labelling pass named nine planes (CONVENTIONS.md §Plane) and marked 28 performance entries `fused-planes`. **The list below is recalled from general knowledge and is unverified**; the pass's first job is to check each claim against the project's own primary sources, then mint surviving takes as rows here. *Closest overall:* **AT Protocol** (signed, host-portable data repositories apart from DID identity, apart from the relay firehose, apart from AppViews that build derived views; "speech versus reach") — custody ≠ identity ≠ projection. **TUF / Sigstore** (root, targets, snapshot and timestamp roles with separate keys and k-of-n thresholds) — authority and freshness as their own planes, threshold as a dial apart from membership. **Git / OCI registries** (immutable content-addressed objects; small mutable refs and tags; signatures beside the bytes) — bytes ≠ head. *Strong on one separation:* **Kubernetes, Envoy/Istio** (control plane vs data plane, declared state reconciled) — the head-as-manifest model. **Ceph CRUSH** (placement across declared failure domains) — custody by independence, not headcount; reads onto [commons-holonic-stewardship](epr:commons-holonic-stewardship-backlog) row 30. **Tahoe-LAFS** (servers hold ciphertext; separate verify/read/write capabilities) — custody ≠ readability. **IPFS/IPLD + IPNS, iroh** (blocks, provider records, naming as distinct subsystems). **BitTorrent** (who-holds-it vs piece transfer vs metainfo). **Willow/Meadowcap, Keyhive** (capabilities gating sync at sub-document grain; already rows 12 and 9's neighbours). *For unlinkability:* **Privacy Pass, Oblivious HTTP** (unlinkable tokens, split-trust relays) — the count-and-payment leg of [confidentiality-plane](epr:arch-confidentiality-plane-backlog) row 12. **Questions for the pass:** for each project, which of our nine planes does it separate, what does the separation cost it, where did fusing planes hurt it and how was that found; which of our 28 fused entries has a direct precedent; is there any project that prices verification by relationship (none is known to — that may be the inversion). Suggested first three: AT Protocol, TUF, CRUSH. **Five design observations from the same session ride with this pass as brainstorm input:** [below](#plane-separation-pass--design-observations-to-check). | Research pass (survey under `genesis/research/`, closing with a mint pass), then brainstorm. No code; each borrow that survives is p2p-design-gated on its own row. Method: the `gradient-reading` skill. | brainstorm / research first |

| 18 | **Holochain fork resource bounds and recovery** | Operator request, 2026-10-01, from the resilience/failover/reactive-streams campaign; six ranked priorities and acceptance criteria in [the item below](#holochain-fork-resource-bounds-and-recovery). Composes row 17 and scale-risk rows 7–8 rather than duplicating their implementation work. | Backlog; establish version-pinned baselines before implementation; data/identity design requires p2p-design-gate. | Holochain conductor/Kitsune2 fork, with upstream collaboration |

**Below the line (dies honestly in the surveys unless resurfaced):** SSB vocabulary imports ("free
listening", "near moderation") — adopt opportunistically in prose, no work item; Holepunch UDX
(DEFER likely-permanent — only if measured iroh-QUIC underperforms post-cutover); Autobase
linearization (redundant vs Automerge); channel-binding via handshake-hash (already owned by the
`agent-peer-binding-cross-signed-proof` backlog entry); **sedimentree as a sync protocol** — it
carries no notarization, no authority, no canonical-head election, and its trust model is
*availability from an untrusted relay* where L2's is *validation by a notary*, so adopting it would
replace L1 transport and leave the DHT notary untouched. Verified 2026-08-07: the tree has **zero**
uses of automerge's own sync protocol (`generate_sync_message` / `receive_sync_message` /
`automerge::sync::State` — no hits), so the DAG-in-memory cost sedimentree exists to fix is one we
never paid. The Automerge 3.0 / Hexane memory win is a *document-representation* win and arrived
with the 0.10.0 bump, unrelated to this. See [the L2 version-DAG record](epr:version-dag-lives-at-l2-not-in-crdt-doc).

## Holochain fork resource bounds and recovery

**Item:** 18 · **Recorded:** 2026-10-01 · **Status:** backlog · **Priority:** high.
**Implementation home:** `elohim/holochain-conductor` and its pinned Kitsune2 dependencies.
**Existing habits served:** `conductor-capacity-represented`, `dataplane-convergence`,
and `doorway-failover`. This item does not change their status or create a new active commitment.

Make the conductor remain responsive within a declared resource budget, expose why progress
has stalled, and recover without amplifying background work. The user-visible outcome is that
a household node can contribute within its means while people continue reading and authoring
through catch-up, peer loss, and host recovery. This is the fork's prioritized engineering
backlog and a reference for upstream discussions; it is not a claim that all six capabilities
are absent from every upstream version.

### Evidence and ownership

- The [serving-edge campaign](../../../docs/superpowers/plans/2026-09-19-serving-edge-failover-balance-stream-campaign-plan.md)
  couples failover, balancing and streaming. Its app retry loops, head-projection correctness,
  and doorway streaming remain Elohim responsibilities; a conductor improvement alone does
  not prove the complete campaign.
- The [capability-grant investigation](conductor-cap-grant-scan-per-zome-call.md), September 29
  delta, records SQL wall times of 19.21–41.88 seconds. These are threshold-selected slow
  statements, not a CPU profile. Its isolated candidate passed a warm household convergence
  run (15/15 steps, about 613 seconds waiting for convergence); the record does not establish
  fleet recovery. Follow that item's current branch and deployment evidence when picking up
  this work, rather than treating a tested patch as a deployed fix.
- The [arc feasibility assessment](../../../docs/superpowers/specs/2026-06-13-conductor-authority-arc-auto-policy.md)
  documents the effective zero/full control and lack of a runtime fractional-arc actuator on
  the assessed line. Recheck the fork pin and dependencies before implementing a replacement.
- [Scale-risk rows 7–8](arch-scale-risk-backlog.md) own the hosted-human memory and full-arc
  CPU concerns. The [earlier memory attribution](conductor-memory-attribution-verdict.md)
  instead identified allocator arena retention, resolved with jemalloc: that historical
  runaway growth was arc-independent and is not evidence that sharding cures memory leaks.
- Row 17 owns publish backpressure. The [validation dependency incident](alpha-conductor-sys-validation-spin-unfetchable-deps.md)
  records sustained CPU pressure and repeatedly unfetched dependencies; its attempted local
  reproductions did not establish the fleet mechanism. Preserve that uncertainty.

### Ranked priorities and acceptance criteria

1. **Bound background work and preserve interactive service.** Cover publishing, validation,
   dependency fetches, authorization lookup, and database maintenance. Bound queues and
   concurrency; back off when progress is impossible; report overload explicitly. Graduate
   publish-specific changes through row 17 and grant changes through their existing item.
   **Acceptance:** on a loaded store, sustained missing dependencies and restart catch-up stay
   within declared CPU/RAM budgets and an agreed interactive p95/p99 latency target. Once
   peers return, pending work drains without weakening validation or authorization. A client
   timeout must not be treated as proof that a write was cancelled.

2. **Support safe partial sharding and resource-aware arc adjustment.** Provide a supported
   partial contribution posture, observable current/target arcs, and runtime adjustment with
   coverage-preserving handoff. Test heterogeneous devices and multiple agents sharing a host.
   **Acceptance:** a constrained node demonstrably holds and serves a partial arc; shrinking,
   growing and losing a peer preserve the declared redundancy floor or explicitly report its
   loss. Several agents on one machine must not be counted as independent physical failure
   domains in the acceptance evidence. Configuration parsing alone is not proof of actuation.

3. **Make multi-agent memory costs predictable.** Attribute costs to agents, cells, DNAs,
   active calls, caches and runtime instance pools. Establish supported budgets and idle-cell
   lifecycle/reclamation semantics, distinct from allocator-retained RSS.
   **Acceptance:** long soaks across increasing hosted populations and repeated activation/
   deactivation show a measured marginal cost and bounded retained memory; overload yields a
   documented admission response rather than an OOM. Record baseline, peak and settled memory.

4. **Expose progress and pressure through supported diagnostics.** Build on existing upstream
   and fork instruments. Cover queue depth and oldest age, pending dependencies/receipts,
   publish and integration progress, database contention, arcs and workflow pressure.
   **Acceptance:** a bounded diagnostic read distinguishes progressing, waiting and stalled
   cases in injected failures, with bounded label cardinality and measured collection cost.
   Readiness must distinguish an open socket from a functioning authorized zome call; our
   hosting layer remains responsible for consuming these signals.

5. **Provide a resumable foundation for reactive projections.** Specify a supported feed or
   reference implementation for relevant committed/integrated changes: cursor, replay,
   bounded retention, gap detection, and snapshot-to-subscription handoff. State local versus
   network scope and ordering explicitly. Existing send-and-forget signals remain useful
   notifications, but are insufficient evidence of durable delivery.
   **Acceptance:** disconnect/reconnect, process restart, duplicate events and retention expiry
   either converge the projection without silent omissions or explicitly require a rebuild.
   No global total-order or exactly-once promise is implied. Elohim owns projection semantics
   and cross-doorway delivery; feed/API design requires the P2P design gate before implementation.

6. **Make identity-preserving recovery operationally supported.** Evaluate existing restore
   and migration work before adding fork machinery. Document backup/restore, host transfer,
   private-state protection and prevention of concurrent source-chain writers. Serving a
   replica of content is distinct from safely resuming an agent's authorship.
   **Acceptance:** crash/transfer drills preserve identity and valid chain continuation,
   refuse unsafe concurrent authorship, and explicitly identify unavailable private state or
   insufficient DHT recovery evidence. Recovery must never silently mint a replacement identity.

### Graduation and verification

Start with priorities 1 and 2; instrument enough of priority 4 to attribute their results.
Each implementation slice must name an existing owning item/habit, a runnable scenario, exact
fork and dependency commits, and numeric resource/latency/recovery budgets before execution.
The scenarios should cover loaded and fresh stores, hosted-agent count, unavailable dependencies,
peer churn, restart catch-up and heterogeneous hardware. Set thresholds from measured baselines;
this backlog deliberately does not invent performance guarantees.

Keep fork patches isolated, retain security and differential regressions, run the owning gates,
and obtain household evidence before changing the deployment pin. A successful build, a quiet
fresh database, or a green unrelated scenario does not establish recovery under campaign load.
Upstream collaboration should offer reproductions, profiles, isolated patches and this acceptance
suite. Recheck upstream support against the exact target release when each priority is claimed.

## Plane-separation pass — design observations to check

**Item:** 19 · **Recorded:** 2026-10-02 · **Status:** brainstorm input, unverified.

The operator asked, at the end of the gradient labelling session, whether any design issue stood
out. These five did. They were read from backlog entries, habit deltas and the campaign 1.4 stop
receipts, not from the code. Each is a hypothesis for the brainstorm to confirm or drop, with the
source that prompted it. None is a finding and none authorizes a change.

1. **Machine bookkeeping shares a source chain with human acts.** The household author chain was
   past 45,000 actions, and a root approval failed with a head-moved error while background storage
   wrote to the same chain (`genesis/docs/superpowers/sprints/2026-10-01-campaign-1.4-household-restart.md`).
   A person's ceremony waits behind machine churn and grows with it. *To check:* which writers land
   on an author's chain and at what rate; whether high-frequency machine writes can live on a
   separate cell or off the chain. Tags: `plane-notary`, `unit-history`, `lane-borrowed`.
2. **Authority is checked per call, not per relationship.** Storage authorises a new signing
   credential on every connect, which is how one agent reached about 15,000 grants that each call
   then scanned ([conductor-cap-grant-scan-per-zome-call](epr:conductor-cap-grant-scan-per-zome-call);
   habit `zome-call-cost-bounded`, DELTA 2026-09-19). A storage peer and its own conductor are one
   steward. *To check:* whether one long-lived credential per relationship, looked up by key, is
   sufficient, and what rotation it needs. Tags: `trustful-self`, `friction-verify`, `plane-authority`.
3. **The pricing signal was built last.** The dataplane trust handshake is a stub that classes
   every edge `public` ([sync-edge-susan-timeouts-per-edge-observability](epr:sync-edge-susan-timeouts-per-edge-observability)),
   so every path was built trust-blind and the gradient cannot be measured. *To check:* the smallest
   verifiable identity-and-relationship claim the handshake could carry, and per-edge cost reporting.
4. **The conductor ran without instruments.** Its telemetry instruments reach no exporter (habit
   `zome-call-cost-bounded`, DELTA 2026-09-23); the grant scan and the publish livelock grew unseen
   until every conductor was pinned. Row 17 step (a) and row 18 priority 4 already own this.
5. **Restart was never a designed state.** 23 of the 65 phase-labelled performance entries are
   transition costs: saturation after a restart, fleet-wide catch-up after a roll
   ([performance-concern-index](epr:performance-concern-index)). Staggering and warm-up arrived as
   incident cures. *To check:* a declared budget and mode for a node coming back.

Observations 1 and 2 are our own design choices and compound: more grants and a longer chain make
every call slower. A smaller sixth — the head moving separately from its content — is already being
cured by campaign story 1.4.

**RECONCILED 2026-10-02** (shem, fork 2b334df7973d, superproject 4a80267f3, code read only, nothing measured): row 17 mechanism present (`chain_op_publish.rs:98-123` has no LIMIT; `publish_dht_ops_workflow.rs:28`); row 16 storage half cured in code (`http.rs:3661-3665`), doorway half present; row 18 priority 4 regressed at this pin, which lacks the Prometheus exporter (`0f26f6703` is not an ancestor). Same mechanism as: row 17 = scale-risk row 8; row 18 p3 = scale-risk row 7. Confirming measurement: ops per publish round against authored ops on one household cell.

**Observation 1 read against the code, 2026-10-02** (shem, fork 2b334df7973d, superproject 4a80267f3; code and the probe mesh's logs read, nothing measured by the reading; all actors test fixtures). *Confirmed:* machine bookkeeping shares the person's lamad source chain and collides with foreground writes. On an idle three-peer, three-doorway mesh the one timed writer on that chain is the doorway peer-health probe: every 300 s, for each live sibling doorway, it calls `infrastructure::record_health_attestation` (`doorway-service/src/services/federation.rs:687-695`), which bridges to the elohim role's `content_store::issue_attestation` (`dna/infrastructure/zomes/infrastructure/src/lib.rs:905,937`) and commits 1 Create + 2 CreateLink on the lamad chain. That is 72 actions an hour per peer with nobody using the mesh. A chain-growth probe lost 14 of 4,000 writes to it, and the doorway's own log shows its attestations failing with the same head-moved error on the 300 s rhythm (20:28:58, 20:38:59, 21:14:06Z). The probed chain's census is attestation-shaped (12,712 links = 2 × 6,356 app entries; no Update, no Delete). *Corrected:* on that mesh the collider was the doorway, not background storage. Storage's 60 s heartbeat writes to the infrastructure cell, and storage wrote nothing to the probed chain; storage's lamad writers (head adoption and contest, reanchor heal, release soak attestation) are event-driven and were silent on an empty mesh. Which writers were active on the campaign 1.4 household is not established. The bridge crosses cells, so storage's chain-write gate cannot serialize it, and the doorway does not retry. Storage's own head-moved retry has a flat 2 s budget, so once a write costs more than about a second (the probe's write p50 crossed 1 s at roughly 14,000–15,000 actions) it gets effectively one attempt. No zome sets relaxed chain-top ordering, so every collision discards a whole zome call, which on a long chain repeats the grant lookup and the init walk. No action carries a writer marker: a census can split by entry shape only, and cannot separate a human publish from a machine adopt. *Placement reading:* a device-health observation is custody-plane and latest-wins between a doorway and its own node. By the design gate it is Ephemeral and belongs in the observation substrate; if a notarized witness is wanted, the node_registry cell already has a `HealthAttestation` type and `attest_health`. Either is a coordinator- or doorway-only move. The audit property given up is an immutable peer-signed history of who saw whom, which nothing reads today; writing only on a status change would also cut most of the volume. Machine head nominations are head-plane and must stay in the lamad DHT; their honest author is a node agent, not the person. *Measurement that remains:* on a stopped long household store, the share of the top author's actions that a person or ceremony authored (device-health and doorway-summary attestations against other content, capability grants, tagged head links), and device-health actions per 300 s round. Census queries and the writer inventory: `genesis/local-dev/perf-deep-dive/background-writers-and-planes.md` (gitignored, on shem).

**Observation 1, the heartbeat leg, 2026-10-03** (shem, code read and an idle profile; attribution
not established). With the doorways stopped, each storage peer's only unconditional zome call at
idle is its 60 s heartbeat, `record_peer_status`, a write to the agent's infrastructure chain
(`elohim-storage/src/heartbeat.rs:166,302`); everything else storage runs makes about one more call
a minute, all reads (`genesis/local-dev/perf-deep-dive/idle-zome-callers.md` on shem). That cannot
by itself explain the ~0.06 cores of idle wasm on matthew's patched conductor. The leading candidate
is the conductor validating, as an authority, the ops each peer's heartbeat commit sends it, so the
cost would scale with write rate times peer count. It cannot be confirmed from the current profiles:
wasm frames are JIT code with no symbols, so validation and coordinator calls cannot be told apart
without per-function counters or a wasm symbol map. If confirmed, a liveness heartbeat is another
latest-wins observation written to a chain on a timer, the same shape as the device-health
attestation above.

**Observation 2 read against the code and a measurement, 2026-10-02** (same pin; measured on the same disposable mesh). *Corrected:* storage no longer mints a grant per connect while its closed-chain fence is armed, since it reuses persisted credentials (`elohim-storage/src/closed_chain_fence.rs`); two paths still mint directly when the fence is absent (`signing.rs:198-200`, `hc_client.rs:191`), and whether every launch path arms it was not checked. The doorway mints once per cell per process. The per-call authority cost that remains is not the grant count: on a chain with 9 grants, the joined lookup walks every action the author wrote (plan `SEARCH a USING INDEX elohim_Action_author_seq_idx (author=?)`), 249–270 ms per execution at 19,081 actions and 56% of statement time across 20 no-op calls. So authority, as implemented, is paid per call and priced by the chain's length. That links observations 1 and 2: every machine write on the person's chain raises the price of every later call's authorisation. Evidence and the fork change under test: [conductor-cap-grant-scan-per-zome-call](epr:conductor-cap-grant-scan-per-zome-call), RECONCILED 2026-10-02.
