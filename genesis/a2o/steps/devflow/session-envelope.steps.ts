/** Native fixture stores and fake runtime ancestors; no deployed resources. */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

import { After, Given, Then, When } from '@cucumber/cucumber';

const fixtures = new WeakMap<object, string>();
const driver = resolve('../agentic/session_envelope_test.py');
function stage(world: object, name: string): void {
  const base = fixtures.get(world);
  assert.ok(base, 'fixture must be created');
  const result = spawnSync('/usr/bin/python3', [driver, '--base', base, '--stage', name], {
    env: process.env,
    encoding: 'utf8',
    timeout: 90_000,
  });
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
}
function setup(world: object): void {
  fixtures.set(world, mkdtempSync(join(tmpdir(), 'a2o-envelope-')));
  stage(world, 'setup');
}
Given(
  "Matthew's test device runs a Claude Code session and a Codex session, each started by its own agent runtime process",
  { timeout: 90_000 },
  function () {
    setup(this);
  }
);
When(
  'each session opens its envelope and takes its own leases and work claims',
  { timeout: 90_000 },
  function () {
    stage(this, 'acquire');
  }
);
Then('each envelope lists only its own leases and claims, under its own session id', function () {
  stage(this, 'joined');
});
When('the Claude Code session is closed twice', function () {
  stage(this, 'close_claude');
});
Then(
  'its leases are released, its covenant is sealed and exactly one close witness is written, while the Codex session still holds its lease',
  function () {
    stage(this, 'claude_closed');
  }
);
When(
  "the Codex runtime process is killed and the device's session collector runs twice",
  function () {
    stage(this, 'collect');
  }
);
Then(
  "the Codex session's leases are released, its covenant is sealed and exactly one close witness is written for it",
  function () {
    stage(this, 'all_closed');
  }
);
Given(
  "Matthew's test device has a session pinned to one stored epr build",
  { timeout: 90_000 },
  function () {
    setup(this);
  }
);
When(
  'the session-start hook opens recall, and epr is rebuilt from source before the next prompt hook runs',
  { timeout: 90_000 },
  function () {
    stage(this, 'hooks');
  }
);
Then(
  "both hooks find recall under the session's envelope id and run the same pinned epr build, with the same CID",
  function () {
    const base = fixtures.get(this);
    assert.ok(base);
    const proof = JSON.parse(readFileSync(join(base, 'hook-proof.json'), 'utf8')) as {
      session: string;
      executor: string;
    };
    assert.equal(proof.session, 'claude-code-4101-77');
    assert.equal(proof.executor, readFileSync(join(base, 'cid'), 'utf8'));
  }
);
After(function () {
  const base = fixtures.get(this);
  if (base) rmSync(base, { recursive: true, force: true });
});
