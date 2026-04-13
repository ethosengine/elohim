---
id: "device-home-nuc"
displayName: "Home NUC"
formFactor: "mini-pc"
capabilityLevel: 4
stage: 4
memoryGb: 16
storageGb: 1000
storageType: "nvme"
cpuCores: 4
cpuClass: "intel-i7-1260p"
gpu: null
sensors: []
battery: false
powerWatts: 25
alwaysOn: true
natType: "public"
bandwidthDownMbps: 500
bandwidthUpMbps: 100
latencyMs: 8
canSteward: true
canInfer: true
canDoorway: false
streamsTo: null
serviceability: "consumer"
healthSurfaces: ["smart", "thermal", "fan-rpm"]
circularity: "repairable"
degradationMode: "graceful"
replacementLeadTime: "days"
expectedLifespanYears: 8
attestationCapabilities: []
---

# Home NUC

Silent, invisible, always-on. The device that hides behind the router
and runs for years without anyone looking at it.

16GB RAM and a 12th-gen P-series Intel chip can run small models via CPU
inference — not the 70B weights of the family node extended, but 7B
models fit comfortably, fast enough for summarization, classification,
and context compression. At 25W it costs less than a dollar a month to
run. The form factor fits in a drawer. It never runs hot enough to be
audible.

Public NAT means peers can reach it directly — no relay overhead, no
waiting for hole-punching. The NUC serves as the always-reachable anchor
for the household's mobile devices, syncing while the phones sleep,
completing replication windows while the laptop is in someone's bag. SMART
monitoring watches the NVMe; thermal and fan-rpm signals catch the single
cooling fan before it fails entirely.

Eight years is the realistic horizon for a well-cooled NUC. Consumer
serviceability means RAM and storage are accessible without voiding anything;
the fan is the only wear item that typically needs attention. Graceful
degradation: as the hardware ages, inference is shed first, then stewardship
scope narrows to local household data only, before the node fully retires.
