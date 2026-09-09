---
id: native-network-lifecycle-sprint
title: Native network lifecycle from acknowledged writes to governed recovery
status: Active
class: substrate
serves: dataplane-convergence
date: 2026-09-09
cites:
  - "peer-byteplane-sharding-proof-chain | Peer byteplane transfer and reconstruction without doorway | sha256:10c8e540a3fac830 | path: genesis/docs/superpowers/plans/2026-09-09-peer-byteplane-sharding-proof-chain.md"
  - "doorway-continuity-proof-chain | Doorway continuity from owned fixtures to public delivery | sha256:b02b249aa72de892 | path: genesis/docs/superpowers/plans/2026-09-09-doorway-continuity-proof-chain.md"
  - "nachalah-supervised-activation-sprint-handoff | Nachalah next sprint | sha256:ce355b7759e4ed56 | path: genesis/docs/superpowers/plans/2026-09-05-nachalah-supervised-activation-sprint-handoff.md"
---

The destination is development and full lifecycle evolution carried by peers, arks
and berths. Kubernetes is a temporary test bench, never the architectural authority
or the definition of maturity. This sprint leaves concurrent eprfs work with its
owner and composes existing network, custody, release and runtime mechanisms.

Two execution lanes serve the existing active dataplane-convergence and
runtime-death-witnessed habits. Supporting blob durability and SDK checks do not
activate a third habit. Each station requires its own source-bound evidence;
successors cannot borrow a predecessor's green result.

## Lane A: acknowledged writes and peer recovery

- [ ] A configured SDK caller retains failed or unacknowledged writes, receives an honest failure, and can retry without discarding a newer concurrent write.

Start at crates/elohim-sdk's existing WriteBuffer and ContentClient. Exercise HTTP
503, transport failure, partial success across groups, cancellation, overlapping
flushes and concurrent updates of the same key. Acknowledge only the exact version
sent successfully. Do not promise exactly-once server effects after an ambiguous
response, embedded offline persistence, or process-restart durability. Existing
native-without-sync-url refusal remains unchanged. Verify both configured sinks.

P2P design boundary: this repair operates on the existing ephemeral outbound buffer
(C), not a new authoritative entity. Existing content identity and write routes are
unchanged. No DHT entry, coordinator, wire type, migration or additional head is
introduced; seed/year-one incremental head cost is zero. Local acknowledgement
must never be described as replicated custody. Any durable journal proposal is a
separate design gate, not a hidden extension of this repair.

- [ ] Consenting remote custodians become eligible through authoritative existing bindings, then a cold reader reconstructs an artifact after losing its ingest peer.

Continue gaps 6 then 2 of peer-byteplane-sharding-proof-chain. Refuse before upload
or fault when eligibility is absent. Preserve real RS 4-of-7 geometry, acknowledged
indices, survivor union at least four, each individual survivor fewer than four,
and no complete retained copy. Require exact reconstructed bytes and transport
evidence. Connected peers alone are not consenting custodians. Use an owned mesh;
do not change incumbent identities, permissions or canonical commitments to fit.

## Lane B: witnessed death and supervised evolution

- [ ] An eligible custodian receives an owned peer's death witness while a fresh peer without custody standing is refused before witness bytes are sent.

Continue death-witness station 3b-ii with the actual standing resolver and both
transport gates. Retain positive custodian delivery, negative late-joiner evidence
and preauthorization skip. Advance existing rendering and recovered-conductor
attestation stations only after their own prerequisites hold.

- [ ] A custodian renders the admitted witness and recovery produces one durable incident attestation despite replay.

Use the existing EPR rendering surface and death-witness story. Observe the page
with pnpm look; retain anonymous refusal and repeat-recovery controls. Do not count
a process death as a durable attestation before the recovered notary witnesses it.

- [ ] A governed runtime activation either adopts the intended executable or restores the previous executable without losing identity, stored state or its outcome witness.

Continue the Nachalah supervised-activation handoff: authenticated candidate/grant
binding and fresh revocation precede activation. Invalid authority leaves the
incumbent untouched. Failed readiness restores the prior runtime; repeated upgrade
cycles retain keys, database and berth. Coordinator-only adoption is not executable
replacement proof. Keep earned-arc actuation behind its existing prerequisites.

## Follow-on stations, admitted in dependency order

- [ ] Both transports derive trust from verified existing evidence and price scheduling without weakening integrity or starving public peers.

Complete the existing trust-priced-sync design gate and falsifier. The current
public-only handshake is not verified household trust. Forged, expired or unavailable
evidence cannot elevate; public service remains a floor. Demonstrate cost and correct
head convergence together, including large historical sets and correction re-arm.

- [ ] Browser discovery, household continuity and independently selected WAN entrances deliver the same declared release through outage and recovery.

Continue doorway-continuity-proof-chain; discovery deadlines precede broad failover.
Repo manifests may project the native contract onto the test bench. Actual public
DNS/cluster changes remain operator-owned. Multi-A membership and a test-only proxy
are insufficient WAN proof. Preserve shell/asset/SSR and real-content acceptance.

- [ ] Declared device budgets are demonstrated under combined runtime load, recovery and competing interactive/archive traffic.

Use the already-chosen operating budgets and existing scoreboards. Measure same
workload, hardware, observation window and trust class; include missing samples and
failed runs. Do not advertise phone support from x86 measurements or voice/video QoS
from a transfer-priority enum. Streaming beyond one frame and network QoS remain
explicit successors, not assumed capabilities.

## Verification and handoff

Use manifest-owned gates for changed projects, focused regression tests, then the
actual owned household scenarios. Serialize heavy cargo and mesh faults through
berth leases; lightweight independent review may run concurrently. Record native
claims, reports and independent verdicts before ratcheting habit evidence. Do not
weaken scenarios, trust, custody eligibility, clocks or fixtures to obtain green.
Keep exact source/runtime/environment limits and unresolved predecessors visible.
