---
id: "device-k8s-pod-256mb"
displayName: "K8s Pod (256MB)"
formFactor: "container"
capabilityLevel: 5
stage: 4
memoryGb: 0.25
storageGb: 1
storageType: "ssd"
cpuCores: 1
cpuClass: "virtual"
gpu: null
sensors: []
battery: false
powerWatts: null
alwaysOn: true
natType: "public"
bandwidthDownMbps: 1000
bandwidthUpMbps: 500
latencyMs: 2
canSteward: true
canInfer: false
canDoorway: true
streamsTo: null
serviceability: "none"
healthSurfaces: []
circularity: "disposable"
degradationMode: "cliff"
replacementLeadTime: "hours"
expectedLifespanYears: null
attestationCapabilities: []
---

# K8s Pod (256MB)

Developer convenience, not a real peer.

250MB RAM, one virtual CPU, 1GB of ephemeral SSD, public NAT courtesy
of the cluster's ingress. From the network's point of view, this node
looks like a capable level-5 participant — it can run doorway, it has
excellent connectivity, and it's always reachable. From the protocol's
point of view, it has the memory of a particularly forgetful goldfish.

It cannot infer. 256MB doesn't hold a tokenizer, let alone a model. It
runs doorway because doorway is lean enough to fit. Stewardship is
technically possible but practically limited to tiny shards with
aggressive eviction. This archetype exists so CI pipelines can test
multi-node scenarios without provisioning real hardware.

No health surfaces. No lifespan. The pod is spun up, used, and destroyed.
When the cluster scheduler decides to reschedule it, the pod vanishes in
under a second and the protocol must not care. Cliff degradation — no
warning, no graceful handoff, just gone. The network's resilience proofs
require that nothing here is irreplaceable, and nothing is.
