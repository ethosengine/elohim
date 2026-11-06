# Heartwood’s hard-earned lessons for an artifact blob plane

## Method and coverage

I treated one changelog bullet—or one coherent prose fix subsection—as a change-unit, deduplicating repeated highlight/summary text within a release. Issue counts are distinct top-level records; classes overlap because one incident can expose several failures. All 190 issues and all 49 patches in the supplied snapshots are `open`, so an open patch is evidence of an intended mechanism, not a shipped fix.

The changelog has 38 units under `Fixes`/`Fixed Bugs`. It has no literal `Changed`, `Removed`, or `Security` headings; relevant changes, reversals, and security work appear under Improvements, New Features, Breaking Changes, Deprecations, and release prose. The exhaustive primary classification of those 38 fix units is:

| Primary class | Fix units |
|---|---:|
| Identity, delegates, canonical-head rules | 7 |
| Gossip propagation/privacy | 3 |
| Keys/authentication UX | 4 |
| Addressing, NAT, reachability | 6 |
| Signed-ref/data integrity | 2 |
| Fetch/data availability | 4 |
| Storage robustness/atomicity | 2 |
| Unrelated CLI/platform/tooling | 10 |
| **Total** | **38** |

The architecture reading matters: Heartwood private repositories are mainly authorization and discovery filtering, not encrypted custody. For our plane, chunk bytes are content-derived data-plane objects; inventory is ephemeral evidence, not authority; the declared channel head remains the control-plane decision.

## Failure classes

Counts below are `changelog records / open issues`; records can carry multiple tags.

### 1. Gossip relay and announcement propagation — 5 / 6; 1.1.0 → Unreleased

The clearest entries are Unreleased: “A node now relays newer versions of an announcement to peers that relayed an older version” and “no longer relays an inventory announcement back to peers that already sent it the same version” ([CHANGELOG:17](/projects/elohim/genesis/research/repos/heartwood/CHANGELOG.md:17), [CHANGELOG:20](/projects/elohim/genesis/research/repos/heartwood/CHANGELOG.md:20)). The open “Smarter Inventory Announcement” issue says a capped node “may never announce a new repository it has added” ([issue:11439](/projects/elohim/genesis/local-dev/radicle-survey/seed-api/issues-z3gqcJUoA1n9HaHKufZs5FCSGazv5.json:11439)).

Mechanism: Heartwood now stores the latest timestamp and every relayer of that exact version, clearing relayers when a newer version arrives, then excludes only same-version relayers ([service.rs:1491](/projects/elohim/genesis/research/repos/heartwood/crates/radicle-protocol/src/service.rs:1491), [service.rs:2342](/projects/elohim/genesis/research/repos/heartwood/crates/radicle-protocol/src/service.rs:2342)).

### 2. Announcement staleness and protocol versioning — 4 / 5; 1.7.1 → 2026-09-11

Versioning failures surfaced as compatibility and misleading transport errors. The 1.7.1 IPv6 change was “Partially undo[ne] … to stay backward compatible” ([CHANGELOG:336](/projects/elohim/genesis/research/repos/heartwood/CHANGELOG.md:336)). “Versioning in Radicle” says they had “run head on into the problem of versioning or the lack thereof”; unsupported identity versions appeared as “failed to perform fetch handshake” or “repository … not found” ([issue:12308](/projects/elohim/genesis/local-dev/radicle-survey/seed-api/issues-z3gqcJUoA1n9HaHKufZs5FCSGazv5.json:12308), [issue:8068](/projects/elohim/genesis/local-dev/radicle-survey/seed-api/issues-z3gqcJUoA1n9HaHKufZs5FCSGazv5.json:8068)).

Structural cure: independently version wire framing, announcement schema, artifact manifests, chunking/hash format, and signed holder claims; negotiate capabilities and return typed incompatibility errors.

### 3. Fetch failures, limits, and data availability — 11 / 7; 1.1.0 → 2026-10-15

In 1.3.1, a control-message size check was wrongly applied to Git streams and caused fetch panics ([CHANGELOG:752](/projects/elohim/genesis/research/repos/heartwood/CHANGELOG.md:752)). In 1.10.0, a configurable 30-second stream timeout was added because Tor/I2P congestion caused “spurious aborts” ([CHANGELOG:171](/projects/elohim/genesis/research/repos/heartwood/CHANGELOG.md:171)). The open pack-limit issue says the limit should remain low but needs a higher one-shot value for an initial large fetch ([issue:4974](/projects/elohim/genesis/local-dev/radicle-survey/seed-api/issues-z3gqcJUoA1n9HaHKufZs5FCSGazv5.json:4974)).

Mechanism: Heartwood separates message ceilings, cumulative pack receive size, inactivity timeout, concurrency, queue capacity, and connection/message rate limits. For us, race holders within independent budgets: per-stream bytes, aggregate bytes, stall timeout, retry budget, concurrency, and cancellation.

### 4. Signed-ref verification and security upgrades — 9 / 1; 1.1.0 → 1.10.1

Version 1.1 added `rad/root`, which “prevents a possible reference grafting attack” ([CHANGELOG:926](/projects/elohim/genesis/research/repos/heartwood/CHANGELOG.md:926)). Version 1.7 added parent binding and replay detection, but 1.7.1 reverted mandatory root verification because “a larger-than-expected portion of the network” failed ([CHANGELOG:356](/projects/elohim/genesis/research/repos/heartwood/CHANGELOG.md:356), [CHANGELOG:341](/projects/elohim/genesis/research/repos/heartwood/CHANGELOG.md:341)). Version 1.8 introduced monotonic `none → root → parent` feature levels and downgrade detection ([CHANGELOG:309](/projects/elohim/genesis/research/repos/heartwood/CHANGELOG.md:309)).

Mechanism: root prevents grafting; parent commitment plus duplicate-signature history detects replay ([refs.rs:267](/projects/elohim/genesis/research/repos/heartwood/crates/radicle/src/storage/refs.rs:267), [read.rs:156](/projects/elohim/genesis/research/repos/heartwood/crates/radicle/src/storage/refs/sigrefs/read.rs:156)). Security floors need dual-read migration, adoption telemetry, and staged enforcement.

### 5. Identity documents and delegate lifecycle — 10 plus 1 Unreleased / 4; 1.1.0 → Unreleased

Version 1.10.3 fixed clones where the founder was no longer a delegate by always fetching the founder namespace ([CHANGELOG:27](/projects/elohim/genesis/research/repos/heartwood/CHANGELOG.md:27)). Version 1.10 added explicit revision states and prohibited a delegate accepting two active sibling revisions ([CHANGELOG:119](/projects/elohim/genesis/research/repos/heartwood/CHANGELOG.md:119), [CHANGELOG:144](/projects/elohim/genesis/research/repos/heartwood/CHANGELOG.md:144)). Unreleased patch merging now uses the target branch’s canonical rule, not the identity threshold ([CHANGELOG:12](/projects/elohim/genesis/research/repos/heartwood/CHANGELOG.md:12)).

The open founder-root patch reports roughly 2,050 of 16,000 repositories becoming unclonable after a stricter root assumption ([patch:1709](/projects/elohim/genesis/local-dev/radicle-survey/seed-api/patches-z3gqcJUoA1n9HaHKufZs5FCSGazv5.json:1709)). Authority lineage must remain retrievable even after membership changes.

### 6. Seeding, blocking, private visibility, and custody — 9 / 9; 1.1.0 → 2026-10-15

Version 1.7 extended blocking from fetched namespaces to direct connections ([CHANGELOG:371](/projects/elohim/genesis/research/repos/heartwood/CHANGELOG.md:371)). Yet the open issue states: “Blocking a node stops fetching its namespace, but whatever was fetched before stays in storage, and the node keeps serving it” ([issue:4645](/projects/elohim/genesis/local-dev/radicle-survey/seed-api/issues-z3gqcJUoA1n9HaHKufZs5FCSGazv5.json:4645)). Other open incidents show block failing to disconnect, relayed gossip bypassing block, and blocked delegates making all fetches fail.

The proposed fixes are separate: disconnect live sessions, filter gossip, filter announced-ref fetches, remove namespaces, and exempt authoritative delegates ([patches:305](/projects/elohim/genesis/local-dev/radicle-survey/seed-api/patches-z3gqcJUoA1n9HaHKufZs5FCSGazv5.json:305), [patches:1477](/projects/elohim/genesis/local-dev/radicle-survey/seed-api/patches-z3gqcJUoA1n9HaHKufZs5FCSGazv5.json:1477), [patches:3285](/projects/elohim/genesis/local-dev/radicle-survey/seed-api/patches-z3gqcJUoA1n9HaHKufZs5FCSGazv5.json:3285)). “Block” therefore needs an explicit operation matrix: acquire, advertise, route, connect, retain, serve, and purge.

### 7. Storage corruption and atomicity — 4 / 5; 1.1.0 → 2026-06-23

Version 1.3.1 replaced SQLite `read` calls that unwrapped failures with `try_read` ([CHANGELOG:760](/projects/elohim/genesis/research/repos/heartwood/CHANGELOG.md:760)). Version 1.7 chose WAL/NORMAL: power loss can roll back transactions, but SQLite remains consistent ([CHANGELOG:380](/projects/elohim/genesis/research/repos/heartwood/CHANGELOG.md:380)). An open issue says “one corrupted Git repository makes the Radicle node unable to start up” ([issue:7298](/projects/elohim/genesis/local-dev/radicle-survey/seed-api/issues-z3gqcJUoA1n9HaHKufZs5FCSGazv5.json:7298)).

Clone handling uses a temporary repository, cleans it on failure, and moves it into place only after success ([fetch.rs:53](/projects/elohim/genesis/research/repos/heartwood/crates/radicle-node/src/worker/fetch.rs:53)). Our equivalent is verify-to-staging, atomic CID publication, per-artifact quarantine, and startup that tolerates isolated corruption.

### 8. Key and passphrase UX — 8 / 11; 1.1.0 → 1.10.3

Version 1.1 fixed empty `RAD_PASSPHRASE` to mean no passphrase ([CHANGELOG:896](/projects/elohim/genesis/research/repos/heartwood/CHANGELOG.md:896)). Version 1.6 added systemd credential sources for both secret key and passphrase ([CHANGELOG:542](/projects/elohim/genesis/research/repos/heartwood/CHANGELOG.md:542)). Version 1.10.2 labels keys loaded into SSH Agent to identify their purpose and source ([CHANGELOG:62](/projects/elohim/genesis/research/repos/heartwood/CHANGELOG.md:62)).

Recurring problems include failed prompts, stale cached passphrases, missing agents, keyrings, passphrase files, and hardware keys. Signing keys and transport-handshake keys should be separate capabilities with explicit, testable source precedence.

### 9. NAT and reachability — 11 / 7; 1.1.0 → 2026-09-11

Version 1.6 explains the core failure: a node selected unsupported IPv6 or Tor and “does not know how to try a follow-up address” ([CHANGELOG:559](/projects/elohim/genesis/research/repos/heartwood/CHANGELOG.md:559)). Version 1.3 changed failed connections from opaque timeouts to immediate errors ([CHANGELOG:827](/projects/elohim/genesis/research/repos/heartwood/CHANGELOG.md:827)). The open “Initial Node Connections Not Satisfied” issue reports insufficient peers despite an apparently available network ([issue:7637](/projects/elohim/genesis/local-dev/radicle-survey/seed-api/issues-z3gqcJUoA1n9HaHKufZs5FCSGazv5.json:7637)).

Advertised addresses must be capability-filtered, canonicalized, raced or retried with fallback, and distinguished from recently successful paths.

### 10. Metering, abuse, and backpressure — 2 / 6; Unreleased and 2025–2026 issues

The metering issue asks to “monitor the amount of data” and rate-limit mobile-network use or particularly busy repositories ([issue:12674](/projects/elohim/genesis/local-dev/radicle-survey/seed-api/issues-z3gqcJUoA1n9HaHKufZs5FCSGazv5.json:12674)). Heartwood currently has token-bucket message/connection limiting and byte counters, but not byte-rate shaping; overfull per-peer inboxes disconnect the peer ([wire.rs:698](/projects/elohim/genesis/research/repos/heartwood/crates/radicle-node/src/wire.rs:698)).

For our plane, meter verified bytes, rejected bytes, amplification, retries, and race waste per peer and artifact; enforce quotas before allocation and streaming.

## Recurring structural classes

These recur across at least three releases:

- **Fetch scale/availability:** 1.1, 1.2, 1.3.1, 1.6.1, 1.7, 1.8, 1.9.1, 1.10, 1.10.3. Prevent with separated budgets, resumable chunk fetch, typed partial results, and holder racing.
- **Signed refs/security evolution:** 1.1, 1.7, 1.7.1, 1.8, 1.10.1. Prevent with negotiated, monotonic feature levels and measured migration before enforcement.
- **NAT/reachability:** 1.3 through 1.10 across at least seven releases. Prevent with multi-address fallback and observed-path health.
- **Gossip/availability:** 1.1, 1.2, 1.4, 1.9, 1.10.3, Unreleased. Prevent with version-scoped idempotence and anti-entropy/set reconciliation rather than truncated snapshots.
- **Storage robustness:** 1.1, 1.3.1, 1.4, 1.7. Prevent with staging, atomic publish, quarantine, and failure isolation.
- **Keys/auth:** 1.1, 1.6, 1.9.1, 1.10.1–1.10.3. Prevent with one credential state machine and separate signing/transport keys.
- **Seeding/block policy:** 1.1, 1.4, 1.7, 1.10.3 plus many open incidents. Prevent with explicit consent/provenance and operation-specific revocation semantics.

## Explicit unmet needs

All are open in the supplied snapshot.

- **Large files:** no source names Git LFS or a large-single-file facility. The evidence is instead large repositories: `fetchPackReceive` needs a one-shot higher initial-clone limit, while client filters request `blob:none` and `tree:0` ([issue](/projects/elohim/genesis/local-dev/radicle-survey/seed-api/issues-z3gqcJUoA1n9HaHKufZs5FCSGazv5.json:4974)).
- **Shallow fetch:** “fetch: look into shallow fetches” proposes blobless/treeless degradation while retaining issue and patch data ([issue](/projects/elohim/genesis/local-dev/radicle-survey/seed-api/issues-z3gqcJUoA1n9HaHKufZs5FCSGazv5.json:10723)).
- **Metering/rate limiting:** requests mobile bandwidth accounting and per-repository prioritization or protection.
- **Private team seeding:** asks for discoverable private repositories, safe no-public-sync defaults, group access instead of per-repository DID edits, and understandable local/remote semantics. A maintainer says the protocol can support it, but the workflow is highly manual ([issue](/projects/elohim/genesis/local-dev/radicle-survey/seed-api/issues-z3gqcJUoA1n9HaHKufZs5FCSGazv5.json:6377)).
- **Official seed images:** a sister-organization OCI image is used operationally and described as “official enough,” but the issue remains open, uses an unpinned `latest`, and lacks a release-owned compose artifact ([issue](/projects/elohim/genesis/local-dev/radicle-survey/seed-api/issues-z3gqcJUoA1n9HaHKufZs5FCSGazv5.json:8152)).
- **Release process:** two open issues request publishing generated configuration schemas and running `cargo-semver-check` ([issues:1248](/projects/elohim/genesis/local-dev/radicle-survey/seed-api/issues-z3gqcJUoA1n9HaHKufZs5FCSGazv5.json:1248), [issues:1823](/projects/elohim/genesis/local-dev/radicle-survey/seed-api/issues-z3gqcJUoA1n9HaHKufZs5FCSGazv5.json:1823)).

## Removed, reversed, or default-flipped

- 1.7.1 reverted mandatory `rad/root` verification and partially reversed strict IPv6 parsing.
- 1.10.3 stopped populating `preferredSeeds` and removed raw canonical-rule matching.
- 1.7 lifted a fixed reference allow-list, removed fetch details from connected-peer JSON, and removed subscriber counts from fetch debugging.
- 1.6 removed bootstrap IPs in favor of DNS and skipped Tor addresses when Tor was unavailable.
- `rad fork`, `rad diff`, several direct config subcommands, and non-working patch-review options were deprecated; `DEFAULT_SOCKET_NAME` was removed.
- Open patches propose further safety reversals: `rad init` should not announce by default, and push should default from synchronous `sync` to `no-sync` ([patch](/projects/elohim/genesis/local-dev/radicle-survey/seed-api/patches-z3gqcJUoA1n9HaHKufZs5FCSGazv5.json:5334)).

## Top 10 hard-earned lessons for an artifact blob plane

1. ✅ **Gossip propagation:** “relays newer versions” is the strongest Unreleased fix ([CHANGELOG:17](/projects/elohim/genesis/research/repos/heartwood/CHANGELOG.md:17)). Track deduplication by `(subject, version, relayer)`, never subject alone.

2. ✅ **Inventory completeness:** “may never announce a new repository” exposes capped snapshot starvation. Use signed/versioned pages, additive deltas, or set reconciliation; never truncate a message interpreted as complete.

3. ✅ **Fetch limits:** the 1.3.1 control-limit-on-stream panic shows every byte plane needs its own budget. Bound chunks, streams, retries, aggregate transfer, and inactivity independently.

4. ✅ **Signed-ref evolution:** 1.7’s security tightening was reverted in 1.7.1 after real-network breakage. Negotiate monotonic security features, dual-read during migration, and enforce only after measuring adoption.

5. ✅ **Identity lifecycle:** 1.10.3 had to fetch the founder even after the founder ceased being a delegate. Preserve immutable authority lineage independently of current membership.

6. ✅ **Blocking/custody:** “keeps serving what was already fetched” proves block, disconnect, retain, serve, and purge are different operations. Make custody provenance and every policy effect explicit.

7. ✅ **Storage corruption:** “One corrupted Git repository makes it impossible to start” is a fault-containment failure. Verify to staging, publish atomically, quarantine locally, and keep the rest of the node available.

8. ✅ **Reachability:** 1.6 admitted there was no follow-up-address attempt. Race or fall back across supported iroh/libp2p paths and measure recent success, not mere advertisement.

9. ◐ **Availability:** inventory says where bytes might be, not that a complete verified closure is currently retrievable. Expire unsigned holder claims, probe chunks, and measure independent failure domains before declaring availability.

10. ◐ **Plane separation:** our 1 MiB chunks should remain raw CID-addressed bytes, while one bounded artifact manifest or corpus digest carries the declared head. Do not create one permanent control-plane head per chunk; offline holders should still read and serve verified bytes without any doorway or elected peer.