/**
 * Device archetype fixtures — hardware profiles from genesis/data/devices/devices.json.
 *
 * These archetypes represent the full capability gradient that the Elohim Protocol
 * must serve: from 4MB IoT sensors to 64GB family nodes. Tests use them to verify
 * that operations adapt correctly to device constraints.
 *
 * The fixture data is the authoritative source for device capability assumptions
 * in a2o scenarios — operational parameters (sync budgets, stewardship thresholds)
 * should be derived from these fields, not hardcoded in steps.
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
  stage: number;
  memoryGb: number;
  storageGb: number;
  storageType: string;
  cpuCores: number;
  cpuClass: string;
  gpu: string | null;
  sensors: string[];
  battery: boolean;
  powerWatts: number | null;
  alwaysOn: boolean;
  natType: string;
  bandwidthDownMbps: number;
  bandwidthUpMbps: number;
  latencyMs: number | null;
  canSteward: boolean;
  canInfer: boolean;
  canDoorway: boolean;
  streamsTo: string | null;
  serviceability: string;
  healthSurfaces: string[];
  circularity: string;
  degradationMode: string;
  replacementLeadTime: string;
  expectedLifespanYears: number | null;
  attestationCapabilities: string[];
}

interface DevicesJson {
  devices: DeviceArchetype[];
}

// ---------------------------------------------------------------------------
// Loader — reads devices.json once and caches
// ---------------------------------------------------------------------------

let _cache: DeviceArchetype[] | null = null;

function loadDevices(): DeviceArchetype[] {
  if (_cache) return _cache;

  const __filename = fileURLToPath(import.meta.url);
  const __dirname = dirname(__filename);
  const jsonPath = resolve(__dirname, '../../../../data/devices/devices.json');

  const raw = readFileSync(jsonPath, 'utf-8');
  const parsed = JSON.parse(raw) as DevicesJson;
  _cache = parsed.devices;
  return _cache;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Look up a device archetype by displayName (case-insensitive) or id.
 * Throws if not found.
 */
export function getDevice(name: string): DeviceArchetype {
  const devices = loadDevices();
  const lower = name.toLowerCase();
  const found = devices.find(
    d => d.displayName.toLowerCase() === lower || d.id === name || d.id === lower
  );
  if (!found) {
    const available = devices.map(d => `"${d.displayName}"`).join(', ');
    throw new Error(
      `Device archetype "${name}" not found in devices.json. Available: ${available}`
    );
  }
  return found;
}

/**
 * Return all device archetypes.
 */
export function getAllDevices(): DeviceArchetype[] {
  return loadDevices();
}

/**
 * Return all device archetypes at the given capability level.
 */
export function getDevicesByLevel(level: number): DeviceArchetype[] {
  return loadDevices().filter(d => d.capabilityLevel === level);
}
