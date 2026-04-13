---
id: "device-observer-mic-array"
displayName: "Observer Mic Array"
formFactor: "iot-sensor"
capabilityLevel: 1
stage: 4
memoryGb: 1
storageGb: 32
storageType: "sd-card"
cpuCores: 4
cpuClass: "arm-cortex-a72"
gpu: null
sensors: ["microphone"]
battery: true
powerWatts: 5
alwaysOn: true
natType: "offline-first"
bandwidthDownMbps: 10
bandwidthUpMbps: 5
latencyMs: 50
canSteward: false
canInfer: false
canDoorway: false
streamsTo: "nearest-elohim-node"
serviceability: "consumer"
healthSurfaces: ["battery-health"]
circularity: "repairable"
degradationMode: "graceful"
replacementLeadTime: "weeks"
expectedLifespanYears: 6
attestationCapabilities: ["voice-presence"]
---

# Observer Mic Array

Tier 2 civic infrastructure. The ears of a meeting room, a classroom, a
community hall — places where voice-presence attestation matters for
governance and learning records.

Raspberry Pi class compute, an array of MEMS microphones, 8-hour battery
backup for when power is interrupted. Offline-first by design: the device
stores observations locally on the SD card and streams to the nearest
elohim-capable node when connectivity is available. The node notarizes
voice-presence attestations as DHT entries, linking them to the session's
governance EPR or learning EPR under the participants' agent keys.

The attestation this device produces — "these voices were present in this
space at this time" — is the raw material for quorum verification, meeting
minutes provenance, and participatory governance records. It doesn't
identify who spoke or what they said; it witnesses presence and temporal
context.

At 5W with a battery backup, the device survives power cuts during
critical governance sessions. Graceful degradation means connectivity
loss triggers local buffering, not silence. Consumer serviceability:
the SD card and battery are accessible with a screwdriver. Six years
before the microphone capsules drift outside calibration tolerance.
