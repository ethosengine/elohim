import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';

import { After, Given, Then, When } from '@cucumber/cucumber';

type ObjectValue = Record<string, unknown>;
interface MemoryFixture {
  root: string;
  env: NodeJS.ProcessEnv;
  view: ObjectValue;
  collective: ObjectValue;
  projection?: ObjectValue;
  baseline?: ObjectValue;
  claimBytes: string;
  recall?: ObjectValue;
  gate?: ObjectValue;
  indexText?: string;
}
const fixtures = new WeakMap<object, MemoryFixture>();
const repository = resolve('../..');
const binary = process.env.EPR_BIN ?? 'epr';
// The footprint measurement is native to `epr flow memory recall`; `--footprint-lens <script>`
// remains only as an explicit override for an external lens and is never needed here (`--lens`
// now names the READER lens level, governed-discovery station 1).
const projectionPath = 'genesis/projection.json';
const purpose =
  'Recover the passage-opening evidence while keeping whole-workflow acceptance undecided';
const findingPath = 'genesis/finding.json';
const qualification = 'Whole-workflow acceptance is undecided';

function object(value: unknown): ObjectValue {
  assert.ok(value && typeof value === 'object' && !Array.isArray(value));
  return value as ObjectValue;
}
function state(world: object): MemoryFixture {
  const result = fixtures.get(world);
  assert.ok(result);
  return result;
}
function write(f: MemoryFixture, path: string, value: unknown): void {
  mkdirSync(dirname(join(f.root, path)), { recursive: true });
  writeFileSync(join(f.root, path), typeof value === 'string' ? value : JSON.stringify(value));
}
function invoke(f: MemoryFixture, argv: string[], expected = 0): ObjectValue {
  const result = spawnSync(argv[0], argv.slice(1), {
    cwd: f.root,
    env: f.env,
    encoding: 'utf8',
    timeout: 30_000,
    maxBuffer: 1024 * 1024,
  });
  assert.equal(result.status, expected, `${argv.join(' ')}\n${result.stdout}\n${result.stderr}`);
  if (result.stdout.trim()) return object(JSON.parse(result.stdout));
  // A native refusal is reported on stderr with a non-zero status; the story asserts on its words.
  return result.stderr.trim() ? { refusal: result.stderr.trim() } : {};
}
function native(f: MemoryFixture, operation: string, args: string[] = []): ObjectValue {
  return invoke(f, [binary, 'flow', 'memory', operation, ...args, '--root', f.root, '--json']);
}
function pin(f: MemoryFixture, path: string): ObjectValue {
  return object(native(f, 'pin', ['--input', path]).resource);
}
// The recall executor carries the investigation. Collective memory is its own set of verbs
// (`collective|contribute|project|feedback|graduate`), each taking an authored request file, so a
// recall session never becomes a second address for a governed act.
function recall(f: MemoryFixture, operation: string, args: string[] = []): ObjectValue {
  f.recall = invoke(f, [
    binary,
    'flow',
    'memory',
    'recall',
    operation,
    '--root',
    f.root,
    '--contract',
    join(f.root, 'contract.json'),
    '--session',
    'reader',
    '--json',
    ...args,
  ]);
  return f.recall;
}
function contribute(f: MemoryFixture, path: string, claim: string, local = false): void {
  write(f, path, {
    version: 1,
    collective: f.collective,
    author: 'agent:investigator@fixture',
    steward: 'repo:ethosengine/elohim',
    scope: 'workspace',
    reach: local ? 'workspace' : 'repository',
    concern: 'qualified passage evidence',
    claim,
    uncertainty: [qualification],
    sources: [
      {
        resource: pin(f, local ? '.claude/memory-kit/local.txt' : 'genesis/source.txt'),
        reach: local ? 'workspace' : 'repository',
      },
    ],
    supersedes: [],
    contradicts: path.endsWith('alternative.json') ? [pin(f, findingPath)] : [],
  });
  native(f, 'contribute', ['--input', path, '--session', 'author']);
}
function project(f: MemoryFixture, includeAlternative = true): void {
  write(f, 'genesis/request.json', {
    version: 1,
    collective: f.collective,
    purpose,
    audience: 'workspace',
    inputs: [
      pin(f, findingPath),
      ...(includeAlternative ? [pin(f, 'genesis/alternative.json')] : []),
    ],
    omissions: includeAlternative
      ? ['No population audit or private native memory selected']
      : ['Alternative not selected'],
  });
  f.view = native(f, 'project', ['--input', 'genesis/request.json']);
  // `project` declares itself ephemeral and tells its caller to save the exact output before any
  // consequential use, then pin the saved bytes. Feedback names those pinned bytes as its target.
  write(f, projectionPath, f.view);
  f.projection = { path: projectionPath, resource: pin(f, projectionPath) };
}
function setup(world: object, seeded = true): MemoryFixture {
  const root = mkdtempSync(join(tmpdir(), 'collective-memory-story-'));
  const env = { ...process.env };
  for (const key of ['GIT_DIR', 'GIT_WORK_TREE', 'GIT_INDEX_FILE', 'GIT_COMMON_DIR'])
    delete env[key];
  Object.assign(env, {
    GIT_AUTHOR_NAME: 'Fixture',
    GIT_AUTHOR_EMAIL: 'fixture@example.test',
    GIT_COMMITTER_NAME: 'Fixture',
    GIT_COMMITTER_EMAIL: 'fixture@example.test',
  });
  const f: MemoryFixture = { root, env, view: {}, collective: {}, claimBytes: '' };
  fixtures.set(world, f);
  write(f, '.epr-meta/repository.yaml', 'version: 1\nagent: repo:ethosengine/elohim\n');
  write(
    f,
    '.epr-meta/collective.json',
    JSON.parse(readFileSync(join(repository, '.epr-meta/collective.json'), 'utf8'))
  );
  // A declaration names no steward: the collective's Stewards are affiliation records, and a
  // collective with none on record is refused. The repository's own sidecar is copied verbatim
  // (affiliations match their collective by declaration path).
  write(
    f,
    '.eprfs/status/affiliations.jsonl',
    readFileSync(join(repository, '.eprfs/status/affiliations.jsonl'), 'utf8')
  );
  write(
    f,
    'contract.json',
    JSON.parse(
      readFileSync(join(repository, '.epr-meta/elohim/algorithms/recall-contract.json'), 'utf8')
    )
  );
  write(
    f,
    'genesis/source.txt',
    'The trial reader opened the cited passage.\n' + qualification + '.\n'
  );
  write(f, '.claude/memory-kit/local.txt', 'Local-only revealing passage');
  write(f, 'private-native/sentinel.txt', 'Private runtime memory must never be imported');
  for (const args of [
    ['init', '-q'],
    ['add', '.epr-meta/collective.json', 'genesis/source.txt'],
    ['commit', '-qm', 'fixture'],
  ]) {
    const result = spawnSync('/usr/bin/git', args, { cwd: root, env, encoding: 'utf8' });
    assert.equal(result.status, 0, result.stderr);
  }
  for (const [session, role] of [
    ['author', 'investigator'],
    ['reader-actor', 'reviewer'],
  ]) {
    invoke(f, [
      binary,
      'actor',
      'claim',
      '--as',
      `agent:${role}@fixture`,
      '--session',
      session,
      '--root',
      root,
      '--json',
    ]);
  }
  f.collective = pin(f, '.epr-meta/collective.json');
  if (!seeded) return f;
  contribute(f, findingPath, 'The trial reader opened the cited passage');
  contribute(f, 'genesis/alternative.json', 'The whole workflow has independent acceptance');
  f.claimBytes = readFileSync(join(root, findingPath), 'utf8');
  return f;
}
After(function () {
  const f = fixtures.get(this);
  if (f) rmSync(f.root, { recursive: true, force: true });
});
Given(
  'a declared local collective with an evidenced finding and a contested alternative',
  function () {
    setup(this);
  }
);
When('another agent opens that collective through the ceremony', function () {
  const f = state(this);
  recall(f, 'open', ['--intent', purpose, '--scope', 'genesis']);
  assert.deepEqual(native(f, 'collective').resource, f.collective);
  project(f);
});
Then(
  'its context identifies the collective, purpose, evidence and unresolved alternative',
  function () {
    const f = state(this);
    const receipt = object(f.view.receipt);
    assert.deepEqual(receipt.collective, f.collective);
    assert.equal(receipt.purpose, purpose);
    const items = receipt.items as ObjectValue[];
    assert.equal(items.length, 2);
    assert.deepEqual(object(items[0].assertion).sources, [
      { resource: pin(f, 'genesis/source.txt'), reach: 'repository' },
    ]);
    assert.ok(JSON.stringify(items).includes(qualification));
    assert.ok(JSON.stringify(items).includes('The whole workflow has independent acceptance'));
  }
);
When('a fresh process resumes the collective ceremony', function () {
  const f = state(this);
  recall(f, 'resume');
  // Resumption re-derives the projection from the same authored request; the receipt is the
  // governed artifact, and it is reproduced rather than cached inside the investigation.
  f.view = native(f, 'project', ['--input', 'genesis/request.json']);
});
Then(
  'the same governed inputs remain traceable without importing private native memory',
  function () {
    const f = state(this);
    const receipt = object(f.view.receipt);
    assert.deepEqual(receipt.collective, f.collective);
    assert.equal(receipt.purpose, purpose);
    assert.ok(JSON.stringify(receipt.items).includes(qualification));
    assert.deepEqual(f.projection?.resource, pin(f, projectionPath));
    for (const view of [f.view, object(f.recall)])
      assert.ok(!JSON.stringify(view).includes('Private runtime memory must never be imported'));
    assert.equal(
      readFileSync(join(f.root, 'private-native/sentinel.txt'), 'utf8'),
      'Private runtime memory must never be imported'
    );
  }
);
When("the reader challenges a projection's omission rather than its source assertion", function () {
  const f = state(this);
  recall(f, 'open', ['--intent', purpose, '--scope', 'genesis']);
  project(f, false);
  assert.ok(f.projection);
  write(f, 'genesis/feedback.json', {
    version: 1,
    collective: f.collective,
    target: f.projection.resource,
    kind: 'omitted-contradiction',
    passage: 'Alternative not selected',
    reason: 'The omitted alternative must remain visible: ' + qualification,
  });
  f.view = native(f, 'feedback', ['--input', 'genesis/feedback.json', '--session', 'reader-actor']);
});
Then(
  'the feedback identifies the exact viewed projection and its omitted undecided qualification',
  function () {
    const f = state(this);
    assert.ok(f.projection);
    const memory = f.view;
    assert.deepEqual(memory.target, f.projection.resource);
    assert.equal(memory.author, 'agent:reviewer@fixture');
    assert.ok(JSON.stringify(memory.event).includes('record_cid'));
    assert.ok(readFileSync(join(f.root, 'genesis/feedback.json'), 'utf8').includes(qualification));
  }
);
Then('the source claim remains unchanged pending its own review', function () {
  const f = state(this);
  assert.equal(readFileSync(join(f.root, findingPath), 'utf8'), f.claimBytes);
  assert.ok(String(f.view.standing).includes('No source rewritten or judgment discharged'));
});
When(
  'the agent rehearses repository reach for a finding depending on local-only evidence',
  function () {
    const f = state(this);
    contribute(f, 'genesis/local-finding.json', 'Local-only revealing passage', true);
    write(f, 'genesis/graduation.json', {
      version: 1,
      collective: f.collective,
      contribution: pin(f, 'genesis/local-finding.json'),
      review: 'unavailable-review',
      audience: 'repository',
    });
    recall(f, 'open', ['--intent', purpose, '--scope', 'genesis']);
    f.view = invoke(
      f,
      [
        binary,
        'flow',
        'memory',
        'graduate',
        '--input',
        'genesis/graduation.json',
        '--root',
        f.root,
        '--json',
      ],
      2
    );
  }
);
Then('the graduation refuses to widen the finding and its revealing receipt', function () {
  const f = state(this);
  assert.ok(JSON.stringify(f.view).includes('contribution restriction forbids repository reach'));
  assert.equal(
    object(JSON.parse(readFileSync(join(f.root, 'genesis/local-finding.json'), 'utf8'))).reach,
    'workspace'
  );
  assert.ok(!JSON.stringify(f.view).includes('Local-only revealing passage'));
});
Given('a ceremony with an explicit scoped burden baseline', function () {
  const f = setup(this);
  write(f, 'genesis/active/a.txt', 'duplicate\n');
  write(f, 'genesis/active/b.txt', 'duplicate\n');
  mkdirSync(join(f.root, 'genesis/archive'), { recursive: true });
  recall(f, 'open', [
    '--intent',
    purpose,
    '--scope',
    'genesis',
    '--measure-scope',
    'authored:genesis/active',
    '--measure-scope',
    'retained:genesis/archive',
  ]);
  f.baseline = object(object(object(f.recall).measurement).evidence);
  project(f);
});
When('a duplicate is consolidated and the ceremony records its closing observation', function () {
  const f = state(this);
  renameSync(join(f.root, 'genesis/active/b.txt'), join(f.root, 'genesis/archive/b.txt'));
  write(f, 'genesis/active/run-artifact.txt', 'run overhead');
  recall(f, 'measure', ['--phase', 'close']);
});
Then(
  'the deterministic report shows active content and total retained byte changes with exact paired inputs',
  function () {
    const f = state(this);
    const measurement = object(object(f.recall).measurement);
    assert.deepEqual(measurement.baseline, f.baseline);
    assert.ok(object(measurement.evidence).sha256);
    const comparison = object(measurement.comparison);
    assert.equal(comparison.comparable, true);
    assert.equal(object(comparison.delta).bytes, 12);
    assert.equal(object(object(comparison.partitions).retained).bytes, 10);
  }
);
Then(
  'archived bytes and new run artifacts are counted rather than mistaken for deletion',
  function () {
    const comparison = object(object(object(state(this).recall).measurement).comparison);
    assert.equal((comparison.same_content_relocations as ObjectValue[])[0].bytes, 10);
    assert.equal(object(comparison.delta).bytes, 12);
  }
);
Then('the report leaves the contested judgment unresolved', function () {
  const f = state(this);
  assert.ok(String(object(object(f.recall).measurement).meaning).includes('not a ceremony grade'));
  recall(f, 'resume');
  f.view = native(f, 'project', ['--input', 'genesis/request.json']);
  assert.ok(JSON.stringify(object(f.view.receipt)).includes(qualification));
});

// ── applies-to: whose each inherited memory is ──────────────────────────────────────────────────
// Mirrors elohim/eprfs/epr-cli/tests/flow_memory_import.rs (`the_index_groups_...`): an agent
// imports the collective's shared entry, a standing human stands for entries no agent authored,
// and the index is projected from the contributions those acts leave.
const memoryDir = '.claude/memory';
const indexPath = `${memoryDir}/MEMORY.md`;
const firstSteward = 'human:matthew';
const secondSteward = 'human:ruth';
const stewardedEntries = ['feedback_ruling', 'user_matthew'];
const gateProbe = resolve(dirname(new URL(import.meta.url).pathname), 'memory-gate-probe.py');

function entry(stem: string, kind: string, extra = '', origin = false): string {
  const meta = origin
    ? '  originSessionId: never-claimed-session\n  modified: 2026-09-25T09:00:00Z\n'
    : '';
  return `---\nname: ${stem}\ndescription: The claim of ${stem}.\n${extra}metadata:\n  type: ${kind}\n${meta}---\n\nbody\n`;
}
function commitAll(f: MemoryFixture): void {
  for (const args of [
    ['add', '-A'],
    ['commit', '-qm', 'entries'],
  ]) {
    const result = spawnSync('/usr/bin/git', args, { cwd: f.root, env: f.env, encoding: 'utf8' });
    assert.equal(result.status, 0, result.stderr);
  }
}
function projectIndex(f: MemoryFixture): string {
  native(f, 'project', ['--index', '--out', indexPath]);
  f.indexText = readFileSync(join(f.root, indexPath), 'utf8');
  return f.indexText;
}
/** The heading a row sits under: the nearest `## ` line above it. */
function sectionOf(index: string, file: string): string | undefined {
  let heading: string | undefined;
  for (const line of index.split('\n')) {
    if (line.startsWith('## ')) heading = line;
    else if (line.includes(`](${file})`)) return heading;
  }
  return undefined;
}
const collectiveHeading = '## Held by the collective';
const directionFrom = (who: string): string => `## Direction from ${who} (theirs, on record)`;
function contributionOf(f: MemoryFixture, stem: string): ObjectValue {
  return object(
    JSON.parse(
      readFileSync(join(f.root, `.eprfs/status/memory/contributions/${stem}.json`), 'utf8')
    )
  );
}
/** The gate: the edit-time resolver's own entry point, on a simulated new-file write. */
function gate(f: MemoryFixture, content: string): ObjectValue {
  const probe = join(f.root, 'gate-content.md');
  writeFileSync(probe, content);
  const result = spawnSync('python3', [gateProbe, repository, probe], {
    encoding: 'utf8',
    timeout: 30_000,
  });
  assert.equal(result.status, 0, result.stderr);
  return object(JSON.parse(result.stdout));
}
const correction = (extra: string): string =>
  `---\nname: feedback_newcomer_correction\ntitle: Newcomer correction\ndescription: The newcomer's first correction of a practice.\n${extra}metadata:\n  type: feedback\n---\n\nbody\n`;
/** The CID `epr` gives a file's raw bytes: CIDv1, raw codec, sha2-256, base32 lower. */
function rawCid(bytes: Buffer): string {
  const digest = createHash('sha256').update(bytes).digest();
  const raw = Buffer.concat([Buffer.from([0x01, 0x55, 0x12, 0x20]), digest]);
  const alphabet = 'abcdefghijklmnopqrstuvwxyz234567';
  let bits = 0;
  let acc = 0;
  let out = '';
  for (const byte of raw) {
    acc = (acc << 8) | byte;
    bits += 8;
    while (bits >= 5) {
      out += alphabet[(acc >>> (bits - 5)) & 31];
      bits -= 5;
    }
    acc &= (1 << bits) - 1;
  }
  if (bits > 0) out += alphabet[(acc << (5 - bits)) & 31];
  return 'b' + out;
}

Given(
  "a collective whose memory holds a shared finding, the first steward's direction and a note describing the first steward",
  function () {
    const f = setup(this, false);
    write(f, `${memoryDir}/project_shared.md`, entry('project_shared', 'project'));
    commitAll(f);
    native(f, 'import', [memoryDir, '--session', 'author']);
    // The first steward's direction and the note about them: written by no witnessable agent,
    // so the standing human stands for them.
    for (const [stem, kind] of [
      ['feedback_ruling', 'feedback'],
      ['user_matthew', 'user'],
    ])
      write(f, `${memoryDir}/${stem}.md`, entry(stem, kind, '', true));
    invoke(f, [
      binary,
      'actor',
      'claim',
      '--as',
      firstSteward,
      '--session',
      'human-session',
      '--root',
      f.root,
      '--json',
    ]);
    native(f, 'import', [
      ...stewardedEntries.map(stem => `${memoryDir}/${stem}.md`),
      '--session',
      'human-session',
      '--steward-of-record',
    ]);
  }
);
When("a second developer's session reads the projected memory index", function () {
  projectIndex(state(this));
});
Then('the shared finding is listed as held by the collective', function () {
  assert.equal(sectionOf(state(this).indexText ?? '', 'project_shared.md'), collectiveHeading);
});
Then("the first steward's direction is listed under the first steward's name", function () {
  assert.equal(
    sectionOf(state(this).indexText ?? '', 'feedback_ruling.md'),
    directionFrom(firstSteward)
  );
});
Then("the note describing the first steward is listed under the first steward's name", function () {
  assert.equal(
    sectionOf(state(this).indexText ?? '', 'user_matthew.md'),
    directionFrom(firstSteward)
  );
});
Then("no row under the first steward's name is listed as held by the collective", function () {
  const index = state(this).indexText ?? '';
  for (const stem of stewardedEntries)
    assert.notEqual(sectionOf(index, `${stem}.md`), collectiveHeading, stem);
  const held = index.split(directionFrom(firstSteward))[0];
  for (const stem of stewardedEntries) assert.ok(!held.includes(`](${stem}.md)`), stem);
});

When(
  "the second developer's session writes a correction that does not say whom it applies to",
  function () {
    const f = state(this);
    f.gate = gate(f, correction(''));
  }
);
Then('the memory gate refuses the entry and names the three audiences', function () {
  const verdict = object(state(this).gate);
  assert.equal(verdict.decision, 'refuse', JSON.stringify(verdict));
  const reason = String(verdict.reason);
  assert.ok(reason.includes('applies-to'), reason);
  for (const audience of ['device', 'steward', 'repository'])
    assert.ok(reason.includes(`\`${audience}\``), `${audience}: ${reason}`);
});
When('the session writes the same correction as applying to its steward', function () {
  const f = state(this);
  const text = correction('applies-to: steward\n');
  f.gate = gate(f, text);
  write(f, `${memoryDir}/feedback_newcomer_correction.md`, text);
});
Then('the entry is accepted', function () {
  const verdict = object(state(this).gate);
  assert.equal(verdict.decision, 'permit', JSON.stringify(verdict));
});
Then("after import the correction is listed under the second developer's name", function () {
  const f = state(this);
  const stem = 'feedback_newcomer_correction';
  const rel = `${memoryDir}/${stem}.md`;
  // `import` fills the steward slot with the collective's own party; a slot naming a participant
  // is written directly with its attributed act (as `plant_stewarded` does in the Rust test).
  const template = contributionOf(f, 'project_shared');
  const imported = object(template.imported);
  const bytes = readFileSync(join(f.root, rel));
  const contribution = {
    ...template,
    steward: secondSteward,
    concern: stem,
    claim: `The claim of ${stem}.`,
    uncertainty: [],
    sources: [{ resource: { path: rel, cid: rawCid(bytes) }, reach: 'repository' }],
    imported: {
      ...imported,
      display: stem,
      file: `${stem}.md`,
      scopePath: rel,
      entryType: 'feedback',
      appliesTo: 'steward',
    },
  };
  const request = `.eprfs/status/memory/contributions/${stem}.json`;
  const text = JSON.stringify(contribution, null, 2) + '\n';
  write(f, request, text);
  const note = spawnSync(
    binary,
    [
      'flow',
      'note',
      '--on',
      request,
      '--kind',
      'observation',
      '--session',
      'author',
      '--reason',
      `Collective contribution ${rawCid(Buffer.from(text))} under ${object(f.collective).cid}; unreviewed, no acceptance`,
      '--root',
      f.root,
    ],
    { cwd: f.root, env: f.env, encoding: 'utf8', timeout: 30_000 }
  );
  assert.equal(note.status, 0, `${note.stdout}\n${note.stderr}`);
  assert.equal(sectionOf(projectIndex(f), `${stem}.md`), directionFrom(secondSteward));
});

When('an entry declared as applying to the device is imported', function () {
  const f = state(this);
  // Named with the writing session and time, so the entry-form import can attribute it to its agent.
  write(
    f,
    `${memoryDir}/project_here.md`,
    entry('project_here', 'project', 'applies-to: device\n').replace(
      'type: project\n',
      `type: project\n  originSessionId: author\n  modified: ${new Date().toISOString()}\n`
    )
  );
  native(f, 'import', [`${memoryDir}/project_here.md`, '--session', 'author']);
});
Then('its contribution reaches no further than the device it was written on', function () {
  const f = state(this);
  assert.equal(contributionOf(f, 'project_here').reach, 'workspace');
  // A device entry is not the collective's memory: it never appears in the shared index.
  assert.ok(existsSync(join(f.root, memoryDir, 'project_here.md')));
  assert.equal(sectionOf(projectIndex(f), 'project_here.md'), undefined);
});
