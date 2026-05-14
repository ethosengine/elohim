# Device Archetypes: Fixture Data for Peer Diversity Testing

**Status**: Design — ready for review
**Date**: 2026-04-13
**Context**: During account seeding, P2P sync overwhelmed 256MB containers. The fix (backpressure) was correct, but it exposed a deeper need: the protocol must prove it works across the full diversity of devices that 7 billion humans bring to the network. Device archetypes are how we test that.

## The Problem

Humans have fixture data — Matthew, Frank, Terrance — each with a story that gives meaning to testable specifications. The protocol has no equivalent for hardware. We test on k8s pods that all look the same. But the real network will have phones, Chromebooks, Raspberry Pis, family nodes with AI accelerators, IoT sensors, recycled laptops, and gaming desktops that are only online when someone isn't playing.

Without device archetypes, we can't:
- Prove operations adapt to resource constraints
- Define the minimum viable peer
- Test that backpressure, sync budgets, and replication behave correctly across the hardware spectrum
- Write a2o scenarios with specific, testable parameters
- Inform operator presets and documentation with real numbers

## Relationship to Humans

Devices and humans are **independent entities**. A "2019 Android phone" is an archetype — not "Frank's phone." The human stories can *infer* what devices a person has today, but:

- Hardware changes over time (the old laptop becomes a hand-me-down)
- One person can have multiple devices and multiple nodes
- The same device archetype serves many different humans
- Device capabilities constrain what protocol operations are safe, regardless of who owns them

Human-device assignments are declared separately (in test fixtures, seed data, or scenario Given steps), not in the device definition itself.

## Capability Gradient

Devices don't divide into "runs the stack" and "doesn't." There's a gradient:

| Level | Name | What it runs | Example archetypes |
|-------|------|--------------|--------------------|
| 0 | **Streams only** | Raw data to a paired node. No conductor, no local state. | Camera, mic array, environmental sensor |
| 1 | **Micro-conductor** | Single-zome Holochain. Text, sensor readings, small DHT footprint. | IoT sensor, solar panel reporter, smart meter |
| 2 | **Light conductor** | Few zomes, intermittent connectivity. Can hold local source chain but not steward others' data. | Phone, Chromebook, spoke device |
| 3 | **Full storage** | elohim-storage + conductor + P2P sync + replication. Can steward content for others. | Raspberry Pi, NUC, always-on laptop |
| 4 | **Full storage + inference** | Level 3 + AI model serving (elohim agent host). | Family Node base, gaming desktop (when on) |
| 5 | **Full storage + inference + doorway** | Level 4 + public-facing web2 bridge. Serves visitors, hosts custodial keys. | Family Node as hub, dedicated server, k8s pod |

Every protocol operation should declare its minimum capability level. Sync requires level 3+. A sensor report requires level 1. Identity attestation requires a signing key and a network connection (level 1+). Elohim inference requires level 4+.

## Device Archetype Schema

Files live in `genesis/data/devices/`, one per archetype, following the same markdown-with-frontmatter pattern as `genesis/data/humans/`.

### Frontmatter Fields

#### Hardware Spec
| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique archetype identifier (e.g., `device-2019-android-phone`) |
| `displayName` | string | Human-readable name |
| `formFactor` | enum | `phone`, `tablet`, `laptop`, `desktop`, `sbc` (single-board computer), `mini-pc`, `rack-module`, `server`, `iot-sensor`, `wearable`, `thin-client` |
| `memory_gb` | float | Total RAM |
| `storage_gb` | float | Total persistent storage |
| `storage_type` | enum | `emmc`, `sd-card`, `ssd`, `nvme`, `hdd`, `flash` |
| `cpu_cores` | int | Total CPU cores |
| `cpu_class` | string | Freeform — e.g., `arm-cortex-a53`, `intel-i7-13700k`, `esp32` |
| `gpu` | string or null | GPU/NPU model, null if none. E.g., `rtx-4070`, `mali-g72`, `apple-neural-engine` |
| `sensors` | string[] | Available sensors: `camera`, `microphone`, `lidar`, `infrared`, `biometric`, `environmental`, `accelerometer`, `gps` |
| `battery` | bool | Battery-powered (affects availability model) |
| `power_watts` | float or null | Continuous power draw. Null for battery-only. |

#### Network Role
| Field | Type | Description |
|-------|------|-------------|
| `capability_level` | int (0-5) | From the gradient table above |
| `stage` | int (1-4) | Hardware spec stage from `hardware-spec.md` |
| `always_on` | bool | Expected to be always available |
| `nat_type` | enum | `public`, `symmetric-nat`, `port-restricted`, `carrier-grade-nat`, `offline-first` |
| `bandwidth_down_mbps` | float | Typical downstream bandwidth |
| `bandwidth_up_mbps` | float | Typical upstream bandwidth |
| `latency_ms` | float | Typical network latency to nearest peer |
| `can_steward` | bool | Can hold and serve others' content |
| `can_infer` | bool | Can run elohim AI inference |
| `can_doorway` | bool | Can serve as public web2 bridge |
| `streams_to` | string or null | For level 0-1 devices: what they stream data to. Null for self-sufficient devices. |

#### Lifecycle
| Field | Type | Description |
|-------|------|-------------|
| `serviceability` | enum | `none` (sealed), `consumer` (user-replaceable battery/storage), `modular` (hot-swap components), `full` (all components replaceable) |
| `health_surfaces` | string[] | What the device can self-report: `smart`, `thermal`, `power`, `usb-enumeration`, `battery-health`, `memory-ecc`, `fan-rpm` |
| `circularity` | enum | `disposable`, `recyclable`, `repairable`, `modular-upgradeable` |
| `degradation_mode` | enum | `cliff` (works until it doesn't), `graceful` (progressively slower), `modular` (individual components degrade independently) |
| `replacement_lead_time` | string | `hours` (commodity), `days` (order online), `weeks` (specialty part) |
| `expected_lifespan_years` | int | Design target operational lifetime |

#### Attestation Capability
| Field | Type | Description |
|-------|------|-------------|
| `attestation_capabilities` | string[] | What this device can attest to: `voice-presence`, `facial-presence`, `physical-presence`, `hardware-key-signing`, `spatial-occupancy`, `environmental-conditions`, `biometric-identity` |

### Narrative Body

Below the frontmatter, a short narrative (like the human files) that tells the device's story: who typically owns it, what its life is like on the network, what stresses it, where it shines, how it ages.

## Initial Portfolio

### Consumer Devices (Stage 1-3)

#### `device-2019-android-phone`
The floor. If the protocol works here, it works anywhere. 3GB RAM, 32GB eMMC, quad-core ARM, carrier-grade NAT, 4G with variable latency. Battery-powered, intermittent. Can run a light conductor for personal source chain but can't steward anyone else's data. Sync budget must be tiny. The backpressure feature we just built exists because of this device. Degradation: cliff (battery dies, storage fills, screen cracks — done). Serviceability: none.

#### `device-chromebook-edu`
The education context. 4GB RAM, 64GB eMMC, Wi-Fi only. Shared between students — identity must handle multi-user. Hub-and-spoke spoke that syncs when at school. Always behind NAT. Can run light conductor. Sensors: camera, microphone (for video calls). Degradation: cliff (keyboard breaks, hinge fails). Serviceability: none. Expected lifespan: 4 years (school refresh cycle).

#### `device-recycled-laptop`
The hand-me-down. 2018 ThinkPad, 8GB RAM, 256GB SSD, Intel i5. Decent storage and compute, but intermittent — used as a secondary device. Wi-Fi + occasional ethernet. Can run full storage when plugged in and connected, but shouldn't be relied on for always-on stewardship. Serviceability: consumer (battery, SSD replaceable). Health surfaces: SMART, thermal, battery-health. Degradation: graceful (slower with age, battery shrinks). This device tells the story of recycled compute entering the network.

#### `device-gaming-desktop`
Burst compute. 32GB RAM, RTX 4070, 1TB NVMe, wired gigabit. Powerful enough for AI inference — but only when the owner isn't gaming. Not always-on. Public NAT when ethernet. Can serve as a temporary inference node when volunteered. This device's story is about surplus capacity: the protocol should be able to use it when available and gracefully handle its absence. Degradation: modular (GPU, RAM, storage all replaceable). Expected lifespan: 6-8 years with upgrades.

### Infrastructure Devices (Stage 4)

#### `device-raspberry-pi-4`
The community backbone at minimum cost. 4GB RAM, 64GB SD card + USB SSD, quad-core ARM, always-on, 15W. Can run full storage. Hub for spoke communities (church, school, community center). Known failure mode: SD card corruption. Health surfaces: thermal (throttles at 80C). Serviceability: modular (SD card, USB drives swappable). This is the device that proves the protocol doesn't require expensive hardware to participate meaningfully.

#### `device-home-nuc`
The intentional mid-tier steward. Intel NUC, 16GB RAM, 1TB NVMe, always-on, 25W. Full storage + limited inference (small models via CPU). Wired ethernet, public NAT or UPnP. The device for technically comfortable families who want self-hosting without the Family Node investment. Serviceability: consumer (RAM, SSD upgradeable). Health surfaces: SMART, thermal, fan-rpm. Degradation: graceful. Expected lifespan: 7-10 years.

#### `device-family-node-base`
The Tier 3 heart of the ecosystem. 64GB DDR5 RAM, 16-core CPU, RTX 4070 class GPU, 2TB NVMe + 10TB bulk RAID. Always-on, <200W, whisper-quiet. Full storage + inference + doorway. Hosts family elohim agent (70B parameter model), custodial keys for less-technical family members, serves as geographic redundancy point. Serviceability: full (hot-swappable modules, tool-free). Health surfaces: everything — SMART, thermal, power, USB enumeration, memory ECC, fan RPM. Circularity: modular-upgradeable. Degradation: modular (individual components degrade independently; the node schedules its own maintenance). Expected lifespan: 10+ years with module replacement.

#### `device-family-node-extended`
Multi-generational scale. 128GB RAM, 4-5 rack modules. Everything the base does, plus capacity for extended family, community hub duties, and running multiple concurrent AI models. The device that proves the protocol scales *up* as well as down.

#### `device-k8s-pod-256mb`
Developer convenience — not a real peer archetype. This is the protocol pretending to be P2P while running in cloud-native infrastructure. 256MB memory, 0.5 CPU, ephemeral storage. Constrained by economics, not physics. The device that proved we needed backpressure: 5 peers syncing 3400 items each overwhelmed it during import.

Useful for testing today, but raises the real question: **how does the protocol modularize compute for heterogeneous peers?** WASM modules? Containers? Something else? This connects to brit (build artifacts as EPR), rakia (distributed build substrate), and the elohim operator (how compute optimizes and shards for its humans). Deep design needed — see open question below.

Serviceability: N/A (cattle, not pets). Degradation: cliff (OOM-killed, rescheduled).

#### `device-dedicated-server`
Institutional infrastructure. Church, school, co-op server room. 64GB+ RAM, multi-TB storage, redundant power, wired gigabit+. Serves as community-scale steward: multi-family data, geographic redundancy, high-availability doorway. The oak tree of the network.

### Observer/IoT Devices (Level 0-1)

#### `device-observer-mic-array`
The Tier 2 civic observer. Raspberry Pi + omnidirectional mic array. Streams transcription data to nearest elohim-capable node. Battery backup for 8+ hours. LoRaWAN + Wi-Fi. Attestation: voice-presence. Doesn't run full storage — micro-conductor for notarizing observation timestamps. The device that makes community governance meetings legible to the protocol.

#### `device-observer-camera`
Privacy-focused visual sensor. Streams to family node only. Attestation: facial-presence (opt-in). No cloud, no recording by default — real-time processing on the family node. The device that proves surveillance can be inverted: the family controls the camera, not a corporation.

#### `device-environmental-sensor`
Soil moisture, air quality, temperature. ESP32 class, solar-powered, LoRaWAN. Micro-conductor with a single reporting zome. DHT entries are 200 bytes. Creates the raw material for place-based attestation and environmental value flows. Expected lifespan: 5+ years (sealed, weather-resistant). Degradation: cliff (battery/solar cell degrades, then dies). The device that connects the protocol to the physical world.

#### `device-biometric-fob`
Hardware identity attestation. YubiKey-class device or purpose-built fob. No conductor — streams signed attestations to paired device. Attestation: hardware-key-signing, biometric-identity. The device that provides the strongest identity proof on the network without requiring biometric data to leave the device.

### Edge Cases

#### `device-thin-client-batch`
The computer lab surplus. 10 units, 2GB RAM each, 16GB flash. Individually useless for full storage. Collectively, they represent a distributed compute resource that the protocol should be able to compose. Micro-conductor each, or collectively managed by a coordinator node. The device that proves the protocol can absorb heterogeneous surplus compute. Degradation: cliff per unit, graceful as a collective (losing one of ten is fine).

## Connection to a2o Scenarios

Device archetypes make performance scenarios testable by providing specific, named parameters:

```gherkin
Background:
  Given device "2019-android-phone" from the device portfolio
  # Implies: 3GB RAM, 32GB storage, carrier-grade NAT, 4G, battery

Scenario: Phone pauses sync during content download
  Given the device has 2 connected peers
  When the device downloads a 50MB learning path
  Then sync is paused during the download
  And peak memory usage stays within the device's sync budget

Scenario: Phone cannot steward others' content
  Given the device capability_level is 2 (light conductor)
  When a replication request arrives for content stewardship
  Then the device declines with reason "capability_level below minimum (3)"
```

### Scenario Categories by Device Dimension

**Memory pressure** (backpressure, sync budget, OOM protection):
- Proven on: `2019-android-phone`, `k8s-pod-256mb`, `chromebook-edu`
- Baseline: `family-node-base` (abundant memory, no pressure)

**Storage stewardship** (what can a device hold for others):
- Level 0-2 devices: cannot steward
- Level 3+: stewardship proportional to available storage
- `raspberry-pi-4`: stewards with caution (SD card failure risk)
- `family-node-base`: primary stewardship backbone

**Network resilience** (NAT traversal, intermittent connectivity, relay):
- `2019-android-phone`: carrier-grade NAT, must use relay
- `chromebook-edu`: Wi-Fi only, hub-and-spoke sync pattern
- `recycled-laptop`: intermittent, protocol must handle disappearance gracefully
- `gaming-desktop`: powerful but unpredictable availability

**AI inference** (elohim hosting, model serving):
- Level 4+: `family-node-base`, `gaming-desktop` (when on), `family-node-extended`
- Level 3: can *request* inference from peers, cannot serve it
- Level 0-2: observers that *generate* data for inference

**Hardware health** (self-awareness, predictive maintenance, ambient service):
- `raspberry-pi-4`: SD card SMART monitoring, thermal throttle detection
- `family-node-base`: full health surface — drives, power, thermal, USB, ECC
- `k8s-pod-256mb`: no health surface (ephemeral, replaced not repaired)
- `environmental-sensor`: sealed, no self-service — replaced as unit

**Attestation** (identity, presence, environmental):
- `biometric-fob`: strongest identity attestation
- `observer-camera`: facial presence (opt-in, family-controlled)
- `observer-mic-array`: voice presence for governance
- `environmental-sensor`: place-based attestation
- `2019-android-phone`: camera + GPS + accelerometer — versatile but less trusted than purpose-built

## Operational Envelope Per Archetype

Each device archetype should define its operational envelope — the parameters that govern how the protocol treats it:

| Parameter | What it governs | Derived from |
|-----------|----------------|-------------|
| `sync_budget_bytes` | Max memory for P2P sync per round | `memory_gb` × ratio |
| `max_concurrent_sync_peers` | How many peers to sync with simultaneously | `memory_gb`, `bandwidth_up_mbps` |
| `replication_batch_size` | Items per replication cycle | `memory_gb`, `cpu_cores` |
| `backpressure_threshold` | When to pause sync for bulk writes | `memory_gb` |
| `stewardship_capacity_gb` | How much content to steward for others | `storage_gb` × ratio, minus local needs |
| `inference_model_class` | What size AI models can run | `memory_gb`, `gpu` |
| `availability_sla` | Expected uptime for stewardship contracts | `always_on`, `battery`, `degradation_mode` |

These parameters are computed from the archetype's hardware spec, not hand-configured. The protocol's job is to derive the right behavior from what the device reports about itself — like an ecosystem where each organism finds its niche based on its capabilities, not because an operator assigned it one.

## Implementation Path

1. **Device archetype files** — `genesis/data/devices/*.md` with frontmatter + narrative
2. **JSON generation** — like humans, markdown renders to `devices.json` for test fixtures
3. **a2o step definitions** — `Given device "<archetype>" from the device portfolio` loads the frontmatter as test context
4. **Operational envelope derivation** — a function that takes hardware spec and returns protocol parameters
5. **Scenario expansion** — backpressure, sync, replication, stewardship, inference scenarios parameterized by device

## What This Is NOT

- Not a hardware product spec (that's `hardware-spec.md`)
- Not a purchasing guide
- Not exhaustive — new archetypes are added as the network encounters new hardware
- Not static — archetypes evolve as hardware generations change

This is **fixture data for testing peer diversity**, the same way humans are fixture data for testing identity diversity. The device archetypes prove the protocol was designed for 7 billion humans on 20 billion devices, not 5 identical k8s pods.

## Open Questions (for future deep design)

### P2P Compute Modularization
The k8s pod archetype exposes a gap: the protocol doesn't yet have a native answer for how compute gets packaged, distributed, and executed across heterogeneous peers. K8s is a developer convenience, not the answer. The real question: what's the P2P equivalent of a container? WASM modules? Something built on rakia's artifact-as-EPR model? How does the elohim operator decide what compute runs where, given the device diversity this spec describes?

This connects to:
- **brit** — build artifacts as EPR, reach-governed branches
- **rakia** — distributed build substrate, artifacts as ContentNodes
- **elohim operator** — how compute and storage optimizes and shards for the humans it operates with/for
- **capability_level gradient** — the operator needs to match workloads to device capabilities

Not blocking for device archetype fixture data, but blocking for any "run this workload on that peer" story.
