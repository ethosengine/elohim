// The eprfs pipeline in CI builds what the device's eprfs gate builds before its tests run.
//
// `bin_build_view` (elohim/eprfs/epr-cli/tests) reads the dep-info cargo writes beside `epr`, and
// cargo writes it only when the bin is built as a top-level target. The device recipe
// (`_gate-eprfs` in the root justfile) got a `bins` step for that on 2026-11-17, with the test.
// The pipeline's tests stage did not, so `elohim-eprfs/dev` failed on that one test in every run
// ("no dep-info at …/target/debug/epr.d") and held the orchestrator's baseline, while every
// device gate stayed green. Found 2026-11-19 from the Jenkins log, two days later.
// This file fails in seconds when the two recipes part again.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const read = rel => readFileSync(resolve(ROOT, rel), 'utf8');

/** The `_gate-eprfs` recipe's text, from its name to the blank line that ends it. */
function gateRecipe() {
  const justfile = read('justfile');
  const start = justfile.indexOf('\n_gate-eprfs:');
  assert.ok(start >= 0, 'the root justfile has a _gate-eprfs recipe');
  const end = justfile.indexOf('\n\n', start + 1);
  return justfile.slice(start, end < 0 ? undefined : end);
}

/** The command of the recipe's `<name> -- <command>` step (gate-steps.sh's form). */
function gateStep(name) {
  const step = new RegExp(`^\\s*(?:::: )?${name} -- (.*?)\\s*\\\\$`, 'm').exec(gateRecipe());
  assert.ok(step, `_gate-eprfs has a "${name} --" step on one line`);
  return step[1];
}

/** The pipeline's tests stage, from its name to the next stage. */
function testsStage() {
  const jenkinsfile = read('elohim/eprfs/Jenkinsfile');
  const start = jenkinsfile.indexOf("stage('Rust — tests')");
  assert.ok(start >= 0, "elohim/eprfs/Jenkinsfile has a stage named 'Rust — tests'");
  const end = jenkinsfile.indexOf('\n    stage(', start + 1);
  return jenkinsfile.slice(start, end < 0 ? undefined : end);
}

/** Shell lines of the stage that are commands: comments and blank lines dropped. */
const commands = text => text.split('\n').map(line => line.trim()).filter(line => line && !line.startsWith('#') && !line.startsWith('//'));

test('the device recipe builds epr and bin-launch as top-level targets before its tests', () => {
  const bins = gateStep('bins');
  assert.match(bins, /^cargo build\b/);
  assert.match(bins, /--bin epr\b/, 'bin_build_view reads the dep-info this build writes beside epr');
  assert.match(bins, /--bin bin-launch\b/);
  const recipe = gateRecipe();
  assert.ok(recipe.indexOf('bins -- ') < recipe.indexOf('test -- '), 'the bins step comes before the test step');
});

test('the eprfs pipeline runs the same build before cargo nextest, in the same stage', () => {
  const bins = gateStep('bins');
  const lines = commands(testsStage());
  const built = lines.indexOf(bins);
  const tested = lines.findIndex(line => line.startsWith('cargo nextest run'));
  assert.ok(tested >= 0, 'the tests stage runs cargo nextest');
  assert.ok(built >= 0,
    `the pipeline's tests stage must run the device recipe's bins step as a command of its own:\n  ${bins}\n` +
    'without it cargo writes no epr.d and bin_build_view fails in CI while every device gate is green');
  assert.ok(built < tested, 'the build comes before cargo nextest');
});

test('a comment that only mentions the build does not satisfy the check', () => {
  const bins = gateStep('bins');
  const stage = `stage('Rust — tests') {\n  # ${bins}\n  cargo nextest run --workspace\n}`;
  assert.equal(commands(stage).indexOf(bins), -1);
});
