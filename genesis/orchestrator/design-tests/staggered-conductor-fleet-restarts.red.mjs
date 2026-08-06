// Standing reds for the restart-mitigation design package.
//
// This file is intentionally NOT in package.json's default test list. Run it
// explicitly until the fenced deployment implementation is available to cure
// the invariants below:
//   node genesis/orchestrator/design-tests/staggered-conductor-fleet-restarts.red.mjs

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const ORCHESTRATOR = resolve(HERE, '..');
const ROOT = resolve(ORCHESTRATOR, '..', '..');

const JENKINS_PATH = join(ROOT, 'elohim', 'holochain', 'Jenkinsfile');
const DEPLOYMENTS_PATH = join(ORCHESTRATOR, 'data', 'deployments.json');
const MANIFESTS = [
  join(ORCHESTRATOR, 'manifests', 'humans', '_edgenode-consolidated.template.yaml'),
  join(ORCHESTRATOR, 'manifests', 'humans', 'adam-firstman.yaml'),
];

const jenkins = readFileSync(JENKINS_PATH, 'utf8');
const deployments = JSON.parse(readFileSync(DEPLOYMENTS_PATH, 'utf8'));

function extractIndentedMapping(source, key) {
  const lines = source.split('\n');
  const start = lines.findIndex(line => line === `  ${key}:`);
  assert.notEqual(start, -1, `missing two-space mapping: ${key}`);

  let end = lines.length;
  for (let i = start + 1; i < lines.length; i += 1) {
    if (/^ {2}[A-Za-z0-9_-]+:/.test(lines[i])) {
      end = i;
      break;
    }
    if (/^---\s*$/.test(lines[i])) {
      end = i;
      break;
    }
  }
  return lines.slice(start, end).join('\n');
}

function render(source, replacements) {
  let rendered = source;
  for (const [needle, value] of Object.entries(replacements)) {
    rendered = rendered.replaceAll(needle, value);
  }
  return rendered;
}

function groovyFunctionSource(signature, nextSignature) {
  const start = jenkins.indexOf(signature);
  assert.notEqual(start, -1, `missing Groovy function: ${signature}`);
  const end = jenkins.indexOf(nextSignature, start + signature.length);
  assert.notEqual(end, -1, `missing boundary after Groovy function: ${nextSignature}`);
  return jenkins.slice(start, end);
}

for (const manifestPath of MANIFESTS) {
  const name = manifestPath.split('/').at(-1);
  const source = readFileSync(manifestPath, 'utf8');

  test(`${name}: deploy provenance does not change the pod template`, () => {
    const first = render(source, { DEPLOY_VERSION_PLACEHOLDER: 'aaaaaaaa' });
    const second = render(source, { DEPLOY_VERSION_PLACEHOLDER: 'bbbbbbbb' });
    assert.equal(
      extractIndentedMapping(first, 'template'),
      extractIndentedMapping(second, 'template'),
      'Git-SHA-only drift currently changes .spec.template and starts RollingUpdate during apply',
    );
  });

  test(`${name}: a real hApp digest change remains rollout-visible`, () => {
    const first = render(source, { HAPP_DIGEST_PLACEHOLDER: 'sha256:first' });
    const second = render(source, { HAPP_DIGEST_PLACEHOLDER: 'sha256:second' });
    assert.notEqual(
      extractIndentedMapping(first, 'template'),
      extractIndentedMapping(second, 'template'),
      'the hApp content digest must remain part of the runtime-effective projection',
    );
  });
}

test('restart admission is independent of kubectl configured/unchanged wording', () => {
  const body = groovyFunctionSource(
    'def deployHumanManifest(',
    '/**\n * Deploy doorway manifest',
  );
  assert.doesNotMatch(
    body,
    /grep -Eq[^\n]+unchanged/,
    'restart admission currently keys on the raw StatefulSet apply word "unchanged"',
  );
});

test('active humans are not applied as one fleet-wide parallel wave', () => {
  const activeHumans = deployments.humans
    .filter(human => human.suspended !== true)
    .map(human => human.name);
  const body = groovyFunctionSource(
    'def deployHumansInParallel(',
    'def emitDeployJunit(',
  );

  assert.ok(activeHumans.length > 2, 'fixture must represent a multi-wave fleet');
  assert.doesNotMatch(
    body,
    /parallel\(branches\)/,
    `all ${activeHumans.length} active humans currently enter one parallel apply group: ${activeHumans.join(', ')}`,
  );
});

test('the human deploy dispatcher admits the next wave through convergence', () => {
  const body = groovyFunctionSource(
    'def deployHumansInParallel(',
    'def emitDeployJunit(',
  );
  assert.match(
    body,
    /(convergence|quiesce|caughtUp)/i,
    'the all-human dispatcher currently has no service-level convergence gate between applies',
  );
});
