---
name: EPR Phase 2B and Recovery M4 converge on the DNA signal stream
description: Cross-epic coordination point — both epics write to `dna-signal-stream.schema.json` as the shared coordination surface
type: project
originSessionId: 8c8c7e97-f63b-4df5-ae26-36e0fb18bcf7
---
EPR Phase 2B Batch A and Recovery M4 (fast-path revocation) both implement different sides of the same pipe: imagodei → elohim-storage DNA signal stream carrying `KeyRotation`, `KeyRevocation`, `AgentPeerBinding`, `RevocationAttestation` events. M4 is the producer side (DNA emits signals on recovery events); 2B Batch A is the consumer side (elohim-storage `ReconcileController` subscribes and sweeps caches/projections).

**Why:** Discovered during the 2B brainstorm on 2026-04-24. The user's recovery epic (resiliency arc) produces attestations that must flow through substrate; 2B's projector is where pillars see those events; the DNA signal stream is the connective tissue. Either epic can land first — whichever does, the other consumes its contract.

**How to apply:** Any work on either branch must keep the schema at `elohim/sdk/schemas/v1/dna-signal-stream.schema.json` (+ `dna-signals/*.schema.json` sub-schemas) as the coordination surface. When reviewing PRs on `feature/recovery-m4-fast-path-revocation` or `feature/epr-phase-2b-*` branches, check whether the changes touch the stream schema — if yes, cross-reference with the other epic's expectations. Open question O4 (stream cursor/durability under subscriber restart) is a shared design concern to resolve jointly. The affected-peer list contract for direct-notify on revocation (Batch D task D.5) is another shared artifact — M4 produces the list, D.5 consumes.

---

## 2026-05-15 update — DNA signals graduate to EPR envelopes (M4 T18 decision)

The signal payload is no longer "schema-shaped notification with notary pointer." It is an EPR envelope — a cross-stack provenance record. Decision driven by the framing that the protocol's capture-resistance comes from ubiquitous wisdom-layer mediation at every node, not from a single chokepoint notary; the wire format must reflect that.

**New signal shape (M4 T18 produces, EPR W2B consumes):**
```
DnaSignal::KeyRevocation {
  attestationKind: "attestation:key-revocation-emit",
  subjectCid:     "<cid of governance-action:key-revocation Content entry>",
  issuer:         "<authoring agent pubkey>",
  issuedAt:       "<RFC3339>",
  signature:      "<signature over canonical envelope bytes>",
  metadata: {
    revocationId:          "<stable logical key — DEDUP KEY for consumers>",
    revokedPubkey:         "<base64 32-byte>",
    agentCid:              "<stage-1: human_id>",
    compromiseAt:          "<RFC3339>",
    effectiveAt:           "<RFC3339>",
    triggeringRevocationId: "<id|null>",
    supersedesCid:         "<cid|null — HONOR for lineage>",
  },
  relayChain: []   // forward-compat: relay-elohim provenance accumulates here in future
}
```

**Three-step verification model (consumer-side contract):**
1. **Shape** — schema-validate against `dna-signal-stream.schema.json` (offline, deterministic, anyone with the schema can do it).
2. **Provenance** — verify `signature` over canonical envelope bytes (offline, cryptographic, anyone with `issuer` pubkey can do it).
3. **Authority + currentness** — consult notary via storage projection at `subjectCid` (only step requiring live infra).

**Migration:** `RecoveryV2Signal::KeyRevocationEffective` becomes `#[deprecated]` for one release cycle; new consumers read `DnaSignal::KeyRevocation`. Outer-tag collision (`type` + `signalType`) dissolves because the new enum has no outer `#[serde(tag = "type")]`.

**Deferred to follow-up backlog (M4 T18 does NOT migrate these — same pattern applies later):**
- `KeyRotationCommitted` — needs producer-site work to align with envelope shape.
- `RevocationVoteSubmitted` — needs producer-site work; my T7 IntegrityNotify arm consumes the OLD slim shape (`revocation-attestation.schema.json`, 11 flat fields, not envelope) — that stays valid because T7 is a separate signal class (vote-aggregation, not effective-revocation), wire format unaffected by T18 decision.
- `AgentPeerBindingCreated` — deferred entirely until iroh Phase 12 (see [[project-w2-agent-peer-binding-deferred]]).
- Relay-elohim provenance accumulation (`relayChain` population) — wire field exists, mechanism is future work.

**EPR W2B (my T5/T6) consumer contract — what I do when M4 lands:**
1. Read `dna-signal-stream.schema.json` (M4 owns).
2. Add `key-revocation:effective` arm as a branch INSIDE M4's `handle_content_signal` (sibling-projector per M4 D1; not a separate module).
3. Dedup on `metadata.revocationId`.
4. Honor `metadata.supersedesCid` for lineage in `key_revocations` projection.
5. Parse `relayChain` (ignore content for now, forward-compat).
6. Run all three verification steps before applying state transition.

**Spec doc (load-bearing for handoff):** `genesis/docs/superpowers/specs/2026-05-15-dna-signal-as-epr-envelope.md` — M4 T18 owns drafting this; my consumer reads it as the contract.

**Why this is the right architecture (capture the generational frame):** The wire carries propagation provenance, not just substrate addresses. The notary's role narrows to coordination registrar (sequence/lineage/effectiveness timing). Wisdom-layer enforcement (author/relay/consume gating) distributes to every node via the elohim at each hop. The substrate floor without the notary is still policy-enforced because elohims at each node gate authoring + reach + consumption — to form a dark web you'd have to remove the elohim layer entirely, which means leaving the protocol. Capture-resistance lives in ubiquitous wisdom, not in a chokepoint. The EPR envelope is the protocol's native primitive for this; treating signals as EPR envelopes aligns the wire with the primitive rather than inventing a parallel "signal schema." Related: [[project-elohim-agent-sense-respond-architecture]], [[project-three-layer-truth-model]], [[project-trust-as-efficiency-signal]].
