REFUTED — the clean break remains defensible, but the CID validator, reset procedure, and retained-head contract contain concrete defects that must change before implementation.

1. **High — the proposed `is_byte_cid` admits malformed CIDs.**  
   The length/prefix/alphabet predicate in [plan.md:101](/projects/elohim/genesis/local-dev/regenesis-1/plan.md:101) does not fully validate the header or canonical base32 encoding. Starting with the plan’s “hello world” vector, replace its eighth character, `g`, with `o`: `bafkr4ioxjga67jykbseaxdmmdgc5a5o3zp3htom2l6mrjznk7fvyggu6eq`. It still passes all three checks, but decoding yields header `01 55 1e 21`: a declared **33-byte digest with only 32 bytes present**. Changing the original final `q` to `r` also passes while introducing nonzero padding bits. I verified both with an in-memory base32 calculation.

   **Fix:** decode and validate the complete canonical 36-byte representation, including header `01 55 1e 20` and padding. Pin negative vectors across the kernel and every wasm validator. The random-valid-input test at plan line 122 cannot catch this.

2. **High — `RESET_STORAGE=true` does not implement the promised storage recast.**  
   [genesis/Jenkinsfile:1576](/projects/elohim/genesis/Jenkinsfile:1576) invokes `reset-storage-pod.sh`. That script attempts to delete only `/data/content.db`, targets container **`elohim-storage`**, suppresses failure with `|| true`, then restarts the pod and prints success ([reset-storage-pod.sh:20](/projects/elohim/genesis/scripts/ci/reset-storage-pod.sh:20)). The actual consolidated container is **`elohim-node`** ([adam-firstman.yaml:180](/projects/elohim/genesis/orchestrator/manifests/humans/adam-firstman.yaml:180)). Even with the container corrected, this leaves the old blob layout and other persistent stores intact.

   **Fix:** replace the assumed reset with an explicit, fail-closed recast operation covering the declared storage state. Verify absence of the retired layout and successful fresh initialization before seeding. A pod restart is insufficient evidence.

3. **High — “KEEP steward heads, unaffected/additive” contradicts T3.3.**  
   T3.3 replaces `signature` with `sig` and changes the signing domain to `v2`; §3 calls the retained heads unaffected and additive ([plan.md:131](/projects/elohim/genesis/local-dev/regenesis-1/plan.md:131), [plan.md:156](/projects/elohim/genesis/local-dev/regenesis-1/plan.md:156)). Existing `ChannelHead` requires `signature`, signs under `elohim:steward-channel-head:v1:`, and verifies that exact message ([head.rs:40](/projects/elohim/elohim/eprfs/eprfs-peer/src/head.rs:40), [head.rs:55](/projects/elohim/elohim/eprfs/eprfs-peer/src/head.rs:55)). Persisted `HeadTable.own` embeds that type ([head_table.rs:114](/projects/elohim/elohim/eprfs/eprfs-peer/src/head_table.rs:114)). Retaining files cannot preserve readability or signature validity after those changes.

   **Fix:** exclude the existing steward-head format from this clean break, or explicitly authorize a separately specified successor ceremony with historical verification. Do not describe a field replacement and domain change as additive.

4. **Medium — non-fixture state exists, but the proposed KEEP classification is inaccurate.**  
   The identity-reserve files contain actual agent-attributed memory lineage, not disposable content fixtures; see [bafkreigv…json:3](/projects/elohim/.eprfs/status/identity-reserve/bafkreigvcsogqvi24k5qmajmpnvxqdtvkkxc22t7veqyee4pj3zqyv6nqe.json:3). Their reader verifies raw SHA-256 CIDs directly, then deserializes `Lineage` ([import.rs:834](/projects/elohim/elohim/eprfs/epr-cli/src/flow/memory/import.rs:834)). These are **not epoch-1 tier manifests**, so blanket `unsupported-profile` treatment is wrong. Conversely, adding a required `profile` field to `TierBlobManifest` can fail deserialization before the existing epoch check ([tier_blob.rs:263](/projects/elohim/elohim/eprfs/eprfs-storage/src/tier_blob.rs:263)).

   Brit evidence also has live consumers: `gate-evidence.mjs` delegates verification to brit ([line 348](/projects/elohim/genesis/orchestrator/gate-evidence.mjs:348)); ark parses `output_cid` without assuming BLAKE3 ([brit_index.rs:429](/projects/elohim/elohim/ark/cli/src/brit_index.rs:429)).

   **Fix:** distinguish lineage, notes and their referenced objects, and actual tier manifests. Preserve historical verification; inspect manifest epoch before requiring the new schema.

5. **Medium — “one human” does not establish that every alpha byte is regenerable.**  
   Matthew is a hosting operator and genesis peer, with substantial persisted storage ([deployments.json:55](/projects/elohim/genesis/orchestrator/data/deployments.json:55)). His checked-in account package includes identity and content assignments ([matthew-manager.json:3](/projects/elohim/genesis/data/account-packages/matthew-manager.json:3)). Those sources demonstrate a reseeding basis; they do not establish that all later authored content, hosted-account state, or chain history is represented there. The hApp manager explicitly documents source-chain deletion and new agent-key minting ([happ_manager.rs:17](/projects/elohim/elohim/elohim-storage/src/happ_manager.rs:17)).

   **Fix:** bind the authorized wipe to a named inventory of regenerable state and separately retained evidence. Under this read-only, no-network challenge, I cannot identify an additional unique alpha payload that would certainly be lost. I therefore weaken the safety assertion rather than invent a counterexample.

6. **High — three integrity edits produce a five-DNA crossing; the roll must use final effective hashes.**  
   Imagodei and mishpat also declare network seeds ([imagodei/dna.yaml:20](/projects/elohim/elohim/holochain/dna/imagodei/dna.yaml:20), [mishpat/dna.yaml:16](/projects/elohim/elohim/holochain/dna/mishpat/dna.yaml:16)). Changing those seeds changes their DNA identities despite unchanged integrity code. “Baseline three lines” is therefore wrong. Moreover, `[dna:migrate]` permits baseline mismatches; it does not prove baseline correctness ([dna-hash-guard.sh:68](/projects/elohim/scripts/ci/dna-hash-guard.sh:68)).

   **Fix:** before rolling adam and matthew, both must resolve the same final hApp, role modifiers and five effective DNA hashes; both embedded-conductor processes must receive intent covering every changed role. The conductor StatefulSet is the correct location ([adam-firstman-conductor.yaml:320](/projects/elohim/genesis/orchestrator/manifests/humans/adam-firstman-conductor.yaml:320)). Partial intent keeps old cells ([happ_manager.rs:310](/projects/elohim/elohim/elohim-storage/src/happ_manager.rs:310)). Update all five baseline entries, deploy matching coordinators, and exclude old-generation peers from service until crossed. Seed only after installed-hash equality is demonstrated.

7. **Medium — inventory topic versioning survives; the steward premise does not.**  
   Storage subscribes to the exact inventory topic ([behaviour.rs:569](/projects/elohim/elohim/elohim-storage/src/p2p/behaviour.rs:569)); iroh derives its topic ID from the topic string ([gossip.rs:51](/projects/elohim/elohim/elohim-storage/src/p2p_iroh/gossip.rs:51)). Old alpha storage peers normally **do not receive** the new-topic messages; they do not misparse them. That isolation is correct for a clean break, but creates an inventory partition until upgraded. Steward/node’s current behaviour contains shard and storage-sync protocols but no gossipsub ([transport.rs:43](/projects/elohim/steward/node/src/p2p/transport.rs:43)).

   **Fix:** correct the census and test mixed-version isolation. Report incompatible peers as unavailable inventory sources, not empty holders. Still rebuild steward for its imported shard protocol.

8. **Low — deleting `BlobToReplicas` survives the source challenge.**  
   The repository search found its declaration, but no production caller. Removing it shifts subsequent indices ([content_store_integrity/lib.rs:3827](/projects/elohim/elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs:3827)); the manifest/location coordinator paths use named variants ([content_store/lib.rs:16277](/projects/elohim/elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:16277)). Sweettest’s literal `LinkType(23)` belongs to mishpat cells, while the lamad injection uses `LinkType(1)`, preceding the deletion ([device_enrollment.rs:1090](/projects/elohim/elohim/holochain/tests/sweettest/src/tests/device_enrollment.rs:1090), [line 655](/projects/elohim/elohim/holochain/tests/sweettest/src/tests/device_enrollment.rs:655)).

   **Fix:** rebuild coordinator and integrity together and run the fresh-DNA link tests. No preserved numeric slot is required for discarded chains.

9. **Medium — `Content.content_hash` deletion survives, but not because canonicalization excludes it.**  
   `Envelope::canonical_bytes` includes the entire supplied payload ([envelope.rs:79](/projects/elohim/elohim/epr/src/envelope.rs:79)). It provides no general exemption for `content_hash`. However, the current head projection uses `content.blob_cid`, not `Content.content_hash` ([epr_head.rs:172](/projects/elohim/elohim/elohim-storage/src/epr_head.rs:172)); `EprHead` has no such field and hashes its serialized representation ([head.rs:108](/projects/elohim/elohim/epr/src/head.rs:108)).

   **Fix:** delete the redundant field, update wire consumers, and pin this separation in a test. Expect new head CIDs when their blob pointers change. Replacing the misnamed `DefaultHasher` helper also survives ([content_store/lib.rs:13775](/projects/elohim/elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:13775)).

10. **Medium — a DELETE-listed CI file remains live.**  
    `scripts/ci/lib/bundle-zip.sh` is sourced by `stage-spa-blob.sh:918` and `fleet-write-readiness.sh:373`; Jenkins invokes those scripts at [Jenkinsfile:296](/projects/elohim/Jenkinsfile:296) and [Jenkinsfile:827](/projects/elohim/Jenkinsfile:827).

    **Fix:** interpret DELETE as removing the old minting implementation and contract, not deleting the helper or consumers. Replace them atomically. The plan’s wording supports that narrower interpretation.

**The ruling I would record instead**

Proceed with a fixture-only clean break: no legacy byte-address fallback, equivalence index, or row migration. Three integrity-zome changes and five seed changes constitute one five-DNA operational crossing. Separate implementation commits are acceptable; only the final coherent bundle is deployed.

Use canonical raw+BLAKE3 CID validation that checks the complete decoded header, digest length and encoding. Recast `ShardManifest`, remove redundant `Content.content_hash`, rename blob/shard address fields, delete unused `BlobToReplicas`, and replace the false SHA-256 helper. Rebuild all dependent coordinators and verify fresh-DNA link behaviour.

Retain device keys, roster, brit notes and referenced evidence, memory lineage, citations and steward history. Keep existing steward-head verification unchanged in this crossing. Hold actual old tier manifests through an epoch-aware reader; do not classify unrelated lineage JSON as tier manifests.

Recast only inventoried, authorized state through a verified reset procedure. Before alpha activation, adam and matthew—and every other participating conductor—must use the final effective hashes, matching coordinators and complete migration intent. Old peers remain excluded until crossed. Topic versioning intentionally isolates old inventory.

Acceptance requires five correct baseline hashes, negative CID vectors, historical-evidence verification, proven storage reset, matching installed DNA sets, regenerated heads, and passing household and alpha checks.