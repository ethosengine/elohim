---
name: Compute commitments are bounded REA primitives
description: Compute is a low-level network primitive expressed as Commitment with signal_kind "compute-allocation"; three trigger_kinds; breach in compute-class never contaminates attribution-class flows.
type: project
originSessionId: 195ee79b-20ed-438e-8388-af439b3a42a7
---
Compute is a first-class economic primitive in the protocol, expressed as an extension of the existing REA `Commitment` entry via `signal_kind: "compute-allocation"`. Breach is recorded as `signal_kind: "compute-breach"`. Sketched in `docs/superpowers/specs/2026-05-04-compute-commitment-substrate-floor-design.md`.

**Two contract families with structurally separated breach contagion:**

- **Attribution-class** (authorship, citation, witness, learning-credit, recognition, vouch) — never breaches when compute fails. Records of what happened, not promises of what will happen. Continue regardless of liveness.
- **Compute-class** (reciprocal hosting, validator participation, inference, shard storage, gossip relay, scheduled tasks) — breach is first-class. Catastrophic compute loss breaches forward-looking compute commitments only; the contributor's authored work, citations, recognition, and standing all remain.

**Three trigger_kinds the substrate floor must execute:**

- `request-driven` — schedule on incoming peer request, k8s-style
- `standing` — fire deterministically when conditions trigger (cron / threshold / gossip-rule); runs on substrate after elohim authors
- `subscription` — reserve capacity in agreed window for specific counterparty

**Why:** The 2026-05-04 shem outage exposed that we had no protocol-layer concept for compute as a bounded economic object. Operational suspension worked (k8s scaled to 0), but the protocol couldn't distinguish "your hardware died" from "you stopped existing." The harvest captures that compute commitments must be bounded, breach-aware, and structurally separate from attribution flows. Real-network family nodes will be 64-128 GiB / GPU-capable, but they will still commit bounded scopes — the alpha cluster's scarcity is a transient teacher, not the design target.

**How to apply:**

- Any new request-response protocol surface that consumes resources should be expressed as a compute-class commitment (or extend the schema if a new resource dimension is needed).
- A contributor's compute breach (especially catastrophic loss like shem) MUST NOT cascade into their attribution-class flows. If you find yourself writing `if (humanIsOffline) skip(humanReferences)`, you've coupled the layers — re-think.
- The capacity ledger at `genesis/data/rakia/compute-capacity.json` is the authoritative input to negotiated bounds; future implementations cite a versioned ledger CID as `balance_basis` on the commitment.
- Pairs with `project_substrate_floor_elohim_ceiling.md`: substrate negotiates allocations deterministically; elohim provides discernment + value-minting on top.
- Pairs with `project_signal_kind_extensible_protocol_class.md`: don't introduce a new entry type for this; extend Commitment via signal_kind.
- Pairs with `project_depin_contracts_are_policy.md`: DHT records the commitment policy; libp2p handles distribution/availability mechanism within bounds.
