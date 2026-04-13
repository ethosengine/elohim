---
id: "device-thin-client-batch"
displayName: "Thin Client Batch"
formFactor: "thin-client"
capabilityLevel: 1
stage: 4
memoryGb: 2
storageGb: 16
storageType: "flash"
cpuCores: 2
cpuClass: "intel-celeron-j4005"
gpu: null
sensors: []
battery: false
powerWatts: 10
alwaysOn: true
natType: "port-restricted"
bandwidthDownMbps: 100
bandwidthUpMbps: 50
latencyMs: 15
canSteward: false
canInfer: false
canDoorway: false
streamsTo: null
serviceability: "none"
healthSurfaces: []
circularity: "recyclable"
degradationMode: "cliff"
replacementLeadTime: "days"
expectedLifespanYears: 8
attestationCapabilities: []
---

# Thin Client Batch

The computer lab surplus — individually weak, collectively meaningful.

A decade ago these machines ran thin-client VDI sessions in a corporate
office. Now they're stacked in a shipping container headed for a rural
school, a library, a village community center. 2GB RAM, 16GB flash,
dual-core Celeron, 10W. Per unit: almost nothing. Twenty units: a
micro-conductor cluster that shares DHT participation across the room.

Each thin client runs a light conductor and participates as a micro-node
in the network. They can't steward anyone else's data — the flash is too
small and too slow. But they can maintain personal source chains for the
learners who sit at them, sync those chains to the room's nearest capable
steward node, and participate in the DHT lookup mesh so discovery doesn't
rely on a single point of failure.

Port-restricted NAT is the norm — district or campus IT doesn't open
inbound ports for surplus hardware. The protocol routes around it via
relay, which costs bandwidth the thin clients don't strain. At 10W
always-on, twenty machines pull 200W total — less than four incandescent
bulbs.

Eight years. Flash storage doesn't have moving parts. The Celeron J4005
runs cool enough that no active cooling is needed. When one fails — and
eventually one will, cliff-style without warning — the remaining nineteen
absorb its load and the room barely notices.
