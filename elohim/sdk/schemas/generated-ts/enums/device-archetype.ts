/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: enums/device-archetype.schema.json -- DO NOT EDIT */

/**
 * Hardware/deployment archetype of the device presenting an AgentPeerBinding. Source of truth: DHT (Notarized, Category A — imagodei integrity zome). DNA-notarized as part of the AgentPeerBinding entry type. Values: 'node' = dedicated elohim-node server/blade; 'desktop' = Tauri desktop app (workstation or laptop); 'mobile' = mobile client (phone or tablet); 'steward' = steward process managing collective infrastructure.
 */
export type DeviceArchetype = 'node' | 'desktop' | 'mobile' | 'steward';
