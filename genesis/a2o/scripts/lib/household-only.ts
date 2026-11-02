import { readFileSync } from 'node:fs';

import { OWNED_SUBSTRATE_CAP } from '../../src/framework/fixtures/substrate-scope.js';

interface HouseholdOnlyFeature {
  feature: string;
  kind: 'processControl' | 'proc-read';
}

const inventory = JSON.parse(
  readFileSync(
    new URL('../../../agentic/compute/stage/household-only-features.json', import.meta.url),
    'utf8'
  )
) as HouseholdOnlyFeature[];

export const HOUSEHOLD_ONLY_HOLD_REASON =
  'household-only — not measured here (reads household processes; observable only on an owned mesh)';

/** Match Cucumber's relative or absolute feature URI against the generated classifier projection. */
export function isHouseholdOnlyFeature(uri: string): boolean {
  const normalized = uri.replaceAll('\\', '/');
  return inventory.some(
    ({ feature }) => normalized === feature || normalized.endsWith(`/${feature}`)
  );
}

export function householdOnlyHoldReason(uri: string, missingCaps: string[]): string | undefined {
  return missingCaps.includes(OWNED_SUBSTRATE_CAP) && isHouseholdOnlyFeature(uri)
    ? HOUSEHOLD_ONLY_HOLD_REASON
    : undefined;
}
