# Recovery M4 — D1–D4 Brainstorm Outcomes

**Date:** 2026-05-15
**Status:** Decisions ready for operator ratification
**Sprint:** /projects/elohim/genesis/docs/plans/2026-05-15-recovery-m4-completion-shamir-optional-kickoff-prompt.md
**Cross-sprint binding:** D2 commits EPR companion sprint's D3.

---

## D1 — RecoveryFlowProjector vs. extend AttestationProjector

### Question

The consolidation introduced `AttestationProjector` (`elohim/elohim-storage/src/services/attestation_projector.rs`) as the single entry point for all `attestation:*` and `governance-action:*` signals. Recovery primitives — `recovery-request:<kind>`, `key-revocation:<kind>`, `identity-freeze` — need a projection home. Do they extend the existing `AttestationProjector` or live in a sibling `RecoveryFlowProjector`?

### Alternatives

**A — Extend AttestationProjector:** Add `content_type.starts_with("recovery-request:")`, `starts_with("key-revocation:")`, and `starts_with("identity-freeze")` branches inside `handle_content_signal`. Recovery rows land in a new `recovery_flows` projection table; the projector does the dispatch. The single `handle_content_signal` entry point remains.

**B — Sibling RecoveryFlowProjector:** Create `elohim/elohim-storage/src/services/recovery_flow_projector.rs` mirroring the module structure. The call site (wherever `handle_content_signal` is invoked) routes by prefix first, then delegates to the appropriate projector. AttestationProjector stays unmodified.

### Tradeoffs

**Dispatch grain is fundamentally different.** `AttestationProjector::handle_content_signal` (line 29–71) routes between `attestations` and `governance_actions` tables. Both are general-purpose accumulator tables — any attestation subtype lands in `attestations`, any governance-action kind in `governance_actions`. Recovery projection is state-machine-shaped: a `recovery-request` entry opens a flow, subsequent `recovery-approval` attestations advance it, and a `key-rotation` closes it. That state machine does not compose cleanly as another branch in the existing accumulator logic.

**The `key_revocations` table is already pre-classified as its own projection (EPR W2D, Category C).** The EPR sprint's Wave 1 adds `key_revocations` as a sibling projection to `attestations` — not a subtable inside it. This is already a precedent for splitting recovery-domain projections away from the general attestation accumulator.

**AttestationProjector already handles `attestation:revocation-*` subtypes.** The `resolve_manifest_ref` function (lines 183–213) already returns `"imagodei"` for `attestation:revocation-*` prefixes. This means `RevocationVote` attestations — the vote-children of `governance-action:key-revocation` — already land in `attestations`. That coverage is correct and should not move. What is NOT covered is the `governance-action:recovery-request` and `governance-action:key-revocation` event kinds that open recovery flows, plus the state-machine projection of vote progress and effective-date tracking. That state belongs in a recovery-specific projector.

**Cohesion and testability.** `AttestationProjector` has clean unit tests (lines 216–331) covering its three routing branches. Adding a fourth family of branches with a state-machine flavor would require additional test fixtures that are recovery-domain-specific. Isolation in a sibling module keeps each projector's test surface crisp.

**EPR W2D inherits the call site, not the projector.** Whichever projector handles `key-revocation:*` governance-action signals, EPR W2D's `key_revocations` table writer can mirror its pattern. If sibling, EPR W2D either gets its own sibling projector (most likely — `key_revocations` is a different projection grain from recovery flow state) or is co-located in `RecoveryFlowProjector`.

### Recommendation

**B — Sibling RecoveryFlowProjector.**

File: `elohim/elohim-storage/src/services/recovery_flow_projector.rs`

The AttestationProjector is an accumulator — it sinks events into general-purpose tables without caring about recovery-flow lifecycle. A `RecoveryFlowProjector` is a controller — it tracks the lifecycle of each recovery or revocation flow as a state machine: `Open → Quorum → Effective`. These are architecturally distinct dispatch grains. Packing the state-machine logic into `handle_content_signal`'s already-branchy match would create a function that is simultaneously an accumulator and a controller, which violates the principle at `elohim/elohim-storage/src/services/attestation_projector.rs:9` (the module doc says "projects attestation + governance-action Content entries" — not "manages recovery flow lifecycle").

The call site (the central signal handler that currently dispatches into `handle_content_signal`) gains a prefix-routing step:
- `attestation:*` | `governance-action:*` (non-recovery) → `AttestationProjector::handle_content_signal`
- `recovery-request:*` | `governance-action:recovery-request` | `governance-action:key-revocation` | `key-revocation:*` | `identity-freeze` → `RecoveryFlowProjector::handle_content_signal`

Note that `attestation:revocation-vote` and `attestation:recovery-approval` continue to be handled by AttestationProjector (they are vote-children that land in the `attestations` accumulator table) — only the governance-action openers and the key-revocation effective events go to the sibling.

The EPR W2D `key_revocations` table writer is co-located in `RecoveryFlowProjector` since it projects `governance-action:key-revocation` signals — the same signal family. This keeps all key-revocation state in one module.

### Cross-sprint impact

EPR W2D (Wave 1 of the EPR sprint) wires its `key_revocations` projection handler by importing from `recovery_flow_projector.rs`, not from `attestation_projector.rs`. The EPR sprint's Wave 2 IntegrityNotify consumer for `RevocationAttestation` reads from the `key_revocations` table, which is written by `RecoveryFlowProjector`. The seam is clean: EPR does not need to modify `AttestationProjector` at all.

---

## D2 — RevocationAttestation shape (binds EPR D3)

### Question

`RevocationAttestation` is both a DNA-signal contract (`elohim/sdk/schemas/v1/dna-signals/revocation-attestation.schema.json`) and an attestation subtype in the consolidation pattern (`attestation:revocation-*` via `AttestationProjector`). When the DNA post-commit hook emits this signal, does the payload (a) **carry the full Content envelope inline** so AttestationProjector can process it on arrival (consolidation-envelope), or (b) **remain a first-class operational payload** where the envelope is referenced by its `actionHash` and the consumer fetches it separately (duality)?

### Alternatives

**A — Consolidation-envelope:** The `RevocationAttestation` signal payload is extended with an additional field (e.g., `contentEnvelope`) carrying the serialized `Content` entry. `AttestationProjector::handle_content_signal` can be invoked directly from the signal receiver — no separate DHT fetch required. Signal becomes self-contained.

**B — Duality (existing contract wins):** The `revocation-attestation.schema.json` contract stands as-is. The payload carries `actionHash` + `revocationId` + attestation progress state. The AttestationProjector and/or IntegrityNotify pipeline fetch the `Content` envelope via the existing `actionHash` → DHT lookup path when they need the full envelope fields.

### Tradeoffs

**Wire payload size.** Option A sends the full Content envelope in the signal on every revocation vote — that is a governance-action body (potentially kilobytes of threshold configuration, eligibility predicates, and metadata) repeated for each vote event. A revocation with 5 steward votes sends the envelope 5 times. Option B keeps the signal slim (20–30 fields, all scalar or short strings) and lets consumers that need the full envelope do one DHT lookup per unique `actionHash` — not per vote.

**Idempotency for the IntegrityNotify pipeline.** The EPR W2B consumer processes `RevocationAttestation` signals to track vote progress and trigger the compromise-window sweep (`derive_compromise_at` in W2D). With Option B, the consumer operates on the slim, already-complete `RevocationAttestation` payload (it has `currentVotes`, `requiredVotes`, `thresholdReached`, `attestedAt`) — no envelope needed for that logic. The `key_revocations` projection writer only needs to join with the `governance_actions` table for threshold configuration, which it already has from the `governance-action:key-revocation` signal that preceded the votes.

**AttestationProjector reuse.** With Option B, `attestation:revocation-vote` entries (the vote-children) are already routed to `AttestationProjector::handle_content_signal` and land in the `attestations` table at `elohim/elohim-storage/src/services/attestation_projector.rs:33`. The `RevocationAttestation` DNA signal (which describes the in-progress vote-collection state, not a single attestation entry) is a separate Category C operational signal — it does not need to go through `AttestationProjector` at all. It goes through `RecoveryFlowProjector` (D1 decision). This distinction is critical: the `revocation-attestation.schema.json` contract already says "Category C — operational projection of revocation vote-collection state" — it is NOT an attestation envelope signal; it is a progress update. Trying to make it carry a Content envelope would misuse it as a Category A signal.

**The existing schema is already the right shape.** `revocation-attestation.schema.json` was written with all the fields the IntegrityNotify pipeline needs: `thresholdReached`, `currentVotes`, `requiredVotes`, `attestationKind` (`request` vs `vote`). There is nothing missing that would force a consumer to go back to the DHT. Adding a `contentEnvelope` field would bloat every vote signal and require changing an already-complete, published contract.

**Schema stability.** Option B requires zero schema changes. Option A changes a published contract that the EPR sprint's W2B already references. Changing the contract mid-flight between the two sprints requires coordinated PR timing — avoidable complexity.

### Recommendation

**B — Duality (existing contract wins).**

The `revocation-attestation.schema.json` at `elohim/sdk/schemas/v1/dna-signals/revocation-attestation.schema.json` is a correct, complete, and appropriately Category C operational signal. Its `actionHash` field provides the provenance link back to the DHT for consumers that need it; its progress-state fields (`currentVotes`, `requiredVotes`, `thresholdReached`) give the IntegrityNotify pipeline everything it needs to operate without a round-trip fetch. Inlining the Content envelope would transform a slim progress signal into a fat per-vote payload, break the signal's Category C classification (it is not a notarized envelope — it is a derived state projection), and require changes to a schema that the EPR sprint is already treating as settled.

The producer (M4 Stage 3) emits against the existing `revocation-attestation.schema.json`. The consumer (EPR W2B) reads from it directly. When the consumer needs Content-envelope fields (e.g., threshold configuration for governance validation), it reads from the local `governance_actions` projection table — which was already written by the earlier `governance-action:key-revocation` signal. No additional DHT fetch is required at vote-processing time.

### EPR D3 commitment

**EPR D3 is committed to duality (existing `revocation-attestation.schema.json` contract).** The EPR sprint's IntegrityNotify pipeline (Wave 2) handles `RevocationAttestation` by reading the existing slim signal payload. It does NOT expect a `contentEnvelope` field. The `key_revocations` table writer (Wave 1, W2D) projects from the existing `actionHash` + progress-state fields. No cross-sprint schema coordination is needed beyond this document — the existing schemas are the contract.

---

## D3 — Branch retirement vs. salvage

### Question

The `feature/recovery-m4-fast-path-revocation` branch is described as "916 commits behind dev." Does it contain unique commits worth salvaging, or should it be retired outright?

### Alternatives

**A — Retire outright:** Delete both local and remote branches. Treat the branch as historical record only. Any still-relevant work has already been re-expressed in the consolidated pattern.

**B — Salvage selected commits:** Cherry-pick specific commits by SHA into dev directly.

### Tradeoffs

**The branch has zero commits unique to it relative to dev.** Running `git log feature/recovery-m4-fast-path-revocation ^dev` produces no output — the merge-base between the branch and dev is the branch's own tip (`5fa8d621f`). This means dev is a strict superset of every commit that was ever on the m4 branch. The branch is not "916 commits behind" in the sense of having diverged work to recover — it is 916 commits behind in the sense that dev has grown far past the branch's last commit, which is now reachable from dev's history.

**The specific commits of value are already on dev.** The five unique-looking commits visible at the branch tip — `5fa8d621f` (rustfmt), `7d0368ca7` (a2o emergency-contact quorum), `16cd4d4e0` (a2o self-revocation), `aad574206` (sweettest scenarios), `e5ba12f49` (storage signals polish) — are reachable from dev. Confirming this: `genesis/a2o/features/auth/recovery/revocation-emergency-quorum.feature` and `revocation-self.feature` both exist on the dev working tree, which would only be true if the commits adding them are already in dev's ancestry.

**The a2o scenarios tagged `@recovery-m4` are the highest-value content.** They are already on dev. The sweettest scenarios (`aad574206`) are pre-consolidation and target the legacy `RecoveryRequestEntryTypes` / `KeyRevocationEntryTypes` — they will need to be rewritten as part of this sprint's Stage 2 work anyway. Retaining the branch to cherry-pick stale sweettest code would create more confusion than value.

**Cost of retention is noise.** Keeping the branch alive means future `git log` and `git branch` output will show it as a live branch even though it has no unique content. This violates the principle of clarity in the repository's branch topology.

### Recommendation

**A — Retire outright.**

The `feature/recovery-m4-fast-path-revocation` branch is a strict ancestor of dev with zero unique commits. Everything valuable from M4 was incorporated into dev during the attestation consolidation sprint (A→G) or earlier merges. The branch should be deleted to keep the branch list readable.

### Proposed git command (do not execute)

```bash
# Delete local branch
git branch -d feature/recovery-m4-fast-path-revocation

# Delete remote branch
git push origin --delete feature/recovery-m4-fast-path-revocation
```

The `-d` (not `-D`) flag is used deliberately. Since the merge-base of the branch and dev is the branch tip, git will confirm the branch is fully merged before allowing the delete — an automated safety check. If git refuses (which would indicate an incorrect analysis), stop and investigate before using `-D`.

---

## D4 — Shamir custodian discovery

### Question

When a recovery agent needs to contact its Shamir share custodians, how does it discover which peers those custodians are? Two candidate mechanisms: (a) **manifest-declared** — the recovery flow document records custodian identities at setup time; or (b) **peer-advertised** — peers broadcast a `share-custody` capability via libp2p and the recovery agent queries the live capability set.

### Alternatives

**A — Manifest-declared:** At Shamir setup time, the recovery flow commits a `governance-action:recovery-request` Content entry on the DHT that carries the custodian CIDs (or a commitment hash over them) in its metadata. At recovery time, the agent reads this Content entry from the DHT, derives the custodian list, and dials them via their known peer multiaddrs (resolved through Kademlia or the `peer_identity_bindings` table). Discovery is deterministic, auditable, and works offline from the capability advertisement layer.

**B — Peer-advertised:** Custodians advertise a `share-custody/<recovery-id>` capability via libp2p identify protocol or a gossipsub topic. At recovery time, the agent queries connected peers for those matching the capability. This requires custodians to be online and advertising at discovery time.

### Tradeoffs

**Substrate floor must be deterministic without elohim.** The memory `project_substrate_floor_elohim_ceiling` establishes that the substrate must produce deterministic outcomes without AI involvement. Peer-advertisement at recovery time introduces a dependency on live peer connectivity that the substrate cannot satisfy deterministically — if the custodian's node is asleep or temporarily offline, the capability advertisement is absent even though the custodian exists and has agreed to participate. A manifest-declared custodian list on the DHT is always available to any peer that can reach the DHT, regardless of whether the custodian's libp2p node is currently connected.

**Grandma standard: recovery must not fail because a custodian's node is offline.** The memory `project_recovery_grandma_standard` is explicit: "errors, edge cases, and crisis scenarios must be handled so the user is never left holding the complexity." If custodian discovery requires the custodian to be advertising on libp2p at the exact moment of recovery, a custodian who is asleep when their family member needs recovery causes a failure the user cannot understand or resolve. With manifest-declared discovery, the substrate knows the custodian's identity from the DHT; the libp2p transport merely attempts to dial them (with appropriate retry/timeout). The transport failure is a recoverable operational condition, not a discovery failure.

**Household horizontal scaling.** The memory `project_household_horizontal_scaling` describes a model where different blades carry different roles. A custodian's `share-custody` role is configured at setup time (when the human chose their emergency contacts), not dynamically. Manifest-declared custodians map naturally to this: setup-time role assignment, DHT-durable, operator-managed. Peer-advertisement would require each blade to know whether it is currently a share custodian and to actively advertise — adding operational complexity that benefits no one.

**`ShamirShareRequest` already carries the custodian identity.** The wire message at `elohim/elohim-storage/src/p2p/shamir_transport.rs:75` includes `custodian_cid` — the recovery agent already knows who it is dialing. This is only coherent if the custodian list is known at dial-time from a durable source (manifest-declared), not discovered dynamically. The protocol is already implicitly committed to manifest-declared custodians.

**Stage 4a swarm wiring is simpler with manifest-declared.** The `TODO(G.1-swarm-wiring)` at `shamir_transport.rs:20` says to follow the `trust_protocol` pattern at `behaviour.rs:88` and `mod.rs:2292`. `trust_protocol` is a per-connection request-response protocol — it dials a specific peer by PeerId. Manifest-declared custodians give Stage 4a a concrete PeerId to dial (derived from the custodian's DHT-registered identity). Peer-advertised discovery would require a gossipsub scan before dialing, adding a Wave-0 scan layer that is not modeled anywhere in `ShamirShareRequest`'s wire contract.

**Elohim adds discernment on top of the substrate's deterministic list.** The memory `project_elohim_as_counsel` describes elohim-as-defender operating at machine speed during attacks. Under the manifest-declared model, the elohim defender can read the DHT custody manifest, verify which custodians have issued `attestation:recovery-approval` entries, and prioritize dialing custodians who are already online (from `peer_identity_bindings` presence data) — enriching the substrate's deterministic list without replacing it. This is the substrate-floor / elohim-ceiling separation applied correctly.

### Recommendation

**A — Manifest-declared.**

Custodian identities are committed to the DHT at recovery-setup time as part of the `governance-action:recovery-request` metadata (or a dedicated `governance-action:shamir-custody-setup` entry if the custody setup is a separate ceremony from the recovery-request opener — that sub-question can be resolved in Stage 4b). At recovery time, the substrate reads the DHT manifest, derives the custodian CID list, resolves each CID to a PeerId via `peer_identity_bindings`, and dials them via the `ShamirShareCodec` request-response protocol. The dial may fail if the custodian is offline — the substrate retries within the recovery-window duration; elohim can suggest fallback custodians from the graduated authority stack if the primary custodians remain unreachable.

### Stage 4a swarm-wiring implication

Stage 4a adds `shamir_share_protocol: RequestResponse<ShamirShareCodec>` to `ElohimStorageBehaviour` at `elohim/elohim-storage/src/p2p/behaviour.rs`, following the exact pattern of `trust_protocol` at line 87. The corresponding `From` impl for `ElohimStorageBehaviourEvent` maps `request_response::Event<ShamirShareRequest, ShamirShareResponse>` to a new `ShamirShareProtocol(...)` variant (mirroring `TrustProtocol` at lines 135–139). The swarm event loop match arm in `p2p/mod.rs` dials custodians by PeerId derived from the manifest-declared custodian CID list — it does NOT scan gossipsub for capability advertisements. The `ShamirShareRequest::custodian_cid` field confirms the identity of the peer being dialed (replay-prevention), not a discovery mechanism.

---

## Summary table

| Decision | Call | Cross-sprint impact |
|---|---|---|
| D1 | Sibling `RecoveryFlowProjector` at `elohim/elohim-storage/src/services/recovery_flow_projector.rs` | EPR W2D `key_revocations` table writer is co-located in the sibling, not in `AttestationProjector`; EPR W2B reads from `key_revocations` table written by this module |
| D2 | Duality — existing `revocation-attestation.schema.json` contract stands; no Content envelope inlined | EPR D3 = duality confirmed; IntegrityNotify pipeline reads slim operational payload; no schema changes required for either sprint |
| D3 | Retire `feature/recovery-m4-fast-path-revocation` outright (zero unique commits; branch is a strict ancestor of dev) | n/a |
| D4 | Manifest-declared custodian discovery; custodian CIDs on DHT at setup time | Stage 4a wiring = dial specific PeerId from manifest-declared list via `ShamirShareCodec`; no gossipsub capability scan; `ElohimStorageBehaviour` gains `shamir_share_protocol` field at `behaviour.rs` following `trust_protocol` pattern at line 87 |

---

## Open questions for operator

**D4 sub-question — custody setup ceremony:** Is custodian identity committed as part of the `governance-action:recovery-request` entry opened at recovery time, or as a separate `governance-action:shamir-custody-setup` entry committed at onboarding/setup time? The manifest-declared decision (D4) assumes setup-time commitment, which is the correct shape, but which governance-action kind carries it is unresolved from in-scope inputs. Stage 4b (`ShareAssembler`) will need to read the custody manifest — that is the natural moment to resolve this sub-question. Recommendation: treat it as a Stage 4b pre-condition that is answered during the Stage 1 gate-reader audit when the imagodei zome's recovery-request creator is being examined.

**D1 sub-question — call site ownership:** The central signal dispatcher that currently calls `attestation_projector::handle_content_signal` (its location in the signal-reception path is not fully visible from these inputs — it is likely in the HTTP/WebSocket signal handler or a service orchestrator) needs to gain the prefix-routing step that routes recovery signals to `RecoveryFlowProjector`. Confirming the exact call site file and line is a Stage 2 pre-condition, not a brainstorm blocker.
