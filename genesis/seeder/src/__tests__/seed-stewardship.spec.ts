import { describe, expect, it } from 'vitest';

import {
  allocationNote,
  buildContentIndex,
  buildPresenceIdByHumanId,
  contributionTypeForRole,
  loadAllPresences,
  resolveAuthoredStewards,
  resolveStewardship,
} from '../seed-stewardship.js';

const presences = loadAllPresences();
const presenceIdByHumanId = buildPresenceIdByHumanId(presences);

describe('buildPresenceIdByHumanId', () => {
  it('binds every contributor presence to its canonical human', () => {
    // Every presence file must declare metadata.humanId — the ONLY bridge
    // between the humans.json namespace (content stewardedBy) and the
    // presence namespace (stewardship_allocations).
    expect(presences.length).toBeGreaterThan(0);
    expect(presenceIdByHumanId.size).toBe(presences.length);
  });

  it('binds the founder (matthew-dowell has no slug-derivable humanId)', () => {
    expect(presenceIdByHumanId.get('human-matthew-manager')).toBe('matthew-dowell');
  });
});

describe('contributionTypeForRole', () => {
  it('maps authored roles onto the storage contribution_type enum', () => {
    expect(contributionTypeForRole('author')).toBe('original_creator');
    expect(contributionTypeForRole('steward')).toBe('maintainer');
    expect(contributionTypeForRole('curator')).toBe('curator');
    expect(contributionTypeForRole('endorser')).toBe('curator');
    expect(contributionTypeForRole(undefined)).toBe('curator');
    // Never emit a value storage rejects (db/models.rs contribution_types).
    const valid = new Set([
      'original_creator',
      'editor',
      'translator',
      'curator',
      'maintainer',
      'inherited',
    ]);
    for (const role of ['author', 'steward', 'endorser', 'curator', 'nonsense', undefined]) {
      expect(valid.has(contributionTypeForRole(role))).toBe(true);
    }
  });
});

describe('resolveAuthoredStewards', () => {
  it('normalizes declared affinities to ratios summing to 1.0', () => {
    const stewards = resolveAuthoredStewards(
      [
        { humanId: 'human-matthew-manager', affinity: 0.8, role: 'author' },
        { humanId: 'human-pete-pastor', affinity: 0.4, role: 'endorser' },
      ],
      presenceIdByHumanId
    );
    expect(stewards).not.toBeNull();
    const sum = stewards!.reduce((a, s) => a + s.ratio, 0);
    expect(sum).toBeCloseTo(1.0, 10);
    expect(stewards!.map((s) => s.presenceId)).toEqual(['matthew-dowell', 'pete-pastor']);
    expect(stewards![0].ratio).toBeCloseTo(0.8 / 1.2, 10);
    expect(stewards![0].contributionType).toBe('original_creator');
    expect(stewards![1].contributionType).toBe('curator');
  });

  it('FAILS CLOSED when any declared human has no contributor presence', () => {
    // human-georgina-grocer appears on 1870 content files and has no presence
    // file. Honoring the array partially would silently redistribute her
    // affinity to the others — a different, unauthored allocation.
    const stewards = resolveAuthoredStewards(
      [
        { humanId: 'human-matthew-manager', affinity: 0.8, role: 'author' },
        { humanId: 'human-georgina-grocer', affinity: 0.7, role: 'steward' },
      ],
      presenceIdByHumanId
    );
    expect(stewards).toBeNull();
  });

  it('returns null for an empty or unusable declaration', () => {
    expect(resolveAuthoredStewards(undefined, presenceIdByHumanId)).toBeNull();
    expect(resolveAuthoredStewards([], presenceIdByHumanId)).toBeNull();
    expect(
      resolveAuthoredStewards([{ humanId: 'human-matthew-manager', affinity: 0 }], presenceIdByHumanId)
    ).toBeNull();
  });

  it('sums duplicate humans before normalizing', () => {
    const stewards = resolveAuthoredStewards(
      [
        { humanId: 'human-matthew-manager', affinity: 0.5, role: 'author' },
        { humanId: 'human-matthew-manager', affinity: 0.5, role: 'endorser' },
      ],
      presenceIdByHumanId
    );
    expect(stewards).toEqual([
      { presenceId: 'matthew-dowell', ratio: 1, contributionType: 'original_creator' },
    ]);
  });
});

describe('resolveStewardship precedence', () => {
  const authored = [{ humanId: 'human-pete-pastor', affinity: 0.7, role: 'steward' }];

  it('keeps the curated category map ahead of a machine-annotated stewardedBy', () => {
    // 3428 of 3431 content files carry a stewardedBy written by
    // annotate-stewardship.py. Letting them win would rewrite the hand-curated
    // affinity graph the a2o stewardship scenarios assert on.
    const resolved = resolveStewardship('governance', authored, presenceIdByHumanId);
    expect(resolved.provenance).toBe('category');
    expect(resolved.stewards.map((s) => s.presenceId)).toEqual([
      'nancy-neighbor',
      'matthew-dowell',
      'eve-firstwoman',
    ]);
  });

  it('honors the authored declaration where the seeder previously knew nothing', () => {
    const resolved = resolveStewardship('church-crisis', authored, presenceIdByHumanId);
    expect(resolved.provenance).toBe('authored');
    expect(resolved.stewards).toEqual([
      { presenceId: 'pete-pastor', ratio: 1, contributionType: 'maintainer' },
    ]);
  });

  it('falls back to the bootstrap steward when there is nothing to honor', () => {
    const resolved = resolveStewardship('some-unmapped-category', undefined, presenceIdByHumanId);
    expect(resolved.provenance).toBe('default');
    expect(resolved.stewards).toEqual([{ presenceId: 'matthew-dowell', ratio: 1.0 }]);
  });

  it('falls back to the bootstrap steward when the declaration is unresolvable', () => {
    const resolved = resolveStewardship(
      'some-unmapped-category',
      [{ humanId: 'human-georgina-grocer', affinity: 1, role: 'steward' }],
      presenceIdByHumanId
    );
    expect(resolved.provenance).toBe('default');
    expect(resolved.stewards).toEqual([{ presenceId: 'matthew-dowell', ratio: 1.0 }]);
  });

  it('records provenance in the allocation note', () => {
    expect(allocationNote('authored', 'protocol-surface')).toMatch(/Authored stewardship/);
    expect(allocationNote('category', 'governance')).toBe(
      'Affinity-based stewardship for governance content'
    );
    expect(allocationNote('default', undefined)).toMatch(/Bootstrap steward assignment/);
  });
});

describe('elohim-host-landing allocation (the Track B target)', () => {
  const index = buildContentIndex();
  const landing = index.get('elohim-host-landing');

  it('reads the landing content stewardedBy off disk', () => {
    expect(landing).toBeDefined();
    expect(landing!.category).toBe('protocol-surface');
    expect(landing!.stewardedBy).toEqual([
      { humanId: 'human-matthew-manager', affinity: 1, role: 'author' },
    ]);
  });

  it('allocates from the authored declaration, not the bootstrap fallback', () => {
    const resolved = resolveStewardship(
      landing!.category,
      landing!.stewardedBy,
      presenceIdByHumanId
    );
    expect(resolved.provenance).toBe('authored');
    expect(resolved.stewards).toEqual([
      { presenceId: 'matthew-dowell', ratio: 1, contributionType: 'original_creator' },
    ]);
  });
});

describe('blast radius of the authored path', () => {
  // Anti-drift flag. The authored path is deliberately scoped to displace only
  // the bootstrap default; this test pins how many content items that moves, so
  // a future edit to CATEGORY_STEWARD_MAP or to the annotation pass cannot
  // quietly rewrite the stewardship graph.
  const index = buildContentIndex();

  it('changes only content that was previously default-allocated', () => {
    const authoredIds: string[] = [];
    for (const [id, facts] of index) {
      const resolved = resolveStewardship(facts.category, facts.stewardedBy, presenceIdByHumanId);
      if (resolved.provenance === 'authored') authoredIds.push(id);
      // An authored resolution is only ever reached when the category is
      // unmapped — i.e. the item used to get matthew-dowell @ 1.0.
      if (resolved.provenance === 'authored') {
        expect(
          resolveStewardship(facts.category, undefined, presenceIdByHumanId).provenance
        ).toBe('default');
      }
    }
    expect(authoredIds).toContain('elohim-host-landing');
    // Measured 2026-07-31: 50 items gain a real second steward, plus the
    // landing surface which keeps matthew-only and gains provenance.
    expect(authoredIds.length).toBeLessThan(120);
  });

  it('leaves every category-mapped item untouched', () => {
    for (const [, facts] of index) {
      if (!facts.category) continue;
      const withAuthored = resolveStewardship(
        facts.category,
        facts.stewardedBy,
        presenceIdByHumanId
      );
      const withoutAuthored = resolveStewardship(facts.category, undefined, presenceIdByHumanId);
      if (withoutAuthored.provenance === 'category') {
        expect(withAuthored.stewards).toEqual(withoutAuthored.stewards);
      }
    }
  });
});
