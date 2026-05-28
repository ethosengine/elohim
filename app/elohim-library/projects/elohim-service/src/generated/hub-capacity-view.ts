/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/hub-capacity-view.schema.json -- DO NOT EDIT */

/**
 * Hub-level storage-capacity aggregate. Sums per-device PeerCapacityView across all devices belonging to a hub. Hub is a *role* (per project_hub_archetype_abstraction); substrate stays kind-agnostic. Source of truth: hub-membership graph (notarized humans.household_id projection OR collective_participations) + per-device PeerCapacityView. Operational Category C — reconstructed per request; not persisted.
 */
export interface HubCapacityView {
  hubId: string;
  hubKind: 'dwelling' | 'collective' | 'computed';
  displayLabel?: string | null;
  memberDeviceCount: number;
  capacity: null | Record<string, unknown>;
}
