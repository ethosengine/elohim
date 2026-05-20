/**
 * Standing — attested roles a viewer holds in this context.
 *
 * The DSL: an element declares requirements as an array of strings. Each entry is
 * AND-combined. Within an entry, `|` means OR. Examples:
 *   ['pilot']                          — must hold pilot
 *   ['pilot | steward']                — pilot OR steward
 *   ['pilot', 'contributor']           — pilot AND contributor
 *   ['pilot | steward', 'contributor'] — (pilot OR steward) AND contributor
 *
 * See: genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md §5.3
 */

import type { Standing } from './profile.js';

/**
 * Parses the contract DSL into normalized form: outer array is AND, inner array is OR.
 * Each inner array element is a Standing name.
 */
export function parseStandingRequirement(requirement: readonly string[]): readonly Standing[][] {
  return requirement.map(entry =>
    entry
      .split('|')
      .map(token => token.trim())
      .filter(token => token.length > 0)
  );
}

/**
 * Returns true if `held` satisfies `requirement`.
 * AND across outer groups; OR within each group.
 */
export function satisfiesRequirement(
  held: readonly Standing[],
  requirement: readonly string[]
): boolean {
  const parsed = parseStandingRequirement(requirement);
  return parsed.every(orGroup => orGroup.some(s => held.includes(s)));
}
