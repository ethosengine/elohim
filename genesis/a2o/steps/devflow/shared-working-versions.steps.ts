import assert from 'node:assert/strict';
import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  chmodSync,
  closeSync,
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  openSync,
  readFileSync,
  readdirSync,
  readlinkSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { isAbsolute, join, resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

import { After, Given, Then, When } from '@cucumber/cucumber';

interface Capture {
  root: string;
  published: boolean;
}
interface Outcome {
  outcome: string;
  key: { input: { form: string; cid: string }; method: string };
  output: string;
  record: unknown;
  hit: boolean;
}
interface Fixture {
  base: string;
  source: string;
  store: string;
  brit: string;
  ark: string;
  env: NodeJS.ProcessEnv;
  capture?: Capture;
  outcome?: Outcome;
  child?: ChildProcess;
  closed?: Promise<number | null>;
  refused?: boolean;
}
const fixtures = new WeakMap<object, Fixture>();
const selected = ['value', 'binary', 'executable', 'link', 'post-checkout'];
const valueA = 'A: captured bytes\n';
const valueB = 'B: edited while A ran\n';

function state(world: object): Fixture {
  const fixture = fixtures.get(world);
  assert.ok(fixture, 'local fixture exists');
  return fixture;
}
function execute(fixture: Fixture, program: string, args: string[]): string {
  // File descriptors work in restricted local runtimes without Node's socket-backed pipes.
  const call = mkdtempSync(join(fixture.base, 'cli-'));
  const outPath = join(call, 'stdout');
  const errPath = join(call, 'stderr');
  const out = openSync(outPath, 'wx');
  const err = openSync(errPath, 'wx');
  try {
    execFileSync(program, args, {
      cwd: fixture.base,
      env: fixture.env,
      timeout: 60_000,
      stdio: ['ignore', out, err],
    });
  } catch (cause) {
    throw new Error(readFileSync(errPath, 'utf8'), { cause });
  } finally {
    closeSync(out);
    closeSync(err);
  }
  assert.ok(lstatSync(outPath).size <= 1024 * 1024, 'bounded CLI response');
  return readFileSync(outPath, 'utf8');
}
function invoke<T>(fixture: Fixture, args: string[]): T {
  return JSON.parse(execute(fixture, fixture.brit, args)) as T;
}
function capture(fixture: Fixture): Capture {
  return invoke(fixture, [
    'snapshot',
    'capture',
    '--source',
    fixture.source,
    '--store',
    fixture.store,
    '--',
    ...selected,
  ]);
}
function restore(fixture: Fixture, root: string, destination: string): void {
  invoke(fixture, [
    'snapshot',
    'restore',
    '--root',
    root,
    '--store',
    fixture.store,
    '--destination',
    destination,
  ]);
}

Given('Matthew has local source files outside Git and a separate private store', function () {
  assert.equal(process.platform, 'linux', 'this fixture exercises the Linux local contract');
  assert.ok(process.env.BRIT_BIN, 'set BRIT_BIN to the exact candidate executable');
  assert.ok(process.env.ARK_BIN, 'set ARK_BIN to the installed Ark executable');
  const base = mkdtempSync(join(tmpdir(), 'brit-local-story-'));
  const source = join(base, 'source');
  mkdirSync(source);
  const env = Object.fromEntries(
    Object.entries(process.env).filter(([key]) => !key.startsWith('GIT_'))
  );
  const fixture = {
    base,
    source,
    store: join(base, 'store'),
    brit: resolve(process.env.BRIT_BIN),
    ark: resolve(process.env.ARK_BIN),
    env,
  };
  fixtures.set(this, fixture);
  writeFileSync(join(source, 'value'), valueA);
  writeFileSync(join(source, 'binary'), Buffer.from([0, 255, 1, 10]));
  writeFileSync(join(source, 'executable'), 'selected executable bytes');
  chmodSync(join(source, 'executable'), 0o755);
  symlinkSync('value', join(source, 'link'));
  writeFileSync(join(source, 'omitted'), 'not selected');
  writeFileSync(join(source, 'post-checkout'), `#!/bin/sh\ntouch '${join(base, 'hook-ran')}'\n`);
  chmodSync(join(source, 'post-checkout'), 0o755);
  assert.ok(!existsSync(join(source, '.git')));
});

When(
  'he captures only his selected files as candidate A and the capture process closes',
  function () {
    const fixture = state(this);
    fixture.capture = capture(fixture);
    // Each invoke is a new CLI process reopening the same on-disk store.
    assert.equal(capture(fixture).root, fixture.capture.root);
    assert.ok(!existsSync(join(fixture.base, 'hook-ran')));
  }
);
When('he changes his original source to B and restores A in a fresh process', function () {
  const fixture = state(this);
  assert.ok(fixture.capture);
  writeFileSync(join(fixture.source, 'value'), valueB);
  restore(fixture, fixture.capture.root, join(fixture.base, 'restored'));
});
Then('restored A retains the selected bytes, executable mode and literal link', function () {
  const restored = join(state(this).base, 'restored');
  assert.equal(readFileSync(join(restored, 'value'), 'utf8'), valueA);
  assert.deepEqual(readFileSync(join(restored, 'binary')), Buffer.from([0, 255, 1, 10]));
  assert.equal(lstatSync(join(restored, 'executable')).mode & 0o777, 0o755);
  assert.equal(readlinkSync(join(restored, 'link')), 'value');
});
Then('omitted files stay absent, captured hooks stay inert and B remains editable', function () {
  const fixture = state(this);
  const alphabetically = (a: string, b: string): number => a.localeCompare(b);
  assert.deepEqual(
    readdirSync(join(fixture.base, 'restored')).sort(alphabetically),
    [...selected].sort(alphabetically)
  );
  assert.ok(!existsSync(join(fixture.base, 'hook-ran')));
  assert.equal(readFileSync(join(fixture.source, 'value'), 'utf8'), valueB);
  writeFileSync(join(fixture.source, 'value'), 'B: another independent edit\n');
  assert.equal(
    readFileSync(join(fixture.source, 'value'), 'utf8'),
    'B: another independent edit\n'
  );
});
Then('the capture reports an unpublished candidate', function () {
  assert.equal(state(this).capture?.published, false);
});

When(
  'he explicitly runs a pinned native task against A while editing his original source to B',
  { timeout: 60_000 },
  async function () {
    const fixture = state(this);
    assert.ok(fixture.capture);
    const artifact = join(fixture.base, 'fixture');
    const rustc = process.env.RUSTC_BIN;
    assert.ok(rustc && isAbsolute(rustc), 'set RUSTC_BIN to the absolute pinned 1.100.0 compiler');
    execute(fixture, rustc, [
      '--edition=2021',
      '-C',
      'opt-level=1',
      resolve('scripts/brit-native-source-fixture.rs'),
      '-o',
      artifact,
    ]);
    const run = join(fixture.base, 'run');
    const request = {
      method: {
        name: 'brit-gate:native-source-loop',
        step: 'native-source-loop',
        deterministic: false,
      },
      child: {
        artifact: {
          pinned: { sha256: createHash('sha256').update(readFileSync(artifact)).digest('hex') },
        },
        argv: ['{artifact}', '{input}', '{output}'],
        env: { PATH: '', HOME: join(run, 'home') },
        envScrub: true,
      },
      outputs: ['answer'],
    };
    const requestPath = join(fixture.base, 'request.json');
    writeFileSync(requestPath, JSON.stringify(request));
    const stdoutPath = join(fixture.base, 'task-stdout');
    const stderrPath = join(fixture.base, 'task-stderr');
    const out = openSync(stdoutPath, 'wx');
    const err = openSync(stderrPath, 'wx');
    const child = spawn(
      fixture.brit,
      [
        'build',
        'run',
        '--root',
        fixture.capture.root,
        '--store',
        fixture.store,
        '--request',
        requestPath,
        '--artifact',
        artifact,
        '--run-dir',
        run,
      ],
      {
        cwd: fixture.base,
        env: { ...fixture.env, ARK_BIN: fixture.ark, PATH: '' },
        stdio: ['ignore', out, err],
      }
    );
    closeSync(out);
    closeSync(err);
    fixture.child = child;
    fixture.closed = new Promise((resolveClose, reject) => {
      child.once('error', reject);
      child.once('close', resolveClose);
    });
    const deadline = Date.now() + 25_000;
    while (!existsSync(join(run, 'output', 'ready'))) {
      assert.equal(
        child.exitCode,
        null,
        `task exited before synchronization: ${readFileSync(stderrPath, 'utf8')}`
      );
      assert.ok(
        Date.now() < deadline,
        `task did not reach synchronization: ${readFileSync(stderrPath, 'utf8')}`
      );
      await delay(20);
    }
    writeFileSync(join(fixture.source, 'value'), valueB);
    writeFileSync(join(run, 'output', 'continue'), 'continue');
    assert.equal(await fixture.closed, 0, readFileSync(stderrPath, 'utf8'));
    fixture.outcome = JSON.parse(readFileSync(stdoutPath, 'utf8')) as Outcome;
  }
);
Then("the stored output contains only A's answer and B survives", function () {
  const fixture = state(this);
  assert.ok(fixture.outcome);
  assert.equal(fixture.outcome.outcome, 'done');
  const output = join(fixture.base, 'output-restored');
  restore(fixture, fixture.outcome.output, output);
  assert.deepEqual(readdirSync(output), ['answer']);
  assert.equal(readFileSync(join(output, 'answer'), 'utf8'), valueA);
  assert.equal(readFileSync(join(fixture.source, 'value'), 'utf8'), valueB);
  assert.ok(!existsSync(join(fixture.base, 'hook-ran')));
});
Then(
  'the result binds A and the original task specification without a reusable gate attestation',
  function () {
    const fixture = state(this);
    assert.ok(fixture.outcome);
    assert.equal(fixture.outcome.key.input.form, 'projected');
    assert.equal(fixture.outcome.key.input.cid, fixture.capture?.root);
    assert.equal(fixture.outcome.hit, false);
    assert.equal(fixture.outcome.record, null);
    const spec = JSON.parse(readFileSync(join(fixture.base, 'run', 'spec.json'), 'utf8')) as {
      child: { argv: string[]; artifact: unknown };
    };
    const declared = JSON.parse(
      execute(fixture, fixture.ark, [
        'task',
        'spec',
        '--root',
        fixture.capture!.root,
        '--store',
        fixture.store,
        '--request',
        join(fixture.base, 'request.json'),
      ])
    ) as { derivation: { method: string }; spec: { child: unknown } };
    assert.equal(fixture.outcome.key.method, declared.derivation.method);
    const request = JSON.parse(readFileSync(join(fixture.base, 'request.json'), 'utf8')) as {
      child: { argv: string[]; artifact: unknown };
    };
    assert.deepEqual(request.child.argv, ['{artifact}', '{input}', '{output}']);
    assert.deepEqual(spec.child.argv, request.child.argv);
    // The parsed TaskSpec writes explicit nulls for omitted optional artifact fields.
    assert.deepEqual(spec.child, declared.spec.child);
  }
);
Then("capturing B gives a different candidate which cannot inherit A's result", function () {
  const fixture = state(this);
  const second = capture(fixture);
  assert.notEqual(second.root, fixture.capture?.root);
  assert.notEqual(second.root, fixture.outcome?.key.input.cid);
});
When('he attempts to capture the unsupported selection {string}', function (selection: string) {
  const fixture = state(this);
  assert.throws(
    () =>
      invoke(fixture, [
        'snapshot',
        'capture',
        '--source',
        fixture.source,
        '--store',
        fixture.store,
        '--',
        selection,
      ]),
    /REFUSED|refused|selection|relative|literal|metadata/
  );
  fixture.refused = true;
});
Then('Matthew receives an explicit unsupported-selection refusal', function () {
  assert.equal(state(this).refused, true);
});
Then('no candidate store is created and his source stays intact', function () {
  const fixture = state(this);
  assert.equal(fixture.refused, true);
  assert.ok(!existsSync(fixture.store));
  assert.equal(readFileSync(join(fixture.source, 'value'), 'utf8'), valueA);
});
After({ tags: '@local-native', timeout: 20_000 }, async function () {
  const fixture = fixtures.get(this);
  if (!fixture) return;
  if (fixture.child?.exitCode === null && fixture.child.signalCode === null) {
    fixture.child.kill('SIGTERM');
    await fixture.closed;
  }
  rmSync(fixture.base, { recursive: true, force: true });
  fixtures.delete(this);
});
