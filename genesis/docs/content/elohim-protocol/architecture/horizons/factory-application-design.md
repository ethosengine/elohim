---
title: Factory-as-Collective — industrial supply chain on the substrate
tier: architecture
status: Horizon (coherent pattern, not on active subsumption path)
created: 2026-05-24
authors: Matthew Dowell + Opus 4.7
realizes:
  - genesis/docs/content/elohim-protocol/economic_coordination/epic.md (Beer's Cybersyn at industrial scale)
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-20-wave3-valueflows-hrea-interop-design.md (REA at industrial logistics scale; this is where humans already author REA)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-23-multi-collective-collaboration-epr-design.md (cross-collective coordination)
defers:
  - Active implementation — industrial trust networks must mature first; substrate-native factory operation depends on supplier/buyer/regulator counterparties also being on substrate
---

## Why this is a horizon, not active

Factories are conceptually the **easiest** subsumption target because industrial logistics is the one place humans already author REA at high-flow scale (ISO 9001, SCOR, GS1). The substrate fits naturally. But factories don't operate alone — they need suppliers, buyers, regulators, certification bodies all on the substrate too, OR robust bridges to each. The horizon is deferred not because the substrate isn't ready, but because the *cooperative network* of industrial counterparties isn't on substrate yet. Active subsumption of R&O (cooperative commerce) and AWS (peer-native compute) builds that network; factories follow once the counterparty mesh exists.

## Primitive composition (factory as a Collective EPR)

| What you see | Primitive | Notes |
|---|---|---|
| Factory | Collective EPR (`content_type: "organization"`) | participates natively, not via external integration |
| Machine | Resource (`resource_classified_as: "machinery"`) | child of Factory; depreciation Events accumulate over life |
| Employee | imagodei Human + Membership EPR | linked to Factory; role attestations |
| Raw materials inventory | Resource (`resource_classified_as: "raw-material"`) | child of Factory; balance derived from receive Events |
| Finished goods inventory | Resource (`resource_classified_as: "finished-good"`) | child of Factory; balance derived from transform Events |
| Production batch | Event (`action: "transform"`) | provider=raw-resources, receiver=finished-product; quantity tracked |
| Supply chain receive | Event (`action: "receive"`) | provider=supplier-EPR (cross-collective), receiver=factory |
| Customer shipment | Event (`action: "transfer"`) | provider=factory, receiver=buyer-EPR |
| Machine maintenance | Event (`action: "maintain"`) | with parent_epr_cid → machine |
| Sensor telemetry | Observation (`observation_kind: "factory:machine-state"`) | libp2p; graduates to maintenance-Events at thresholds |
| Quality check | Attestation (`content_type: "attestation:quality-check"`) | per-batch |
| ISO / regulatory audit | Attestation (`content_type: "attestation:regulatory-audit"`) | mishpat-DNA governance flow |

## Stress points the substrate handles

- **Multi-party coordination**: supplier + factory + buyer + regulator each have their own elohim-node; reach-coupling federates queries across the supply chain without centralizing the data
- **Industrial-stake provenance**: every Event is DHT-notarized; cryptographic proof of every transfer + transform survives any single party's failure
- **Sensor volume**: machine telemetry is high-frequency Observations on libp2p; only meaningful patterns (anomalies, maintenance thresholds, batch boundaries) graduate to DHT Events
- **Bridges to legacy ERPs**: SAP, Oracle, NetSuite become bridge crates authoring substrate-native Events on behalf of the factory until the cooperative-network is dense enough to operate fully native

## Why this is deferred

Active subsumption of cooperative commerce (R&O) + peer-native compute (AWS) builds the substrate-network of counterparties a factory would need. Once a factory's suppliers, buyers, and certification partners are routinely on substrate, the factory itself is the natural next move.
