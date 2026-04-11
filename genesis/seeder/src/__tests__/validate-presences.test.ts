import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, writeFile, mkdir, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  validatePresenceFrontmatter,
  validateObservationEntry,
  validatePresencesDirectory,
  PRESENCE_TYPES,
  EXTERNAL_ID_TYPES,
  WORK_KINDS,
} from '../validate-presences.js';

// ---------------------------------------------------------------------------
// Per-presence frontmatter validation
// ---------------------------------------------------------------------------

describe('validatePresenceFrontmatter', () => {
  const validObservation = () => ({
    observerId: 'human-alice',
    observedAt: '2026-04-10T12:00:00Z',
    context: 'read in Consilience Garden',
  });

  const valid = () => ({
    id: 'presence-virginia-eubanks',
    displayName: 'Virginia Eubanks',
    presenceType: 'person',
    observations: [validObservation()],
  });

  it('accepts a minimal valid presence', () => {
    const result = validatePresenceFrontmatter(valid() as any, 'virginia-eubanks.md');
    expect(result.errors).toEqual([]);
  });

  it('rejects missing required fields', () => {
    const result = validatePresenceFrontmatter({} as any, 'virginia-eubanks.md');
    expect(result.errors.some(e => e.includes('id'))).toBe(true);
    expect(result.errors.some(e => e.includes('displayName'))).toBe(true);
    expect(result.errors.some(e => e.includes('presenceType'))).toBe(true);
    expect(result.errors.some(e => e.includes('observations'))).toBe(true);
  });

  it('rejects id not matching slug pattern', () => {
    const fm = { ...valid(), id: 'presence_virginia_eubanks' };
    const result = validatePresenceFrontmatter(fm as any, 'virginia-eubanks.md');
    expect(result.errors.some(e => e.includes('slug'))).toBe(true);
  });

  it('rejects id without presence- prefix', () => {
    const fm = { ...valid(), id: 'virginia-eubanks' };
    const result = validatePresenceFrontmatter(fm as any, 'virginia-eubanks.md');
    expect(result.errors.some(e => e.includes('slug'))).toBe(true);
  });

  it('rejects id not matching filename stem', () => {
    const fm = { ...valid(), id: 'presence-wrong-name' };
    const result = validatePresenceFrontmatter(fm as any, 'virginia-eubanks.md');
    expect(result.errors.some(e => e.includes('filename'))).toBe(true);
  });

  it('accepts valid id/filename match', () => {
    const result = validatePresenceFrontmatter(valid() as any, 'virginia-eubanks.md');
    expect(result.errors.filter(e => e.includes('filename'))).toEqual([]);
  });

  it('rejects presenceType not in enum', () => {
    const fm = { ...valid(), presenceType: 'robot' };
    const result = validatePresenceFrontmatter(fm as any, 'virginia-eubanks.md');
    expect(result.errors.some(e => e.includes('presenceType'))).toBe(true);
  });

  it('accepts presenceType organization', () => {
    const fm = { ...valid(), presenceType: 'organization' };
    const result = validatePresenceFrontmatter(fm as any, 'virginia-eubanks.md');
    expect(result.errors).toEqual([]);
  });

  it('rejects observations empty array', () => {
    const fm = { ...valid(), observations: [] };
    const result = validatePresenceFrontmatter(fm as any, 'virginia-eubanks.md');
    expect(result.errors.some(e => e.includes('observations'))).toBe(true);
  });

  it('rejects observation missing observerId', () => {
    const fm = {
      ...valid(),
      observations: [{ observedAt: '2026-04-10T12:00:00Z', context: 'x' }],
    };
    const result = validatePresenceFrontmatter(fm as any, 'virginia-eubanks.md');
    expect(result.errors.some(e => e.includes('observerId'))).toBe(true);
  });

  it('rejects observation missing observedAt', () => {
    const fm = {
      ...valid(),
      observations: [{ observerId: 'human-alice', context: 'x' }],
    };
    const result = validatePresenceFrontmatter(fm as any, 'virginia-eubanks.md');
    expect(result.errors.some(e => e.includes('observedAt'))).toBe(true);
  });

  it('rejects observation missing context', () => {
    const fm = {
      ...valid(),
      observations: [
        { observerId: 'human-alice', observedAt: '2026-04-10T12:00:00Z' },
      ],
    };
    const result = validatePresenceFrontmatter(fm as any, 'virginia-eubanks.md');
    expect(result.errors.some(e => e.includes('context'))).toBe(true);
  });

  it('accepts observation with null contextContentId', () => {
    const fm = {
      ...valid(),
      observations: [{ ...validObservation(), contextContentId: null }],
    };
    const result = validatePresenceFrontmatter(fm as any, 'virginia-eubanks.md');
    expect(result.errors).toEqual([]);
  });

  it('rejects externalIdentifiers with unknown type', () => {
    const fm = {
      ...valid(),
      externalIdentifiers: [{ type: 'facebook', value: 'vaeubanks' }],
    };
    const result = validatePresenceFrontmatter(fm as any, 'virginia-eubanks.md');
    expect(result.errors.some(e => e.includes('externalIdentifiers'))).toBe(true);
  });

  it('accepts externalIdentifiers with known type', () => {
    const fm = {
      ...valid(),
      externalIdentifiers: [{ type: 'orcid', value: '0000-0001-2345-6789' }],
    };
    const result = validatePresenceFrontmatter(fm as any, 'virginia-eubanks.md');
    expect(result.errors).toEqual([]);
  });

  it('rejects works with unknown kind', () => {
    const fm = {
      ...valid(),
      works: [{ title: 'Automating Inequality', kind: 'manifesto' }],
    };
    const result = validatePresenceFrontmatter(fm as any, 'virginia-eubanks.md');
    expect(result.errors.some(e => e.includes('works'))).toBe(true);
  });

  it('accepts works with valid kind and null year', () => {
    const fm = {
      ...valid(),
      works: [{ title: 'Automating Inequality', kind: 'book', year: null }],
    };
    const result = validatePresenceFrontmatter(fm as any, 'virginia-eubanks.md');
    expect(result.errors).toEqual([]);
  });

  it('rejects image.local not matching pattern', () => {
    const fm = {
      ...valid(),
      image: { local: 'pictures/virginia.png' },
    };
    const result = validatePresenceFrontmatter(fm as any, 'virginia-eubanks.md');
    expect(result.errors.some(e => e.includes('image.local'))).toBe(true);
  });

  it('accepts image with placeholder true and non-null sourceUrl', () => {
    const fm = {
      ...valid(),
      image: {
        local: 'images/virginia-eubanks.webp',
        placeholder: true,
        sourceUrl: 'https://example.com/photo.jpg',
      },
    };
    const result = validatePresenceFrontmatter(fm as any, 'virginia-eubanks.md');
    expect(result.errors).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// Observation referential integrity
// ---------------------------------------------------------------------------

describe('validateObservationEntry referential integrity', () => {
  const obs = () => ({
    observerId: 'human-alice',
    observedAt: '2026-04-10T12:00:00Z',
    context: 'read it',
  });

  it('reports observer not in knownHumanIds', () => {
    const errors = validateObservationEntry(obs() as any, 0, 'presence-test', {
      knownHumanIds: new Set(['human-bob']),
    });
    expect(errors.some(e => e.includes('observerId') && e.includes('human-alice'))).toBe(true);
  });

  it('accepts observer in knownHumanIds', () => {
    const errors = validateObservationEntry(obs() as any, 0, 'presence-test', {
      knownHumanIds: new Set(['human-alice']),
    });
    expect(errors).toEqual([]);
  });

  it('reports contextContentId not in knownContentIds', () => {
    const entry = { ...obs(), contextContentId: 'content-ghost' };
    const errors = validateObservationEntry(entry as any, 0, 'presence-test', {
      knownContentIds: new Set(['content-real']),
    });
    expect(errors.some(e => e.includes('contextContentId') && e.includes('content-ghost'))).toBe(true);
  });

  it('accepts null contextContentId even when knownContentIds given', () => {
    const entry = { ...obs(), contextContentId: null };
    const errors = validateObservationEntry(entry as any, 0, 'presence-test', {
      knownContentIds: new Set(['content-real']),
    });
    expect(errors).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// Exported vocabularies
// ---------------------------------------------------------------------------

describe('exported vocabularies', () => {
  it('PRESENCE_TYPES has person and organization', () => {
    expect(PRESENCE_TYPES).toContain('person');
    expect(PRESENCE_TYPES).toContain('organization');
    expect(PRESENCE_TYPES.length).toBe(2);
  });

  it('EXTERNAL_ID_TYPES has 14 entries', () => {
    expect(EXTERNAL_ID_TYPES.length).toBe(14);
    expect(EXTERNAL_ID_TYPES).toContain('orcid');
    expect(EXTERNAL_ID_TYPES).toContain('email');
  });

  it('WORK_KINDS has 9 entries', () => {
    expect(WORK_KINDS.length).toBe(9);
    expect(WORK_KINDS).toContain('book');
    expect(WORK_KINDS).toContain('other');
  });
});

// ---------------------------------------------------------------------------
// Directory-level validation
// ---------------------------------------------------------------------------

describe('validatePresencesDirectory', () => {
  let tmpDir: string;

  beforeEach(async () => {
    tmpDir = await mkdtemp(join(tmpdir(), 'presences-test-'));
  });

  afterEach(async () => {
    await rm(tmpDir, { recursive: true, force: true });
  });

  const makePresenceFile = async (
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

  const minimalFm = (id: string, displayName: string) => ({
    id,
    displayName,
    presenceType: 'person',
    observations: [
      {
        observerId: 'human-alice',
        observedAt: '2026-04-10T12:00:00Z',
        context: 'read it',
      },
    ],
  });

  it('accepts a minimal valid directory', async () => {
    await makePresenceFile('virginia-eubanks.md', minimalFm('presence-virginia-eubanks', 'Virginia Eubanks'));
    const result = await validatePresencesDirectory(tmpDir);
    expect(result.errors).toEqual([]);
    expect(result.presences.size).toBe(1);
  });

  it('detects duplicate presence ids', async () => {
    await makePresenceFile('a.md', minimalFm('presence-a', 'A One'));
    await makePresenceFile('b.md', { ...minimalFm('presence-a', 'A Two') });
    const result = await validatePresencesDirectory(tmpDir);
    expect(result.errors.some(e => e.includes('duplicate id'))).toBe(true);
  });

  it('detects duplicate (type, value) external identifier pairs across presences', async () => {
    await makePresenceFile('alice.md', {
      ...minimalFm('presence-alice', 'Alice'),
      externalIdentifiers: [{ type: 'orcid', value: '0000-0001-2345-6789' }],
    });
    await makePresenceFile('bob.md', {
      ...minimalFm('presence-bob', 'Bob'),
      externalIdentifiers: [{ type: 'orcid', value: '0000-0001-2345-6789' }],
    });
    const result = await validatePresencesDirectory(tmpDir);
    expect(result.errors.some(e => e.includes('duplicate') && e.includes('orcid'))).toBe(true);
  });

  it('accepts two presences with distinct external identifiers', async () => {
    await makePresenceFile('alice.md', {
      ...minimalFm('presence-alice', 'Alice'),
      externalIdentifiers: [{ type: 'orcid', value: '0000-0001-1111-1111' }],
    });
    await makePresenceFile('bob.md', {
      ...minimalFm('presence-bob', 'Bob'),
      externalIdentifiers: [{ type: 'orcid', value: '0000-0002-2222-2222' }],
    });
    const result = await validatePresencesDirectory(tmpDir);
    expect(result.errors).toEqual([]);
  });

  it('reports missing image file when imagesDir is given', async () => {
    await makePresenceFile('virginia.md', {
      ...minimalFm('presence-virginia', 'Virginia'),
      image: { local: 'images/virginia.webp' },
    });
    const result = await validatePresencesDirectory(tmpDir, { imagesDir: join(tmpDir, 'images') });
    expect(result.errors.some(e => e.includes('image') && e.includes('virginia.webp'))).toBe(true);
  });

  it('accepts existing image file when imagesDir is given', async () => {
    await mkdir(join(tmpDir, 'images'), { recursive: true });
    await writeFile(join(tmpDir, 'images', 'virginia.webp'), 'fake-webp-bytes');
    await makePresenceFile('virginia.md', {
      ...minimalFm('presence-virginia', 'Virginia'),
      image: { local: 'images/virginia.webp' },
    });
    const result = await validatePresencesDirectory(tmpDir, { imagesDir: join(tmpDir, 'images') });
    expect(result.errors).toEqual([]);
  });

  it('handles missing image field gracefully', async () => {
    await makePresenceFile('virginia.md', minimalFm('presence-virginia', 'Virginia'));
    const result = await validatePresencesDirectory(tmpDir, { imagesDir: join(tmpDir, 'images') });
    expect(result.errors).toEqual([]);
  });

  it('accepts multiple observations with the same observerId', async () => {
    await makePresenceFile('virginia.md', {
      id: 'presence-virginia',
      displayName: 'Virginia',
      presenceType: 'person',
      observations: [
        { observerId: 'human-alice', observedAt: '2026-04-10T12:00:00Z', context: 'first encounter' },
        { observerId: 'human-alice', observedAt: '2026-04-11T12:00:00Z', context: 'second encounter' },
      ],
    });
    const result = await validatePresencesDirectory(tmpDir);
    expect(result.errors).toEqual([]);
  });

  it('propagates knownHumanIds to observation validation', async () => {
    await makePresenceFile('virginia.md', minimalFm('presence-virginia', 'Virginia'));
    const result = await validatePresencesDirectory(tmpDir, {
      knownHumanIds: new Set(['human-bob']),
    });
    expect(result.errors.some(e => e.includes('observerId') && e.includes('human-alice'))).toBe(true);
  });

  it('propagates knownContentIds to works citedInContentIds validation', async () => {
    await makePresenceFile('virginia.md', {
      ...minimalFm('presence-virginia', 'Virginia'),
      works: [
        {
          title: 'Automating Inequality',
          kind: 'book',
          citedInContentIds: ['content-ghost'],
        },
      ],
    });
    const result = await validatePresencesDirectory(tmpDir, {
      knownContentIds: new Set(['content-real']),
    });
    expect(result.errors.some(e => e.includes('citedInContentIds') && e.includes('content-ghost'))).toBe(true);
  });

  it('skips relationships.md and README.md', async () => {
    await makePresenceFile('virginia.md', minimalFm('presence-virginia', 'Virginia'));
    await writeFile(join(tmpDir, 'README.md'), '# Presences\n');
    await writeFile(join(tmpDir, 'relationships.md'), '---\n---\n');
    const result = await validatePresencesDirectory(tmpDir);
    expect(result.errors).toEqual([]);
    expect(result.presences.size).toBe(1);
  });
});
