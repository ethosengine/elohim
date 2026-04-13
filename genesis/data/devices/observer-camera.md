---
id: "device-observer-camera"
displayName: "Observer Camera"
formFactor: "iot-sensor"
capabilityLevel: 0
stage: 4
memoryGb: 0.5
storageGb: 8
storageType: "flash"
cpuCores: 2
cpuClass: "arm-cortex-a53"
gpu: null
sensors: ["camera", "infrared"]
battery: false
powerWatts: 3
alwaysOn: true
natType: "offline-first"
bandwidthDownMbps: 20
bandwidthUpMbps: 10
latencyMs: 20
canSteward: false
canInfer: false
canDoorway: false
streamsTo: "nearest-elohim-node"
serviceability: "none"
healthSurfaces: []
circularity: "recyclable"
degradationMode: "cliff"
replacementLeadTime: "days"
expectedLifespanYears: 5
attestationCapabilities: ["facial-presence"]
---

# Observer Camera

The simplest visual witness. A single purpose: notice who is present
and stream that observation to the family node.

0.5GB RAM, 8GB of flash for local buffering, dual-core ARM running edge
inference for facial-presence detection — not identification, presence.
The infrared channel means it works in the dark, at the door, in the
meeting room at 11pm when the governance vote closes. Powered off mains
at 3W; no battery, no pretense of portability.

Everything it observes streams to the nearest elohim-capable node, which
does the notarization. The camera itself holds no persistent records and
has no agent key of its own — it's a peripheral to the node's agent
identity, not an independent participant. Sealed optics, weatherproof
for outdoor entryway installation, no user-serviceable parts.

Five years before the image sensor's dynamic range degrades past
usefulness. Cliff degradation: the image quality drops suddenly rather
than fading gracefully, because silicon doesn't negotiate. When it fails,
the unit is replaced and the old one recycled. The attestation record
it contributed continues to live in the DHT, signed by the node that
notarized it.
