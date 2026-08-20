import { describe, expect, it } from 'vitest';

import {
  allocationNote,
  buildContentIndex,
  buildPresenceIdByHumanId,
  contributionTypeForRole,
  indexStoredStewardsByContent,
  loadAllPresences,
  planContentAllocations,
  resolveAuthoredStewards,
  resolveStewardship,
  unresolvedStoredStewardsNotice,
} from '../seed-stewardship.js';

import type { ResolvedStewardship, StoredAllocation } from '../seed-stewardship.js';

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

describe('planContentAllocations — ratio reconciliation', () => {
  const authored: ResolvedStewardship = {
    provenance: 'authored',
    stewards: [
      { presenceId: 'matthew-dowell', ratio: 2 / 3, contributionType: 'original_creator' },
      { presenceId: 'pete-pastor', ratio: 1 / 3, contributionType: 'curator' },
    ],
  };

  /** The exact row shape alpha holds for the 50 drifted items. */
  const staleMatthew: StoredAllocation = {
    id: 'alloc-matthew',
    contentId: 'unit-appeals-process',
    stewardPresenceId: 'matthew-dowell',
    allocationRatio: 1.0,
    allocationMethod: 'computed',
    contributionType: 'curator',
    governanceState: 'active',
    disputeId: null,
    elohimRatifiedAt: null,
    note: 'Bootstrap steward assignment - uncategorized content',
  };

  it('repairs the stale bootstrap ratio that makes the sum 1.333', () => {
    // The observed defect: matthew@1.0 (seeded when the item was
    // default-allocated) + pete@0.333 (added later by the authored path)
    // = 1.333. Insert-only idempotency can never see it.
    const plan = planContentAllocations(
      'unit-appeals-process',
      authored,
      undefined,
      new Map([['matthew-dowell', staleMatthew]])
    );

    expect(plan.creates.map((c) => c.stewardPresenceId)).toEqual(['pete-pastor']);
    expect(plan.repairs).toHaveLength(1);
    expect(plan.repairs[0]).toMatchObject({
      allocationId: 'alloc-matthew',
      stewardPresenceId: 'matthew-dowell',
      fromRatio: 1.0,
      contributionType: 'original_creator',
    });
    expect(plan.repairs[0].toRatio).toBeCloseTo(2 / 3, 6);

    // The whole point: after the plan is applied the item sums to 1.0.
    const applied =
      plan.repairs[0].toRatio + plan.creates[0].allocationRatio;
    expect(applied).toBeCloseTo(1.0, 6);
  });

  it('repairs the stale provenance note along with the ratio', () => {
    const plan = planContentAllocations(
      'unit-appeals-process',
      authored,
      undefined,
      new Map([['matthew-dowell', staleMatthew]])
    );
    expect(plan.repairs[0].note).toBe(allocationNote('authored', undefined));
    expect(plan.repairs[0].note).not.toBe(staleMatthew.note);
  });

  it('is a no-op when every stored row already matches the resolved set', () => {
    const plan = planContentAllocations(
      'unit-appeals-process',
      authored,
      undefined,
      new Map([
        [
          'matthew-dowell',
          { ...staleMatthew, allocationRatio: 2 / 3, contributionType: 'original_creator' },
        ],
        [
          'pete-pastor',
          {
            ...staleMatthew,
            id: 'alloc-pete',
            stewardPresenceId: 'pete-pastor',
            allocationRatio: 1 / 3,
            contributionType: 'curator',
          },
        ],
      ])
    );
    expect(plan.creates).toHaveLength(0);
    expect(plan.repairs).toHaveLength(0);
    expect(plan.governedSkips).toHaveLength(0);
  });

  it('treats an f32 round-trip as the same ratio, not drift', () => {
    // Storage stores allocation_ratio as f32: 2/3 reads back as
    // 0.6666666865348816. Re-writing that every run would be churn, not repair.
    const plan = planContentAllocations(
      'unit-appeals-process',
      authored,
      undefined,
      new Map([
        [
          'matthew-dowell',
          {
            ...staleMatthew,
            allocationRatio: 0.6666666865348816,
            contributionType: 'original_creator',
          },
        ],
      ])
    );
    expect(plan.repairs).toHaveLength(0);
  });

  it('refuses to overwrite a row the governance plane owns', () => {
    // A disputed / ratified / non-active allocation is a negotiated outcome.
    // The seeder reports the drift and leaves the row alone.
    for (const governed of [
      { disputeId: 'dispute-1' },
      { elohimRatifiedAt: '2026-07-01 00:00:00' },
      { governanceState: 'disputed' },
    ]) {
      const plan = planContentAllocations(
        'unit-appeals-process',
        authored,
        undefined,
        new Map([['matthew-dowell', { ...staleMatthew, ...governed }]])
      );
      expect(plan.repairs).toHaveLength(0);
      expect(plan.governedSkips).toHaveLength(1);
    }
  });

  it('refuses to overwrite a human-authored allocation method', () => {
    // `manual` is the column default AND what the app writes for every
    // UI-created split; `negotiated` is a governance outcome. Neither is this
    // seeder's output, so neither is the seeder's to repair — even though such
    // a row is active, undisputed and unratified, and therefore invisible to
    // all three isGovernanceOwned() checks.
    //
    // staleMatthew's ratio DOES drift, so without the method guard each of
    // these is repaired to `computed` with the affinity ratio and the seeder's
    // note — silently discarding the human's split.
    for (const allocationMethod of ['manual', 'negotiated']) {
      const plan = planContentAllocations(
        'unit-appeals-process',
        authored,
        undefined,
        new Map([['matthew-dowell', { ...staleMatthew, allocationMethod }]])
      );
      expect(plan.repairs).toHaveLength(0);
      expect(plan.governedSkips).toHaveLength(1);
    }
  });

  it('keeps every category-allocated item a pure no-op', () => {
    // The curated affinity graph the a2o scenarios assert on must not move.
    const category = resolveStewardship('value-scanner', undefined, presenceIdByHumanId);
    const stored = new Map<string, StoredAllocation>(
      category.stewards.map((s, i) => [
        s.presenceId,
        {
          ...staleMatthew,
          id: `alloc-${i}`,
          contentId: 'some-value-scanner-item',
          stewardPresenceId: s.presenceId,
          allocationRatio: s.ratio,
          contributionType: 'curator',
        },
      ])
    );
    const plan = planContentAllocations(
      'some-value-scanner-item',
      category,
      'value-scanner',
      stored
    );
    expect(plan.creates).toHaveLength(0);
    expect(plan.repairs).toHaveLength(0);
  });
});

describe('indexStoredStewardsByContent', () => {
  // The reconciler counts, per content item, the stewards stored against it
  // that are no longer in any resolved set. It used to answer that by scanning
  // EVERY allocation key for a `${contentId}::` prefix, once per content item —
  // O(content x allocations), ~10^8 iterations on alpha, for a number that is
  // only logged. `indexStoredStewardsByContent` answers it in one pass.
  //
  // These tests pin EQUIVALENCE, not speed: the oracle below is the exact
  // prefix scan that was replaced, and the count must match it row for row.

  /** The pre-optimisation implementation, verbatim, as the test oracle. */
  function countByPrefixScan(
    allContentIds: readonly string[],
    existing: ReadonlyMap<string, StoredAllocation>,
    resolvedIdsFor: (contentId: string) => Set<string>
  ): number {
    let unresolved = 0;
    for (const contentId of allContentIds) {
      const resolvedIds = resolvedIdsFor(contentId);
      for (const key of existing.keys()) {
        if (!key.startsWith(`${contentId}::`)) continue;
        const stewardId = key.slice(contentId.length + 2);
        if (!resolvedIds.has(stewardId)) unresolved++;
      }
    }
    return unresolved;
  }

  /** The optimised implementation's counting loop, as `main()` runs it. */
  function countByIndex(
    allContentIds: readonly string[],
    existing: ReadonlyMap<string, StoredAllocation>,
    resolvedIdsFor: (contentId: string) => Set<string>
  ): number {
    const byContent = indexStoredStewardsByContent(existing);
    let unresolved = 0;
    for (const contentId of allContentIds) {
      const resolvedIds = resolvedIdsFor(contentId);
      for (const stewardId of byContent.get(contentId) ?? new Set<string>()) {
        if (!resolvedIds.has(stewardId)) unresolved++;
      }
    }
    return unresolved;
  }

  function row(contentId: string, stewardPresenceId: string): StoredAllocation {
    return {
      id: `alloc-${contentId}-${stewardPresenceId}`,
      contentId,
      stewardPresenceId,
      allocationRatio: 1.0,
      allocationMethod: 'computed',
      contributionType: 'curator',
    };
  }

  /** Keyed exactly as StewardshipClient.getContentWithAllocations keys it. */
  function storedMap(...rows: StoredAllocation[]): Map<string, StoredAllocation> {
    return new Map(rows.map((r) => [`${r.contentId}::${r.stewardPresenceId}`, r]));
  }

  it('inverts the flat allocation map into content -> stored steward ids', () => {
    const existing = storedMap(
      row('unit-appeals-process', 'matthew-dowell'),
      row('unit-appeals-process', 'pete-pastor'),
      row('some-value-scanner-item', 'adam-firstman')
    );
    const byContent = indexStoredStewardsByContent(existing);

    expect(byContent.size).toBe(2);
    expect([...(byContent.get('unit-appeals-process') ?? [])].sort()).toEqual([
      'matthew-dowell',
      'pete-pastor',
    ]);
    expect([...(byContent.get('some-value-scanner-item') ?? [])]).toEqual(['adam-firstman']);
    // Content with no stored rows is absent, not an empty set — the caller
    // falls back to a shared empty set rather than allocating one per item.
    expect(byContent.get('never-allocated')).toBeUndefined();
  });

  it('counts exactly what the prefix scan counted', () => {
    const allContentIds = [
      'unit-appeals-process',
      'some-value-scanner-item',
      'never-allocated', // no stored rows at all
      'orphan-free-item', // stored rows exist, all resolved
    ];
    const existing = storedMap(
      row('unit-appeals-process', 'matthew-dowell'), // resolved
      row('unit-appeals-process', 'pete-pastor'), // NOT resolved -> counts
      row('unit-appeals-process', 'frank-farmer'), // NOT resolved -> counts
      row('some-value-scanner-item', 'adam-firstman'), // resolved
      row('some-value-scanner-item', 'jessica-spouse'), // NOT resolved -> counts
      row('orphan-free-item', 'matthew-dowell'), // resolved
      // A row for a content id storage still holds but the run never listed.
      // Neither implementation may attribute it to any listed content item.
      row('deleted-content-item', 'matthew-dowell')
    );
    const resolved = new Map<string, Set<string>>([
      ['unit-appeals-process', new Set(['matthew-dowell'])],
      ['some-value-scanner-item', new Set(['adam-firstman'])],
      ['never-allocated', new Set(['matthew-dowell'])],
      ['orphan-free-item', new Set(['matthew-dowell'])],
    ]);
    const resolvedIdsFor = (contentId: string) => resolved.get(contentId) ?? new Set<string>();

    expect(countByIndex(allContentIds, existing, resolvedIdsFor)).toBe(3);
    expect(countByIndex(allContentIds, existing, resolvedIdsFor)).toBe(
      countByPrefixScan(allContentIds, existing, resolvedIdsFor)
    );
  });

  it('agrees with the prefix scan on ids that share a prefix', () => {
    // `startsWith` is a prefix test, so a content id that is a proper prefix of
    // another ('unit-appeals' vs 'unit-appeals-process') is the case where a
    // naive key-splitting rewrite would diverge. Reading stewardPresenceId off
    // the row cannot: the two implementations must still agree.
    const allContentIds = ['unit-appeals', 'unit-appeals-process'];
    const existing = storedMap(
      row('unit-appeals', 'matthew-dowell'),
      row('unit-appeals-process', 'pete-pastor')
    );
    const resolvedIdsFor = () => new Set<string>(['matthew-dowell']);

    expect(countByIndex(allContentIds, existing, resolvedIdsFor)).toBe(
      countByPrefixScan(allContentIds, existing, resolvedIdsFor)
    );
    expect(countByIndex(allContentIds, existing, resolvedIdsFor)).toBe(1);
  });

  it('agrees with the prefix scan across a randomised corpus', () => {
    // Alpha-shaped fan-out (many content items, a small steward pool) sampled
    // widely enough that any off-by-one in the inversion shows up.
    const stewardPool = ['matthew-dowell', 'adam-firstman', 'jessica-spouse', 'pete-pastor'];
    const allContentIds = Array.from({ length: 120 }, (_, i) => `content-${i}`);
    const rows: StoredAllocation[] = [];
    const resolved = new Map<string, Set<string>>();
    // Deterministic pseudo-random (no seedable RNG in vitest by default).
    let seed = 20260820;
    const next = () => {
      seed = (seed * 1103515245 + 12345) % 2147483648;
      return seed / 2147483648;
    };
    for (const contentId of allContentIds) {
      const resolvedIds = new Set<string>();
      for (const steward of stewardPool) {
        if (next() < 0.5) rows.push(row(contentId, steward));
        if (next() < 0.5) resolvedIds.add(steward);
      }
      resolved.set(contentId, resolvedIds);
    }
    // Rows for content the run no longer lists, plus a listed-but-unstored item.
    rows.push(row('content-retired', 'matthew-dowell'));
    const existing = storedMap(...rows);
    const resolvedIdsFor = (contentId: string) => resolved.get(contentId) ?? new Set<string>();

    const expected = countByPrefixScan(allContentIds, existing, resolvedIdsFor);
    expect(expected).toBeGreaterThan(0);
    expect(countByIndex(allContentIds, existing, resolvedIdsFor)).toBe(expected);
  });
});

describe('unresolvedStoredStewardsNotice', () => {
  it('reports the count in the run log, verbatim', () => {
    // The optimisation must not move the operator-visible line by one byte.
    expect(unresolvedStoredStewardsNotice(42)).toBe(
      '   Stored stewards no longer in any resolved set: 42 ' +
        '(retirement is a supersede decision — not written here)'
    );
  });

  it('says nothing when there is no drift', () => {
    expect(unresolvedStoredStewardsNotice(0)).toBeNull();
    expect(unresolvedStoredStewardsNotice(-1)).toBeNull();
  });
});
