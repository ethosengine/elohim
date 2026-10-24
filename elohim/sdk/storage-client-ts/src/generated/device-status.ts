/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/device-status.schema.json -- DO NOT EDIT */

/**
 * Ephemeral arrival checks in dependency order. Only refused causes a failing status exit.
 */
export type DeviceStatus = {
  name: string;
  state: 'ok' | 'degraded' | 'refused' | 'unknown';
  detail: string;
  cure: string | null;
}[];
