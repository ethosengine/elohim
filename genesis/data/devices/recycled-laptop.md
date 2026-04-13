---
id: "device-recycled-laptop"
displayName: "Recycled 2018 ThinkPad"
formFactor: "laptop"
capabilityLevel: 3
stage: 3
memoryGb: 8
storageGb: 256
storageType: "ssd"
cpuCores: 4
cpuClass: "intel-i5-8250u"
gpu: null
sensors: ["camera", "microphone"]
battery: true
powerWatts: null
alwaysOn: false
natType: "port-restricted"
bandwidthDownMbps: 100
bandwidthUpMbps: 20
latencyMs: 15
canSteward: true
canInfer: false
canDoorway: false
streamsTo: null
serviceability: "consumer"
healthSurfaces: ["smart", "thermal", "battery-health"]
circularity: "repairable"
degradationMode: "graceful"
replacementLeadTime: "days"
expectedLifespanYears: 8
attestationCapabilities: []
---

# Recycled 2018 ThinkPad

Someone's corporate castoff — a lease-return ThinkPad that got a 256GB
SSD and 8GB of RAM for under forty dollars in parts, then joined the
network as a capable steward node.

When plugged in, it runs full storage and can hold a slice of the DHT
for its household. Unplugged, the battery health gauge tells the truth:
six-year-old cells running at 60% of original capacity mean two hours
if you're lucky. The protocol knows. When battery-health drops below
threshold, stewardship duties shift to plugged-in peers without a word
to the human.

SMART on the SSD watches for reallocated sectors. Thermal sensors catch
the fan degrading. Graceful degradation means the machine doesn't go
from healthy to dead — it flags each surface early, giving months of
warning before the node needs to wind down its stewardship commitments
and transfer replication to others.

Eight-year expected lifespan isn't wishful thinking — it's the reality
of ThinkPad repairability. Keyboard, battery, RAM, and storage are all
user-replaceable without special tools. The longest-lived node in the
fleet is often the humblest one.
