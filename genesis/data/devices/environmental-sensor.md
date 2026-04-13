---
id: "device-environmental-sensor"
displayName: "Environmental Sensor"
formFactor: "iot-sensor"
capabilityLevel: 1
stage: 4
memoryGb: 0.004
storageGb: 0.016
storageType: "flash"
cpuCores: 1
cpuClass: "esp32"
gpu: null
sensors: ["environmental"]
battery: false
powerWatts: 0.5
alwaysOn: true
natType: "offline-first"
bandwidthDownMbps: 0.01
bandwidthUpMbps: 0.01
latencyMs: null
canSteward: false
canInfer: false
canDoorway: false
streamsTo: "nearest-elohim-node"
serviceability: "none"
healthSurfaces: []
circularity: "repairable"
degradationMode: "cliff"
replacementLeadTime: "weeks"
expectedLifespanYears: 5
attestationCapabilities: ["environmental-conditions"]
---

# Environmental Sensor

Soil moisture, air quality, temperature. The protocol's connection to
the physical world.

ESP32 class, solar-powered, LoRaWAN. Micro-conductor with a single
reporting zome. DHT entries are 200 bytes. Creates the raw material for
place-based attestation and environmental value flows — a farm's soil
health over time, a neighborhood's air quality, a river's temperature
profile.

Doesn't run full storage. Streams observations to the nearest
elohim-capable node, which notarizes them as EPR stories. The sensor's
contribution is perception, not compute.

Sealed, weather-resistant. No self-service — when the solar cell
degrades past usefulness, the unit is replaced. But the observations
it recorded live on in the DHT, attributed to its agent key.
