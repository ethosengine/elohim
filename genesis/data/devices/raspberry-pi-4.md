---
id: "device-raspberry-pi-4"
displayName: "Raspberry Pi 4"
formFactor: "sbc"
capabilityLevel: 3
stage: 4
memoryGb: 4
storageGb: 128
storageType: "sd-card"
cpuCores: 4
cpuClass: "arm-cortex-a72"
gpu: null
sensors: []
battery: false
powerWatts: 15
alwaysOn: true
natType: "port-restricted"
bandwidthDownMbps: 100
bandwidthUpMbps: 50
latencyMs: 10
canSteward: true
canInfer: false
canDoorway: false
streamsTo: null
serviceability: "modular"
healthSurfaces: ["thermal"]
circularity: "repairable"
degradationMode: "graceful"
replacementLeadTime: "weeks"
expectedLifespanYears: 7
attestationCapabilities: []
---

# Raspberry Pi 4

The community backbone at minimum cost.

Seventy-five dollars of hardware in a case the size of a deck of cards,
drawing 15W from a wall outlet, running the full storage stack
continuously. SD card wear is the known failure mode — the protocol
watches write amplification, rotates logs to RAM, and nudges the operator
toward a USB SSD when the card's endurance budget is running low.
Thermal throttling is the other risk in summer; a heatsink and a small
fan buy years of headroom.

This is the device that makes the network real for households that can't
afford or justify a family node. A church puts one in the server closet.
A school's tech teacher puts one on the classroom shelf. A cooperative
mounts one in the community center's utility room. Each one holds a slice
of the DHT, participates in replication, and serves as the always-on
anchor for the mobile devices in its orbit.

Seven years is realistic if the SD card is replaced once and the power
supply isn't undersized. Modular in the sense that matters: the SD card
comes out, the USB storage swaps, the OS image reflashes. The agent key
and all its relationships persist in the DHT, waiting for the node to
return.
