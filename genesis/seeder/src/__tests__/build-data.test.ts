import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { buildHumansJson, buildPresencesJson } from '../build-data.js';

describe('buildHumansJson', () => {
  let dir: string;

  beforeEach(async () => {
    dir = await mkdtemp(join(tmpdir(), 'humans-build-'));
  });

  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  const writeHuman = async (file: string, fm: Record<string, unknown>) => {
    const lines = ['---'];
    for (const [k, v] of Object.entries(fm)) {
      lines.push(`${k}: ${JSON.stringify(v)}`);
    }
    lines.push('---', '', '# Narrative');
    await writeFile(join(dir, file), lines.join('\n'));
  };

  it('generates humans.json with sorted humans array', async () => {
    await writeHuman('bob.md', { id: 'human-bob', displayName: 'Bob', category: 'core-family', profileReach: 'community' });
    await writeHuman('alice.md', { id: 'human-alice', displayName: 'Alice', category: 'core-family', profileReach: 'community' });
    const result = await buildHumansJson(dir);
    expect(result.humans).toHaveLength(2);
    expect(result.humans[0].id).toBe('human-alice');
    expect(result.humans[1].id).toBe('human-bob');
  });

  it('includes version and description metadata (deterministic — no timestamps)', async () => {
    await writeHuman('a.md', { id: 'human-alice', displayName: 'Alice', category: 'core-family', profileReach: 'community' });
    const result = await buildHumansJson(dir);
    expect(result.version).toBe('2.0.0');
    expect(result.description).toContain('Generated from');
    // Regression: no timestamps allowed — they break the pre-push freshness diff
    expect((result as Record<string, unknown>).generatedAt).toBeUndefined();
  });

  it('skips relationships.md and README.md from human walk', async () => {
    await writeHuman('alice.md', { id: 'human-alice', displayName: 'Alice', category: 'core-family', profileReach: 'community' });
    await writeFile(join(dir, 'README.md'), '# readme\n');
    await writeFile(join(dir, 'relationships.md'), '---\nrelationships: []\n---\n');
    const result = await buildHumansJson(dir);
    expect(result.humans).toHaveLength(1);
    expect(result.humans[0].id).toBe('human-alice');
  });

  it('parses relationships from relationships.md when present', async () => {
    await writeHuman('alice.md', { id: 'human-alice', displayName: 'Alice', category: 'core-family', profileReach: 'community' });
    await writeHuman('bob.md', { id: 'human-bob', displayName: 'Bob', category: 'core-family', profileReach: 'community' });
    const relsContent = [
      '---',
      'relationships:',
      '  - source: human-alice',
      '    target: human-bob',
      '    relationshipType: sibling',
      '    intimacyLevel: trusted',
      '---',
    ].join('\n');
    await writeFile(join(dir, 'relationships.md'), relsContent);
    const result = await buildHumansJson(dir);
    expect(result.relationships).toHaveLength(1);
    expect(result.relationships[0].relationshipType).toBe('sibling');
  });

  it('returns empty relationships when relationships.md is absent', async () => {
    await writeHuman('alice.md', { id: 'human-alice', displayName: 'Alice', category: 'core-family', profileReach: 'community' });
    const result = await buildHumansJson(dir);
    expect(result.relationships).toEqual([]);
  });

  it('skips files with no parseable frontmatter', async () => {
    await writeHuman('alice.md', { id: 'human-alice', displayName: 'Alice', category: 'core-family', profileReach: 'community' });
    await writeFile(join(dir, 'broken.md'), '# Just a plain markdown file\nno frontmatter here.');
    const result = await buildHumansJson(dir);
    expect(result.humans).toHaveLength(1);
  });
});

describe('buildPresencesJson', () => {
  let dir: string;

  beforeEach(async () => {
    dir = await mkdtemp(join(tmpdir(), 'presences-build-'));
  });

  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  const writePresence = async (file: string, fm: Record<string, unknown>) => {
    const lines = ['---'];
    for (const [k, v] of Object.entries(fm)) {
      lines.push(`${k}: ${JSON.stringify(v)}`);
    }
    lines.push('---', '', '# Narrative');
    await writeFile(join(dir, file), lines.join('\n'));
  };

  it('generates presences.json with sorted presences array', async () => {
    await writePresence('virginia-eubanks.md', {
      id: 'presence-virginia-eubanks',
      displayName: 'Virginia Eubanks',
      presenceType: 'person',
      observations: [{ observerId: 'human-pete-pastor', observedAt: '2026-01-01T00:00:00Z', context: 'Cited' }],
    });
    await writePresence('james-carse.md', {
      id: 'presence-james-p-carse',
      displayName: 'James P. Carse',
      presenceType: 'person',
      observations: [{ observerId: 'human-matthew-manager', observedAt: '2021-08-07T00:00:00Z', context: 'Keen' }],
    });
    const result = await buildPresencesJson(dir);
    expect(result.presences).toHaveLength(2);
    expect(result.presences[0].id).toBe('presence-james-p-carse');
    expect(result.presences[1].id).toBe('presence-virginia-eubanks');
  });

  it('includes version and description metadata (deterministic — no timestamps)', async () => {
    await writePresence('test.md', {
      id: 'presence-test',
      displayName: 'Test',
      presenceType: 'person',
      observations: [{ observerId: 'human-test', observedAt: '2026-01-01T00:00:00Z', context: 'test' }],
    });
    const result = await buildPresencesJson(dir);
    expect(result.version).toBe('1.0.0');
    expect(result.description).toContain('DO NOT EDIT BY HAND');
    // Regression: no timestamps allowed — they break the pre-push freshness diff
    expect((result as Record<string, unknown>).generatedAt).toBeUndefined();
  });

  it('skips relationships.md and README.md', async () => {
    await writePresence('test.md', {
      id: 'presence-test',
      displayName: 'Test',
      presenceType: 'person',
      observations: [{ observerId: 'human-test', observedAt: '2026-01-01T00:00:00Z', context: 'test' }],
    });
    await writeFile(join(dir, 'README.md'), '# readme\n');
    await writeFile(join(dir, 'relationships.md'), '---\nrelationships: []\n---\n');
    const result = await buildPresencesJson(dir);
    expect(result.presences).toHaveLength(1);
  });
});
