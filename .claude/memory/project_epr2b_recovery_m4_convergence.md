---
name: EPR Phase 2B and Recovery M4 converge on the DNA signal stream
description: Cross-epic coordination point — both epics write to `dna-signal-stream.schema.json` as the shared coordination surface
type: project
originSessionId: 8c8c7e97-f63b-4df5-ae26-36e0fb18bcf7
---
EPR Phase 2B Batch A and Recovery M4 (fast-path revocation) both implement different sides of the same pipe: imagodei → elohim-storage DNA signal stream carrying `KeyRotation`, `KeyRevocation`, `AgentPeerBinding`, `RevocationAttestation` events. M4 is the producer side (DNA emits signals on recovery events); 2B Batch A is the consumer side (elohim-storage `ReconcileController` subscribes and sweeps caches/projections).

**Why:** Discovered during the 2B brainstorm on 2026-04-24. The user's recovery epic (resiliency arc) produces attestations that must flow through substrate; 2B's projector is where pillars see those events; the DNA signal stream is the connective tissue. Either epic can land first — whichever does, the other consumes its contract.

**How to apply:** Any work on either branch must keep the schema at `elohim/sdk/schemas/v1/dna-signal-stream.schema.json` (+ `dna-signals/*.schema.json` sub-schemas) as the coordination surface. When reviewing PRs on `feature/recovery-m4-fast-path-revocation` or `feature/epr-phase-2b-*` branches, check whether the changes touch the stream schema — if yes, cross-reference with the other epic's expectations. Open question O4 (stream cursor/durability under subscriber restart) is a shared design concern to resolve jointly. The affected-peer list contract for direct-notify on revocation (Batch D task D.5) is another shared artifact — M4 produces the list, D.5 consumes.
