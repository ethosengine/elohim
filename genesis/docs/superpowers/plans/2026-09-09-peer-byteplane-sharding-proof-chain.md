---
id: peer-byteplane-sharding-proof-chain
title: Peer byteplane transfer and reconstruction without doorway
status: Active
class: substrate
serves: blob-durability
date: 2026-09-09
cites:
  - "doorway-continuity-proof-chain | Doorway continuity from owned fixtures to public delivery | sha256:b02b249aa72de892 | path: genesis/docs/superpowers/plans/2026-09-09-doorway-continuity-proof-chain.md"
  - "live-distribute-shards-household-observation-plan | Wave 1.3 | sha256:1cc01de165e2e0ef | path: genesis/docs/superpowers/plans/2026-06-26-live-distribute-shards-household-observation-plan.md"
---

Continue the byte-durability station named by the doorway continuity chain, at the
peer-hoster seam. Use the existing development valueflow recipe and blob-durability
habit. Peer retries, transport selection and shard-holder rotation exist; their
presence does not prove cold reconstruction after losing the ingest peer.

- [ ] A real shard from a 68 MiB RS-4-7 artifact crosses both permanent peer transports within an explicit bounded frame budget, with byte identity and oversize refusal verified.

The encoder emits 17 MiB shards, while targeted libp2p blob responses and Iroh shard
frames default to 16 MiB. Repair the transfer contract without changing the existing
RS geometry, globally widening unrelated protocols, or equating local upload success
with replicated durability. Preserve existing wire variants, custody checks and hash
verification. Test real encoded bytes through the codecs, including old wire shape
compatibility and oversized frame prefixes refused before allocation. A larger bounded
frame only closes this station; it is not arbitrary-size streaming or negotiated QoS.

- [ ] After real acknowledged shard placement, a cold surviving storage peer reconstructs an identical RS artifact while the ingest holder is unavailable, without using doorway or a complete local copy.

Extend app-blob-heal-on-read.feature. Generate run-specific bytes and ingest once via
the existing interface. Require seven distinct shard indices, a survivor union of at
least four, each survivor locally below four, and no complete-copy shortcut. Inventory
must use local-only shard presence checks, never healing GETs that prime the fixture.
Actual custody and transport bindings must support placement; seeded manifest rows
are not evidence. Own the fault with the mesh berth and process lock, restore in
teardown, record the serving peer/transport and exact digest. Three colocated peers
prove process loss only, not physical household diversity, private-note recovery or
seven-hub durability. Refuse an unsuitable fixture before faulting it.

- [ ] Peers transfer artifacts larger than one admitted frame using bounded chunk or stream work, preserving custody and integrity across interruption and retry.

This successor requires a compatible transfer design. The existing Iroh BLAKE3 stream
store is distinct from SHA256 shard storage and cannot silently replace it. Both
libp2p and Iroh remain supported. A frame budget protects memory; it must not become
an undocumented whole-file limit. Keep this obligation open after the bounded repair.

- [ ] Archive replication yields to interactive pantry and stock reads under each peer's declared runtime budget, with throttling distinguished from unavailability.

Operator direction: EPR purpose and custody context must support inclusive laptop,
household-hub, storage and serving participation. Inspect existing tier and scheduling
policies before adding any surface. Runtime capacity advertisements are evidence for
selection, not authority over another peer's budget or consent. Link the smallest
missing negotiation/scheduling station to existing policy rather than inventing a
parallel configuration hierarchy. This is an unfulfilled successor, not a claim that
raising a frame cap delivers scheduling or capacity-aware load balancing.

- [ ] Real-time voice and video retain their declared latency and jitter budgets under competing archive traffic when the network supports QoS, with honest fallback on networks that do not.

Backlog note requested by the operator, 2026-09-09: investigate network QoS separately
from application transfer scheduling. Reuse EPR application purpose and ark/device
context to distinguish latency-sensitive streams, interactive pantry/stock reads and
background archive replication. Evaluate transport scheduling and optional DSCP/router
integration without assuming markings are honored across a WAN. Each peer controls
its resource budget; a remote priority claim grants neither unlimited capacity nor
authority to starve other participants. Before delivery, measure latency, jitter,
loss and bulk-transfer progress under contention on supported and unsupported paths.
Keep this successor open until the capability and fallback are actually verified.

Read-only inventory, 2026-09-09: existing node registration and NodeShape declare
device capacity; ReplicatesDwelling and commons commitments carry scope/capacity/rate
bounds. services/replication_prioritizer.rs supplies High/Medium/Skip but the P2P
receiver currently shares one semaphore, not deadline or bandwidth lanes. Reuse these
inputs for an ephemeral admit/defer/refuse decision before existing BlobSwarm and
acquisition executors. services/conductor_admission.rs provides a local precedent
for background yielding. PeerCapabilityFlags names traffic_class/current_load as an
additive evolution point; current load and bandwidth reports are incompletely wired.
The source audit found no DSCP/socket priority setting or fetch-carried deadline.
Network markings should derive from admitted purpose, never an untrusted urgent label.

- [ ] A connected ingest peer can resolve the other consenting peers' current custody scope, availability and transport identity before assigning them real shards.

Chain / between peer connectivity and acknowledged placement (gap2) / missing node:
authoritative eligibility converges across peers. Read-only owned-mesh observation
2026-09-09: all three direct storage APIs answer and report two connected peers, but
each local projection contains only its own human, availability and active commons
provider. Transport bindings alone do not supply custody consent. Probe the existing
authoritative convergence path and require the selector to see actual consenting
remote providers; do not insert fixture rows or author commitments on their behalf
to manufacture a green recovery result. This prerequisite is open pending diagnosis.

P2P design gate for the first station: transfer buffers and limits are Ephemeral C,
reconstructed per request from existing runtime configuration. Content retains its
content-derived address; legacy SHA256 shard keys remain an internal compatibility
boundary, not newly named CIDs. No new DHT entry, coordinator function, signal, table,
Automerge document or HTTP route. DNA-hash-neutral; zero added heads at seed/year one
and zero added quiesce work. Existing notarized content and custody authority stand.
All four network stages retain authorization and integrity floors; neither is priced
away. Transport affinity remains auto across permanent libp2p and Iroh implementations.

Concern answers before implementation: C0 answered by locating the mismatch in byte
transfer, not notary/head truth; C1/C2/C9/C13 n-a because no authority, lineage or role
is created. C3 partial until supported-size transfer and exhaustion checks pass;
C4 partial until oversized transfers return honest errors; C5 answered because peer
capability is not authority; C6a partial until allocation and serialization bounds
are tested; C6b answered by immutable hash-addressed retries. C7 partial until actual
transfer succeeds, C8 partial with explicit errors and test witnesses, C10 partial
until old/new wire compatibility tests pass, C11 partial until peer-imposed bounds
are respected, C12 answered by retaining shared ShardService custody checks, C14
partial with witnessed residuals and the larger-transfer successor above. Register
any new Rust decision point with these concern answers and actual contract tests.
