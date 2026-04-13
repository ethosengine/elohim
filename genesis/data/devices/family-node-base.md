---
id: "device-family-node-base"
displayName: "Family Node (Base)"
formFactor: "rack-module"
capabilityLevel: 5
stage: 4
memoryGb: 64
storageGb: 12000
storageType: "nvme"
cpuCores: 16
cpuClass: "intel-i7-13700k"
gpu: "rtx-4070"
sensors: []
battery: false
powerWatts: 200
alwaysOn: true
natType: "public"
bandwidthDownMbps: 1000
bandwidthUpMbps: 500
latencyMs: 5
canSteward: true
canInfer: true
canDoorway: true
streamsTo: null
serviceability: "full"
healthSurfaces: ["smart", "thermal", "power", "usb-enumeration", "memory-ecc", "fan-rpm"]
circularity: "modular-upgradeable"
degradationMode: "modular"
replacementLeadTime: "days"
expectedLifespanYears: 10
attestationCapabilities: []
---

# Family Node (Base)

The Tier 3 heart of the ecosystem. The oak tree.

64GB DDR5 RAM, 16-core CPU, RTX 4070 class GPU, 2TB NVMe primary plus
10TB bulk RAID. Always-on, under 200W, whisper-quiet. Runs the full
stack: storage, conductor, P2P sync, replication, AI inference (70B
parameter model), and public doorway.

Hosts the family elohim agent, custodial keys for less-technical
relatives, serves as geographic redundancy point for the trust network.
This is the device that replaces cloud subscriptions with something
you own.

Serviceability: full. Hot-swappable modules, tool-free maintenance. The
node monitors its own health — SMART on every drive, thermal sensors,
power draw, USB enumeration, ECC memory. When a drive shows increasing
reallocated sectors, the node shifts replication priority to other
stewards, orders the replacement part, and notifies the family:
"Storage module needs replacing. Compatible drive shipped. Service
window: Tuesday afternoon."

The human's only decision is scheduling. The protocol handles urgency,
replication safety, and parts sourcing.
