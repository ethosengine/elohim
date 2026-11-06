**REFUTED — R3’s blanket Track E exemption is unsound: E4 changes the storage backend, device admission does not establish object authorization, and H1 needs stronger completion criteria than author equality.**

1. **High — “H1 and H2 do not gate E2–E4” conflates different dependencies.**

   The directory-store claim checks out: it verifies incoming bytes, writes a temporary file, syncs it, publishes without clobbering, verifies a competing winner, and syncs directories ([eprfs-storage/lib.rs:272](/projects/elohim/elohim/eprfs/eprfs-storage/src/lib.rs:272)). This supports allowing directory-backed work to proceed independently.

   But E4 explicitly composes `BlobStoreTier` over `BlobStore` ([plan:304](/projects/elohim/genesis/docs/superpowers/plans/2026-10-29-native-dev-tooling-over-peers-plan.md:304)). That backend writes directly to final paths, treats metadata existence as success, and returns inline bytes without verification ([blob_store.rs:160](/projects/elohim/elohim/elohim-storage/src/blob_store.rs:160), [606](/projects/elohim/elohim/elohim-storage/src/blob_store.rs:606)). The directory backend’s guarantees do not transfer through the adapter.

   Landing H2 “first” might accidentally satisfy this dependency, but explicitly saying E4 does not depend on it records the wrong architecture. **Fix:** H2 gates acceptance and activation of E4’s `BlobStore` composition. E2 and directory-backed E3 may proceed independently. H1 gates consumers of holder evidence, rather than every operation touching `BlobStore`.

2. **High — “The tier ALPN admits only the person’s own devices” is substantially true, but insufficient.**

   The current implementation derives the requester from iroh’s authenticated connection identity, maps it through roster admission, and rereads admission per request ([tier.rs:285](/projects/elohim/elohim/eprfs/eprfs-peer/src/tier.rs:285), [admission.rs:28](/projects/elohim/elohim/eprfs/eprfs-peer/src/admission.rs:28)). The precise claim is **devices admitted under the configured lineage**, not an intrinsic ALPN property.

   `StewardDevicesGate` ignores both root and object CIDs ([gate.rs:64](/projects/elohim/elohim/eprfs/eprfs-peer/src/gate.rs:64)). The governing spec already identifies why this becomes unsafe when mounting general blobs: the channel allow-list currently supplies an implicit boundary that the general mount removes ([design §10.12:574](/projects/elohim/genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:574)).

   **Failure scenario 1:** an admitted device carrying one task requests an unrelated mounted CID. The existing gate accepts it. Same-person admission does not implement the task’s grant.

   **Fix:** retain E3’s independent object-scope gate and unauthorized-CID receipt. R3 cannot use device admission as evidence that this obligation is already satisfied. I found no basis for claiming outsiders currently bypass roster admission.

3. **High — H1 author equality closes one impersonation path, not the whole custody-evidence boundary.**

   Libp2p uses strict validation and signed messages ([behaviour.rs:524](/projects/elohim/elohim/elohim-storage/src/p2p/behaviour.rs:524)). Comparing the payload’s libp2p `peer_id` with authenticated `message.source` is appropriate, including for relayed messages. However, dispatch currently carries only `propagation_source`, the forwarding neighbor ([mod.rs:7964](/projects/elohim/elohim/elohim-storage/src/p2p/mod.rs:7964)); the author must be threaded through explicitly.

   Iroh supplies `delivered_from` ([gossip_receive.rs:172](/projects/elohim/elohim/elohim-storage/src/p2p_iroh/gossip_receive.rs:172)). Comparing against that rejects legitimate relays and cannot establish the original holder. Inventory signatures currently require only a nonempty byte vector ([inventory_gossip.rs:171](/projects/elohim/elohim/elohim-storage/src/p2p/inventory_gossip.rs:171)).

   **Failure scenario 2:** a participant injects an iroh snapshot naming another holder after the libp2p-only fix lands. Shared dispatch still applies the claimed identity. Existing salvage logic counts fresh inventory rows and stops repairing when its target appears satisfied ([custody.rs:507](/projects/elohim/elohim/elohim-storage/src/reconcile/custody.rs:507)).

   Another ingress remains: custody announcements structurally validate and project `holder_agent_cid` without binding it to authenticated authorship ([gossip_dispatch.rs:795](/projects/elohim/elohim/elohim-storage/src/p2p/gossip_dispatch.rs:795)).

   **Fix:** H1 must cover snapshots, deltas, and every additional claim admitted to custody decisions; authenticate iroh authors end-to-end; resolve transport identities to the claimed principal; and enforce freshness and replay rules before mutation. Authentication proves authorship, not possession or commitment. R2 still needs its verified-fetch and commitment evidence.

4. **Medium — “Stage 2 ships behind H1” needs an explicit migration contract.**

   The plan already includes holder signatures on iroh inside H1 ([plan:319](/projects/elohim/genesis/docs/superpowers/plans/2026-10-29-native-dev-tooling-over-peers-plan.md:319)). Therefore, postponing *all* payload authentication until Stage 2 would contradict H1 itself. Postponing universal rejection of older advertisements is a different, defensible decision.

   D7 requires exact received-body signatures and an epoch change for fields that restrict or authorize decisions ([design:410](/projects/elohim/genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:410)). A new signature level cannot safely become an ignorable extension while deciding whether custody counts.

   **Fix:** specify authenticated evidence separately from compatibility discovery. Older messages may remain fetch hints, but cannot satisfy an authenticated custody floor. Introduce the signed envelope with explicit epoch behavior, authenticated downgrade protection, adoption measurements, and replay tests. Over ten years and thousands of mixed-version peers, an underspecified transition forces another wire migration and leaves projections unable to distinguish verified claims from legacy rows.

5. **High — H2 needs representation-aware publication; replacing `fs::write` is insufficient.**

   `BlobStore` has multiple physical representations. Large blobs use numbered files in `<hash>.d`, a `<hash>.chunks` index, and a final `CHUNKED` marker ([blob_store.rs:430](/projects/elohim/elohim/elohim-storage/src/blob_store.rs:430)). Reads interpret that marker and verify the reconstructed object. Separately, Reed–Solomon manifests name hashes of each shard’s actual bytes ([sharding.rs:236](/projects/elohim/elohim/elohim-storage/src/sharding.rs:236)).

   **Failure scenario 3:** H2 atomically publishes the marker but leaves chunk files and the index unsynced. Power loss preserves an apparently published object with incomplete backing data. Conversely, a generic verifier hashing the marker against the composite address rejects valid chunked objects; checking a shard against the composite hash rejects valid shards.

   Temporary files introduce another problem: `list_hashes` excludes `.chunks` and `.d`, but otherwise includes nonempty filenames; tally scanning similarly lacks a staging exclusion ([blob_store.rs:736](/projects/elohim/elohim/elohim-storage/src/blob_store.rs:736), [884](/projects/elohim/elohim/elohim-storage/src/blob_store.rs:884)).

   **Fix:** preserve existing addresses and representations. Durably prepare chunks and index before publishing the marker; sync directory entries; verify shards against shard hashes; exclude staging and corruption-quarantine files from inventory, tallies, and serving. Test crashes, competing writers, deletion races, `ENOSPC`, and cleanup failure separately. Atomic rename itself need not break shards; an indiscriminate implementation can.

6. **Medium — “Lane 0 first” is weaker than explicit acceptance dependencies.**

   The strongest alternative runs hardening alongside independent tier work, with mandatory dependencies where the affected capability becomes usable. It avoids serializing E2 behind unrelated gossip work while preventing E4 from inheriting unsafe storage semantics.

   Neither H1 nor H2 inherently requires a DNA move. D6 reserves zero DHT heads for self/intimate caches; the plan separates local artifact storage from release-head custody ([design:559](/projects/elohim/genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md:559)). Turning operational authentication into per-message notarization would add needless long-term reconciliation cost.

   Preserve the three planes: authenticated inventory is evidence, commitments express obligations, and elected heads govern adoption. P1 still requires verified local head resolution ([runtime-artifacts:300](/projects/elohim/genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md:300)). Neither hardening grants authority to conscript peers, expose private objects, or withdraw another peer’s placement; [holds.rs:4](/projects/elohim/elohim/elohim-storage/src/services/holds.rs:4) preserves that last boundary. R4 supplies no exemption. The avoidable generational rework is a second authentication migration, contaminated custody projections, and storage-format repair after broad deployment.

**The ruling I would record instead**

H1 and H2 are mandatory hardening work in this sprint, on separate write-sets alongside independent Track E work. Their acceptance dependencies are explicit.

E2 may proceed independently. E3 may proceed on the directory backend after its tolerant-reader prerequisite, but requires per-object scope enforcement and refusal of an unrelated CID requested by an admitted carrier. Roster admission alone does not authorize general blob access.

H2 gates acceptance and activation of E4’s `BlobStore` backend and every F2 receipt using that backend. H2 provides verified, durable publication across inline, chunked, and shard representations; verified existing-object handling; corruption containment; directory durability; and staging exclusion from inventories and accounting. Existing identities and layouts remain readable.

H1 gates every use of remotely advertised holdings as authoritative custody evidence, regardless of reach or transport. Libp2p binds payload identity to the signed author, preserving relays. Iroh requires end-to-end holder authentication. Every qualifying ingress enforces identity binding, freshness, replay protection, and explicit evidence provenance before updating authoritative projections. Custody counting additionally requires the applicable commitment and verified possession evidence.

Stage 2’s universal enforcement may follow measured adoption. Authentication needed by H1 cannot be deferred with it. Wire evolution follows D7; legacy hints never silently become authenticated evidence.

H1 receives the required integrity review before landing. Release-head admission remains gated by R2’s complete evidence contract. Local puts and builds remain available without peers, custody quorums, or new DHT heads.
