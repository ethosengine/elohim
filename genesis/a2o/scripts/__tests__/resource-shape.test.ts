import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import {
  classifyResource,
  documentedTunablesFor,
  parseFrontmatter,
  exhaustedText,
  buildShape,
  type ShapeEntry,
} from '../build-resource-shape-report.js';

describe('classifyResource', () => {
  it('classifies an OOM/memory incident', () => {
    const classes = classifyResource('jessica edgenode OOMKilled at the memcg limit', ['oom']);
    assert.ok(classes.includes('memory'));
  });

  it('classifies a reconnect-storm as sessions', () => {
    const classes = classifyResource('conductor closing app-ws sessions ~40/min', ['reconnect']);
    assert.ok(classes.includes('sessions'));
  });

  it('classifies an arc/dht working-set incident', () => {
    const classes = classifyResource('per-node RAM tracks the full authority arc DHT working set', ['gossip']);
    assert.ok(classes.includes('arc-dht'));
  });

  it('classifies a crash-loop/liveness incident as runtime-sched', () => {
    const classes = classifyResource('doorway crash-loop: liveness watchdog SIGKILL on a wedged runtime', ['crash-loop']);
    assert.ok(classes.includes('runtime-sched'));
  });

  it('returns empty for an unrelated entry', () => {
    assert.deepEqual(classifyResource('add a new content type to the lamad manifest', ['lamad']), []);
  });
});

describe('documentedTunablesFor', () => {
  it('maps db-pool to STORAGE_DB_POOL_SIZE', () => {
    assert.deepEqual(documentedTunablesFor(['db-pool']), ['STORAGE_DB_POOL_SIZE']);
  });
  it('dedupes across multiple classes', () => {
    const t = documentedTunablesFor(['cpu', 'memory']);
    assert.ok(t.includes('deployments.json edgenodeCpuLimit'));
    assert.ok(t.includes('deployments.json edgenodeMemoryLimit'));
  });
});

describe('parseFrontmatter', () => {
  const md = [
    '---',
    'slug: "matthew-edge-storm"',
    'title: "matthew edge reconnect storm"',
    'severity: high',
    'self_heal_status: in-progress',
    'nodes: [matthew, doorway-alpha]',
    'tags: [self-heal, sessions, reconnect]',
    'fingerprints: []',
    '---',
    '',
    '## What is exhausted',
    '',
    'The conductor closes app-ws sessions ~40/min under load.',
    '',
    '## Fix path',
    'tune the pool.',
  ].join('\n');

  it('parses scalars and inline arrays', () => {
    const fm = parseFrontmatter(md)!;
    assert.equal(fm.slug, 'matthew-edge-storm');
    assert.equal(fm.severity, 'high');
    assert.equal(fm.selfHealStatus, 'in-progress');
    assert.deepEqual(fm.nodes, ['matthew', 'doorway-alpha']);
    assert.deepEqual(fm.tags, ['self-heal', 'sessions', 'reconnect']);
    assert.deepEqual(fm.fingerprints, []);
  });

  it('extracts the "What is exhausted" paragraph', () => {
    const t = exhaustedText(md)!;
    assert.ok(t.includes('app-ws sessions'));
    assert.ok(!t.includes('Fix path'));
  });

  it('returns null without frontmatter', () => {
    assert.equal(parseFrontmatter('# no frontmatter here'), null);
  });
});

describe('buildShape', () => {
  const entries: ShapeEntry[] = [
    {
      slug: 'a', title: 'a', severity: 'high', selfHealStatus: 'in-progress',
      nodes: ['matthew'], classes: ['sessions', 'db-pool'], fingerprints: [], ciOccurrences: 3,
      hasExhaustedSection: true, documentedTunables: ['STORAGE_DB_POOL_SIZE'],
    },
    {
      slug: 'b', title: 'b', severity: 'high', selfHealStatus: 'cured',
      nodes: ['matthew', 'james'], classes: ['memory'], fingerprints: [], ciOccurrences: 0,
      hasExhaustedSection: true, documentedTunables: ['deployments.json edgenodeMemoryLimit'],
    },
  ];

  it('builds a node×class matrix and lifecycle tallies', () => {
    const shape = buildShape(entries);
    assert.equal(shape.matrix.cells['matthew']['sessions'], 1);
    assert.equal(shape.matrix.cells['matthew']['db-pool'], 1);
    assert.equal(shape.matrix.cells['james']['memory'], 1);
    assert.equal(shape.byLifecycle['in-progress'], 1);
    assert.equal(shape.byLifecycle['cured'], 1);
    assert.equal(shape.byClass['memory'].nodes.length, 2);
  });
});
