---
id: "device-biometric-fob"
displayName: "Biometric Fob"
formFactor: "fob"
capabilityLevel: 0
stage: 3
memoryGb: 0.001
storageGb: 0.001
storageType: "flash"
cpuCores: 1
cpuClass: "secure-element"
gpu: null
sensors: []
battery: true
powerWatts: null
alwaysOn: false
natType: "offline-first"
bandwidthDownMbps: 0.001
bandwidthUpMbps: 0.001
latencyMs: null
canSteward: false
canInfer: false
canDoorway: false
streamsTo: "nearest-elohim-node"
serviceability: "none"
healthSurfaces: []
circularity: "recyclable"
degradationMode: "cliff"
replacementLeadTime: "days"
expectedLifespanYears: 10
attestationCapabilities: ["hardware-key-signing", "biometric-identity"]
---

# Biometric Fob

The agent key made physical. A secure element the size of a house key,
worn on a lanyard or kept in a pocket, producing cryptographic attestations
that no software-only device can fake.

1KB of RAM, 1KB of flash, one secure enclave running a FIDO2 stack.
The biometric reader — fingerprint or vein pattern — gates the signing
operation: the key never leaves the element, but the element won't
sign without a living hand. The result is an attestation that carries
physical presence, biological continuity, and hardware-root signing
simultaneously. No phone, no network, no conductor needed — just the
fob, the reader on the paired device, and a cryptographic commitment.

Battery lifespan is measured in years of standby, not hours of use. The
element only activates when touched to a reader. Not always-on in any
meaningful sense — it's a signing oracle, dormant until called. When the
battery finally fails, the fob stops working immediately — cliff, no
warning — and the agent key must be recovered from the DHT through the
established key rotation protocol.

Ten years before the battery requires replacement. Sealed against
tampering; no user-serviceable parts; that's the point.
