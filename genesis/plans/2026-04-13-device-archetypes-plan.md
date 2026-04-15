# Device Archetypes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create fixture data for device archetypes (`genesis/data/devices/`) with schema validation, JSON generation, and a2o step definitions — following the same pattern as `genesis/data/humans/`.

**Architecture:** Markdown files with YAML frontmatter (one per device archetype) validated by a hand-rolled validator against `devices.schema.json`. A generation script produces `devices.json` for test fixtures. A2O step definitions load device archetypes by name to parameterize performance scenarios.

**Tech Stack:** TypeScript (tsx), YAML frontmatter parsing, JSON Schema (hand-rolled validation matching `validate-humans.ts` pattern), Cucumber step definitions.

**Design spec:** `genesis/plans/2026-04-13-device-archetypes-design.md`

**P2P Design Gate:** Category C (operational). Device archetypes are seed/fixture data for testing — not DHT-notarized entities, not storage tables, not protocol data. `devices.schema.json` validates markdown frontmatter (same pattern as `humans.schema.json`). No new entry types. No conductor involvement. Source of truth: the markdown files in `genesis/data/devices/`.

---

### Task 1: Device Schema

**Files:**
- Create: `genesis/data/devices/devices.schema.json`

- [ ] **Step 1: Write the schema**

Follow the `genesis/data/humans/humans.schema.json` pattern. The schema defines the YAML frontmatter contract for device archetype markdown files.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "elohim:protocol:devices",
  "title": "Device Archetype Frontmatter Schema",
  "description": "Operational IoC contract (Category C) for device archetypes in genesis/data/devices/*.md. These are abstract hardware archetypes — the species of the network — not specific physical devices. Each archetype defines a performance envelope that parameterizes a2o scenarios and informs operator presets.",
  "type": "object",
  "required": [
    "id", "displayName", "formFactor", "capabilityLevel",
    "memoryGb", "storageGb", "storageType", "cpuCores",
    "alwaysOn", "natType", "bandwidthDownMbps", "bandwidthUpMbps",
    "serviceability", "circularity", "degradationMode"
  ],
  "properties": {
    "id": {
      "type": "string",
      "pattern": "^device-[a-z0-9][a-z0-9-]*[a-z0-9]$",
      "description": "Unique slug. Must match filename stem (e.g. 2019-android-phone.md -> device-2019-android-phone)."
    },
    "displayName": {
      "type": "string",
      "minLength": 1
    },
    "formFactor": {
      "type": "string",
      "enum": [
        "phone", "tablet", "laptop", "desktop", "sbc",
        "mini-pc", "rack-module", "server", "iot-sensor",
        "wearable", "thin-client", "fob", "container"
      ]
    },
    "capabilityLevel": {
      "type": "integer",
      "minimum": 0,
      "maximum": 5,
      "description": "0=streams-only, 1=micro-conductor, 2=light-conductor, 3=full-storage, 4=storage+inference, 5=storage+inference+doorway"
    },
    "stage": {
      "type": "integer",
      "minimum": 1,
      "maximum": 4,
      "description": "Hardware spec stage from hardware-spec.md"
    },
    "memoryGb": { "type": "number", "minimum": 0 },
    "storageGb": { "type": "number", "minimum": 0 },
    "storageType": {
      "type": "string",
      "enum": ["emmc", "sd-card", "ssd", "nvme", "hdd", "flash"]
    },
    "cpuCores": { "type": "integer", "minimum": 1 },
    "cpuClass": { "type": ["string", "null"] },
    "gpu": { "type": ["string", "null"] },
    "sensors": {
      "type": "array",
      "items": {
        "type": "string",
        "enum": [
          "camera", "microphone", "lidar", "infrared", "biometric",
          "environmental", "accelerometer", "gps", "nfc", "lora"
        ]
      },
      "default": []
    },
    "battery": { "type": "boolean" },
    "powerWatts": { "type": ["number", "null"] },
    "alwaysOn": { "type": "boolean" },
    "natType": {
      "type": "string",
      "enum": ["public", "symmetric-nat", "port-restricted", "carrier-grade-nat", "offline-first"]
    },
    "bandwidthDownMbps": { "type": "number", "minimum": 0 },
    "bandwidthUpMbps": { "type": "number", "minimum": 0 },
    "latencyMs": { "type": ["number", "null"] },
    "canSteward": { "type": "boolean", "default": false },
    "canInfer": { "type": "boolean", "default": false },
    "canDoorway": { "type": "boolean", "default": false },
    "streamsTo": {
      "type": ["string", "null"],
      "description": "For level 0-1 devices: what they stream data to. Null for self-sufficient devices."
    },
    "serviceability": {
      "type": "string",
      "enum": ["none", "consumer", "modular", "full"]
    },
    "healthSurfaces": {
      "type": "array",
      "items": {
        "type": "string",
        "enum": [
          "smart", "thermal", "power", "usb-enumeration",
          "battery-health", "memory-ecc", "fan-rpm"
        ]
      },
      "default": []
    },
    "circularity": {
      "type": "string",
      "enum": ["disposable", "recyclable", "repairable", "modular-upgradeable"]
    },
    "degradationMode": {
      "type": "string",
      "enum": ["cliff", "graceful", "modular"]
    },
    "replacementLeadTime": {
      "type": ["string", "null"],
      "enum": ["hours", "days", "weeks", null]
    },
    "expectedLifespanYears": { "type": ["integer", "null"] },
    "attestationCapabilities": {
      "type": "array",
      "items": {
        "type": "string",
        "enum": [
          "voice-presence", "facial-presence", "physical-presence",
          "hardware-key-signing", "spatial-occupancy",
          "environmental-conditions", "biometric-identity"
        ]
      },
      "default": []
    }
  },
  "additionalProperties": false
}
```

- [ ] **Step 2: Commit**

```bash
git add genesis/data/devices/devices.schema.json
git commit -m "feat(devices): add device archetype frontmatter schema"
```

---

### Task 2: First Three Device Archetype Files

Write the three archetypes that span the widest capability range: the floor (phone), the backbone (family node), and the observer surface (environmental sensor). This validates the schema covers the full gradient before writing all 15.

**Files:**
- Create: `genesis/data/devices/2019-android-phone.md`
- Create: `genesis/data/devices/family-node-base.md`
- Create: `genesis/data/devices/environmental-sensor.md`

- [ ] **Step 1: Write the phone archetype (level 2 — light conductor)**

```markdown
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
```

- [ ] **Step 2: Write the family node archetype (level 5 — full stack)**

```markdown
---
id: "device-family-node-base"
displayName: "Family Node (Base)"
formFactor: "rack-module"
capabilityLevel: 5
stage: 4
memoryGb: 64
storageGb: 12000
storageType: "nvme"
cpuCores: 16
cpuClass: "intel-i7-13700k"
gpu: "rtx-4070"
sensors: []
battery: false
powerWatts: 200
alwaysOn: true
natType: "public"
bandwidthDownMbps: 1000
bandwidthUpMbps: 500
latencyMs: 5
canSteward: true
canInfer: true
canDoorway: true
streamsTo: null
serviceability: "full"
healthSurfaces: ["smart", "thermal", "power", "usb-enumeration", "memory-ecc", "fan-rpm"]
circularity: "modular-upgradeable"
degradationMode: "modular"
replacementLeadTime: "days"
expectedLifespanYears: 10
attestationCapabilities: []
---

# Family Node (Base)

The Tier 3 heart of the ecosystem. The oak tree.

64GB DDR5 RAM, 16-core CPU, RTX 4070 class GPU, 2TB NVMe primary plus
10TB bulk RAID. Always-on, under 200W, whisper-quiet. Runs the full
stack: storage, conductor, P2P sync, replication, AI inference (70B
parameter model), and public doorway.

Hosts the family elohim agent, custodial keys for less-technical
relatives, serves as geographic redundancy point for the trust network.
This is the device that replaces cloud subscriptions with something
you own.

Serviceability: full. Hot-swappable modules, tool-free maintenance. The
node monitors its own health — SMART on every drive, thermal sensors,
power draw, USB enumeration, ECC memory. When a drive shows increasing
reallocated sectors, the node shifts replication priority to other
stewards, orders the replacement part, and notifies the family:
"Storage module needs replacing. Compatible drive shipped. Service
window: Tuesday afternoon."

The human's only decision is scheduling. The protocol handles urgency,
replication safety, and parts sourcing.
```

- [ ] **Step 3: Write the environmental sensor archetype (level 1 — micro-conductor)**

```markdown
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
```

- [ ] **Step 4: Commit**

```bash
git add genesis/data/devices/2019-android-phone.md genesis/data/devices/family-node-base.md genesis/data/devices/environmental-sensor.md
git commit -m "feat(devices): add first 3 device archetypes (phone, family node, sensor)"
```

---

### Task 3: Device Validator

Hand-rolled TypeScript validator following the `genesis/seeder/src/validate-humans.ts` pattern. Validates frontmatter against schema constants, checks referential integrity (IDs match filenames, no duplicates).

**Files:**
- Create: `genesis/seeder/src/validate-devices.ts`
- Modify: `genesis/seeder/package.json` (add `validate:devices` script)

- [ ] **Step 1: Write the validator**

```typescript
/**
 * Device Archetype Seed Data Validator
 *
 * Validates genesis/data/devices/*.md (markdown with YAML frontmatter)
 * against devices.schema.json. Hand-rolled to match validate-humans.ts.
 *
 * Usage:
 *   npx tsx src/validate-devices.ts
 */

import { readdirSync, readFileSync } from 'node:fs';
import { join, basename } from 'node:path';
import { parse as parseYaml } from 'yaml';

// =============================================================================
// Constants — sources of truth (mirrored from devices.schema.json)
// =============================================================================

export const FORM_FACTORS = [
  'phone', 'tablet', 'laptop', 'desktop', 'sbc',
  'mini-pc', 'rack-module', 'server', 'iot-sensor',
  'wearable', 'thin-client', 'fob', 'container',
] as const;

export const STORAGE_TYPES = [
  'emmc', 'sd-card', 'ssd', 'nvme', 'hdd', 'flash',
] as const;

export const NAT_TYPES = [
  'public', 'symmetric-nat', 'port-restricted',
  'carrier-grade-nat', 'offline-first',
] as const;

export const SERVICEABILITY = [
  'none', 'consumer', 'modular', 'full',
] as const;

export const CIRCULARITY = [
  'disposable', 'recyclable', 'repairable', 'modular-upgradeable',
] as const;

export const DEGRADATION_MODES = [
  'cliff', 'graceful', 'modular',
] as const;

export const SENSORS = [
  'camera', 'microphone', 'lidar', 'infrared', 'biometric',
  'environmental', 'accelerometer', 'gps', 'nfc', 'lora',
] as const;

export const HEALTH_SURFACES = [
  'smart', 'thermal', 'power', 'usb-enumeration',
  'battery-health', 'memory-ecc', 'fan-rpm',
] as const;

export const ATTESTATION_CAPABILITIES = [
  'voice-presence', 'facial-presence', 'physical-presence',
  'hardware-key-signing', 'spatial-occupancy',
  'environmental-conditions', 'biometric-identity',
] as const;

export const REPLACEMENT_LEAD_TIMES = [
  'hours', 'days', 'weeks',
] as const;

// =============================================================================
// Types
// =============================================================================

interface DeviceFrontmatter {
  id: string;
  displayName: string;
  formFactor: string;
  capabilityLevel: number;
  stage?: number;
  memoryGb: number;
  storageGb: number;
  storageType: string;
  cpuCores: number;
  cpuClass?: string | null;
  gpu?: string | null;
  sensors?: string[];
  battery: boolean;
  powerWatts?: number | null;
  alwaysOn: boolean;
  natType: string;
  bandwidthDownMbps: number;
  bandwidthUpMbps: number;
  latencyMs?: number | null;
  canSteward?: boolean;
  canInfer?: boolean;
  canDoorway?: boolean;
  streamsTo?: string | null;
  serviceability: string;
  healthSurfaces?: string[];
  circularity: string;
  degradationMode: string;
  replacementLeadTime?: string | null;
  expectedLifespanYears?: number | null;
  attestationCapabilities?: string[];
}

// =============================================================================
// Validation
// =============================================================================

function extractFrontmatter(content: string): Record<string, unknown> | null {
  const match = content.match(/^---\n([\s\S]*?)\n---/);
  if (!match) return null;
  return parseYaml(match[1]) as Record<string, unknown>;
}

function validateDevice(
  filename: string,
  fm: DeviceFrontmatter,
): string[] {
  const errors: string[] = [];
  const warn = (msg: string) => errors.push(`${filename}: ${msg}`);

  // ID matches filename
  const expectedId = `device-${basename(filename, '.md')}`;
  if (fm.id !== expectedId) {
    warn(`id "${fm.id}" must match filename stem: expected "${expectedId}"`);
  }

  // Required string fields
  if (!fm.displayName || fm.displayName.length === 0) {
    warn('displayName is required');
  }

  // Enum validations
  if (!FORM_FACTORS.includes(fm.formFactor as any)) {
    warn(`formFactor "${fm.formFactor}" not in: ${FORM_FACTORS.join(', ')}`);
  }
  if (!STORAGE_TYPES.includes(fm.storageType as any)) {
    warn(`storageType "${fm.storageType}" not in: ${STORAGE_TYPES.join(', ')}`);
  }
  if (!NAT_TYPES.includes(fm.natType as any)) {
    warn(`natType "${fm.natType}" not in: ${NAT_TYPES.join(', ')}`);
  }
  if (!SERVICEABILITY.includes(fm.serviceability as any)) {
    warn(`serviceability "${fm.serviceability}" not in: ${SERVICEABILITY.join(', ')}`);
  }
  if (!CIRCULARITY.includes(fm.circularity as any)) {
    warn(`circularity "${fm.circularity}" not in: ${CIRCULARITY.join(', ')}`);
  }
  if (!DEGRADATION_MODES.includes(fm.degradationMode as any)) {
    warn(`degradationMode "${fm.degradationMode}" not in: ${DEGRADATION_MODES.join(', ')}`);
  }

  // Capability level range
  if (fm.capabilityLevel < 0 || fm.capabilityLevel > 5) {
    warn(`capabilityLevel ${fm.capabilityLevel} out of range [0, 5]`);
  }

  // Stage range (optional but validated if present)
  if (fm.stage != null && (fm.stage < 1 || fm.stage > 4)) {
    warn(`stage ${fm.stage} out of range [1, 4]`);
  }

  // Numeric minimums
  if (fm.memoryGb < 0) warn('memoryGb must be >= 0');
  if (fm.storageGb < 0) warn('storageGb must be >= 0');
  if (fm.cpuCores < 1) warn('cpuCores must be >= 1');
  if (fm.bandwidthDownMbps < 0) warn('bandwidthDownMbps must be >= 0');
  if (fm.bandwidthUpMbps < 0) warn('bandwidthUpMbps must be >= 0');

  // Array enum validations
  for (const s of fm.sensors ?? []) {
    if (!SENSORS.includes(s as any)) {
      warn(`sensor "${s}" not in: ${SENSORS.join(', ')}`);
    }
  }
  for (const h of fm.healthSurfaces ?? []) {
    if (!HEALTH_SURFACES.includes(h as any)) {
      warn(`healthSurface "${h}" not in: ${HEALTH_SURFACES.join(', ')}`);
    }
  }
  for (const a of fm.attestationCapabilities ?? []) {
    if (!ATTESTATION_CAPABILITIES.includes(a as any)) {
      warn(`attestationCapability "${a}" not in: ${ATTESTATION_CAPABILITIES.join(', ')}`);
    }
  }
  if (fm.replacementLeadTime != null &&
      !REPLACEMENT_LEAD_TIMES.includes(fm.replacementLeadTime as any)) {
    warn(`replacementLeadTime "${fm.replacementLeadTime}" not in: ${REPLACEMENT_LEAD_TIMES.join(', ')}`);
  }

  // Semantic checks
  if (fm.capabilityLevel <= 1 && fm.canSteward) {
    warn('level 0-1 devices cannot steward (canSteward should be false)');
  }
  if (fm.capabilityLevel < 4 && fm.canInfer) {
    warn('inference requires capability level 4+ (canInfer should be false)');
  }
  if (fm.capabilityLevel < 5 && fm.canDoorway) {
    warn('doorway requires capability level 5 (canDoorway should be false)');
  }

  return errors;
}

// =============================================================================
// Main
// =============================================================================

const DEVICES_DIR = join(import.meta.dirname, '../../data/devices');

const files = readdirSync(DEVICES_DIR).filter(
  f => f.endsWith('.md') && !f.startsWith('_')
);

if (files.length === 0) {
  console.error('No device archetype files found in', DEVICES_DIR);
  process.exit(1);
}

let totalErrors = 0;
const allIds: string[] = [];
const allNames: string[] = [];

for (const file of files) {
  const content = readFileSync(join(DEVICES_DIR, file), 'utf-8');
  const fm = extractFrontmatter(content) as DeviceFrontmatter | null;

  if (!fm) {
    console.error(`${file}: no YAML frontmatter found`);
    totalErrors++;
    continue;
  }

  const errors = validateDevice(file, fm);
  for (const e of errors) {
    console.error(`ERROR: ${e}`);
    totalErrors++;
  }

  // Track for duplicate detection
  allIds.push(fm.id);
  allNames.push(fm.displayName);
}

// Directory-level checks
const dupeIds = allIds.filter((id, i) => allIds.indexOf(id) !== i);
for (const id of dupeIds) {
  console.error(`ERROR: duplicate device id "${id}"`);
  totalErrors++;
}

const dupeNames = allNames.filter((n, i) => allNames.indexOf(n) !== i);
for (const name of dupeNames) {
  console.error(`ERROR: duplicate displayName "${name}"`);
  totalErrors++;
}

console.log(`\nValidated ${files.length} device archetypes: ${totalErrors} errors`);
if (totalErrors > 0) process.exit(1);
console.log('All device archetypes valid.');
```

- [ ] **Step 2: Add the validate:devices script to package.json**

In `genesis/seeder/package.json`, add to the `"scripts"` section:

```json
"validate:devices": "tsx src/validate-devices.ts",
```

And add `validate:devices` to the `validate:all` script chain.

- [ ] **Step 3: Run the validator against the 3 archetypes**

Run: `cd genesis/seeder && pnpm run validate:devices`
Expected: `Validated 3 device archetypes: 0 errors`

- [ ] **Step 4: Commit**

```bash
git add genesis/seeder/src/validate-devices.ts genesis/seeder/package.json
git commit -m "feat(seeder): add device archetype validator"
```

---

### Task 4: Remaining Device Archetypes

Write the remaining 12 archetypes from the design spec. Each follows the same frontmatter structure validated in Task 3.

**Files:**
- Create: `genesis/data/devices/chromebook-edu.md`
- Create: `genesis/data/devices/recycled-laptop.md`
- Create: `genesis/data/devices/gaming-desktop.md`
- Create: `genesis/data/devices/raspberry-pi-4.md`
- Create: `genesis/data/devices/home-nuc.md`
- Create: `genesis/data/devices/family-node-extended.md`
- Create: `genesis/data/devices/k8s-pod-256mb.md`
- Create: `genesis/data/devices/dedicated-server.md`
- Create: `genesis/data/devices/observer-mic-array.md`
- Create: `genesis/data/devices/observer-camera.md`
- Create: `genesis/data/devices/biometric-fob.md`
- Create: `genesis/data/devices/thin-client-batch.md`

- [ ] **Step 1: Write all 12 archetype files**

Each file follows the pattern from Task 2. Use the design spec (`genesis/plans/2026-04-13-device-archetypes-design.md`) for the specific values. Key parameters per archetype:

| Archetype | Level | Memory | Storage | Always-on | NAT | Form factor |
|-----------|-------|--------|---------|-----------|-----|-------------|
| chromebook-edu | 2 | 4GB | 64GB eMMC | false | port-restricted | laptop |
| recycled-laptop | 3 | 8GB | 256GB SSD | false | port-restricted | laptop |
| gaming-desktop | 4 | 32GB | 1000GB NVMe | false | public | desktop |
| raspberry-pi-4 | 3 | 4GB | 128GB SD+USB | true | port-restricted | sbc |
| home-nuc | 4 | 16GB | 1000GB NVMe | true | public | mini-pc |
| family-node-extended | 5 | 128GB | 20000GB NVMe | true | public | rack-module |
| k8s-pod-256mb | 5 | 0.25GB | 1GB SSD | true | public | container |
| dedicated-server | 5 | 64GB | 8000GB HDD | true | public | server |
| observer-mic-array | 1 | 1GB | 32GB SD | true | offline-first | iot-sensor |
| observer-camera | 0 | 0.5GB | 8GB flash | true | offline-first | iot-sensor |
| biometric-fob | 0 | 0.001GB | 0.001GB flash | false | offline-first | fob |
| thin-client-batch | 1 | 2GB | 16GB flash | true | port-restricted | thin-client |

Write the full narrative body for each — who owns it, what stresses it, where it shines, how it ages.

- [ ] **Step 2: Run validator**

Run: `cd genesis/seeder && pnpm run validate:devices`
Expected: `Validated 15 device archetypes: 0 errors`

- [ ] **Step 3: Commit**

```bash
git add genesis/data/devices/
git commit -m "feat(devices): add remaining 12 device archetypes (15 total)"
```

---

### Task 5: JSON Generation Script

Generate `devices.json` from the markdown frontmatter files, matching the `humans.json` pattern.

**Files:**
- Create: `genesis/seeder/src/generate-devices-json.ts`
- Modify: `genesis/seeder/package.json` (add `generate:devices` script)

- [ ] **Step 1: Write the generator**

```typescript
/**
 * Generate devices.json from genesis/data/devices/*.md frontmatter.
 *
 * Usage:
 *   npx tsx src/generate-devices-json.ts
 *
 * Output:
 *   genesis/data/devices/devices.json
 */

import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { parse as parseYaml } from 'yaml';

const DEVICES_DIR = join(import.meta.dirname, '../../data/devices');
const OUTPUT = join(DEVICES_DIR, 'devices.json');

function extractFrontmatter(content: string): Record<string, unknown> | null {
  const match = content.match(/^---\n([\s\S]*?)\n---/);
  if (!match) return null;
  return parseYaml(match[1]) as Record<string, unknown>;
}

const files = readdirSync(DEVICES_DIR)
  .filter(f => f.endsWith('.md') && !f.startsWith('_'))
  .sort();

const devices = files.map(file => {
  const content = readFileSync(join(DEVICES_DIR, file), 'utf-8');
  const fm = extractFrontmatter(content);
  if (!fm) throw new Error(`${file}: no frontmatter`);
  return fm;
});

const output = { devices };
writeFileSync(OUTPUT, JSON.stringify(output, null, 2) + '\n');
console.log(`Generated ${OUTPUT} with ${devices.length} devices`);
```

- [ ] **Step 2: Add script to package.json**

In `genesis/seeder/package.json`:

```json
"generate:devices": "tsx src/generate-devices-json.ts",
```

- [ ] **Step 3: Run the generator**

Run: `cd genesis/seeder && pnpm run generate:devices`
Expected: `Generated .../devices.json with 15 devices`

- [ ] **Step 4: Verify the output**

Run: `cat genesis/data/devices/devices.json | head -20`
Expected: JSON array of device objects with camelCase keys matching frontmatter.

- [ ] **Step 5: Commit**

```bash
git add genesis/seeder/src/generate-devices-json.ts genesis/seeder/package.json genesis/data/devices/devices.json
git commit -m "feat(seeder): add devices.json generator from markdown frontmatter"
```

---

### Task 6: A2O Device Fixture Loader

Create the a2o framework integration so scenarios can reference device archetypes by name.

**Files:**
- Create: `genesis/a2o/src/framework/fixtures/devices.ts`

- [ ] **Step 1: Write the device fixture loader**

Follow the `genesis/a2o/src/framework/fixtures/humans.ts` pattern:

```typescript
/**
 * Device fixtures — archetype specs from genesis/data/devices/devices.json.
 *
 * Used by a2o step definitions to parameterize performance scenarios
 * with specific device characteristics (memory, bandwidth, NAT type, etc.).
 */

import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

// ---------------------------------------------------------------------------
// Types mirroring devices.json schema
// ---------------------------------------------------------------------------

export interface DeviceArchetype {
  id: string;
  displayName: string;
  formFactor: string;
  capabilityLevel: number;
  stage?: number;
  memoryGb: number;
  storageGb: number;
  storageType: string;
  cpuCores: number;
  cpuClass?: string | null;
  gpu?: string | null;
  sensors?: string[];
  battery: boolean;
  powerWatts?: number | null;
  alwaysOn: boolean;
  natType: string;
  bandwidthDownMbps: number;
  bandwidthUpMbps: number;
  latencyMs?: number | null;
  canSteward: boolean;
  canInfer: boolean;
  canDoorway: boolean;
  streamsTo?: string | null;
  serviceability: string;
  healthSurfaces?: string[];
  circularity: string;
  degradationMode: string;
  replacementLeadTime?: string | null;
  expectedLifespanYears?: number | null;
  attestationCapabilities?: string[];
}

interface DevicesJson {
  devices: DeviceArchetype[];
}

// ---------------------------------------------------------------------------
// Load devices.json
// ---------------------------------------------------------------------------

const __dirname = dirname(fileURLToPath(import.meta.url));
const DEVICES_PATH = resolve(__dirname, '../../../../data/devices/devices.json');

let cachedDevices: DevicesJson | null = null;

function loadDevices(): DevicesJson {
  if (cachedDevices) return cachedDevices;
  const raw = readFileSync(DEVICES_PATH, 'utf-8');
  cachedDevices = JSON.parse(raw) as DevicesJson;
  return cachedDevices;
}

/**
 * Get a device archetype by displayName (case-insensitive partial match).
 * Throws if not found — test should fail loudly on unknown device names.
 */
export function getDevice(name: string): DeviceArchetype {
  const { devices } = loadDevices();
  const lower = name.toLowerCase();
  const device = devices.find(
    d => d.displayName.toLowerCase() === lower
      || d.id === `device-${lower}`
      || d.id === lower
  );
  if (!device) {
    const available = devices.map(d => d.displayName).join(', ');
    throw new Error(
      `Device archetype "${name}" not found. Available: ${available}`
    );
  }
  return device;
}

/**
 * Get all device archetypes.
 */
export function getAllDevices(): DeviceArchetype[] {
  return loadDevices().devices;
}

/**
 * Get devices filtered by capability level.
 */
export function getDevicesByLevel(level: number): DeviceArchetype[] {
  return loadDevices().devices.filter(d => d.capabilityLevel === level);
}
```

- [ ] **Step 2: Commit**

```bash
git add genesis/a2o/src/framework/fixtures/devices.ts
git commit -m "feat(a2o): add device archetype fixture loader"
```

---

### Task 7: A2O Step Definitions for Device Scenarios

Create step definitions that let Gherkin scenarios reference device archetypes and their properties.

**Files:**
- Create: `genesis/a2o/steps/fixture-devices.steps.ts`

- [ ] **Step 1: Write the step definitions**

```typescript
/**
 * Device archetype step definitions — load device specs for performance scenarios.
 *
 * Example:
 *   Given device "2019 Android Phone" from the device portfolio
 *   Then the device memory should be 3 GB
 *   And the device capability level should be 2
 */

import { Given, Then } from '@cucumber/cucumber';
import { expect } from 'chai';

import { getDevice, type DeviceArchetype } from '../src/framework/fixtures/devices.js';
import { E2EWorld } from '../src/framework/world.js';

// Store current device on the world for use across steps
declare module '../src/framework/world.js' {
  interface E2EWorld {
    currentDevice?: DeviceArchetype;
  }
}

Given(
  'device {string} from the device portfolio',
  function (this: E2EWorld, deviceName: string) {
    this.currentDevice = getDevice(deviceName);
  }
);

Then(
  'the device memory should be {float} GB',
  function (this: E2EWorld, expectedGb: number) {
    expect(this.currentDevice).to.exist;
    expect(this.currentDevice!.memoryGb).to.equal(expectedGb);
  }
);

Then(
  'the device capability level should be {int}',
  function (this: E2EWorld, expectedLevel: number) {
    expect(this.currentDevice).to.exist;
    expect(this.currentDevice!.capabilityLevel).to.equal(expectedLevel);
  }
);

Then(
  'the device can steward content',
  function (this: E2EWorld) {
    expect(this.currentDevice).to.exist;
    expect(this.currentDevice!.canSteward).to.be.true;
  }
);

Then(
  'the device cannot steward content',
  function (this: E2EWorld) {
    expect(this.currentDevice).to.exist;
    expect(this.currentDevice!.canSteward).to.be.false;
  }
);

Then(
  'the device should be always-on',
  function (this: E2EWorld) {
    expect(this.currentDevice).to.exist;
    expect(this.currentDevice!.alwaysOn).to.be.true;
  }
);

Then(
  'the device should not be always-on',
  function (this: E2EWorld) {
    expect(this.currentDevice).to.exist;
    expect(this.currentDevice!.alwaysOn).to.be.false;
  }
);

Then(
  'the device NAT type should be {string}',
  function (this: E2EWorld, expectedNat: string) {
    expect(this.currentDevice).to.exist;
    expect(this.currentDevice!.natType).to.equal(expectedNat);
  }
);

Then(
  'the device degradation mode should be {string}',
  function (this: E2EWorld, expectedMode: string) {
    expect(this.currentDevice).to.exist;
    expect(this.currentDevice!.degradationMode).to.equal(expectedMode);
  }
);
```

- [ ] **Step 2: Commit**

```bash
git add genesis/a2o/steps/fixture-devices.steps.ts
git commit -m "feat(a2o): add device archetype step definitions"
```

---

### Task 8: A2O Performance Scenarios — Peer Diversity Feature File

Write the feature file that proves operations adapt to device constraints. Uses the device fixtures from Tasks 6-7.

**Files:**
- Create: `genesis/a2o/features/deployment/peer-diversity.feature`

- [ ] **Step 1: Write the feature file**

```gherkin
@e2e @deployment @p2p @peer-diversity @requires:seeded-content
Feature: Peer Diversity — Operations Adapt to Device Constraints
  As the Elohim Protocol
  I want every operation to be aware of the device it runs on
  So that 7 billion humans on 20 billion devices can all participate
  according to what their hardware can offer

  The protocol doesn't get to wish people had better hardware. It serves
  them where they are. A phone with 3GB RAM is a full citizen. A family
  node with 64GB is a backbone. An IoT sensor with 4MB is a witness.
  Each finds its niche — like species in an ecosystem, not racks in a
  data center.

  # --- Capability Gradient ---

  @wip
  Scenario: Device portfolio covers the full capability gradient
    Given the device portfolio is loaded
    Then there should be at least 1 device at capability level 0
    And there should be at least 1 device at capability level 1
    And there should be at least 1 device at capability level 2
    And there should be at least 1 device at capability level 3
    And there should be at least 1 device at capability level 4
    And there should be at least 1 device at capability level 5

  # --- Memory Pressure (Backpressure) ---

  @wip @regression
  Scenario: Phone pauses sync during bulk content download
    Given device "2019 Android Phone" from the device portfolio
    # 3GB RAM, carrier-grade NAT, 4G
    And the device has 2 connected peers syncing 500 items each
    When the device downloads a 50MB learning path
    Then sync is paused during the download
    And peak memory stays within the device sync budget
    # Operational parameter: sync_budget = memoryGb * 0.05 = 150MB

  @wip @regression
  Scenario: K8s pod pauses sync during account import
    Given device "K8s Pod (256MB)" from the device portfolio
    # 256MB memory, 5 peers, 3400+ inventory items
    And the device has 5 connected peers syncing 3400 items each
    When an account package with 200 items is imported
    Then sync is paused for the duration of the import
    And the import completes without OOM
    # This is the scenario that discovered the backpressure need.

  @wip
  Scenario: Family node does not need backpressure for normal imports
    Given device "Family Node (Base)" from the device portfolio
    # 64GB RAM — sync overhead is negligible
    And the device has 10 connected peers syncing 5000 items each
    When an account package with 500 items is imported
    Then sync continues running during the import
    And no backpressure is triggered
    # 64GB node has headroom to sync and import concurrently.

  # --- Stewardship Boundaries ---

  @wip
  Scenario: Phone cannot accept stewardship requests
    Given device "2019 Android Phone" from the device portfolio
    Then the device capability level should be 2
    And the device cannot steward content
    # Level 2 = light conductor. No always-on, no replication capacity.

  @wip
  Scenario: Raspberry Pi can steward modest content volumes
    Given device "Raspberry Pi 4" from the device portfolio
    Then the device capability level should be 3
    And the device can steward content
    And the device should be always-on
    # Level 3 = full storage. SD card is a known risk — health monitoring matters.

  @wip
  Scenario: Family node is primary stewardship backbone
    Given device "Family Node (Base)" from the device portfolio
    Then the device capability level should be 5
    And the device can steward content
    And the device should be always-on
    And the device degradation mode should be "modular"
    # Individual components degrade; the node schedules its own maintenance.

  # --- Network Resilience ---

  @wip
  Scenario: Carrier-grade NAT devices must use relay
    Given device "2019 Android Phone" from the device portfolio
    Then the device NAT type should be "carrier-grade-nat"
    # This device cannot accept inbound connections. The protocol must
    # route through relay peers or doorways for sync initiation.

  @wip
  Scenario: Offline-first devices stream to paired nodes
    Given device "Environmental Sensor" from the device portfolio
    Then the device capability level should be 1
    And the device NAT type should be "offline-first"
    # LoRaWAN, solar-powered. Streams observations to nearest node.
    # Never initiates P2P sync — doesn't have the stack for it.

  # --- Hardware Health Self-Awareness ---

  @wip
  Scenario: Family node reports full health surface
    Given device "Family Node (Base)" from the device portfolio
    Then the device should report health surfaces:
      | surface          |
      | smart            |
      | thermal          |
      | power            |
      | usb-enumeration  |
      | memory-ecc       |
      | fan-rpm          |
    # This device knows its own body. Failing drives, thermal throttle,
    # power anomalies, disconnected USB — all observable.

  @wip
  Scenario: Phone reports minimal health surface
    Given device "2019 Android Phone" from the device portfolio
    Then the device should report health surfaces:
      | surface         |
      | battery-health  |
    # Sealed device. The only thing it can self-report is battery state.
    # When the battery dies, the network must have already replicated
    # everything this device held.

  # --- Lifecycle and Circularity ---

  @wip
  Scenario: Modular devices schedule their own maintenance
    Given device "Family Node (Base)" from the device portfolio
    Then the device degradation mode should be "modular"
    And the device serviceability should be "full"
    # When a drive shows increasing SMART errors, the node shifts
    # replication priority, orders the part, notifies the family.
    # The human's only decision is scheduling.

  @wip
  Scenario: Cliff-degradation devices need proactive replication
    Given device "2019 Android Phone" from the device portfolio
    Then the device degradation mode should be "cliff"
    And the device serviceability should be "none"
    # This device will work until it doesn't. The protocol must ensure
    # that anything valuable on this device is already replicated
    # elsewhere before the cliff arrives.

  # --- Attestation Surface ---

  @wip
  Scenario: Biometric fob provides strongest identity attestation
    Given device "Biometric Fob" from the device portfolio
    Then the device capability level should be 0
    And the device should support attestation:
      | capability              |
      | hardware-key-signing    |
      | biometric-identity      |
    # Level 0 — doesn't run Holochain. But provides the strongest
    # identity proof on the network via hardware key signing.

  @wip
  Scenario: Environmental sensor provides place-based attestation
    Given device "Environmental Sensor" from the device portfolio
    Then the device should support attestation:
      | capability                |
      | environmental-conditions  |
    # "This sensor observed this at this place at this time."
    # The raw material for environmental value flows.
```

- [ ] **Step 2: Commit**

```bash
git add genesis/a2o/features/deployment/peer-diversity.feature
git commit -m "feat(a2o): add peer diversity performance scenarios (15 archetypes)"
```

---

### Task 9: Wire into Pre-Push Hook

Add device validation to the pre-push gate so device archetype changes are validated before push, matching the humans validation pattern.

**Files:**
- Modify: `genesis/seeder/package.json` (ensure `validate:devices` is in `validate:all`)
- Modify: `.husky/pre-push` (add devices validation trigger on `genesis/data/devices/` changes)

- [ ] **Step 1: Check current validate:all script and add validate:devices**

In `genesis/seeder/package.json`, the `validate:all` script should include `validate:devices`:

```json
"validate:all": "pnpm validate:humans && pnpm validate:presences && pnpm validate:collectives && pnpm validate:account-packages && pnpm validate:devices",
```

- [ ] **Step 2: Add trigger to pre-push hook**

In `.husky/pre-push`, after the humans/presences validation block, add:

```bash
if echo "$CHANGED" | grep -qE "genesis/data/devices/"; then
  echo "[pre-push] Validating device archetypes..."
  (cd genesis/seeder && pnpm run validate:devices) || exit 1
fi
```

- [ ] **Step 3: Test the hook locally**

Run: `cd genesis/seeder && pnpm run validate:devices`
Expected: `Validated 15 device archetypes: 0 errors`

- [ ] **Step 4: Commit**

```bash
git add genesis/seeder/package.json .husky/pre-push
git commit -m "chore: wire device validation into pre-push gate"
```

---

### Task 10: Final Push and Verify

- [ ] **Step 1: Run full validation suite**

```bash
cd genesis/seeder && pnpm run validate:all
```

Expected: All validators pass including `validate:devices`.

- [ ] **Step 2: Run a2o typecheck**

```bash
cd genesis/a2o && pnpm run typecheck
```

Expected: No type errors from the new fixture loader or step definitions.

- [ ] **Step 3: Push**

```bash
git push
```

Expected: Pre-push gate passes. All projects green.
