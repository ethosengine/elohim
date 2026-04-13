---
id: "device-chromebook-edu"
displayName: "Education Chromebook"
formFactor: "laptop"
capabilityLevel: 2
stage: 3
memoryGb: 4
storageGb: 64
storageType: "emmc"
cpuCores: 4
cpuClass: "intel-celeron-n4020"
gpu: null
sensors: ["camera", "microphone"]
battery: true
powerWatts: null
alwaysOn: false
natType: "port-restricted"
bandwidthDownMbps: 50
bandwidthUpMbps: 10
latencyMs: 30
canSteward: false
canInfer: false
canDoorway: false
streamsTo: null
serviceability: "none"
healthSurfaces: []
circularity: "recyclable"
degradationMode: "cliff"
replacementLeadTime: "days"
expectedLifespanYears: 4
attestationCapabilities: []
---

# Education Chromebook

Shared. Managed. Port-restricted by district IT policy. Seven students
may touch it on any given school day, each with a different account, each
picking up a learning path mid-thread and setting it down again.

4GB RAM, 64GB eMMC, quad-core Celeron — enough to run a light conductor
for a personal source chain, not enough to hold anyone else's. The
battery lasts a school day if the hinges don't give out first. Wi-Fi
only; no ethernet path, no way around the captive portal. The protocol
learns to compress what it sends here.

The district's NAT is the real constraint. Port-restricted means no
incoming connections — this device can reach the network but the network
cannot reach it. Relay nodes absorb the cost. Backpressure is critical:
if the upstream can't accept, this device stops sending rather than
buffering into the eMMC until it fills.

When the device reaches end-of-lease, it's returned, wiped, and shipped
to a recycler. Four years, then gone — but every learning record it
produced lives on in the DHT under the student's own agent key, waiting
on whatever device comes next.
