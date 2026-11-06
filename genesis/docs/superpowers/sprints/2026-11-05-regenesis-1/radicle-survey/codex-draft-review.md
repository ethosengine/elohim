Read-only review completed; no files changed or Git commands run. I checked all six evidence notes and more than eight draft file:line citations against source. Findings reflect the current checkout; tests were not run.

1. **High — Artifact CID and manifest CID are conflated.**  
   **Passage:** T6: “`output_cid ≠ manifest CID`”; lesson 7: “result verifies against the manifest CID.”  
   **Problem:** These contradict lesson 8’s more precise “manifest’s artifact CID.” The manifest and reconstructed artifact address different bytes.  
   **Evidence:** [tier_blob.rs:29](/projects/elohim/elohim/eprfs/eprfs-storage/src/tier_blob.rs:29) defines `record.output` as the whole artifact’s raw CID; [line 213](/projects/elohim/elohim/eprfs/eprfs-storage/src/tier_blob.rs:213) separately computes the DAG-CBOR manifest CID.  
   **Fix:** Name both identifiers consistently. Verify reconstructed bytes against `record.output`, verify the manifest separately, and bind the attestation to the intended output. Add a test preventing substitution between them.

2. **High — T5/D7 partly prescribe existing inventory machinery and risk removing useful information.**  
   **Passage:** “Manifest-granular, paginated inventory”; “never per chunk.”  
   **Problem:** Pagination already exists. Exclusively advertising complete manifests also conflicts with lesson 7’s requirement that partially downloading peers become sources.  
   **Evidence:** [inventory_broadcaster.rs:275](/projects/elohim/elohim/elohim-storage/src/p2p/inventory_broadcaster.rs:275) builds bounded snapshot-plus-delta pages; [mod.rs:4242](/projects/elohim/elohim/elohim-storage/src/p2p/mod.rs:4242) retries backpressured pages. A [shard-only inventory test](/projects/elohim/elohim/elohim-storage/src/p2p/inventory_broadcaster.rs:1067) preserves partial-holder discovery.  
   **Fix:** Mark paging as built. Measure manifest aggregation before adopting it, preserve partial-holder discoverability, and test interrupted/reordered refreshes and partial downloads.

3. **High — T3/D1 transfer publication-history requirements into the byte manifest without establishing that boundary.**  
   **Passage:** “Start from `TierBlobManifest`” and add channel/root and parent.  
   **Problem:** Heartwood signs mutable reference state. A reusable immutable artifact need not acquire a different storage manifest whenever promoted to another channel. The draft treats this design choice as a necessary consequence of sigrefs hardening.  
   **Evidence:** [tier_blob.rs:27](/projects/elohim/elohim/eprfs/eprfs-storage/src/tier_blob.rs:27) separates custody from acceptance/reach; the existing [release schema](/projects/elohim/elohim/rakia/schemas/v1/release-manifest.schema.json:30) already places channel binding in release state.  
   **Fix:** Separate the byte manifest from its signed publication envelope. Test cross-channel publication forgery at the latter boundary; grade the proposed composition ◐.

4. **Medium — Manifest-version handling is already implemented in the proposed starting type.**  
   **Passage:** Lesson 6: whether manifests carry a checked version is “open”; D1 adds a schema version.  
   **Problem:** This understates current implementation and could create parallel version fields.  
   **Evidence:** [tier_blob.rs:16](/projects/elohim/elohim/eprfs/eprfs-storage/src/tier_blob.rs:16) declares the manifest epoch; [line 377](/projects/elohim/elohim/eprfs/eprfs-storage/src/tier_blob.rs:377) rejects unsupported epochs with `UnsupportedEpoch`. Streaming verification also [already exists](/projects/elohim/elohim/eprfs/eprfs-storage/src/tier_blob.rs:224).  
   **Fix:** Classify these as local capabilities needing network integration. Retain lesson 6’s end-to-end test, but reuse the existing discriminator and typed refusal.

5. **High — “Custody-before-head” needs a measurable continuing contract.**  
   **Passage:** Lesson 1: every chunk has “≥ k committed custodians,” each with “a recorded verified fetch,” maintained by reconciliation.  
   **Problem:** Historical fetch success does not establish present availability; distinct holders may share one failure domain. The proposed test only removes the publisher, so it does not test repair.  
   **Evidence:** Current [custody counting](/projects/elohim/elohim/elohim-storage/src/reconcile/custody.rs:507) uses fresh inventory rows. The [changelog analysis](/projects/elohim/genesis/local-dev/radicle-survey/codex-changelog-lessons.md:137) explicitly calls for independent failure domains and separate availability evidence.  
   **Fix:** Define evidence freshness, failure-domain policy, full closure coverage, and degraded-state behavior. Test custodian loss after publication, bounded repair, and insufficient consenting capacity.

6. **Medium — T7 confuses device sequencing with release arbitration.**  
   **Passage:** Lesson 12: “two concurrent release proposals” must produce typed divergence; `ChannelHead.seq` supports this adjudication.  
   **Problem:** Two proposals are not necessarily two elected, incompatible candidates. Per-device sequencing does not resolve that distinction.  
   **Evidence:** [head.rs:1](/projects/elohim/elohim/eprfs/eprfs-peer/src/head.rs:1) describes steward-topic device announcements. Heartwood first filters candidates by threshold in [quorum.rs:173](/projects/elohim/genesis/research/repos/heartwood/crates/radicle/src/git/canonical/quorum.rs:173).  
   **Fix:** Name the release-election component and authority rule. Test two non-linear candidates that both qualify, plus the contrasting case where only one qualifies. Keep `at` irrelevant.

7. **High — Hash verification is overstated as sufficient inventory safety.**  
   **Passage:** Lesson 3: “Advertisements are unauthenticated today. This is safe only because fetch verifies hashes.”  
   **Problem:** Hashes protect received bytes, not holder attribution, discovery availability, or custody counting. Authentication is also layer-specific.  
   **Evidence:** Libp2p uses [strict signed gossipsub](/projects/elohim/elohim/elohim-storage/src/p2p/behaviour.rs:521), while shared dispatch accepts the payload’s holder identity after [structural checks](/projects/elohim/elohim/elohim-storage/src/p2p/gossip_dispatch.rs:413).  
   **Fix:** Distinguish transport authentication from portable holder claims. Add spoofed-holder, replay, and downgrade tests. Lesson 5 also needs a monotonic per-peer security floor; adoption percentages alone do not prevent downgrades.

8. **Medium — Lesson 2 conflates integrity verification, authorization, and write-failure modes.**  
   **Passage:** “Objects land before verify”; a crash “or a full disk” leaves an unrepaired inline blob.  
   **Problem:** Heartwood verifies pack integrity while writing, before later ref authorization. Locally, ordinary write errors attempt cleanup; abrupt interruption remains the stronger demonstrated risk.  
   **Evidence:** [PackWriter:50](/projects/elohim/genesis/research/repos/heartwood/crates/radicle-fetch/src/transport/fetch.rs:50) selects `Mode::Verify`; [write_entry:166](/projects/elohim/elohim/elohim-storage/src/blob_store.rs:166) removes failed writes. The [existence shortcut](/projects/elohim/elohim/elohim-storage/src/blob_store.rs:289) and [unverified inline read](/projects/elohim/elohim/elohim-storage/src/blob_store.rs:618) are confirmed.  
   **Fix:** Preserve T2, narrow its rationale, and distinguish crash, ENOSPC, and cleanup-failure tests.

9. **Medium — The pack-transfer claims exceed their citations.**  
   **Passage:** Lesson 7: “one git pack,” “re-sent in full,” “capped at 500 MiB” ✅.  
   **Problem:** The cited configuration establishes a configurable default, not an immutable ceiling or complete retry behavior. Fetch has multiple stages capable of transferring packs.  
   **Evidence:** [config.rs:148](/projects/elohim/genesis/research/repos/heartwood/crates/radicle/src/node/config.rs:148) says “Default: 500MiB”; [state.rs:399](/projects/elohim/genesis/research/repos/heartwood/crates/radicle-fetch/src/state.rs:399) and [line 454](/projects/elohim/genesis/research/repos/heartwood/crates/radicle-fetch/src/state.rs:454) invoke separate stages.  
   **Fix:** Say “pack-oriented, without demonstrated interrupted-pack resume.” Downgrade full-retransmission claims to ◐ pending a trace.

10. **High — D5 is premature while the actual iroh storage/fetch boundary remains unexamined.**  
    **Passage:** “Join the eprfs tier blob store to elohim-storage’s `BlobStore`”; B10 says reachability is avoided through iroh.  
    **Problem:** The implementation includes another physical store and a mode-specific fetch gap. Relay reachability does not establish artifact delivery.  
    **Evidence:** [IrohBlobStore](/projects/elohim/elohim/elohim-storage/src/p2p_iroh/blob_store.rs:1) is BLAKE3-keyed and disjoint from the SHA256 store. [inventory_fetch.rs:1](/projects/elohim/elohim/elohim-storage/src/p2p_iroh/inventory_fetch.rs:1) states that pure-iroh inventory has no fetch-queue consumer.  
    **Fix:** Scope D5 to supported modes and specify identity/storage adaptation. Test inventory-to-byte delivery independently in libp2p, dual, and pure-iroh modes.

11. **Medium — Lesson 9 omits a concrete race-budget failure already present locally.**  
    **Passage:** “1 MiB chunks bound per-request bytes naturally”; test only a trickling holder.  
    **Problem:** Storage chunk size does not impose the network frame limit, and winning a race does not currently stop all losing work.  
    **Evidence:** [blob_protocol.rs:13](/projects/elohim/elohim/elohim-storage/src/p2p/blob_protocol.rs:13) documents a 64 MiB response ceiling. [blob_fetch.rs:235](/projects/elohim/elohim/elohim-storage/src/p2p/blob_fetch.rs:235) explicitly says dropped join handles leave detached tasks running.  
    **Fix:** Add cancellation, rejected-byte accounting, and race-waste tests. Specify both inactivity and total deadlines; a trickle can avoid an inactivity timeout indefinitely.

12. **Medium — Two important authority lessons remain unconverted into sprint properties.**  
    **Passage:** Lessons 8/11/12 cover signing and lineage but omit provenance attribution and rule scope.  
    **Evidence:** The broker [confused relay with origin](/projects/elohim/genesis/research/repos/radicle-ci-broker/NEWS.md:632). Heartwood’s [Unreleased fix](/projects/elohim/genesis/research/repos/heartwood/CHANGELOG.md:12) replaces identity-document thresholds with the target branch’s canonical rule.  
    **Fix:** Add tests that relaying changes neither builder identity nor standing, and that release admission uses the target channel’s rule rather than an unrelated identity quorum. Neither property follows from a valid signature alone.

13. **Medium — Lessons 10/11 and the publication recovery rule remain incomplete acceptance criteria.**  
    **Passage:** Yank/supersede cells are “an operator decision”; historical standing permits verification; L4 says use durable failures.  
    **Problem:** These leave policy fixtures and recovery transitions unspecified. Preserving cryptographic validity is distinct from permitting new adoption after revocation.  
    **Evidence:** The [grounding report](/projects/elohim/genesis/local-dev/radicle-survey/elohim-blob-plane-grounding.md:103) identifies standing history as open; broker [queue removal before execution](/projects/elohim/genesis/research/repos/radicle-ci-broker/NEWS.md:401) demonstrates the recovery hazard.  
    **Fix:** Supply an approved operation matrix, explicit revocation semantics, and restart tests at each publication boundary proving idempotent recovery without duplicate execution or lost pending publication.

**Strongest parts, keep as is**

- The central distinction between push-time replication and continuing custody.
- The exact inventory-truncation finding and staged-security migration history.
- Opt-in placement as the consent floor.
- Separating signed CI attribution from proof of execution.
- The explicit uncertainty labels for web disclosures and unavailable proposal texts.