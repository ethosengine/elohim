---
id: "device-dedicated-server"
displayName: "Dedicated Server"
formFactor: "server"
capabilityLevel: 5
stage: 4
memoryGb: 64
storageGb: 8000
storageType: "hdd"
cpuCores: 16
cpuClass: "intel-xeon-e2388g"
gpu: null
sensors: []
battery: false
powerWatts: 400
alwaysOn: true
natType: "public"
bandwidthDownMbps: 1000
bandwidthUpMbps: 1000
latencyMs: 5
canSteward: true
canInfer: true
canDoorway: true
streamsTo: null
serviceability: "full"
healthSurfaces: ["smart", "thermal", "power", "fan-rpm"]
circularity: "repairable"
degradationMode: "graceful"
replacementLeadTime: "days"
expectedLifespanYears: 10
attestationCapabilities: []
---

# Dedicated Server

The church server room. The school's IT closet. The cooperative's
basement rack. Rack-mounted, dual-PSU if they planned ahead, on a UPS
if they're serious.

16-core Xeon, 64GB ECC RAM, 8TB of spinning HDD in a RAID — enough to
hold a substantial community's working set with plenty of redundancy.
At 400W it's the most power-hungry node most communities will ever run,
but it serves dozens of households and pays for itself in displaced
cloud subscriptions. Symmetric gigabit means it's not the bottleneck for
anyone in its orbit.

No GPU. CPU inference at this core count is viable for 7B and 13B models
— not fast, but continuous. The Xeon handles inference queues, replication,
doorway traffic, and DHT participation across all 16 cores without
saturation. If the community grows into heavier inference needs, the PCIe
slot is waiting.

SMART on every HDD, thermal, power draw, and fan-rpm. Full serviceability
means drives, RAM, and PSUs are hot-swap in a properly configured rack
enclosure. Graceful degradation: as HDDs age, capacity is voluntarily
shed from the stewardship commitment before any data is at risk. Ten
years is realistic for Xeon-class hardware with professional maintenance.
