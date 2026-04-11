import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  validateHumanFrontmatter,
  validateRelationshipEntry,
  validateHumansDirectory,
  validateA2oNarrativeCoherence,
  HUMAN_RELATIONSHIP_TYPES,
  type HumanFrontmatter,
} from '../validate-humans.js';

// ---------------------------------------------------------------------------
// Per-human frontmatter validation
// ---------------------------------------------------------------------------

describe('validateHumanFrontmatter', () => {
  const valid = () => ({
    id: 'human-test-user',
    displayName: 'Test',
    category: 'core-family',
    profileReach: 'community',
  });

  it('accepts a minimal valid human', () => {
    const result = validateHumanFrontmatter(valid(), 'test-user.md');
    expect(result.errors).toEqual([]);
  });

  it('rejects missing required fields', () => {
    const result = validateHumanFrontmatter({} as any, 'test.md');
    expect(result.errors.some(e => e.includes("id"))).toBe(true);
    expect(result.errors.some(e => e.includes("displayName"))).toBe(true);
    expect(result.errors.some(e => e.includes("category"))).toBe(true);
    expect(result.errors.some(e => e.includes("profileReach"))).toBe(true);
  });

  it('rejects id not matching slug pattern', () => {
    const fm = { ...valid(), id: 'human_test_user' };
    const result = validateHumanFrontmatter(fm, 'test-user.md');
    expect(result.errors.some(e => e.includes('slug'))).toBe(true);
  });

  it('rejects id not matching filename stem', () => {
    const fm = { ...valid(), id: 'human-wrong-name' };
    const result = validateHumanFrontmatter(fm, 'test-user.md');
    expect(result.errors.some(e => e.includes('filename'))).toBe(true);
  });

  it('accepts valid filename/id match', () => {
    const result = validateHumanFrontmatter(valid(), 'test-user.md');
    expect(result.errors.filter(e => e.includes('filename'))).toEqual([]);
  });

  it('rejects unknown category', () => {
    const fm = { ...valid(), category: 'not-a-category' };
    const result = validateHumanFrontmatter(fm, 'test-user.md');
    expect(result.errors.some(e => e.includes('category'))).toBe(true);
  });

  it('rejects unknown profileReach', () => {
    const fm = { ...valid(), profileReach: 'unknown-reach' };
    const result = validateHumanFrontmatter(fm, 'test-user.md');
    expect(result.errors.some(e => e.includes('profileReach'))).toBe(true);
  });

  it('rejects unknown agencyPhase', () => {
    const fm = { ...valid(), agencyPhase: 'spaceship' };
    const result = validateHumanFrontmatter(fm as any, 'test-user.md');
    expect(result.errors.some(e => e.includes('agencyPhase'))).toBe(true);
  });

  it('accepts null agencyPhase', () => {
    const fm = { ...valid(), agencyPhase: null };
    const result = validateHumanFrontmatter(fm as any, 'test-user.md');
    expect(result.errors.filter(e => e.includes('agencyPhase'))).toEqual([]);
  });

  it('rejects location with invalid layer', () => {
    const fm = { ...valid(), location: { layer: 'galaxy', name: 'Andromeda' } };
    const result = validateHumanFrontmatter(fm as any, 'test-user.md');
    expect(result.errors.some(e => e.includes('location.layer'))).toBe(true);
  });

  it('rejects claimedAttestations with invalid status', () => {
    const fm = {
      ...valid(),
      claimedAttestations: [{ claim: 'MD', status: 'maybe' }],
    };
    const result = validateHumanFrontmatter(fm as any, 'test-user.md');
    expect(result.errors.some(e => e.includes('claimedAttestations'))).toBe(true);
  });

  it('warns when minor has no guardianIds (Tiffany red-team case)', () => {
    const fm = { ...valid(), ageCategory: 'minor', guardianIds: [] };
    const result = validateHumanFrontmatter(fm as any, 'test-user.md');
    expect(result.warnings.some(w => w.includes('minor'))).toBe(true);
    expect(result.errors.filter(e => e.includes('guardian'))).toEqual([]);
  });

  it('does not warn when adult has no guardianIds', () => {
    const fm = { ...valid(), ageCategory: 'adult' };
    const result = validateHumanFrontmatter(fm as any, 'test-user.md');
    expect(result.warnings.filter(w => w.includes('minor'))).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// Relationship validation
// ---------------------------------------------------------------------------

describe('validateRelationshipEntry', () => {
  const valid = () => ({
    source: 'human-alice',
    target: 'human-bob',
    relationshipType: 'sibling',
    intimacyLevel: 'trusted',
  });

  it('accepts a valid relationship', () => {
    expect(validateRelationshipEntry(valid(), 0)).toEqual([]);
  });

  it('rejects unknown relationship type', () => {
    const rel = { ...valid(), relationshipType: 'bestie' };
    const errors = validateRelationshipEntry(rel, 0);
    expect(errors.some(e => e.includes('relationshipType'))).toBe(true);
  });

  it('rejects unknown intimacy level', () => {
    const rel = { ...valid(), intimacyLevel: 'casual' };
    const errors = validateRelationshipEntry(rel, 0);
    expect(errors.some(e => e.includes('intimacyLevel'))).toBe(true);
  });

  it('requires context for coworker relationships', () => {
    const rel = { ...valid(), relationshipType: 'coworker' };
    const errors = validateRelationshipEntry(rel, 0);
    expect(errors.some(e => e.includes('context'))).toBe(true);
  });

  it('accepts coworker with context', () => {
    const rel = { ...valid(), relationshipType: 'coworker', context: 'org-ethosengine' };
    expect(validateRelationshipEntry(rel, 0)).toEqual([]);
  });

  it('includes key-steward-of in the vocabulary', () => {
    expect(HUMAN_RELATIONSHIP_TYPES).toContain('key-steward-of');
  });

  it('has exactly 22 relationship types', () => {
    expect(HUMAN_RELATIONSHIP_TYPES.length).toBe(22);
  });
});

// ---------------------------------------------------------------------------
// Directory-level validation
// ---------------------------------------------------------------------------

describe('validateHumansDirectory', () => {
  let tmpDir: string;

  beforeEach(async () => {
    tmpDir = await mkdtemp(join(tmpdir(), 'humans-test-'));
  });

  afterEach(async () => {
    await rm(tmpDir, { recursive: true, force: true });
  });

  const makeHumanFile = async (
    filename: string,
    fm: Record<string, unknown>,
  ) => {
    const lines = ['---'];
    for (const [k, v] of Object.entries(fm)) {
      lines.push(`${k}: ${JSON.stringify(v)}`);
    }
    lines.push('---', '', '# Narrative');
    await writeFile(join(tmpDir, filename), lines.join('\n'));
  };

  it('detects duplicate ids', async () => {
    await makeHumanFile('alice.md', {
      id: 'human-alice', displayName: 'A', category: 'core-family', profileReach: 'community',
    });
    await makeHumanFile('bob.md', {
      id: 'human-alice', displayName: 'B', category: 'core-family', profileReach: 'community',
    });
    const result = await validateHumansDirectory(tmpDir);
    expect(result.errors.some(e => e.includes('duplicate id'))).toBe(true);
  });

  it('detects duplicate displayName (Pete aliasing rule)', async () => {
    await makeHumanFile('alice.md', {
      id: 'human-alice', displayName: 'Pete', category: 'core-family', profileReach: 'community',
    });
    await makeHumanFile('bob.md', {
      id: 'human-bob', displayName: 'Pete', category: 'core-family', profileReach: 'community',
    });
    const result = await validateHumansDirectory(tmpDir);
    expect(result.errors.some(e => e.includes('duplicate displayName'))).toBe(true);
  });

  it('detects unresolved guardianIds', async () => {
    await makeHumanFile('kid.md', {
      id: 'human-kid', displayName: 'Kid', category: 'core-family', profileReach: 'community',
      ageCategory: 'minor', guardianIds: ['human-ghost'],
    });
    const result = await validateHumansDirectory(tmpDir);
    expect(result.errors.some(e => e.includes('guardianIds') && e.includes('human-ghost'))).toBe(true);
  });

  it('accepts valid graph', async () => {
    await makeHumanFile('parent.md', {
      id: 'human-parent', displayName: 'Parent', category: 'core-family', profileReach: 'community',
    });
    await makeHumanFile('kid.md', {
      id: 'human-kid', displayName: 'Kid', category: 'core-family', profileReach: 'community',
      ageCategory: 'minor', guardianIds: ['human-parent'],
    });
    const result = await validateHumansDirectory(tmpDir);
    expect(result.errors).toEqual([]);
  });

  it('validates relationships from relationships.md', async () => {
    await makeHumanFile('alice.md', {
      id: 'human-alice', displayName: 'A', category: 'core-family', profileReach: 'community',
    });
    await makeHumanFile('bob.md', {
      id: 'human-bob', displayName: 'B', category: 'core-family', profileReach: 'community',
    });
    const relsContent = [
      '---',
      'relationships:',
      '  - source: human-alice',
      '    target: human-bob',
      '    relationshipType: sibling',
      '    intimacyLevel: trusted',
      '---',
      '',
      '# Relationships',
    ].join('\n');
    await writeFile(join(tmpDir, 'relationships.md'), relsContent);
    const result = await validateHumansDirectory(tmpDir);
    expect(result.errors).toEqual([]);
    expect(result.relationships.length).toBe(1);
  });

  it('detects relationship source not in humans', async () => {
    await makeHumanFile('bob.md', {
      id: 'human-bob', displayName: 'B', category: 'core-family', profileReach: 'community',
    });
    const relsContent = [
      '---',
      'relationships:',
      '  - source: human-phantom',
      '    target: human-bob',
      '    relationshipType: sibling',
      '    intimacyLevel: trusted',
      '---',
    ].join('\n');
    await writeFile(join(tmpDir, 'relationships.md'), relsContent);
    const result = await validateHumansDirectory(tmpDir);
    expect(result.errors.some(e => e.includes('source') && e.includes('human-phantom'))).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Circular guardianship detection
// ---------------------------------------------------------------------------

describe('validateHumansDirectory circular guardianship', () => {
  let tmpDir: string;

  beforeEach(async () => {
    tmpDir = await mkdtemp(join(tmpdir(), 'humans-cycle-test-'));
  });

  afterEach(async () => {
    await rm(tmpDir, { recursive: true, force: true });
  });

  const makeHumanFile = async (
    filename: string,
    fm: Record<string, unknown>,
  ) => {
    const lines = ['---'];
    for (const [k, v] of Object.entries(fm)) {
      lines.push(`${k}: ${JSON.stringify(v)}`);
    }
    lines.push('---', '', '# Narrative');
    await writeFile(join(tmpDir, filename), lines.join('\n'));
  };

  const base = (id: string, displayName: string) => ({
    id,
    displayName,
    category: 'core-family',
    profileReach: 'community',
  });

  it('detects a self-cycle (human guardian of itself)', async () => {
    await makeHumanFile('alice.md', {
      ...base('human-alice', 'Alice'),
      guardianIds: ['human-alice'],
    });
    const result = await validateHumansDirectory(tmpDir);
    expect(
      result.errors.some(e => e.includes('circular guardianship')),
    ).toBe(true);
  });

  it('detects a two-cycle (A -> B -> A)', async () => {
    await makeHumanFile('alice.md', {
      ...base('human-alice', 'Alice'),
      guardianIds: ['human-bob'],
    });
    await makeHumanFile('bob.md', {
      ...base('human-bob', 'Bob'),
      guardianIds: ['human-alice'],
    });
    const result = await validateHumansDirectory(tmpDir);
    const cycleErrors = result.errors.filter(e =>
      e.includes('circular guardianship'),
    );
    expect(cycleErrors.length).toBeGreaterThan(0);
  });

  it('detects a three-cycle (A -> B -> C -> A)', async () => {
    await makeHumanFile('alice.md', {
      ...base('human-alice', 'Alice'),
      guardianIds: ['human-bob'],
    });
    await makeHumanFile('bob.md', {
      ...base('human-bob', 'Bob'),
      guardianIds: ['human-carol'],
    });
    await makeHumanFile('carol.md', {
      ...base('human-carol', 'Carol'),
      guardianIds: ['human-alice'],
    });
    const result = await validateHumansDirectory(tmpDir);
    const cycleErrors = result.errors.filter(e =>
      e.includes('circular guardianship'),
    );
    expect(cycleErrors.length).toBeGreaterThan(0);
  });

  it('detects two disjoint cycles in the same directory', async () => {
    // Cycle 1: alice <-> bob
    await makeHumanFile('alice.md', {
      ...base('human-alice', 'Alice'),
      guardianIds: ['human-bob'],
    });
    await makeHumanFile('bob.md', {
      ...base('human-bob', 'Bob'),
      guardianIds: ['human-alice'],
    });
    // Cycle 2: carol <-> dave
    await makeHumanFile('carol.md', {
      ...base('human-carol', 'Carol'),
      guardianIds: ['human-dave'],
    });
    await makeHumanFile('dave.md', {
      ...base('human-dave', 'Dave'),
      guardianIds: ['human-carol'],
    });
    const result = await validateHumansDirectory(tmpDir);
    const cycleErrors = result.errors.filter(e =>
      e.includes('circular guardianship'),
    );
    // Each cycle should be reported at least once; the two cycles are
    // disjoint so there should be two distinct cycle errors.
    expect(cycleErrors.length).toBeGreaterThanOrEqual(2);
    // Sanity: each cycle's members appear in at least one error
    expect(
      cycleErrors.some(
        e => e.includes('human-alice') && e.includes('human-bob'),
      ),
    ).toBe(true);
    expect(
      cycleErrors.some(
        e => e.includes('human-carol') && e.includes('human-dave'),
      ),
    ).toBe(true);
  });

  it('does NOT flag a non-cycle chain A -> B -> C', async () => {
    await makeHumanFile('alice.md', {
      ...base('human-alice', 'Alice'),
      guardianIds: ['human-bob'],
    });
    await makeHumanFile('bob.md', {
      ...base('human-bob', 'Bob'),
      guardianIds: ['human-carol'],
    });
    await makeHumanFile('carol.md', {
      ...base('human-carol', 'Carol'),
    });
    const result = await validateHumansDirectory(tmpDir);
    expect(
      result.errors.filter(e => e.includes('circular guardianship')),
    ).toEqual([]);
  });

  it('does NOT flag a shared-ancestor graph (A -> C and B -> C)', async () => {
    // Both A and B have C as guardian; C is a shared ancestor but there is
    // no cycle since C has no guardian pointing back.
    await makeHumanFile('alice.md', {
      ...base('human-alice', 'Alice'),
      guardianIds: ['human-carol'],
    });
    await makeHumanFile('bob.md', {
      ...base('human-bob', 'Bob'),
      guardianIds: ['human-carol'],
    });
    await makeHumanFile('carol.md', {
      ...base('human-carol', 'Carol'),
    });
    const result = await validateHumansDirectory(tmpDir);
    expect(
      result.errors.filter(e => e.includes('circular guardianship')),
    ).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// A2O narrative coherence
// ---------------------------------------------------------------------------

describe('validateA2oNarrativeCoherence', () => {
  let tmpDir: string;

  beforeEach(async () => {
    tmpDir = await mkdtemp(join(tmpdir(), 'a2o-test-'));
  });

  afterEach(async () => {
    await rm(tmpDir, { recursive: true, force: true });
  });

  const makeHumans = (): Map<string, HumanFrontmatter> => {
    const m = new Map<string, HumanFrontmatter>();
    m.set('human-matthew', {
      id: 'human-matthew',
      displayName: 'Matthew',
      category: 'core-family',
      profileReach: 'community',
    } as HumanFrontmatter);
    m.set('human-pete', {
      id: 'human-pete',
      displayName: 'Pete',
      category: 'affinity',
      profileReach: 'community',
    } as HumanFrontmatter);
    return m;
  };

  it('flags names that do not resolve to registered humans', async () => {
    await writeFile(
      join(tmpDir, 'bad.feature'),
      'Feature: test\n  Scenario: ...\n    Given human "Ghost" is logged in\n',
    );
    const result = await validateA2oNarrativeCoherence(tmpDir, makeHumans());
    expect(result.unresolved).toContain('Ghost');
  });

  it('allows synthetic scenario names', async () => {
    await writeFile(
      join(tmpDir, 'auth.feature'),
      'Feature: auth\n  Scenario: ...\n    Given human "Alice" is registered\n    And human "Bob" is registered\n',
    );
    const result = await validateA2oNarrativeCoherence(tmpDir, makeHumans());
    expect(result.unresolved).not.toContain('Alice');
    expect(result.unresolved).not.toContain('Bob');
  });

  it('allows registered humans', async () => {
    await writeFile(
      join(tmpDir, 'login.feature'),
      'Feature: test\n  Scenario: ...\n    Given human "Matthew" is logged in\n    And human "Pete" is logged in\n',
    );
    const result = await validateA2oNarrativeCoherence(tmpDir, makeHumans());
    expect(result.unresolved).not.toContain('Matthew');
    expect(result.unresolved).not.toContain('Pete');
  });

  it('recurses into subdirectories', async () => {
    const { mkdir } = await import('node:fs/promises');
    const subdir = join(tmpDir, 'auth');
    await mkdir(subdir, { recursive: true });
    await writeFile(
      join(subdir, 'nested.feature'),
      'Feature: test\n  Scenario: ...\n    Given human "Nested" is logged in\n',
    );
    const result = await validateA2oNarrativeCoherence(tmpDir, makeHumans());
    expect(result.unresolved).toContain('Nested');
  });

  it('returns empty when all names resolve', async () => {
    await writeFile(
      join(tmpDir, 'clean.feature'),
      'Feature: test\n  Scenario: ...\n    Given human "Matthew" is logged in\n',
    );
    const result = await validateA2oNarrativeCoherence(tmpDir, makeHumans());
    expect(result.unresolved).toEqual([]);
  });
});
