/**
 * Reach resolution — the ONE place a seeder turns an authored grade into the
 * `reach` column elohim-storage gates reads on.
 *
 * It lives here because it used to live in exactly one seeder. `seed-sqlite.ts`
 * grew the inverted-burden resolver (commit cc0cacd85) while `seed.ts` — the
 * doorway path that `pnpm seed` runs, and the one that seeds the fleet and the
 * local mesh — kept a literal `reach: 'public'`. The authored grade was
 * discarded on the seeder that matters. Two seeders, two policies, no shared
 * unit: that is the drift this module closes.
 */
import { REACH_OPENNESS, isReach } from './generated/schema-enums.js';
import type { Reach } from './generated/schema-enums.js';

/**
 * Assert a string is a canonical reach value (from the generated ordinal).
 * HARD-FAILS on non-canonical input — no silent coalesce. Legacy keys
 * (invited, local, neighborhood, municipal) are NOT canonical and throw here.
 */
export function assertReach(v: string, ctx: string): Reach {
  if (!isReach(v)) {
    throw new Error(
      `non-canonical reach "${v}" in ${ctx} — must be one of ${Object.keys(REACH_OPENNESS).join(', ')}`
    );
  }
  return v;
}

/**
 * Resolve effective reach under an inverted burden: default `private`, rise
 * only via an authored value or an archetype advisory (account-package
 * relationship assignment). The most-open candidate across
 * {private, advisory, authored} wins, compared by the generated REACH_OPENNESS
 * ordinal. Any non-canonical candidate HARD-FAILS via assertReach.
 */
export function earnedReach(input: { authored?: string; advisory?: string }): Reach {
  const candidates: Reach[] = ['private'];
  if (input.advisory) candidates.push(assertReach(input.advisory, 'archetype advisory'));
  if (input.authored) candidates.push(assertReach(input.authored, 'authored reach'));
  return candidates.reduce((a, b) => (REACH_OPENNESS[a] >= REACH_OPENNESS[b] ? a : b));
}

/**
 * The grade an UNGRADED corpus row keeps on the doorway seed path.
 *
 * This is deliberately NOT the inverted-burden `private` that `earnedReach`
 * resolves to. Only 88 of 3431 content files carry an authored `reach`, so
 * flipping the ungraded remainder to `private` would take the anonymous corpus
 * from ~3400 rows to ~90 in one commit — a corpus-wide product decision, not a
 * defect fix. Named (not a bare literal) so the disagreement between the two
 * seeders is greppable and pinned by a test until the grading pass lands:
 * genesis/data/timeline/backlog/seed-doorway-unauthored-reach-default.md
 */
export const UNAUTHORED_CORPUS_REACH: Reach = 'public';

/**
 * Resolve the reach for one doorway-seeded content or path JSON.
 *
 * `reach` is the authored grade; `visibility` is the legacy path key, read only
 * when `reach` is absent. An ungraded row keeps `UNAUTHORED_CORPUS_REACH`; a
 * graded one is honored verbatim and never widened.
 */
export function seedReach(json: { reach?: string; visibility?: string }): Reach {
  const authored = json.reach ?? json.visibility;
  if (!authored) return UNAUTHORED_CORPUS_REACH;
  return assertReach(authored, 'authored reach');
}
