import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';

const paths = ['../../Jenkinsfile', '../../elohim/holochain/Jenkinsfile',
  '../../app/elohim-library/Jenkinsfile', '../../elohim/holochain/dna/Jenkinsfile', './Jenkinsfile'];
const files = new Map(paths.map(path => [path, readFileSync(new URL(path, import.meta.url), 'utf8')]));
const branches = ['PR-215', 'feat-closure', 'claude/closure', 'ci/manual', 'staging', 'staging-test', 'dev-test', '', 'DEV'];

function balancedBlock(text, openBrace) {
  let depth = 1;
  let i = openBrace + 1;

  while (i < text.length && depth > 0) {
    if (text.startsWith("//", i)) {
      i = text.indexOf("\n", i + 2);
      if (i === -1) return text.slice(openBrace + 1);
      continue;
    }
    if (text.startsWith("/*", i)) {
      const end = text.indexOf("*/", i + 2);
      assert.notEqual(
        end,
        -1,
        "unterminated block comment while parsing Jenkinsfile",
      );
      i = end + 2;
      continue;
    }
    if (text[i] === "'" || text[i] === '"') {
      const quote = text[i];
      const triple = text.startsWith(quote.repeat(3), i);
      i += triple ? 3 : 1;
      while (i < text.length) {
        if (triple && text.startsWith(quote.repeat(3), i)) {
          i += 3;
          break;
        }
        if (!triple && text[i] === quote) {
          i += 1;
          break;
        }
        if (text[i] === "\\") i += 1;
        i += 1;
      }
      continue;
    }
    if (text[i] === "{") depth += 1;
    if (text[i] === "}") depth -= 1;
    i += 1;
  }

  assert.equal(depth, 0, "unterminated Jenkinsfile block");
  return text.slice(openBrace + 1, i - 1);
}

function method(text, name) {
  const start = text.search(new RegExp(`^def ${name}\\(`, 'm'));
  assert.ok(start >= 0, `missing helper ${name}`);
  return balancedBlock(text, text.indexOf('{', start));
}

function stages(text) {
  const result = new Map();
  for (const m of text.matchAll(/stage\(\s*(['"])([^'"]+)\1\s*\)\s*\{/g)) {
    const body = balancedBlock(text, m.index + m[0].lastIndexOf('{'));
    const at = body.indexOf('when {');
    if (at >= 0) result.set(m[2], balancedBlock(body, body.indexOf('{', at)));
  }
  return result;
}

// Parse branch arms as a tree, retaining anyOf/allOf precedence. Non-authority
// predicates (changesets, force flags, selected steps, credentials) are allowed:
// a branch restriction must hold even when ALL other gates would let it run.
function allows(when, branch, text) {
  const env = { BRANCH_NAME: branch };
  const gate = files.get('../../elohim/holochain/Jenkinsfile');
  const orchestrator = files.get('./Jenkinsfile');
  const isGateOnly = () => new Function('env', `return ${method(gate, 'isGateOnly').trim()}`)(env);
  const gateOnlyRun = () => new Function('env', `return ${method(orchestrator, 'gateOnlyRun').trim()}`)(env);
  function expression(code) {
    // A source expression carrying branch authority is evaluated literally.
    // Neutralize unrelated predicates without neutralizing their &&/|| shape.
    code = code.replace(/readJSON\(text: env.BUILD_RESULTS \?: '\{\}'\)\.any \{ it.value\?\.success \}/g, 'true')
      .replace(/\(params.MODE \?: 'auto'\) != 'status'|\(env.PIPELINES_TO_RUN \?: ''\) != ''/g, 'true')
      .replace(/env\.PIPELINE_SKIPPED != 'true'|env\.DEPLOY_REFUSED != 'true'/g, 'true')
      .replace(/!isValidateOnly\(\)|isValidateOnly\(\)|!skipBuildStage\(\)/g, 'true')
      .replace(/!owesValidationToSibling\(params\.RUN_CLASS\)/g, 'true')
      .replace(/params\.[A-Z_]+(?:\s*(?:==|!=)\s*(?:true|false))?/g, 'true')
      .replace(/env\.BRANCH_NAME ==~ \/((?:\\.|[^/])+)\//g, (_, regex) => `new RegExp(${JSON.stringify(`^(?:${regex})$`)}).test(env.BRANCH_NAME)`);
    if (!/BRANCH_NAME|isGateOnly|gateOnlyRun|skipPublishStage/.test(code)) return true;
    const skipPublishStage = () => new Function('skipBuildStage', 'isGateOnly',
      `return ${method(gate, 'skipPublishStage').trim()}`)(() => false, isGateOnly);
    return Boolean(new Function('env', 'isGateOnly', 'gateOnlyRun', 'skipPublishStage',
      `return (${code})`)(env, isGateOnly, gateOnlyRun, skipPublishStage));
  }
  function children(source) {
    const nodes = [];
    source = source.replace(/\/\/[^\n]*/g, '').trim();
    while (source) {
      source = source.replace(/^[\s;]+/, '');
      if (!source) break;
      const block = source.match(/^(allOf|anyOf|not|expression)\s*\{/);
      if (block) {
        const open = source.indexOf('{'), inside = balancedBlock(source, open);
        const type = block[1];
        nodes.push(type === 'expression' ? expression(inside.trim()) :
          type === 'anyOf' ? children(inside).some(Boolean) :
          type === 'not' ? !children(inside).every(Boolean) : children(inside).every(Boolean));
        source = source.slice(open + inside.length + 2); continue;
      }
      const branchNode = source.match(/^branch\s+(['"])([^'"]+)\1/);
      if (branchNode) {
        const regex = '^' + branchNode[2].replace(/[.+?^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '.*') + '$';
        nodes.push(new RegExp(regex).test(branch));
        source = source.slice(branchNode[0].length); continue;
      }
      const other = source.match(/^(?:changeset|beforeAgent|beforeInput|beforeOptions)\s+(?:['"][^'"]*['"]|true|false)/);
      assert.ok(other, `unknown when arm: ${source.slice(0, 120)}`);
      nodes.push(true); source = source.slice(other[0].length);
    }
    return nodes;
  }
  return children(when).every(Boolean);
}

const publicationStages = new Map([
  ['../../Jenkinsfile', ['Apply Ingress (pre-upload)', 'Publish and Verify App Delivery', 'Push to Harbor Registry', 'Deploy to Staging', '🚀 Deploy to Alpha', 'Deploy to Prod']],
  ['../../elohim/holochain/Jenkinsfile', ['Package Compute Worker', 'Push to Harbor', 'Deploy Edge Node - Alpha', 'Deploy Edge Node - Staging', 'Deploy Edge Node - Prod', 'Dataplane Validation']],
  ['../../app/elohim-library/Jenkinsfile', ['Push to Harbor Registry', 'Deploy', 'Verify Deploy']],
  ['../../elohim/holochain/dna/Jenkinsfile', ['Push to Harbor', 'Fleet Coordinator Hot-Swap']],
  ['./Jenkinsfile', ['Verify Deployment', 'E2E Federation Smoke (Advisory)']],
]);

for (const [path, names] of publicationStages) {
  test(`${path}: no publication/deployment branch arm admits an off-integration branch`, () => {
    const text = files.get(path), parsed = stages(text);
    for (const name of names) {
      assert.ok(parsed.has(name), `missing stage ${name}`);
      for (const branch of branches) assert.equal(allows(parsed.get(name), branch, text), false, `${name} admits ${branch}`);
    }
    // Every stage with an explicit registry writer is in the audited set, except
    // Build DNA: its in-stage candidate helper has its own branch guard below.
    for (const m of text.matchAll(/stage\(\s*(['"])([^'"]+)\1\s*\)\s*\{/g)) {
      const body = balancedBlock(text, m.index + m[0].lastIndexOf('{'));
      if (/nerdctl[^\n]*push |oras push |\b(?:pnpm|npm) publish\b/.test(body)) {
        if (path === '../../Jenkinsfile' && m[2] === 'Install Dependencies') {
          const guard = body.indexOf('if (mayPublishPackages()) {');
          assert.ok(guard >= 0, 'Nexus writer must be guarded');
          const guarded = balancedBlock(body, body.indexOf('{', guard));
          assert.match(guarded, /\bpnpm publish\b/);
          assert.doesNotMatch(body.replace(guarded, ''), /\b(?:pnpm|npm) publish\b/, 'no package writer may escape the guard');
        } else assert.ok(names.includes(m[2]), `unreviewed writer stage ${m[2]}`);
      }
    }
  });
}

test('negative control: a permissive force arm cannot defeat the branch boundary', () => {
  assert.equal(allows("anyOf { branch 'dev'; expression { params.FORCE_BUILD } }", 'feat-x', ''), true);
  assert.equal(allows("allOf { expression { !isGateOnly() }; anyOf { branch 'dev'; expression { params.FORCE_BUILD } } }", 'feat-x', ''), false);
  assert.equal(allows("branch 'dev'", 'dev', ''), true);
  assert.equal(allows("branch 'main'", 'main', ''), true);
});

test('DNA candidate is guarded before credentials; dev-latest is dev-only', () => {
  const dna = files.get('../../elohim/holochain/dna/Jenkinsfile');
  const body = method(dna, 'pushHappCandidate');
  assert.match(body, /^\s*if \(env.BRANCH_NAME != 'dev' && env.BRANCH_NAME != 'main'\) return/);
  assert.match(dna, /def floatingTag = \(env.BRANCH_NAME == 'dev'\) \? 'dev-latest' : 'latest'/);
});

test('every off-integration wave narrows to edge build class, including untagged manual branches', () => {
  const text = files.get('./Jenkinsfile');
  for (const branch of branches) assert.equal(new Function('env', `return ${method(text, 'gateOnlyRun').trim()}`)({BRANCH_NAME:branch}), true);
  assert.match(method(text, 'applyGateOnlyOverride'), /if \(!gateOnlyRun\(\)\) return graphPipelines/);
  assert.match(method(text, 'applyGateOnlyOverride'), /env.PIPELINE_RUN_CLASSES = 'elohim-edge=build'/);
  assert.match(method(text, 'applyGateOnlyOverride'), /return \['elohim-edge'\]/);
  assert.match(text, /if \(gateOnlyDownstream\) jobBranch = env.CHANGE_ID \? "PR-\$\{env.CHANGE_ID\}" : env.BRANCH_NAME/);
  assert.match(text, /def isDevBranch = !env.CHANGE_ID && env.BRANCH_NAME == 'dev'\s*\n/);
  assert.match(text, /cron\(!env.CHANGE_ID && env.BRANCH_NAME == 'dev' \? '0 9 \* \* \*' : ''\)/);
});

test('off-integration edge waves run strict gates and local closure image builds', () => {
  const edge = files.get('../../elohim/holochain/Jenkinsfile');
  for (const name of ['runDoorwayStageBody', 'runStorageStageBody']) assert.match(method(edge, name), /^\s*if \(closurePresent/);
  // Execute the reuse guard for both oracle outcomes; gate-only must never skip.
  const present = new Function('isGateOnly', 'env', 'kind', method(edge, 'closurePresent').trim().split('\n')[0].replace('"${kind}_CLOSURE_HIT"', '`${kind}_CLOSURE_HIT`') + '; return true');
  for (const branch of branches) {
    const gateOnly = () => new Function('env', `return ${method(edge, 'isGateOnly').trim()}`)({BRANCH_NAME:branch});
    for (const hit of ['true', 'false']) assert.equal(present(gateOnly, {STORAGE_CLOSURE_HIT:hit}, 'STORAGE'), false);
  }
  for (const name of ['runStorageQualityGate', 'runDoorwayQualityGate']) {
    assert.match(method(edge, name), /if \(isGateOnly\(\)\) \{ throw e \}/);
  }
  assert.match(method(edge, 'runDoorwayStageBody'), /build-storage-image\.sh' build-doorway/);
  assert.match(method(edge, 'runStorageStageBody'), /buildStorageImage\(\)/);
  assert.match(edge, /GATE_ONLY — strict cargo quality gates and local builds; publish, deploy and dataplane validation disabled/);
  const parsed = stages(edge);
  for (const name of ['Build Doorway', 'Build Storage']) {
    for (const branch of branches) assert.equal(allows(parsed.get(name), branch, edge), true, `${name}: ${branch}`);
  }
  for (const name of ['Build Doorway App', 'Build Agent SDK', 'Build Edge Node Image', 'Build hApp Installer']) {
    for (const branch of branches) assert.equal(allows(parsed.get(name), branch, edge), false, `${name}: ${branch}`);
  }
});


test('dev-to-main and main-sourced PRs dispatch only their PR job in build class', () => {
  const text = files.get('./Jenkinsfile');
  assert.doesNotMatch(text, /^\s*BRANCH_NAME\s*=/m);
  assert.match(text, /SOURCE_BRANCH = "\$\{env.CHANGE_BRANCH \?: env.BRANCH_NAME \?: ''\}"/);
  assert.match(text, /branches: \[\[name: "\*\/\$\{env.SOURCE_BRANCH\}"\]\]/);
  const predicate = new Function('env', `return ${method(text, 'gateOnlyRun').trim()}`);
  const branchLine = text.match(/if \(gateOnlyDownstream\) jobBranch = ([^\n]+)/)[1];
  const branch = new Function('env', `return ${branchLine.replace('"PR-${env.CHANGE_ID}"', '`PR-${env.CHANGE_ID}`')}`);
  const cron = text.match(/cron\((.*?) \? '0 9/)[1];
  const nightly = new Function('env', `return ${cron}`);
  for (const source of ['dev', 'main', 'feature/test']) {
    const env = {BRANCH_NAME: 'PR-215', CHANGE_ID: '215', CHANGE_BRANCH: source, CHANGE_TARGET: 'main'};
    assert.equal(predicate(env), true);
    assert.equal(branch(env), 'PR-215');
    assert.equal(nightly(env), false);
    // CHANGE_ID independently denies even if a caller accidentally supplies the source as job branch.
    assert.equal(predicate({...env, BRANCH_NAME: source}), true);
    assert.equal(nightly({...env, BRANCH_NAME: source}), false);
  }
  for (const name of ['dev', 'main']) assert.equal(predicate({BRANCH_NAME: name}), false);
});

test('Nexus publication guard executes for every job; local install and compilation precede it', () => {
  const text = files.get('../../Jenkinsfile');
  const allowed = new Function('env', `return ${method(text, 'mayPublishPackages').trim()}`);
  const start = text.indexOf('if (mayPublishPackages()) {');
  const guarded = balancedBlock(text, text.indexOf('{', start));
  const block = guarded.match(/sh '''([\s\S]*?)'''/)[1];
  assert.ok(text.lastIndexOf("sh 'pnpm install --frozen-lockfile && pnpm run build'", start) > text.indexOf("stage('Install Dependencies')"));
  const tmp = mkdtempSync(new URL('./.publish-test-', import.meta.url));
  try {
    for (const command of ['node', 'npm', 'pnpm']) writeFileSync(join(tmp, command), '#!/bin/bash\nprintf "%s\\n" "' + command + ' $*" >> "$CALLS"\n' + (command === 'node' ? 'echo 1.2.3\n' : command === 'npm' ? 'exit 1\n' : ''), {mode: 0o755});
    for (const name of [...branches, 'dev', 'main']) for (const change of ['', '215']) {
      const calls = join(tmp, 'calls');
      writeFileSync(calls, '');
      const result = spawnSync('bash', ['-c', allowed({BRANCH_NAME: name, CHANGE_ID: change}) ? block : 'true'], {encoding: 'utf8', env: {...process.env, PATH: `${tmp}:${process.env.PATH}`, CALLS: calls, BRANCH_NAME: name, CHANGE_ID: change}});
      assert.equal(result.status, 0, result.stderr);
      assert.equal(readFileSync(calls, 'utf8').includes('pnpm publish'), !change && ['dev', 'main'].includes(name), `${name} CHANGE_ID=${change}`);
    }
  } finally { rmSync(tmp, {recursive: true, force: true}); }
});
