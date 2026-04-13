---
id: "device-gaming-desktop"
displayName: "Gaming Desktop"
formFactor: "desktop"
capabilityLevel: 4
stage: 3
memoryGb: 32
storageGb: 1000
storageType: "nvme"
cpuCores: 8
cpuClass: "amd-ryzen-7-7700x"
gpu: "rtx-4070"
sensors: []
battery: false
powerWatts: null
alwaysOn: false
natType: "public"
bandwidthDownMbps: 500
bandwidthUpMbps: 100
latencyMs: 10
canSteward: true
canInfer: true
canDoorway: false
streamsTo: null
serviceability: "modular"
healthSurfaces: ["smart", "thermal", "fan-rpm"]
circularity: "modular-upgradeable"
degradationMode: "modular"
replacementLeadTime: "days"
expectedLifespanYears: 7
attestationCapabilities: []
---

# Gaming Desktop

It exists to play games. The protocol is a tenant, not the landlord.

When the owner boots a session, stewardship pauses — gracefully, without
data loss, no half-written entries left dangling. When the game ends and
the owner wanders off to bed, the RTX 4070 turns its attention to
inference: embedding generation, local model runs, context compression
for the household elohim agent. The CPU handles DHT participation during
the hours the GPU is busy rendering.

32GB of DDR5, an 8-core Ryzen 7, 1TB NVMe — this is more compute than
most universities had a decade ago, sitting in a teenager's bedroom,
contributing to the network eight hours a night. SMART, thermal, and
fan-rpm monitoring catch problems before they become failures; modular
construction means a failed GPU doesn't retire the node.

The tension is real: inference windows are unpredictable. When the owner
decides to play until 3am, the network loses its most powerful local
inferencer for the night. The protocol schedules around human rhythms,
not the other way around. Availability signals let other nodes plan
accordingly.
