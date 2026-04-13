---
id: "device-2019-android-phone"
displayName: "2019 Android Phone"
formFactor: "phone"
capabilityLevel: 2
stage: 3
memoryGb: 3
storageGb: 32
storageType: "emmc"
cpuCores: 4
cpuClass: "arm-cortex-a53"
gpu: "mali-g72"
sensors: ["camera", "microphone", "accelerometer", "gps", "nfc"]
battery: true
powerWatts: null
alwaysOn: false
natType: "carrier-grade-nat"
bandwidthDownMbps: 25
bandwidthUpMbps: 5
latencyMs: 80
canSteward: false
canInfer: false
canDoorway: false
streamsTo: null
serviceability: "none"
healthSurfaces: ["battery-health"]
circularity: "recyclable"
degradationMode: "cliff"
replacementLeadTime: "days"
expectedLifespanYears: 4
attestationCapabilities: ["voice-presence", "facial-presence", "biometric-identity"]
---

# 2019 Android Phone

The floor. If the protocol works here, it works anywhere.

3GB RAM, 32GB eMMC, quad-core ARM, carrier-grade NAT, 4G with variable
latency. Battery-powered, intermittent. Can run a light conductor for a
personal source chain but can't steward anyone else's data. Sync budget
must be tiny — the backpressure feature exists because of this device.

This archetype represents billions of humans whose only compute is a
phone they bought three years ago. The protocol doesn't get to wish they
had better hardware. It serves them where they are.

Degradation: cliff. Battery dies, storage fills, screen cracks — done.
No SMART monitoring, no self-repair. When it dies, the network must
have already replicated everything this device held.
