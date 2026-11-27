/**
 * Steps for `features/devflow/feedback-drains-when-cured.feature` (FB-1): a refused command
 * names its claim and one session leaves one record; a device that could not keep its records
 * says so in its status and names the command that catches up.
 *
 * Device A is a private fixture workspace (a directory holding a feedback policy) with its own
 * state directory. "ferry" is a fixture script these steps write; it calls the explicitly
 * selected `epr` (`EPR_BIN`, else `epr` on PATH) with the claim variables set, the way Brit does.
 */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash, randomBytes, randomUUID } from 'node:crypto';
import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { After, Given, Then, When } from '@cucumber/cucumber';

/** Every variable that names a session, an invocation or a claim; each run sets its own. */
const SESSION_ENV = [
  'EPR_FEEDBACK_CLAIM',
  'EPR_FEEDBACK_EMITTER',
  'EPR_FEEDBACK_INVOCATION',
  'ELOHIM_SESSION_ID',
  'CLAUDE_SESSION_ID',
  'CLAUDE_CODE_SESSION_ID',
  'CODEX_THREAD_ID',
  'CODEX_SESSION_ID',
];

interface Occurrence {
  source: { producer: string; run: string; item: string };
  subject: string;
  participant: string | null;
  outcome: string;
}

interface StatusLine {
  name: string;
  state: string;
  detail: string;
  cure: string | null;
}

interface FerryWorld {
  ferryRoot?: string;
  ferryState?: string;
  ferryTool?: string;
  ferryRefusals?: { status: number | null; stderr: string }[];
  ferryGapNotes?: { path: string; at: string; session: string }[];
  ferryStatus?: StatusLine;
}

function epr(): string {
  return process.env.EPR_BIN ?? 'epr';
}

/** The environment of one session on device A: none of the caller's session names leak in. */
function sessionEnv(world: FerryWorld, session?: string): NodeJS.ProcessEnv {
  const env: NodeJS.ProcessEnv = { ...process.env, XDG_STATE_HOME: world.ferryState! };
  for (const name of SESSION_ENV) delete env[name];
  if (session) env.ELOHIM_SESSION_ID = session;
  return env;
}

function byText(a: string, b: string): number {
  return a.localeCompare(b);
}

function shellQuote(value: string): string {
  const escaped = value.replaceAll("'", String.raw`'\''`);
  return `'${escaped}'`;
}

function makeDevice(world: FerryWorld, person: string): void {
  const root = mkdtempSync(join(tmpdir(), 'ferry-device-'));
  world.ferryRoot = root;
  world.ferryState = join(root, 'state');
  mkdirSync(join(root, '.epr-meta'), { recursive: true });
  mkdirSync(world.ferryState, { recursive: true });
  writeFileSync(
    join(root, '.epr-meta/feedback-policy.json'),
    JSON.stringify({
      id: 'ferry-story@1',
      authority: `human:${person}`,
      participant: `human:${person}`,
      producers: ['ferry', 'brit'],
      automaticCapture: true,
      deliberateEvidence: true,
      maxBytes: 1000000,
      maxRecordBytes: 65536,
      response: 'repair',
      sharing: 'separately-authorized',
    })
  );
}

function feedback(world: FerryWorld, args: string[]): Record<string, unknown> {
  const result = spawnSync(epr(), ['feedback', ...args, '--root', world.ferryRoot!, '--json'], {
    encoding: 'utf8',
    env: sessionEnv(world),
    timeout: 10000,
  });
  assert.equal(result.status, 0, result.stderr);
  return JSON.parse(result.stdout) as Record<string, unknown>;
}

function occurrences(world: FerryWorld): Occurrence[] {
  const list = feedback(world, ['list']) as { experiences: { occurrence: Occurrence }[] };
  return list.experiences.map(row => row.occurrence);
}

/** Every byte the feedback store keeps on device A, whatever its layout. */
function storeBytes(world: FerryWorld): string {
  const walk = (dir: string): string[] =>
    existsSync(dir)
      ? readdirSync(dir).flatMap(name => {
          const path = join(dir, name);
          return statSync(path).isDirectory() ? walk(path) : [readFileSync(path, 'utf8')];
        })
      : [];
  return walk(join(world.ferryRoot!, '.eprfs')).join('\n');
}

function feedbackRecorderLine(world: FerryWorld): StatusLine {
  const result = spawnSync(epr(), ['device', 'status', '--json', '--root', world.ferryRoot!], {
    encoding: 'utf8',
    env: sessionEnv(world),
    timeout: 60000,
  });
  // Status exits non-zero when any line is refused; the lines are on stdout either way.
  assert.ok(result.stdout, `device status printed nothing: ${result.stderr}`);
  const lines = JSON.parse(result.stdout) as StatusLine[];
  const line = lines.find(l => l.name === 'feedback recorder');
  assert.ok(line, `device status has no feedback recorder line: ${lines.map(l => l.name)}`);
  return line;
}

Given(
  'device A runs {string} release {int} and its feedback store is empty',
  function (this: FerryWorld, tool: string, release: number) {
    assert.equal(tool, 'ferry');
    assert.equal(release, 1);
    makeDevice(this, 'matthew');
    const claim = 'ferry/capability-absent:list.since';
    const bin = join(this.ferryRoot!, 'bin');
    mkdirSync(bin);
    this.ferryTool = join(bin, 'ferry');
    // Release 1 cannot do "list --since": it refuses, names the claim from its declaration
    // (never what was typed) and reports it to the recorder; a recorder failure keeps the exit.
    writeFileSync(
      this.ferryTool,
      [
        '#!/usr/bin/env bash',
        `claim=${shellQuote(claim)}`,
        'if [ "$1" = list ] && [ "$2" = --since ]; then',
        `  EPR_FEEDBACK_CLAIM="$claim" EPR_FEEDBACK_EMITTER=unrecorded:1+exe.000000000001 ${shellQuote(epr())} \\`,
        '    feedback capture --producer ferry --operation list --outcome refused \\',
        `    --root ${shellQuote(this.ferryRoot!)} --json >/dev/null 2>&1 || true`,
        '  echo "ferry: list --since is not in release 1; refused: $claim" >&2',
        '  exit 2',
        'fi',
        'echo "ferry: ok"',
        '',
      ].join('\n')
    );
    chmodSync(this.ferryTool, 0o755);
    assert.deepEqual(occurrences(this), []);
  }
);

When(
  'a session of {string} on device A runs {string} five times',
  function (this: FerryWorld, person: string, command: string) {
    assert.equal(person, 'matthew');
    const [tool, ...args] = command.split(' ');
    assert.equal(tool, 'ferry');
    const session = randomUUID();
    this.ferryRefusals = [];
    for (let i = 0; i < 5; i++) {
      const result = spawnSync(this.ferryTool!, args, {
        encoding: 'utf8',
        env: sessionEnv(this, session),
        timeout: 10000,
      });
      this.ferryRefusals.push({ status: result.status, stderr: result.stderr });
    }
  }
);

Then(
  "each refusal ends with the claim's name {string}",
  function (this: FerryWorld, claim: string) {
    assert.equal(this.ferryRefusals?.length, 5);
    for (const refusal of this.ferryRefusals) {
      assert.notEqual(refusal.status, 0, 'the command was refused');
      assert.ok(refusal.stderr.trimEnd().endsWith(claim), refusal.stderr);
    }
  }
);

Then(
  "device A's feedback store holds one record for that claim, in matthew's name",
  function (this: FerryWorld) {
    const rows = occurrences(this);
    assert.equal(rows.length, 1, JSON.stringify(rows));
    assert.equal(rows[0].subject, 'ferry/capability-absent:list.since');
    assert.equal(rows[0].participant, 'human:matthew');
    assert.equal(rows[0].outcome, 'refused');
  }
);

Then('that record does not contain the word {string}', function (this: FerryWorld, word: string) {
  const bytes = storeBytes(this);
  assert.ok(bytes.length > 0, 'the store holds the record');
  assert.ok(!bytes.includes(word), `the store retains "${word}"`);
});

/** The release of the tool that left the fixture gap notes. */
const GAP_NOTE_RELEASE = 'unrecorded:0.3.6+exe.0123456789ab';

Given(
  'the recorder on device A did not finish in time in two sessions, and each session left a gap note saying so',
  function (this: FerryWorld) {
    makeDevice(this, 'matthew');
    const spool = join(this.ferryState!, 'brit/feedback-spool');
    mkdirSync(spool, { recursive: true });
    this.ferryGapNotes = ['2026-11-26T09:15:00Z', '2026-11-27T08:30:00Z'].map(at => {
      const session = createHash('sha256').update(randomUUID()).digest('hex');
      const path = join(spool, `${randomBytes(16).toString('hex')}.json`);
      // Brit's gap note: one per session, at most 1,024 bytes.
      writeFileSync(
        path,
        JSON.stringify({
          v: 1,
          claim: 'epr/recorder-unavailable:timeout',
          cause: 'timeout',
          at,
          emitter: GAP_NOTE_RELEASE,
          session,
          recorder: { path: null, size: null, mtime: null },
          stderr: null,
        })
      );
      return { path, at, session };
    });
  }
);

When('{string} asks device A for its status', function (this: FerryWorld, person: string) {
  assert.equal(person, 'matthew');
  this.ferryStatus = feedbackRecorderLine(this);
});

Then(
  'the status line for the feedback recorder says {int} gap notes are waiting and since when, and names the catch-up command',
  function (this: FerryWorld, gaps: number) {
    const line = this.ferryStatus!;
    assert.equal(line.state, 'degraded', JSON.stringify(line));
    const oldest = this.ferryGapNotes!.map(n => n.at).sort(byText)[0];
    assert.match(line.detail, new RegExp(`^${gaps} capture gaps waiting since ${oldest}`));
    assert.equal(line.cure, `epr feedback drain-spool --root ${this.ferryRoot}`);
  }
);

When(
  'a session of {string} on device A runs that command',
  function (this: FerryWorld, person: string) {
    assert.equal(person, 'matthew');
    const [tool, ...args] = this.ferryStatus!.cure!.split(' ');
    assert.equal(tool, 'epr');
    const result = spawnSync(epr(), args, {
      encoding: 'utf8',
      env: sessionEnv(this, randomUUID()),
      timeout: 10000,
    });
    assert.equal(result.status, 0, result.stderr);
  }
);

Then(
  "device A's feedback store holds one record for each of the two sessions, under the claim {string} and from the release its note names",
  function (this: FerryWorld, claim: string) {
    const rows = occurrences(this).filter(o => o.subject === claim);
    assert.equal(rows.length, 2, JSON.stringify(rows));
    for (const row of rows) {
      assert.equal(row.source.item, `${claim} failed ${GAP_NOTE_RELEASE}`, JSON.stringify(row));
    }
    assert.deepEqual(
      rows.map(o => o.source.run).sort(byText),
      this.ferryGapNotes!.map(n => `spool-${n.session}`).sort(byText)
    );
  }
);

Then('the two gap notes are gone', function (this: FerryWorld) {
  for (const note of this.ferryGapNotes!) assert.ok(!existsSync(note.path), note.path);
});

Then(
  'the status line for the feedback recorder says the recorder is well',
  function (this: FerryWorld) {
    const line = feedbackRecorderLine(this);
    assert.equal(line.state, 'ok', JSON.stringify(line));
    assert.equal(line.cure, null);
  }
);

After({ tags: '@concern:private-feedback-loop' }, function (this: FerryWorld) {
  if (this.ferryRoot) rmSync(this.ferryRoot, { recursive: true, force: true });
});
