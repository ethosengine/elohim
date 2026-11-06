---
title: "Radicle / Heartwood — hard-earned lessons for artifact storage on the blob plane"
id: radicle-heartwood-cross-pollination-2026-11-05
status: Capture
date: 2026-11-05
sovereignty-frame: bounded
---

# Radicle / Heartwood Cross-Pollination — November 2026

**[Radicle](https://radicle.dev)** is the longest-running attempt at a peer-to-peer code forge. Its current protocol, **Heartwood**, stores each repository as one bare git repository with a namespace per peer, signs each peer's whole ref set ("sigrefs"), elects canonical branches by a delegate threshold, gossips "I have this repository" announcements, and moves bytes as a single git pack. To us it is a **failure archive for the sprint we are about to run**. Heartwood is a working state plane: signed name-to-hash maps, threshold-elected heads, and verify-before-apply for refs. It never built a data plane for artifacts. Large files are pushed to git-annex/LFS, CI results carry no output digest, availability is whoever chooses to seed, and the replication target exists only on the client at push time. **The single most important lesson for the artifact sprint: a push-time replica count is not custody.** Heartwood's `DEFAULT_REPLICATION_FACTOR = 3` is reached once, when the publisher syncs, and nothing holds it afterwards. There is no repair loop, and a fetch that finds no seed is never retried. So a rakia channel head must not move until the artifact's whole chunk closure is held under committed custody by holders other than the publisher, and the reconciler, not the publish command, must keep that true.

Method: three Sonnet readers (Heartwood source anatomy at mirror head `2fce5b4e`; web history and blockers; Elohim blob-plane grounding) and three Codex legs (RIPs, CHANGELOG and seed issues, CI broker), synthesised here by Opus with targeted re-checks in source. **Verification key:** ✅ verified in source (re-checked or read directly) · ◐ single-source, inference, or primary web page read through a summariser · ⚠ secondary web or unverified. Abbreviations: **HW** = `genesis/research/repos/heartwood/crates/`, **CIB** = `genesis/research/repos/radicle-ci-broker/`, **R1–R4** = RIPs 0001–0004 in `genesis/research/repos/radicle-rips/`, **ES** = `elohim/elohim-storage/src/`.

**Membrane (sovereignty-frame: bounded).** Radicle presents its identity as *self-certifying*: a Node ID is an Ed25519 key, a repository's authority is a set of delegate keys plus a threshold, and the RID is the hash of a document those keys signed (R2, "Overview"). The TUF-style mechanics are sound. The ontology is not ours. R2 equates node, user and public key, and R1 says delegates "need not map one-to-one to maintainers", which concedes that the key set is not the people. We keep **ngit L1** unchanged: a signature is provenance; standing is held by people and collectives and backstopped by community; the network holds the root. Our identity ontology is *imago dei*, not crypto self-sovereignty. Radicle adds new evidence for L1 (lesson 11) without moving it.

---

## Lineage and org guard (read first)

- **Five generations, two abandoned stacks** ◐ ([radicle.dev/history](https://radicle.dev/history), via summariser): the alpha on IPFS (Feb 2019); **Radicle Link**, SSB-modelled with git replication and IPFS dropped (Oct 2019); the **Upstream** desktop client (Dec 2020, sunset Jul 2022); **Heartwood**, the third protocol generation (announced Jul 2022; 1.0.0 Sep 2024); the **1.x** line, now at 1.10.3 (2026-09-07) with an Unreleased section on master ✅ (`HW/../CHANGELOG.md:8, :23`); and a **planned 2.0 on iroh** ◐. Why IPFS was dropped: "repos are mutable", plus performance ⚠ (quoted only through a fork's docs and the FAQ). **Why Link was abandoned: not found.** No maintainer retrospective was located. Do not infer one.
- **Master has no iroh dependency yet** ✅ (grep of every `Cargo.toml` in HW finds none). The 2.0 transport is a plan, not code. Development moved to master for 2.0, with 1.x backports on `releases/1` ◐ (1.10.0 post).
- **Funding and leadership risk** ◐: the Foundation funded core development until May 2023, then split into Radicle and Drips under the Radworks DAO ([docs.radworks.org](https://docs.radworks.org/community/our-story)). In Q1 2025 the lead (cloudhead) stepped down as Org Lead and protocol development moved under the Foundation council ([Q1 2025 update](https://community.radworks.org/t/radicle-org-q1-2025-update/3636)). The organisation is single and DAO-grant dependent, publishes no adoption figures (that quarter: six non-core contributors), and has carried multi-device identity and hole punching as "in development" for years. Cite Radicle for mechanisms and failures, never for roadmap.
- **The 2026-09-23 transport disclosure** ⚠ ([disclosure post](https://radicle.dev/2026/09/23/disclosure-of-vulnerability-in-network-protocol), via summariser; corroborated by [InfoQ](https://www.infoq.com/news/2026/09/radicle-network-vulnerabilities/)). In **every released version** the node transport discards the Noise session keys, so all traffic (gossip and packs) is cleartext. A second flaw lets a node present another node's Node ID and fetch private repositories past the allow list. The advice is to treat private repositories sent over the network as disclosed; Tor, I2P or a VPN do not fix it. Signed refs still detect tampering. The fix is a **partitioning move to iroh**: a breaking major release that cannot talk to 1.x, keeping the storage layout, with no date. We did **not** locate the defect in `radicle-node/src/wire.rs`. The source confirms only that the transport is Noise XK with the node key as static identity (`HW/radicle-node/src/wire.rs:3, 42-45`) ✅. The vulnerability specifics stay ⚠.
- **Process** ✅ (`radicle-rips` tree; seed patch JSON): four numbered RIPs in tree (0001 Heartwood, 0002 Identity, 0003 Storage layout, all Final; 0004 URI scheme, **Draft**), and seven open RIP patches, all with empty `merges`. Three of the seven are editorial. Identity 2 has been open 408 days, and RIP-0000 (the process itself) asks for a volunteer shepherd. The scouting count of "4 merged RIPs" is sharpened here: 0004 is a draft in tree. Heartwood's own repository on `seed.radicle.xyz` shows 3 delegates, threshold 1, about 200 seeders ◐ (orchestrator scouting, not re-read). A threshold of 1 means any one delegate key moves the canonical branch, so the flagship does not exercise its own quorum.
- **Prior coverage**: none. The Research Index mentions Radicle only under Playnet's code host (`genesis/research/README.md:91`), and the ngit survey names Radicle "the obvious unsurveyed sibling". This survey supplies the missing verdicts and **re-verdicts nothing ngit settled**.

---

## What Heartwood is, mechanically (only what bears on artifacts)

| Mechanism | How it works | Grade, cite |
|---|---|---|
| **Storage namespaces** | One bare git repository per RID; each peer's refs under `refs/namespaces/<nid>/…`; one shared object DB, so forks deduplicate. A peer writes only its own namespace; the remote helper refuses a push to another NID | ✅ R3:23-33, 120-137, 346-355 |
| **Sigrefs** | The signed unit is a flat `"<oid> <refname>\n"` map in BTreeMap order (`Refs::canonical`), and Ed25519 signs exactly those bytes. The sigrefs commit holds two blobs, `refs` and `signature`; commit metadata is not signed | ✅ `HW/radicle/src/storage/refs.rs:170-183`, `…/sigrefs/write.rs:327-334` |
| **Root/parent hardening** | Feature levels `None < Root < Parent`. `refs/rad/root` inside the signed map binds it to the RID (stops grafting); `refs/rad/sigrefs-parent` must equal the real parent commit (stops replay). The reader walks history for duplicate signatures and refuses downgrade | ✅ `refs.rs:278-311`; `write.rs:108-113`; `read.rs` ~145-300 |
| **Identity doc + canonical refs** | The doc (delegates, threshold, visibility, payloads) is itself a COB at `refs/rad/id`; the RID is the git OID of the initial doc blob. `xyz.radicle.crefs` gives per-pattern `allow` + `threshold`. A commit's votes come from allowed tips that descend from it; candidates past threshold are kept and the longest linear chain wins. **Two over-threshold candidates with no linear relation fail with `DivergingCommits`; there is no recency tiebreak** | ✅ `HW/radicle/src/git/canonical/quorum.rs:173-237`, `error.rs:25,35`; `identity/crefs.rs:1-80` |
| **COB embeds** | Issues, patches and identity are signed commit DAGs under `refs/cobs/<type>/<oid>`. Attachments are `Embed{name, content}` stored as git blobs **inside the signed change tree**, referenced as `git:<oid>`. They travel only when the COB ref is fetched (all or nothing), with no per-embed size cap | ✅ `HW/radicle-cob/src/change/store.rs:205-215`, `HW/radicle/src/cob/common.rs:367-399`; cap GAP |
| **Gossip** | Node, inventory and refs announcements, signed and timestamped. Inventory is **repository-granular** and capped at `INVENTORY_LIMIT = 2973` RIDs per message. Above the cap the announcement is **silently truncated** with only a local warning ("other nodes will see only some of your projects") | ✅ `HW/radicle-protocol/src/service/message.rs:27, 235`; `service/gossip.rs:38-47` |
| **Seeding + replica target** | Seeding is a per-repo `Allow{scope}`/`Block` policy, chosen by the operator; R1 rejects DHT placement because operators must choose what they host. The only replica target is client-side: `DEFAULT_REPLICATION_FACTOR = 3`, reached by a sans-IO fetcher or announcer at sync time. There is no network-wide target and no repair | ✅ `HW/radicle/src/node/sync.rs:14`; `node/policy.rs:160-169`; R1 "Topology" |
| **Fetch** | Git smart protocol (gix) over a Radicle stream. Stages: canonical `rad/id`, delegates, sigrefs, verify, data refs, validate, apply only if valid delegates ≥ threshold. Enforced cap: `fetchPackReceive` 500 MiB by a byte-counting reader. Nominal 5 MiB / 5 GiB stage limits sit behind `#[allow(dead_code)]` fields | ✅ `HW/radicle/src/node/config.rs:136,150`; `HW/radicle-fetch/src/state.rs:27,30`; `stage.rs:197,284,365`. That the 5 GiB limit is inert: ◐ |
| **No quarantine before verify** | Ref updates stage in an in-memory refdb and commit only after validation, but **objects are written straight into the live object store** as `.keep`-held packs. Rejected objects are left for GC. Clone (not fetch) uses a temp repository moved into place on success | ✅ `HW/radicle-fetch/src/state.rs:152-168, 246-250`; `HW/radicle-node/src/worker/fetch.rs:53` |
| **CI broker job COBs** | `cib` turns node ref events into CI events in a SQLite queue, runs adapter subprocesses (a two-message JSON protocol: `run_id`, then `success`/`failure`), and writes **one job COB per commit** signed by the node's device key. The adapter response has **no artifact field**, and COB update failures are **logged and ignored** | ✅ `CIB/adapterlib/src/lib.rs:435-455`; `CIB/src/cob.rs:92-130`; `CIB/NEWS.md:247-249` |

---

## HARD-EARNED LESSONS

Ranked by consequence for the artifact sprint. Each lesson names the failure Radicle hit, the property it teaches, how it adjudicates against **our current build state**, and a sprint-testable form.

### 1. A push-time replica count is not custody (R12 b, e)

**Failure.** Replication is a client-side target reached once (`sync.rs:14`, the `must_reach(3)` default). There is no network replica target and no repair, and the source admits that a fetch with no connected seed is never retried: `// TODO: We should make sure that this fetch is retried later` (`HW/radicle-protocol/src/service.rs:2544`) ✅. R1 rejects compulsory placement and admits that a non-delegate contributor's patches are unavailable unless they stay online, with "no satisfactory answer at the protocol level" (R1:418-441) ✅. The FAQ's availability promise is "as long as at least one seeding node is online" ◐.
**Property.** Custody is a *standing commitment held by a reconciler*, distinct from the act of publishing, and a head that names unheld bytes is a dangling pointer.
**Against us.** We are ahead on mechanism and behind on wiring. The custody controller exists (`ES/reconcile/custody.rs:1-40`). It kicks `race_fetch` when a provider lacks a committed blob and emits `placement-gap` when a provider goes silent ✅. Salvage is opt-in with a **per-config** `target_replicas` and never conscripts (`custody.rs:397-404, 525-530`) ✅. But **R12(b) is absent**: nothing mints a `custody-blob` commitment for a build output (grounding §4) ◐. The replica target is per node, not per artifact. And `custody.rs:15-17` records that gossip inventory publish dies on every pod restart ✅, so the reconciler's view of holders is weakest exactly after a deploy.
**Sprint form.** *Invariant:* a rakia channel head moves only when every chunk of the artifact's manifest closure has ≥ k committed custodians other than the publisher, each with a recorded verified fetch. *a2o:* "Given an artifact published to channel C by Matthew's device, when Matthew's device goes offline, then every chunk is fetchable and hash-verifies from at least two other holders, and C's head was not observed to move before that held." This also lifts the publishing-order clause out of **ngit H8** (every reachable blob has a confirmed copy before the tag map moves). Radicle supplies the failure case for it. H8's OCI gateway stays held.

### 2. Verify into staging, publish atomically, contain corruption (storage)

**Failure.** Fetched objects land in the live store before verification (`state.rs:165-168`) ✅. An open issue reports that "one corrupted Git repository makes the Radicle node unable to start" (seed issue 7298) ◐. In 1.3.1, SQLite `read` unwraps had to become `try_read` (`CHANGELOG.md:760`) ✅. Only clone got the right shape: a temp repository moved into place after success (`worker/fetch.rs:53`) ✅.
**Property.** Bytes become visible under their address only after verification, by an atomic rename. A corrupt object is quarantined, not fatal.
**Against us.** On the network edge we are better: `race_fetch` verifies sha256 before it persists (`ES/p2p/blob_fetch.rs:9-15`) ✅, and the eprfs tier reader verifies every chunk and then the whole CID ✅. **Locally we repeat Radicle's mistake in a smaller form (new finding, re-checked in source):**
- `BlobStore::write_entry` writes straight to the final path with `fs::write`, with no temp file and rename (`ES/blob_store.rs:160-172`) ✅.
- "Already exists" is decided by a `metadata` probe alone (`:289-299`) ✅.
- The inline read path, for blobs up to 16 MiB, returns the file **without** re-hashing (`:606-624`). Only `get_chunked` verifies (`:645-651`) ✅.

So a torn inline write, from a crash or a full disk, sits at a valid address, is served unverified, and is never repaired by a re-store, because the store reports it as already present. Remote fetchers will refuse it, but they burn race budget on it every time.
**Sprint form.** *Invariant:* write to a sibling temp path, fsync, rename; "already exists" means "exists and verifies". *a2o:* "Given a held blob truncated on disk, when a peer requests it, then this holder refuses to serve it, records it as corrupt, and re-fetches from another holder; the node keeps serving every other blob."

### 3. Inventory is evidence, never a complete snapshot, and never truncated silently (dataplane)

**Failure.** Inventory is repository-granular and truncated at 2,973 RIDs with only a local warning (`gossip.rs:38-47`) ✅. The open issue "Smarter Inventory Announcement" reports that a capped node "may never announce a new repository it has added" ◐. In the Unreleased section, relay had to track `(subject, version, relayer)`, because peers that had relayed an older version were never sent the newer one (`CHANGELOG.md:17, 20`; `service.rs:1491-1505`) ✅. An inventory entry says "I have this repository", with no object-level knowledge (`service.rs` ~1610) ✅.
**Property.** An advertisement is a hint that expires. Any page the receiver might read as complete must be versioned, paginated or reconciled, never cut.
**Against us.**
- Granularity: we already advertise per blob, with snapshot plus delta and per-peer sequence gap detection (`ES/p2p/inventory_gossip.rs:1-11`) ✅. That avoids the repository granularity but not the scale. A 1 GiB artifact is 1,024 chunk CIDs, so per-chunk inventory multiplies volume by three orders of magnitude.
- Signatures: inventory signatures are a **Stage-1 placeholder, where one null byte is sufficient** (`inventory_gossip.rs:13-18, 121, 143, 175`) ✅. Advertisements are unauthenticated today. This is safe only because fetch verifies hashes, which is the trust contract's I1.
**Sprint form.** *Design:* advertise at **manifest** granularity (manifest CID plus held-chunk completeness), never per chunk, and paginate snapshots, never truncate. *a2o:* "Given a peer whose holdings exceed one snapshot page, when it takes custody of a new artifact, then a peer that has seen only its earlier snapshot learns of the new artifact within one gossip interval."

### 4. Bind the signed unit to its root and its parent from day one (R12 a)

**Failure.** Sigrefs shipped without a repository binding. `rad/root` arrived in 1.1.0 "to prevent a possible reference grafting attack" (`CHANGELOG.md:926`). Parent binding and replay detection arrived in 1.7 (`:356`), and feature levels with downgrade refusal in 1.8 (`:309`). That is three releases to close two attacks on the signed map ✅.
**Property.** A signed name-to-hash map must carry, inside the signed bytes, the identity it belongs to and the hash of its predecessor.
**Against us.** The rakia release manifest binds `channelId` and carries the previous-release CID as "a HINT ONLY", verified against the channel's L2 version chain, with a mismatch as a typed refusal (`elohim/rakia/schemas/v1/release-manifest.schema.json:32, 296`) ✅. That is the parent half, done correctly. **R12(a), the generic artifact entry, is absent** (grounding §5) ◐, and the eprfs `TierBlobManifest {epoch, record, byte_len, chunks}` carries neither a channel binding nor a parent (`elohim/eprfs/eprfs-storage/src/tier_blob.rs`) ◐.
**Sprint form.** *Invariant:* the artifact manifest entry's signed bytes include its channel or lineage root and its parent CID. *Tests:* a replayed older manifest is refused as stale; a manifest signed for channel A and presented on channel B is refused as grafted.

### 5. Tighten a security floor in stages: feature levels, dual-read, adoption measured first (inventory Stage 2)

**Failure.** 1.7.0 made `rad/root` mandatory "under the assumption that the overwhelming majority of repositories … would have this reference". In 1.7.1 it was reverted because "a larger-than-expected portion of the network is affected" (`CHANGELOG.md:341-350`) ✅. An open founder-root patch reports about 2,050 of 16,000 repositories becoming unclonable under a stricter assumption (seed patch JSON :1709) ◐.
**Property.** A security floor is a migration. Declare levels, read both forms, measure adoption, then enforce.
**Against us.** Our pending instance is exact: inventory signatures go from Stage 1 (null byte) to Stage 2 (Ed25519) (`inventory_gossip.rs:16-17`) ✅, and the north star is "mixed versions talk; no big-bang rolls". No grep in `p2p/inventory_*.rs` shows Stage 2 started ✅. Whether it landed elsewhere is unverified.
**Sprint form.** *Design:* inventory messages carry a declared `sig_level`; receivers count peers per level as a metric; refusing unsigned advertisements is a separate, later flip, gated on a measured adoption threshold. *Test:* a mixed household (one peer at Stage 1, two at Stage 2) still converges custody.

### 6. Version every surface independently; incompatibility is a typed error (manifests, wire)

**Failure.** The open issue "Versioning in Radicle" says they "run head on into the problem of versioning or the lack thereof". Unsupported identity versions surfaced as "failed to perform fetch handshake" or "repository … not found" (seed issues :12308, :8068) ◐. The IPv6 parse fix broke configs and was half-undone in 1.7.1 (`CHANGELOG.md:336-340`) ✅. A transport with no negotiation could only be replaced by partitioning the network ⚠ (disclosure).
**Property.** Wire framing, announcement schema, manifest schema, chunking/hash format and holder claims each carry their own version. A peer that cannot read something says so in a typed refusal.
**Against us.** The libp2p protocol id carries a version (`/elohim/blob/1.0.0`, `ES/p2p/blob_protocol.rs`) ◐. The rakia release-manifest schema is v1 and tolerates unknown properties ✅. Whether artifact manifests will carry a schema version that readers check is **open**, since R12(a) is unbuilt.
**Sprint form.** *a2o:* "Given an artifact manifest at a schema version this peer does not support, when it resolves the channel head, then it reports `unsupported-manifest-version` naming the version, never 'not found', and keeps serving the chunks it holds."

### 7. Transfer must be chunked, resumable and multi-source; an atomic pack is not (R12 d)

**Failure.** Heartwood moves a fetch as one git pack: atomic, re-sent in full on interruption, capped at 500 MiB (`config.rs:136,150`) ✅, with no resume code ◐. The open pack-limit issue asks for a one-shot higher limit for initial clones (seed issue 4974) ◐. Q1 2025 work was still about cloning repositories over 1 GB ◐. The third-party LFS fork stores bytes on IPFS, and its seeding still pulls whole files ⚠.
**Property.** An artifact is many independently verifiable chunks raced across holders. A peer that lands a chunk becomes a source.
**Against us.** Partially built. `blob_swarm.rs` races each shard independently across holders, and landed shards become sources ✅ (grounding). But this covers only the **over-16 MiB shard-manifest** path. Plain 1 MiB chunk-index blobs do not appear to use it ◐. eprfs has only `LocalOnly` and `FetchIfMissing` (`elohim/eprfs/eprfs-core/src/storage.rs:41-45`) ◐. And **the eprfs tier blob store is local-only and joined to nothing**: "no network head, publication, or daemon dependency" (`tier_blob.rs:3-4`) ✅. Separately, `get_chunked` assembles the whole artifact in one `Vec` (`ES/blob_store.rs:628-654`) ✅, so serving a multi-GiB artifact costs its full size in memory.
**Sprint form.** *a2o:* "Given a 64 MiB artifact held by two peers, when the fetch is interrupted halfway and one holder goes away, then resuming fetches only the missing chunks, from the remaining holder, and the result verifies against the manifest CID."

### 8. The attestation binds the output bytes, and publishing it is not best-effort (R12 c)

**Failure.** The CI broker's adapter response is `{run_id, info_url}` then `{result}`, with no artifact field (`CIB/adapterlib/src/lib.rs:435-455`) ✅. Artifact publication is "a future goal" (`CIB/doc/requirements.md:27-31`) ✅. Any adapter can emit `success`, and the broker signs it with the node key: attribution, not proof of execution ◐. Job COB update failures are "logged, but otherwise ignored" (`CIB/NEWS.md:247-249`) ✅. Poison events forced at-most-once processing, which leaves recovery to humans (`NEWS.md:401-409`) ✅.
**Property.** The signed verdict is small, keyed to the exact input closure, and carries the output CID. A missing attestation blocks publication rather than being logged.
**Against us.** brit's `BuildAttestationContentNode` already signs `{manifest_cid, inputs_hash, output_cid, agent_id, …}` with Ed25519 (`elohim/brit/brit-build-ref/src/build_cmd.rs:72-93`) ◐ (grounding). That is ahead of Radicle. **R12(c) is partial**: `--output <CID>` works, but nothing makes that CID a stored blob on the plane. ngit T5's three invariants stand. Radicle adds the best-effort anti-pattern.
**Sprint form.** *Invariant:* the channel head refuses an artifact whose attestation's `output_cid` differs from the manifest's artifact CID, and a failed attestation write fails the publish. *a2o:* "Given an artifact built and stored, when the attestation write fails, then the channel head does not move and the failure is a typed, durable state, not a log line."

### 9. Each byte plane gets its own budget (fetch)

**Failure.** In 1.3.1 a control-message size check applied to git streams made fetches panic (`CHANGELOG.md:752`) ✅. In 1.10.0 the stream timeout became configurable (30 s default) because Tor/I2P congestion caused "spurious aborts" (`:171`) ✅. The 5 GiB data-refs limit is declared but sits behind `dead_code` fields (`state.rs:30`, `stage.rs:365`) ✅/◐.
**Property.** Message ceilings, per-stream bytes, aggregate bytes, inactivity and total-duration timeouts, retries and concurrency are separate knobs. A declared limit that nothing enforces is worse than none.
**Against us.** `race_fetch` has batch parallelism 3 (`ES/http.rs:1582`) ✅, and 1 MiB chunks bound per-request bytes naturally. No per-artifact aggregate budget, stall timeout distinct from total, or retry budget was found in the grounding ◐.
**Sprint form.** *Design:* a per-artifact fetch budget (bytes, stall, retries), separate from per-chunk limits. *Test:* a holder that trickles bytes is abandoned on stall, and the chunk is re-raced elsewhere within the budget.

### 10. Block, withdraw, retain, serve and purge are different operations (custody policy)

**Failure.** "Blocking a node stops fetching its namespace, but whatever was fetched before stays in storage, and the node keeps serving it" (seed issue 4645) ◐. 1.7 extended blocking to connections (`CHANGELOG.md:371`) ✅. Open incidents show block failing to disconnect, relayed gossip bypassing it, and a blocked delegate failing every fetch. The proposed fixes are five separate patches ◐. A bloom-filter TODO says bits for blocked repositories are never removed (`service.rs:2323`) ✅.
**Property.** Each policy effect is named per operation: acquire, advertise, route, connect, retain, serve, purge.
**Against us.** Our release rule is already per operation for *retain*. `services/holds.rs` releases what a peer fetched for itself once nothing names it, holds what another peer placed until that peer withdraws, and never releases bytes with no arrival record (grounding §4) ◐. There is no matrix for a **yanked or superseded artifact** (stop advertise? stop serve? retain for audit?).
**Sprint form.** *Design:* a one-table operation matrix for `yank` and `supersede`. *a2o:* "Given a yanked release, when a peer asks for its chunks, then holders stop advertising it and still serve its chunks to a peer pinned to it by an older head." The exact cells are an operator decision.

### 11. Authority lineage must stay retrievable after membership changes (R12 e; ngit L1)

**Failure.** In 1.10.3, repositories whose founder was no longer a delegate could not be cloned, because the identity root needed the founder's namespace (`CHANGELOG.md:27-30`) ✅. The fix fetches the founder namespace forever (`HW/radicle-fetch/src/state.rs` ~318-335) ✅. In 1.10.0 identity evaluation was reworked because ambiguous majorities were possible ◐.
**Property.** Verifying an old artifact must not depend on the current presence of whoever signed it.
**Against us.** ngit L1 already settles the doctrine. Radicle adds the mechanical cost of ignoring it. The applicable ngit take is T2's second half: an act is judged by the role its signer held when they performed it. brit's `AgentKey` is per clone, with no roster binding until the S2 `binds-signer` row (D5) ◐.
**Sprint form.** *a2o:* "Given an artifact attested by a device whose recognition was later revoked, when a fresh peer verifies the channel's history, then the artifact verifies against the standing recorded at publication, and the revoked device need not be reachable."

### 12. Divergence is an error, not a recency contest (R12 e)

**Failure and counter-failure.** Canonical refs refuse to pick between non-linear over-threshold candidates (`quorum.rs:214`) ✅. That is right. But R1 proposed recovering sigrefs-history forks "by picking the history with the latest timestamp" (R1:487-488) ✅, and R2 admits that valid but stale identity history can be served until an honest peer shows newer history ◐.
**Property.** Two elected heads with no linear relation are a typed conflict for governance, never a clock comparison.
**Against us.** ngit L2 settles this, and our `ChannelHead` moves by per-device `seq` with `at` display-only (`elohim/eprfs/eprfs-peer/src/head.rs:3-6`) ✅. Radicle's `DivergingCommits` sharpens it into a *test*.
**Sprint form.** *Test:* two concurrent release proposals on one channel yield a typed divergence surfaced to the steward, and neither wins by `at`.

### 13. Identifiers must be self-describing (manifests)

**Failure.** RIDs are a bare 20-byte SHA-1 OID (R2:265-305). Supporting SHA-256 repositories needs an open patch to accept bare SHA-1, CID-wrapped SHA-1 and CID-wrapped SHA-256, with changes to grammar, parsers and tests ("SHA-256 Repository Identifiers", revision `a82c2e55`) ✅ (seed patch JSON).
**Property.** Every reference carries its codec and hash function.
**Against us.** Blob addresses are CIDv1 raw sha2-256 ✅, but `BlobStore` still accepts the legacy `sha256-<hex>` path form (`ES/blob_store.rs:221-240`) ✅.
**Sprint form.** *Invariant:* an artifact manifest names chunks and outputs only as full CIDs, and schema validation rejects bare hex.

---

## Blockers — what Radicle never solved, and whether we would inherit it

| # | Unsolved in Radicle | Evidence | Our plane |
|---|---|---|---|
| B1 | **Large files and releases punted** to git-annex/LFS; no artifact, chunking or release model in source, RIPs or CI broker; Radicle's own binaries ship as curl tarballs with SHA-256 | FAQ ◐; grep of `lfs`/`release artifact` in HW empty ✅ (anatomy §4); RIPs ◐; `CIB/doc/requirements.md:27-31` ✅; 1.10.0 post ◐. Issue "Add missing spec for Collaborative Objects" names a `radicle-artifact COB` discussion with no design ✅ | **Avoids** the model: a chunked CID store exists. **Would hit** it in practice until R12(a,b,c) are wired, because today build outputs have no manifest, no custody and no stored output CID |
| B2 | **Availability = whoever chooses to seed**; in practice, always-on public seeds | R1 "Topology" ✅; FAQ ◐; LWN commenters ⚠ | **Partly avoids**: `custody-blob` commitments plus a reconciler. Build outputs are not committed (R12 b) |
| B3 | **Repository-granular, truncated inventory**; no object-level holder knowledge | `gossip.rs:38-47`, `message.rs:27` ✅ | **Avoids** the granularity. **Would hit** scale if we advertised per chunk, and advertisements are unsigned (Stage 1) |
| B4 | **No repair; no retry** when seeds appear | `sync.rs:14`, `service.rs:2544` ✅ | **Partly avoids**: custody sweep on timer and on connection; `placement-gap` events. The replica target is per config and opt-in |
| B5 | **Key as the root of standing**; founder namespace fetched forever; one key = node = user | R2:64-83 ✅; 1.10.3 ✅; multi-device and key recovery not found ◐ | **Avoids** by doctrine (ngit L1). **Partly** in mechanism: brit's per-clone key has no roster binding until S2 |
| B6 | **Atomic pack transfer**, no resume | `config.rs:136` ✅; resume absent ◐ | **Partly avoids**: the shard swarm. Chunk-index blobs and the eprfs side are single-source |
| B7 | **Objects land before verify** | `state.rs:165-168` ✅ | **Avoids** on fetch (`blob_fetch.rs:9-15`). **Hits** it locally: non-atomic write plus unverified inline read (lesson 2) |
| B8 | **Private = access control over cleartext**; Node ID usable as a credential | HW `doc.rs:360-405`, `worker.rs:212-227` ✅; disclosure ⚠ | **Would hit** it if reach-restricted artifacts rode the plane before the confidentiality plane exists (ngit L5) |
| B9 | **A hand-rolled transport with no negotiation**, so the only fix partitions the network | disclosure ⚠; master has no iroh dep ✅ | **Partly avoids**: iroh plus libp2p behind versioned protocol ids. Manifest-level versioning is open (lesson 6) |
| B10 | **No NAT traversal** in source; reachability through seeds, Tor or open ports | grep empty in HW (anatomy §8) ◐; FAQ ◐ | **Avoids**: the household mesh homes conductors and storage to an iroh relay |

---

## Verdicts

Ids are local to this survey; ngit ids are always prefixed "ngit". Every row is adjudicated against the build state above.

### TAKE

| # | What | Adjudicated against | Lands in |
|---|---|---|---|
| T1 | **Custody-before-head**: a channel head names only a closure with ≥ k committed non-publisher custodians, and the reconciler, not the publish, maintains it | `custody.rs` built; R12(b) absent; per-config replicas only. Lifts ngit H8's publishing-order clause out of the hold | arch-native-dev-plane (R12 b/e) |
| T2 | **Verify-to-staging + atomic rename + verify-on-read** for the local store; corrupt objects are quarantined, not fatal | `blob_store.rs:160-172, 289, 606-624` (new finding) | arch-native-dev-plane; blob-durability habit |
| T3 | **Root- and parent-bound signed manifest** (Heartwood sigrefs `Root`/`Parent` levels as day-one fields) | rakia has parent-as-hint ✅; R12(a) entry absent | arch-native-dev-plane (R12 a) |
| T4 | **Staged security floors**: declared feature level, dual-read, adoption metric, then enforcement | inventory Stage 1 → 2 (`inventory_gossip.rs:16-17`) | arch-dataplane-borrows |
| T5 | **Manifest-granular, paginated inventory**; never truncate a page the receiver reads as complete; relay dedupe by `(subject, version, relayer)` | per-blob snapshot/delta exists; per-chunk would not scale | arch-dataplane-borrows |
| T6 | **Attestation output binding**: head refuses `output_cid ≠ manifest CID`; attestation write failure fails the publish | `build_cmd.rs:72-93` partial; sharpens ngit T5 | brit-daily-driver-handoff; arch-native-dev-plane (R12 c) |
| T7 | **Typed divergence**, tested: two non-linear elected candidates are a governance conflict | `head.rs` seq ordering; ngit L2 stands, this adds the test | arch-native-dev-plane (R12 e) |

### STUDY

| # | What | Why study, not take |
|---|---|---|
| S1 | The **sans-IO sync fetcher** with a replicas-or-preferred-seeds target (`HW/radicle/src/node/sync/fetch.rs:12-49`) | A clean shape for our per-artifact race, but its target is client-side. It informs `race_fetch`'s stopping rule, not custody |
| S2 | **Separated limit taxonomy** (message, frame, pack, inactivity, concurrency, token buckets; `config.rs:852-875`) | Input to lesson 9's budget design; numbers are not transferable |
| S3 | The **operation matrix for block** (five separate open patches) | Input to the yank/supersede matrix (lesson 10); cells are an operator call |
| S4 | **External COB helper protocol** (`HW/radicle/src/cob/external.rs:1-50`): typed objects evaluated by a subprocess over JSON Lines | A possible bridge-seam shape for foreign artifact types; low priority |
| S5 | **Simulation harness** (Talos + timoni, cross-version) | Cross-version testing matters for lesson 5; theirs is README-level ⚠ |

### LEAVE

| # | What | Why |
|---|---|---|
| L1 | **Repository-as-store**: artifacts as git objects or COB embeds | All-or-nothing with the ref fetch, no per-embed cap, SHA-1 identity. Contradicts "no second store" (grounding fact 1); ngit L4 and H9 already rule git a bridge |
| L2 | **Delegate keys plus threshold as the source of standing** | ngit L1 stands unchanged. Radicle adds the founder-namespace evidence (lesson 11). A threshold *mechanism* may still serve an election, but whose standing it counts belongs to the roster home (station 6) |
| L3 | **Client-side replica count as the availability promise** | Lesson 1. A sync-time count is not custody |
| L4 | **Best-effort attestation publication; at-most-once event loss** | `CIB/NEWS.md:247-249, 401-409`. Use idempotency keys and durable failure states instead |
| L5 | **Sigrefs-fork recovery by latest timestamp** (R1:487-488) | Already ngit L2 |
| L6 | **Visibility allow-list as privacy** | Already ngit L5; the disclosure ⚠ is the cost |

### WATCH

| # | What | Trigger |
|---|---|---|
| W1 | **Radicle 2.0 on iroh**: the first git forge moving onto the transport we run | an iroh dependency lands on HW master; read their storage-layout compatibility story |
| W2 | **Identity 2 RIP** (version field; crefs optional; open 408 days) | merged, or the patch text becomes readable (the capture has revision summaries only) |
| W3 | **`radicle-artifact` COB discussion** | a design appears; compare it with R12(a) |
| W4 | **Inventory-at-scale fix** ("Smarter Inventory Announcement") | merged; compare with T5 |
| W5 | **Org continuity** (DAO funding, post-2025 leadership) | any funding or leadership change; affects W1–W4 likelihood |

---

## What this changes for the artifact sprint

1. **D1, R12(a), the artifact manifest entry.** Start from `TierBlobManifest` (chunks, byte length, derivation record) and add the following inside the signed bytes: a schema version, the channel or lineage root, the parent CID, and `dependsOn`/`supersedes`, all as full CIDs. Ride the rakia release manifest's `Content` + `metadata_json` valve so the DNA hash stays neutral, and pass it through the p2p-design-gate before the HTTP route. (Lessons 4, 6, 13.)
2. **D2, R12(b), custody as a commitment.** The publish path mints `custody-blob` commitments for the manifest closure with a **per-artifact** replica target. Opt-in salvage stays the consent floor. (Lesson 1.)
3. **D3, R12(e), the head gate.** A generic channel head moves only when D2 holds, refuses typed divergence, and never orders by `at`. (Lessons 1, 12.)
4. **D4, R12(c), attestation binding.** The head checks that the attestation's `output_cid` equals the manifest's artifact CID. The attestation write is part of the publish transaction, not a best-effort side effect. (Lesson 8.)
5. **D5, R12(d), one store, multi-source chunks.** Join the eprfs tier blob store to elohim-storage's `BlobStore` (today they are unjoined). Extend the shard swarm, or its race discipline, to chunk-index blobs with resume, and stream `get_chunked` instead of buffering. (Lessons 7, 9.)
6. **D6, local store hardening (prerequisite).** Temp-and-rename writes, verify-on-exists, verify-on-read for inline blobs, and quarantine. Do this before artifacts land, because artifacts multiply the bytes exposed to torn writes. (Lesson 2.)
7. **D7, inventory.** Manifest-granular advertisements with completeness, paginated snapshots, and Stage-2 signatures shipped behind a declared level with an adoption metric. (Lessons 3, 5.)
8. **D8, yank/supersede matrix.** One table, decided by the operator, before the first release is superseded. (Lesson 10.)

The sprint needs a habit bound to a runnable check. Proposed: `artifact-custody-before-head`, born `unwired`, with its `@concern:` on the lesson-1 a2o scenario and `retire-when:` set to "the channel-head gate refuses an under-custodied closure in the fleet as well as the household".

---

## Open questions / frontier

- **Where the Noise defect lives.** We did not locate session-key discard or Node-ID spoofing in `wire.rs`. The specifics stay ⚠ until someone reads the fix diff.
- **Why Link was abandoned**, and any maintainer retrospective: not found.
- **Whether the 5 GiB `DEFAULT_FETCH_DATA_REFS_LIMIT` is enforced anywhere** outside radicle-fetch: open. The fields are `dead_code` at the stage.
- **The Identity 2, threshold-semantics and network-configuration RIP texts**: the seed capture holds revision summaries and empty descriptions, not file contents. Nothing is claimed about their designs.
- **Radicle's `RELEASE.md`**: not read.
- **Our side, unverified:** the iroh blob wire path (`p2p_iroh/blob_store.rs`, `inventory_fetch.rs`); whether `blob_swarm` covers chunk-index blobs; whether inventory Stage 2 has landed outside `p2p/inventory_*.rs`; whether brit's `verified_reads.rs` (brit-build-ref) and `cas_writes.rs` (brit-epr) close ngit's evidence-read gaps; and the R12 plan file outside the repository.
- **Per-chunk vs per-manifest inventory volume**: the T5 choice needs one measurement of advertisement size against artifact count on the household mesh.

---

## Method + credit

Three **Sonnet** readers:
- **Source anatomy** of Heartwood at GitHub mirror head `2fce5b4e` (2026-11-01; 1.10.3 + Unreleased).
- **Web history and blockers**, through search plus per-page summaries, hence ◐/⚠ throughout.
- **Elohim grounding**, a read-only pass over elohim-storage, eprfs, brit and rakia.

Three **Codex** legs:
- **gpt-6-astra** over the RIPs and the RIP seed patches and issues.
- **gpt-5.6-sol** over the CHANGELOG and the Heartwood seed issues and patches (190 issues, 49 patches; 38 fix units classified).
- **gpt-5.6-sol** over the CI broker.

**Opus** synthesised the survey and re-checked in source every citation the top lessons rest on. That pass corrected two line numbers (`sync.rs:14`, `state.rs:27,30`) and closed one gap (inventory above the cap is truncated, `gossip.rs:38-47`). It moved one location: `cas_writes.rs` is in brit-epr, not brit-build-ref. It sharpened one scouting fact: RIP-0004 is Draft, not merged. It also added one local finding (lesson 2: non-atomic `BlobStore` writes and an unverified inline read). Orchestrator scouting supplied the mirror head, the iroh absence (re-confirmed) and the seed delegate figures (not re-read). **An adversarial verification pass follows this draft**, and its corrections belong in a "Challenge pass" section as in the ngit survey.

---

## Outputs

Proposed mint rows, each citing `epr:radicle-heartwood-cross-pollination-2026-11-05`. **Proposed, not yet minted**: the fold belongs to the review pass.

- **arch-native-dev-plane-backlog**: T1/D2/D3, custody-before-head for build outputs (R12 b, e) *(proposed)*.
- **arch-native-dev-plane-backlog**: T3/D1, artifact manifest entry with root, parent, version and relations inside the signed bytes (R12 a) *(proposed)*.
- **arch-native-dev-plane-backlog**: T2/D6, atomic `BlobStore` writes, verify-on-exists, verify-on-read for inline blobs, quarantine (`blob_store.rs:160-172, 289, 606-624`) *(proposed)*.
- **arch-native-dev-plane-backlog**: D5, join the eprfs tier blob store to elohim-storage; resumable multi-source fetch for chunk-index blobs; streaming chunked read (R12 d) *(proposed)*.
- **arch-dataplane-borrows-backlog**: T4/T5/D7, manifest-granular paginated inventory, version-scoped relay dedupe, Stage-2 signatures behind a feature level with an adoption metric *(proposed)*.
- **arch-dataplane-borrows-backlog**: D8/S3, yank/supersede operation matrix (advertise, serve, retain, purge) *(proposed)*.
- **brit-daily-driver-handoff-2026-10-27**: T6/D4, attestation `output_cid` must equal the manifest's artifact CID, and an attestation write failure fails the publish (sharpens ngit T5) *(proposed)*.
- **commons-holonic-stewardship-backlog**: lesson 11, an artifact verifies against the standing recorded at publication, never against current membership (Radicle's founder-namespace evidence for ngit L1 and T2) *(proposed)*.
- **Habit (proposed)**: `artifact-custody-before-head`, `unwired`, bound to the lesson-1 a2o scenario.

Repos read: `genesis/research/repos/heartwood`, `radicle-rips`, `radicle-ci-broker`. Register them in `research-manifest.json` at the mint pass if they are not already there.
