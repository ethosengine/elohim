import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import {
  cpSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  renameSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

import { After, Given, Then, When } from '@cucumber/cucumber';

interface Declaration {
  sources: { items: { path: string } };
  select: { value: string };
  fields: string[];
}
interface Subject {
  identity: string;
  source: string;
  fields: Record<string, unknown>;
  readings: Record<string, { state: string }>;
}
interface Reading {
  source?: { contents: string; encoding: string; cid: string };
  projection: { subjects: Subject[]; sources: Record<string, string> };
  snapshot: string;
  comparison?: { missing: string[]; additions: string[]; cohort: { state: string }[] };
}
interface Fixture {
  base: string;
  workshop: string;
  orchard: string;
  store: string;
  before: Record<string, string>;
  workshopReading?: Reading;
  orchardReading?: Reading;
  saved?: Reading;
}
const fixtures = new WeakMap<object, Fixture>();
function state(world: object): Fixture {
  const value = fixtures.get(world);
  assert.ok(value);
  return value;
}
function inventory(root: string, prefix = ''): Record<string, string> {
  const files: Record<string, string> = {};
  for (const entry of readdirSync(join(root, prefix), { withFileTypes: true })) {
    const path = join(prefix, entry.name);
    if (entry.isDirectory()) Object.assign(files, inventory(root, path));
    else files[path] = readFileSync(join(root, path)).toString('base64');
  }
  return files;
}
function run(args: string[]): Reading {
  return JSON.parse(
    execFileSync(process.env.EPR_BIN ?? 'epr', ['view', ...args, '--json'], {
      encoding: 'utf8',
      timeout: 30_000,
      maxBuffer: 8 * 1024 * 1024,
    })
  ) as Reading;
}
function read(root: string, extras: string[] = []): Reading {
  return run(['view.json', '--root', root, ...extras]);
}
function saved(world: object): Reading {
  const value = state(world).saved;
  assert.ok(value);
  return value;
}
Given('a workshop and an orchard with their own local view declarations', function () {
  const base = mkdtempSync(join(tmpdir(), 'declared-views-'));
  const workshop = join(base, 'workshop');
  const orchard = join(base, 'orchard');
  const source = resolve('../../elohim/eprfs/epr-cli/tests/fixtures/views');
  cpSync(join(source, 'workshop'), workshop, { recursive: true });
  cpSync(join(source, 'orchard'), orchard, { recursive: true });
  fixtures.set(this, {
    base,
    workshop,
    orchard,
    store: join(base, 'store'),
    before: inventory(workshop),
  });
});
When('I read each collection with the same native binary', function () {
  const fixture = state(this);
  fixture.workshopReading = read(fixture.workshop);
  fixture.orchardReading = read(fixture.orchard);
});
Then('the workshop shows its table task and the orchard shows its north bed', function () {
  const fixture = state(this);
  assert.equal(fixture.workshopReading?.projection.subjects[0].identity, 'table');
  assert.equal(fixture.orchardReading?.projection.subjects[0].identity, 'north-bed');
});
Then('neither collection needs repository-specific runtime configuration', function () {
  for (const root of [state(this).workshop, state(this).orchard]) {
    const names = readdirSync(root);
    for (const name of ['genesis', '.claude', '.eprfs', '.git']) assert.ok(!names.includes(name));
  }
});
When('the orchard declaration selects apples instead of pears', function () {
  const path = join(state(this).orchard, 'view.json');
  const declaration = JSON.parse(readFileSync(path, 'utf8')) as Declaration;
  declaration.select.value = 'apple';
  writeFileSync(path, JSON.stringify(declaration));
  state(this).orchardReading = read(state(this).orchard);
});
Then('the orchard reading contains no matching subjects', function () {
  assert.deepEqual(state(this).orchardReading?.projection.subjects, []);
});
When('the orchard declaration follows its moved plots and shows their category', function () {
  const root = state(this).orchard;
  renameSync(join(root, 'plots'), join(root, 'nursery'));
  const path = join(root, 'view.json');
  const declaration = JSON.parse(readFileSync(path, 'utf8')) as Declaration;
  declaration.sources.items.path = 'nursery';
  declaration.select.value = 'pear';
  declaration.fields = ['category'];
  writeFileSync(path, JSON.stringify(declaration));
  state(this).orchardReading = read(root);
});
Then('the north bed appears from its new location with only the requested field', function () {
  const subject = state(this).orchardReading?.projection.subjects[0];
  assert.equal(subject?.source, 'nursery/north.md');
  assert.deepEqual(subject.fields, { category: 'planting' });
});
When('I read the workshop collection', function () {
  state(this).workshopReading = read(state(this).workshop);
});
Then('the table retains its authored open status', function () {
  assert.equal(state(this).workshopReading?.projection.subjects[0].fields.status, 'open');
});
Then('structured acceptance and measurement readings are explicitly unavailable', function () {
  const readings = state(this).workshopReading?.projection.subjects[0].readings;
  assert.equal(readings?.experience.state, 'unavailable');
  assert.equal(readings?.['field measurements'].state, 'unavailable');
});
When('I save the workshop reading', function () {
  const fixture = state(this);
  fixture.saved = read(fixture.workshop, ['--save', fixture.store]);
});
Then(
  'its source address resolves to the complete original document including frontmatter',
  function () {
    const fixture = state(this);
    const source = saved(this).projection.sources['tasks/table.md'];
    const reopened = run([
      '--snapshot',
      saved(this).snapshot,
      '--store',
      fixture.store,
      '--source',
      'tasks/table.md',
    ]);
    assert.equal(reopened.source?.encoding, 'utf-8');
    assert.equal(
      reopened.source?.contents,
      readFileSync(join(fixture.workshop, 'tasks/table.md'), 'utf8')
    );

    assert.deepEqual(
      readFileSync(join(fixture.store, source)),
      readFileSync(join(fixture.workshop, 'tasks/table.md'))
    );
  }
);
When('the workshop checkout is removed', function () {
  rmSync(state(this).workshop, { recursive: true });
});
Then('I can reopen the saved reading without consulting a checkout', function () {
  const reopened = run(['--snapshot', saved(this).snapshot, '--store', state(this).store]);
  assert.deepEqual(reopened.projection, saved(this).projection);
});
When('the table document is replaced by a chair document', function () {
  const root = state(this).workshop;
  const path = join(root, 'tasks/table.md');
  writeFileSync(
    join(root, 'tasks/chair.md'),
    readFileSync(path, 'utf8').replace('id: table', 'id: chair')
  );
  rmSync(path);
});
Then('comparison reports the table as missing and the chair as added', function () {
  const fixture = state(this);
  const comparison = read(fixture.workshop, [
    '--baseline',
    saved(this).snapshot,
    '--store',
    fixture.store,
  ]).comparison;
  assert.deepEqual(comparison?.missing, ['table']);
  assert.deepEqual(comparison?.additions, ['chair']);
  assert.equal(comparison?.cohort[0].state, 'missing');
});
Then('reopening the earlier reading still shows the table as open', function () {
  const reopened = run(['--snapshot', saved(this).snapshot, '--store', state(this).store]);
  assert.equal(reopened.projection.subjects[0].identity, 'table');
  assert.equal(reopened.projection.subjects[0].fields.status, 'open');
});
When('the retained table source is corrupted', function () {
  writeFileSync(
    join(state(this).store, saved(this).projection.sources['tasks/table.md']),
    'broken'
  );
});
Then('reopening refuses with a corruption diagnostic', function () {
  const result = spawnSync(
    process.env.EPR_BIN ?? 'epr',
    ['view', '--snapshot', saved(this).snapshot, '--store', state(this).store, '--json'],
    { encoding: 'utf8', timeout: 30_000 }
  );
  assert.equal(result.status, 2);
  assert.match(result.stdout, /corrupt snapshot object/);
});
Then('every workshop file is unchanged and no workflow store has been created', function () {
  assert.deepEqual(inventory(state(this).workshop), state(this).before);
});
After({ tags: '@concern:idle-is-free and @act:host' }, function () {
  const fixture = fixtures.get(this);
  if (fixture) rmSync(fixture.base, { recursive: true, force: true });
});
