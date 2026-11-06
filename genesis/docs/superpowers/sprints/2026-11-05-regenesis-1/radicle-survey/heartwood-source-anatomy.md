# Heartwood source anatomy (head 2fce5b4e, 1.10.3 + Unreleased)

Paths are relative to `genesis/research/repos/heartwood/crates/` unless noted. Grades: ✅ verified in source, ◐ inferred from source, ⚠ comments/docs only, GAP = not found in source. Line numbers are as read; some are approximate to the surrounding block.

## 1. Storage model, sigrefs, forgery resistance

- One bare Git storage per node; each peer's refs live under `refs/namespaces/<nid>/…`. Sigrefs branch is `refs/namespaces/<ns>/refs/rad/sigrefs` (`radicle/src/storage/refs/sigrefs/write.rs` doc ~L117-120). ✅
- What is signed: a `Refs` map (refname -> Oid) serialised by `Refs::canonical()` as `"<oid> <refname>\n"` lines in BTreeMap order (`radicle/src/storage/refs.rs:170-183`). The signature is an Ed25519 signature over exactly those bytes (`write.rs:327-334`). Each sigrefs commit is a tree with two blobs, `refs` and `signature` (`refs.rs:34-36`). ✅ The commit metadata is NOT what is signed; the `/refs` blob is.
- The signed map must include `refs/rad/root` (identity root = the RID's initial doc blob, `read.rs` IdentityRoot reader ~L360-390) and, at the newest feature level, `refs/rad/sigrefs-parent` = the previous sigrefs commit (`write.rs:108-113`). ✅
- Feature levels `None < Root < Parent` (`refs.rs:278-311`): Root defends graft attacks (a signed set re-homed under another repo); Parent defends replay (old signed blob re-committed). ✅
- Fetcher verification (`read.rs:SignedRefsReader::read`, ~L145-300): verify signature against the namespace key; check `rad/root` matches the RID; check `sigrefs-parent` equals the commit's real parent; if the head lacks parent-level, walk history, collect signatures, detect duplicates (replay), and refuse downgrade (`Read::Downgrade`). ✅
- Per-ref validation after objects arrive: `validate_remote` (`radicle/src/storage/git.rs:655-699`) compares every ref in the namespace against the signed map: `MismatchedRef`, `UnsignedRef`, `MissingRef`, `MissingRadSigRefs`. ✅
- Failure handling (`radicle-fetch/src/state.rs:~480-650`): offending remote's tips are pruned from the in-memory update set (`prune`). Non-delegate failure: silently dropped, recorded in `validations`. Delegate failure: removed from `valid_delegates`; the whole fetch applies only if `valid_delegates.len() >= threshold`, else `FetchResult::Failed`. A delegate whose new sigrefs `Diverged` from the stored one aborts with `Protocol::Diverged` (hard error); `Behind` is skipped. `config.level_min` prunes remotes below a minimum feature level. ✅
- Collision/forgery: namespacing by public key means a peer can only write its own namespace; the signed ref map binds contents to the key; the RID binding (`rad/root`) plus parent chain closes graft/replay. The RID itself is the Git OID of the initial identity doc blob (`state.rs` run_special_refs reads `find_blob(repo.id)`). ✅
- Teaches us: the signed unit is a flat name->hash map with a hash-chained history, verified before any ref moves. This maps cleanly to a package manifest over blob CIDs; Heartwood had to bolt on replay and graft defences in two later feature levels, so include repo-id binding and parent-pointer from day one.

## 2. Identity document and canonical refs

- Doc has delegates, threshold, `visibility`, payloads (`xyz.radicle.project`, `xyz.radicle.crefs`) (`radicle/src/identity/doc.rs:229-230, 699-700`). The doc is itself a COB (`xyz.radicle.id`, `radicle/src/cob/identity.rs:33`) and `refs/rad/id` is the per-namespace head. ✅
- Canonical reference rules live in payload `xyz.radicle.crefs` (`identity/crefs.rs:1-80`): `rules` (pattern -> `allow` + `threshold`) plus `symbolic` refs (e.g. `HEAD` -> branch). `allow` is `"delegates"` or an explicit DID list (`git/canonical/rules.rs:349-363`). Rules are validated; `HEAD` rule must use `allow: delegates` and the doc threshold (`doc.rs:1102-1107`). Most specific pattern wins (component count, then asterisk position, `rules.rs:71-121`). ✅
- Computation (`git/canonical.rs`, `canonical/quorum.rs`, `voting.rs`, `convergence.rs`): for a refname, each allowed DID's namespace tip is a vote; a commit gets a vote for itself and for each voter whose tip descends from it; candidates past `threshold` are kept; the longest linear chain wins; if two over-threshold candidates have a merge base equal to neither, it fails with `DivergingCommits` (`quorum.rs:173-237`). Tags: exact-OID vote, more than one candidate past threshold is `DivergingTags` (`quorum.rs:30-52`). ✅ There is no recency tiebreak: divergence is an error, never "latest wins". This is the closest analog to our declared channel head.
- Founder leaving (1.10.3, CHANGELOG L19-22): clones failed because the founder's namespace (needed to resolve the identity COB root) was not fetched once they stopped being a delegate. Fix at `radicle-fetch/src/state.rs:~318-335`: the doc root's sole delegate is always added to the fetched set. ✅ Lesson: the identity root has a permanent dependency on the founder's key material; our channel/package root must not.
- Unreleased fix (CHANGELOG L11-15): patch merges now count the matching rule's `allow`/`threshold`, not the doc threshold; unmatched target branch cannot merge. ⚠ (changelog only; not traced to code).

## 3. COBs

- Storage: a COB is a Git commit DAG under `refs/cobs/<typename>/<oid>`; each change commit holds a manifest, a `contents` blob and an optional `embeds/` subtree (`radicle-cob/src/backend/git/change.rs:28-29, 352-390`). Signature is an `ExtendedSignature` over the change's tree `revision` (`radicle-cob/src/change/store.rs:~100-170`, `valid_signatures`). ✅
- Type names: `xyz.radicle.issue` (`cob/issue.rs:31`), `xyz.radicle.patch` (`patch.rs:40`), `xyz.radicle.id` (`identity.rs:33`), `xyz.radicle.thread` (`thread.rs:19`). TypeName max 255 bytes, 63 per component (`radicle-cob/src/type_name.rs:25-26`). No job/CI/release type in tree. ✅ `cob/external.rs` defines external COBs: a helper executable reads/writes JSON Lines to evaluate ops (`external.rs:1-50`). ✅ This is a plug-in point, but evaluation is delegated to a subprocess.
- Embeds: `Embed<T> {name, content}` where content is bytes or a blob `Oid` (`change/store.rs:205-215`). `Embed::store` calls `repo.blob(content)` and keeps the Oid. Referenced from COB bodies as `Uri("git:<oid>")` (`cob/common.rs:367-399`); also a `DataUri` variant exists (`common.rs:425`). Because the blob is in the change's tree, it is covered by the change signature and travels with `refs/cobs/*`, which are listed in sigrefs. ✅
- Size limit on embeds: GAP. No per-embed cap found (grep of MAX/size in cob code finds only TypeName and string-length caps). Only the whole-fetch caps in section 4 apply.
- Replication: embeds replicate as ordinary Git objects whenever the COB ref is fetched; there is no per-attachment fetch, dedupe-by-name, or lazy fetch. ◐
- Teaches us: attachments as blobs addressed by hash, inside a signed tree, is the same shape as manifest-names-blobs. The limit is that blob existence is coupled to the ref fetch (all-or-nothing).

## 4. Large or binary content, hard caps

- No LFS, artifact, or release story: grep for `lfs`, `large file`, `release artifact` over `*.rs`, `*.md`, `*.adoc` returns nothing. ✅ (absence)
- Caps found:
  - Fetch size: `DEFAULT_FETCH_SPECIAL_REFS_LIMIT` 5 MiB and `DEFAULT_FETCH_DATA_REFS_LIMIT` 5 GiB (`radicle-fetch/src/state.rs:25-29`). But the stage `limit` fields are `#[allow(dead_code)]` (`stage.rs:~198, 283, 365`), so these appear unenforced in radicle-fetch. ✅ (the dead_code allows) ◐ (that they are inert)
  - Enforced pack cap: `limits.fetchPackReceive` default 500 MiB, applied by a `ChannelReader` byte counter that aborts the stream (`radicle/src/node/config.rs:136, 205-236`; `radicle-node/src/worker/channels.rs:156-216`). ✅
  - `fetchTimeout`: per-stream inactivity timeout, not total duration (`config.rs:138-145`). ✅
  - Wire: message max = `u16::MAX` less type tag (`radicle-protocol/src/wire/message.rs:61`); frame limit 128 KiB (`wire/frame.rs:214`); `max_open_files` 4096; routing table 1000 entries; fetch concurrency 1 per peer; inbound 128 / outbound 16 connections (`config.rs:852-866`). ✅
  - Doc strings capped at 255 (`identity/doc.rs:40`); alias 32 bytes (`node.rs:66`).
- Implication: large artifacts would be one Git pack, atomic, re-sent in full on interruption; no chunking, resumption, or partial fetch (◐; nothing in source implements resume).

## 5. Gossip

- Messages: `Subscribe{filter, since, until}`, announcements `Inventory`, `Node`, `Refs` (signed, `AnnouncementMessage`), `Info`, `Ping/Pong` (`radicle-protocol/src/service/message.rs:30-37, 253-260, 401-406`). ✅
- Caps: `ADDRESS_LIMIT` 16, `REF_REMOTE_LIMIT` 1024, `INVENTORY_LIMIT` 2973 RIDs per inventory message (`message.rs:23-27`). 2973 x 22-byte RIDs approximates the 64 KiB message cap. ◐ A node seeding more public repos than that: how it chunks is GAP (not traced).
- Subscription filter: bloom filter in three sizes (about 855 / 3,419 / 13,675 items at 1% FPR) built from seeding policy (`service/filter.rs:7-45`); default filter matches everything. ✅ `// TODO: Remove bits for blocked repos` (`service.rs:2323`).
- Relay: announcements are relayed unless the peer is the announcer or already relayed that version; relay to a peer only if the repo `is_visible_to` it (`service.rs:2355-2380`); announcements older than `MAX_TIME_DELTA` 60 min are not relayed except node announcements (`service.rs:97, 1516-1522`). `relayed_by` map tracks per-subject latest version and relayers (`service.rs:342-344`, `1491-1505`). The Unreleased changelog fixes are about re-relaying newer versions to a peer that relayed an older one (CHANGELOG L16-20). ✅
- Learning who holds a repo: inventory announcements update a SQLite `routing` table (rid, nid); refs announcements call `seed_discovered` (`service.rs:~1610`); `sync_routing`. Routing capped at 1000 entries and aged out (`config.rs:114-121, 773-796`). ✅ The inventory announcement only means "I have this repo", with no per-object information.
- Refs announce carries `(remote, at)` pairs (`RefsAt`), up to 1024 remotes; recipient fetches only if it seeds the repo and is directly connected to the announcer (`service.rs:1672-1700`).
- Admitted scaling limits in comments: `service.rs:1970` "Only subscribe to outbound connections; otherwise, we will consume too much bandwidth"; `1525` "Refs announcements should not be delayed, since they are only sent to subscribers" (FIXME); `1677` indirect-follow reachability (Alice cannot fetch Eve via Bob). Announcement PoW exists for node announcements (`message.rs:360-364`). ✅

## 6. Seeding and replication policy

- Per-repo `SeedingPolicy::{Allow{scope}, Block}` with `Scope::{Followed, All}` (default All) and a node-level default (`radicle/src/node/policy.rs:160-169`; `config.rs:425`); per-node `Policy::{Allow, Block}` follow list stored in SQLite tables `following` and `seeding` (`policy/schema.sql`). ✅
- Fetch scope: `Allowed::All` or `Allowed::Followed{remotes}` (`radicle-fetch/src/policy.rs:11-52`); delegates are always fetched regardless of scope (`stage.rs` SpecialRefs ~L255-280, `state.rs`). ✅
- Triggers: inventory announcement from a connected peer for a seeded repo we lack, in shuffled order (`service.rs:1575-1620`); refs announcement for a seeded repo (`1700`); explicit `rad sync`/clone commands. A fetch with no connected seed is not retried: `TODO: We should make sure that this fetch is retried later` (`service.rs:2544-2547`). ✅
- Replication factor: exists only on the client side of sync: `DEFAULT_REPLICATION_FACTOR = 3` (`radicle/src/node/sync.rs:13`), `ReplicationFactor` min/max, a sans-IO `Fetcher` that tries candidate seeds in given order until it reaches a replica count or preferred-seed set (`radicle/src/node/sync/fetch.rs:12-49, 192-200`; `announce.rs` mirrors it for pushing). ✅ There is no network-wide replica target or repair loop; replication is whoever chooses to seed. ◐
- Per-peer fetch queue with max capacity and concurrency (default 1) (`radicle-protocol/src/fetcher/state.rs:330-355`, `service.rs:1018-1046`). ✅

## 7. Fetch protocol

- Git smart protocol v2-style ls-refs + fetch over a Radicle stream, driven with gix-protocol (`radicle-fetch/src/lib.rs`, `transport/ls_refs.rs`, `fetch.rs`); a `// FIXME: the delegate pattern will be removed` marks gix API debt (`transport/fetch.rs:94`). ✅
- Stage order in `FetchState::run` (`state.rs:~350-470`, doc comment at L322-335): 1 fetch canonical `rad/id` (always; "our delegate set might be ahead"); 2 compute delegates minus blocked, threshold (minus self if local is a delegate); 3 fetch `rad/id` + `rad/sigrefs` for delegates + followed (or all namespaces if scope All); 4 load and verify sigrefs; 5 fetch data refs named in sigrefs (wants/haves from signed oids; `ls_refs` is skipped, `DataRefs::ls_refs` returns None); 6 validate; 7 apply tips only if enough delegates valid; 8 signal done. ✅
- Fetch-then-verify, with staging only for refs: new refs go to an in-memory `git::mem::Refdb` and are committed to the real refdb only after validation (`state.rs:152-156`). Objects, however, are written straight into the repo's object store as packs held by `.keep` files until the state drops (`git/packfile.rs`, `state.rs:165-168, 246-250`). There is no object quarantine directory; unreferenced objects from a rejected fetch are left to GC. ✅ Pushing only after verification is the right invariant for us; the object-level staging is the part Heartwood lacks.
- Pruning is per remote: one bad namespace does not poison others, except delegates (threshold).

## 8. Transport and network

- Noise XK over TCP with the node Ed25519 key as static identity (`radicle-node/src/wire.rs:3, 42-45`); frames multiplex gossip and git streams (`radicle-protocol/src/wire/frame.rs`, stream ids with initiator bit). ✅
- Addressing: IP, DNS, Tor onion v3, I2P, via SOCKS5 proxy; features `tor`, `i2p` default-on (`radicle-node/Cargo.toml:14-19`; `config.rs:406-420, 606-609`). ✅
- NAT traversal: GAP. grep for upnp/hole-punch/stun in node and protocol finds nothing; reachability relies on inbound ports, Tor, or well-known seed nodes. ◐
- SQLite: node db (`nodes`, `addresses`, `routing`, `announcements`, `repo-sync-status`; `node/db/schema.sql`), policy db (`following`, `seeding`), notifications db, and a COB cache db (`issues`, `patches`; `cob/cache/migrations`). Numbered `.sql`/`.rs` migrations. ✅
- Rate limiting: token bucket per connection (inbound 5/s fill, cap 1024; outbound 10/s) (`config.rs:274-300, 857-875`). ✅

## 9. Private repositories

- `Visibility::{Public, Private{allow}}` in the signed identity doc; visible to delegates plus `allow` DIDs (`identity/doc.rs:360-405, 962-967`). ✅
- Enforcement points: (a) upload-pack refuses unless `doc.is_visible_to(remote)` where remote is the Noise-authenticated NodeId (`radicle-node/src/worker.rs:212-227`); (b) private repos are removed from the advertised inventory (`service.rs:439-497`); (c) announcements about a repo are relayed only to visible peers (`service.rs:2365-2380`); (d) sync restricts candidate seeds to the allowed set (`node/sync.rs:16-40`). ✅
- Limits: access control only, no encryption of stored objects or per-object keys. Anyone allowed can redistribute; revocation cannot recall copies; the doc (including the allow list) is itself stored in the repo, so membership is visible to every allowed party. If a node does not hold the repo it "can't determine if it's private" and so does not relay (`service.rs:2370`). ◐ (inference about leakage). Gossip peers learn repo existence only via refs announcements to visible peers.

## 10. Engineering discipline

- Workspace clippy lints deny `indexing_slicing`, `fallible_impl_from`, `wildcard_enum_match_arm`, `must_use_candidate`, `fn_params_excessive_bools`, `unneeded_field_pattern` (`Cargo.toml:80-86`); edition 2024, MSRV 1.88; `#[non_exhaustive]` on error and level enums. ✅
- `deny.toml`: licenses allow-list; advisories delegated to `cargo-audit` (`deny.toml`, advisories section). ✅
- `VERSIONING.md`: stack version is a signed annotated git tag `vX.Y.Z`, dev builds use `git describe`; crate versions follow SemVer separately. `RELEASE.md` exists (not read). ⚠
- Schemas: `schemars` JSON Schema generation for config, sigrefs feature levels with documented introducing commits (`refs.rs:284-304`).
- Tests: unit and property tests with qcheck/proptest (sigrefs `property.rs`, `mock.rs`), a node e2e suite (`radicle-node/src/tests/e2e.rs`), CLI snapshot tests (`radicle-cli-test`), and `fetcher/test`. `simulation/` is a Kubernetes (Talos + timoni/CUE) harness for multi-node, cross-version and cross-platform tests; adverse-network emulation (drops, delay) is still an unchecked item (`simulation/README.md:3-5, 149-192`). ⚠ (README), no Rust sim crate yet.
- CHANGELOG has Keep-a-Changelog structure and is precise about security-relevant fixes.

## 11. Ten most revealing TODO/FIXME

1. `radicle-protocol/src/service.rs:2544` fetches that fail for lack of a connected seed are not retried when seeds appear.
2. `service.rs:1525` FIXME: refs announcements are delayed/stale-dropped by the generic gossip freshness logic though only subscribers get them.
3. `service.rs:1970` subscribing on inbound connections would consume too much bandwidth.
4. `service.rs:1677` TODO(Ade): cannot fetch from followed-but-unconnected peers reachable only through a relay.
5. `service.rs:2088` TODO: should fetch from an already-connected seed that has refs we lack.
6. `service.rs:2323` TODO: bloom filter bits for blocked repos are never removed.
7. `radicle-fetch/src/state.rs:419` TODO: should any delegate be blockable (could end up ignoring delegates).
8. `radicle-fetch/src/transport/fetch.rs:94` FIXME: the gix delegate pattern will be removed; fetch is coupled to gix internals.
9. `radicle/src/rad.rs:251` TODO: look into sharing object databases (each checkout/clone duplicates objects).
10. `radicle-protocol/src/service.rs:1432` and `radicle-node/src/wire.rs:962` FIXME: sessions left in inconsistent states after errors (peer still processed; session stuck in "initial").
Also relevant: `radicle-protocol/src/fetcher.rs:16` fetch logic is split across two layers; `radicle/src/cob/patch.rs:2523` merge-twice not forbidden.

## Five mechanisms most relevant to our blob plane

1. Signed flat name->hash map as the unit of publication, hash-chained to its parent and bound to the repo id (sigrefs; `Parent` and `Root` levels). Maps to our signed package manifest plus channel head.
2. Canonical head as threshold vote among an allow-set, with linear-ancestry convergence and explicit divergence failure, no recency (`quorum.rs`). Direct template for "latest = declared channel head".
3. Verify-then-apply in memory: stage ref updates, validate all, commit atomically, prune per-remote, threshold across delegates (`state.rs`).
4. Sync fetcher as a sans-IO state machine with a replication-factor-or-preferred-seeds target (`node/sync/fetch.rs`).
5. Embeds as blobs in a signed tree, referenced by content hash URI (`git:<oid>`), plus external-COB helper protocol as a typed-object extension seam.
(Honourable: bloom-filter subscriptions and announcement-version relay tracking; visibility checks at three planes.)

## Five limits Heartwood visibly hit

1. No large-file/artifact story: no LFS, no chunking or resume; one pack per fetch, hard 500 MiB receive cap, 5 GiB nominal cap that appears unenforced.
2. Gossip inventory is repo-granular and capped at 2973 RIDs per message; no object- or blob-level inventory, so no custody proof or partial replication knowledge.
3. No network-level replication target or repair; replication factor exists only client-side, and failed fetches are not retried when seeds appear.
4. Object-store coupling: git objects land in the live object store before verification (no quarantine), and attachments are all-or-nothing with the ref fetch; no per-embed size cap.
5. Reachability and scaling: no NAT traversal, relay-only discovery through connected peers, per-connection fetch concurrency 1, routing table 1000 entries, plus an identity root permanently tied to the founder key (the 1.10.3 clone bug). Private repos are access control, not confidentiality.

## Gaps not resolved

- RELEASE.md content, inventory chunking above 2973 repos, how `refs/rad/id` canonical head is chosen across delegates in code (inferred from `CanonicalId` stage), whether the dead_code fetch limits are enforced elsewhere, Unreleased merge-quorum fix code path.
