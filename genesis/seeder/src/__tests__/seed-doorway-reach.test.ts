import { describe, it, expect } from 'vitest';
import { seedReach, UNAUTHORED_CORPUS_REACH } from '../reach-resolver.js';

// The doorway seed path (`seed.ts`, what `pnpm seed` runs) built its
// CreateContentInput with a LITERAL `reach: 'public'` for every concept, and
// read only the legacy `visibility` key for paths. The authored `reach` grade
// in genesis/data/lamad/**.json was therefore discarded on the one seeder that
// actually seeds the fleet and the local mesh: 49 commons-graded and 38
// community-graded rows all landed `public`, and a path with no authored grade
// landed `public` too. Measured on the local mesh 2026-08-21 — every one of
// 1993 listable rows read `reach="public"` in matthew's content.db, which is
// what silently disarmed the reach-enforced-http non-vacuity control.
describe('seedReach — the doorway seed path honors the authored grade', () => {
  it('honors an authored commons grade', () => {
    expect(seedReach({ reach: 'commons' })).toBe('commons');
  });

  it('never widens an authored community grade to public', () => {
    expect(seedReach({ reach: 'community' })).toBe('community');
  });

  it('honors an authored restricted grade on a path fixture', () => {
    expect(seedReach({ reach: 'private' })).toBe('private');
  });

  it('still reads a path legacy `visibility` when `reach` is absent', () => {
    expect(seedReach({ visibility: 'intimate' })).toBe('intimate');
  });

  it('prefers the authored `reach` over the legacy `visibility`', () => {
    expect(seedReach({ reach: 'commons', visibility: 'public' })).toBe('commons');
  });

  it('HARD-FAILS on a non-canonical authored grade (no silent coalesce to public)', () => {
    expect(() => seedReach({ reach: 'invited' })).toThrow(/non-canonical reach/i);
  });

  // NOT the inverted-burden `private` that seed-sqlite.ts resolves to. The two
  // seeders disagree on the UNAUTHORED default and this pins the disagreement
  // so it cannot drift silently while the corpus grading is unfinished — see
  // genesis/data/timeline/backlog/seed-doorway-unauthored-reach-default.md.
  it('leaves an ungraded row at the documented corpus default', () => {
    expect(seedReach({})).toBe(UNAUTHORED_CORPUS_REACH);
    expect(UNAUTHORED_CORPUS_REACH).toBe('public');
  });
});
