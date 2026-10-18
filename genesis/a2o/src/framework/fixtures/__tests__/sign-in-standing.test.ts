/**
 * Every scenario signs in only as a fixture human who may sign in at all.
 *
 * Standing is derived from humans.json (agencyPhase) and deployments.json
 * (pods), never from a list kept here: see `signInStanding`. A feature that
 * logs in as a persona with no peer runtime and no hosted stage is
 * misdeclared; the seeder never registers that persona. A scenario about such
 * a persona asserts the refusal instead, and that refusal must name a human
 * who really lacks standing.
 */

import { strict as assert } from 'node:assert';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { describe, it } from 'node:test';
import { fileURLToPath } from 'node:url';

import { signInStanding } from '../humans.js';

const A2O_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../../../..');
const FEATURES = join(A2O_ROOT, 'features');

const SIGN_IN = /\bhuman "([^"]+)" is logged in on doorway(?:-app)? "/g;
const REFUSAL = /\bhuman "([^"]+)" is refused sign-in on doorway "/g;

function featureFiles(dir: string): string[] {
  const out: string[] = [];
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) out.push(...featureFiles(path));
    else if (name.endsWith('.feature')) out.push(path);
  }
  return out;
}

interface Use {
  human: string;
  where: string;
}

function uses(pattern: RegExp): Use[] {
  const found: Use[] = [];
  for (const file of featureFiles(FEATURES)) {
    const lines = readFileSync(file, 'utf-8').split('\n');
    lines.forEach((line, i) => {
      if (line.trimStart().startsWith('#')) return;
      for (const m of line.matchAll(pattern)) {
        found.push({ human: m[1], where: `${relative(A2O_ROOT, file)}:${i + 1}` });
      }
    });
  }
  return found;
}

void describe('a2o sign-in standing', () => {
  void it('finds the sign-in steps it is meant to police', () => {
    assert.ok(uses(SIGN_IN).length > 0, 'no named sign-in steps found; the pattern drifted');
  });

  void it('signs in only as humans with their own peer runtime or a hosted stage', () => {
    const violations = uses(SIGN_IN)
      .map(u => ({ ...u, standing: signInStanding(u.human) }))
      .filter(u => !u.standing.allowed)
      .map(u => `${u.where}: "${u.human}" — ${u.standing.reason}`);
    assert.deepEqual(
      violations,
      [],
      'scenarios sign in as humans with no standing; rehome them or assert ' +
        '\'human "<Name>" is refused sign-in on doorway "<id>"\':\n  ' +
        violations.join('\n  ')
    );
  });

  void it('asserts refusal only for humans who really lack standing', () => {
    const wrong = uses(REFUSAL)
      .filter(u => signInStanding(u.human).allowed)
      .map(u => `${u.where}: "${u.human}" has standing (${signInStanding(u.human).reason})`);
    assert.deepEqual(wrong, [], wrong.join('\n'));
  });
});
