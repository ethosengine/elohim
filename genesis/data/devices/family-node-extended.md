---
id: "device-family-node-extended"
displayName: "Family Node Extended"
formFactor: "rack-module"
capabilityLevel: 5
stage: 4
memoryGb: 128
storageGb: 20000
storageType: "nvme"
cpuCores: 24
cpuClass: "amd-threadripper-7960x"
gpu: "rtx-4090"
sensors: []
battery: false
powerWatts: 350
alwaysOn: true
natType: "public"
bandwidthDownMbps: 2000
bandwidthUpMbps: 1000
latencyMs: 3
canSteward: true
canInfer: true
canDoorway: true
streamsTo: null
serviceability: "full"
healthSurfaces: ["smart", "thermal", "power", "usb-enumeration", "battery-health", "memory-ecc", "fan-rpm"]
circularity: "modular-upgradeable"
degradationMode: "modular"
replacementLeadTime: "days"
expectedLifespanYears: 12
attestationCapabilities: []
---

# Family Node Extended

Multi-generational scale. The node that handles not just one household
but an extended family network, a small congregation, or a neighborhood
cooperative — dozens of agents, terabytes of memory, years of context.

128GB of DDR5 ECC RAM means the elohim agent holds long context without
thrashing. The RTX 4090 runs 70B parameter models at interactive speeds
alongside replication, inference queues from the surrounding smaller
nodes, and real-time governance event processing. 20TB of NVMe primary
storage holds the active working set; bulk archival lives on attached
spinning drives. At 350W it's a serious power draw — the protocol tracks
this as an REA economic event, balancing electricity cost against the
storage and compute value contributed to the network.

All seven health surfaces fire continuously. SMART on every drive, ECC
corrected errors tracked per DIMM, thermal on every zone, power draw at
the PSU, USB enumeration to catch failing hubs, fan RPM across six
cooling channels. When a drive begins failing, the node initiates
replication promotion on its shard of the DHT before the human even
knows there's a problem. Parts arrive. The human schedules a service
window. The protocol never lost a byte.

Twelve years is achievable because modular upgrades aren't optional
extras — they're the design. GPU, RAM, storage, and cooling modules all
hot-swap or tool-free swap. The agent key persists across every hardware
generation.
