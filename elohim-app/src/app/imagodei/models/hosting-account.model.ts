/**
 * Hosting Account Model
 *
 * Types for the doorway hosting account endpoint (GET /auth/account).
 * Represents the hosted human's resource usage and stewardship status.
 */

export interface HostingAccount {
  humanId: string;
  identifier: string;
  permissionLevel: string;
  storageBytes: number;
  storageLimit: number;
  storagePercent: number;
  projectionQueries: number;
  dailyQueryLimit: number;
  queriesPercent: number;
  bandwidthBytes: number;
  dailyBandwidthLimit: number;
  bandwidthPercent: number;
  conductorId: string | null;
  isSteward: boolean;
  stewardshipAt: string | null;
  keyExported: boolean;
  createdAt: string;
  lastLoginAt: string | null;
}

export type UsageLevel = 'normal' | 'warning' | 'critical';

export function getUsageLevel(percent: number): UsageLevel {
  if (percent >= 90) return 'critical';
  if (percent >= 70) return 'warning';
  return 'normal';
}
