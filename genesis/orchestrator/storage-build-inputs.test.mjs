/**
 * Storage build inputs — one dependency surface, three lists kept honest
 *
 * The storage binary is compiled in Docker from a build context that includes
 * its sibling path-dep crates. Three lists describe that one surface:
 *
 *   1. the `path = "…"` closure of elohim/elohim-storage/Cargo.toml (the truth)
 *   2. the Dockerfile's COPY sources (what reaches the compiler)
 *   3. the edge manifest's `cargo-build-storage` source globs (what re-triggers
 *      the build)
 *
 * A crate in (1) missing from (2) fails the image build. A path in (2) missing
 * from (3) is worse: a change to that crate alone matches nothing, the
 * orchestrator does not rebuild edge, and the fleet runs a STALE storage binary
 * with no red anywhere (backlog edge-buildmanifest-sibling-crate-source-globs).
 * Every crate carved out of elohim-storage widens that hole unless this holds.
 *
 * Run:
 *   node --test genesis/orchestrator/storage-build-inputs.test.mjs
 */
import { test } from 'node:test';
import { spawnSync } from 'node:child_process';
import assert from 'node:assert/strict';
import { existsSync, readFileSync, mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { walkGraph } from './graph-walker.mjs';
import { dirname, join, normalize, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..', '..');
const STORAGE = 'elohim/elohim-storage';

/** Repo-relative dirs of every `path = "…"` dependency, transitively. */
function pathDepClosure(crateDir, seen = new Set()) {
  const manifest = join(root, crateDir, 'Cargo.toml');
  if (!existsSync(manifest)) return seen;
  const text = readFileSync(manifest, 'utf8');
  for (const m of text.matchAll(/\bpath\s*=\s*"([^"]+)"/g)) {
    const dep = relative(root, resolve(root, crateDir, m[1]));
    if (dep.startsWith('..') || seen.has(dep) || dep === STORAGE) continue;
    if (!existsSync(join(root, dep, 'Cargo.toml'))) continue; // a [[bin]]/[lib] path, not a crate
    seen.add(dep);
    pathDepClosure(dep, seen);
  }
  return seen;
}

/** Build-context sources the storage Dockerfile copies (stage copies excluded). */
function dockerCopySources() {
  const text = readFileSync(join(root, STORAGE, 'Dockerfile'), 'utf8');
  const sources = new Set();
  for (const line of text.split('\n')) {
    const m = line.match(/^COPY\s+(?!--from)(.+)$/);
    if (!m) continue;
    const parts = m[1].trim().split(/\s+/).filter(p => !p.startsWith('--'));
    parts.slice(0, -1).forEach(p => sources.add(normalize(p).replace(/\/$/, '')));
  }
  return sources;
}

function manifestSources() {
  const manifest = JSON.parse(readFileSync(join(root, 'elohim/holochain/build-manifest.json'), 'utf8'));
  return manifest.steps['cargo-build-storage'].inputs.sources;
}

/** True when `path` (a file or dir) falls under a `dir/**` glob or equals a literal source. */
function covered(path, globs) {
  return globs.some(g => {
    if (g.endsWith('/**')) {
      const base = g.slice(0, -3);
      return path === base || path.startsWith(`${base}/`) || base.startsWith(`${path}/`);
    }
    return g === path || g.startsWith(`${path}/`);
  });
}

test('every path-dep crate in the storage closure reaches the Docker build context', () => {
  const copies = [...dockerCopySources()];
  const missing = [...pathDepClosure(STORAGE)].filter(
    dep => !copies.some(c => dep === c || dep.startsWith(`${c}/`)),
  );
  assert.deepEqual(missing, [], `path-dep crates the storage Dockerfile never COPYs: ${missing.join(', ')}`);
});

test('every source the storage Dockerfile copies re-triggers the edge build', () => {
  const globs = manifestSources();
  const unwatched = [...dockerCopySources()].filter(src => !covered(src, globs));
  assert.deepEqual(
    unwatched,
    [],
    `copied into the storage image but absent from cargo-build-storage sources — a change there ships a stale binary: ${unwatched.join(', ')}`,
  );
});

// The Dockerfile builds storage from /app with every sibling crate COPY'd flat beside it, so each
// of storage's own relative path deps is re-rooted by a `sed 's|path = "<rel>"|…|'` line. A COPY
// without its rewrite still fails the image: cargo looks for /<crate>/Cargo.toml (edge #1484,
// elohim-epr-index at ../epr-index — COPY'd, never re-rooted, invisible to the native gates).
test("every direct path dep of storage is re-rooted by the Dockerfile's sed rewrites", () => {
  const cargo = readFileSync(join(root, STORAGE, 'Cargo.toml'), 'utf8');
  const docker = readFileSync(join(root, STORAGE, 'Dockerfile'), 'utf8');
  const copyDests = [...docker.matchAll(/^COPY\s+(?!--from)\S+\s+(\/\S+?)\/?$/gm)].map(m => m[1]);
  const unrooted = [...cargo.matchAll(/\bpath\s*=\s*"(\.\.\/[^"]+)"/g)]
    .map(m => m[1])
    .filter(rel => existsSync(join(root, STORAGE, rel, 'Cargo.toml')))
    .filter(rel => !docker.includes(`s|path = "${rel}"|`))
    // …or COPY'd to the absolute place the unrewritten path resolves to from /app (/sdk, /vendor).
    .filter(rel => {
      const abs = resolve('/app', rel);
      return !copyDests.some(d => abs === d || abs.startsWith(`${d}/`));
    });
  assert.deepEqual(
    unrooted,
    [],
    `storage path deps with no Dockerfile path rewrite (cargo resolves them outside /app): ${unrooted.join(', ')}`,
  );
});

// The same fixture exercises both scripts with fake Git/registry commands only.
// Including it here makes the existing orchestrator test lane run the shell suite.
test('closure images: derivation, fail-closed oracle and nerdctl config-label builder contract', () => {
  const result = spawnSync('bash', ['scripts/ci/closure-image.test.sh'], {
    cwd: root, encoding: 'utf8', timeout: 120000,
  });
  assert.equal(result.status, 0, result.stdout + result.stderr);
});

test('closure reuse shares one decision across gates, release builds and pushes', () => {
  const edge = readFileSync(join(root, 'elohim/holochain/Jenkinsfile'), 'utf8');
  assert.match(edge, /def runQualityGates\(\) \{\s*decideImageClosures\(\)/);
  assert.equal((edge.match(/^\s+decideImageClosures\(\)/gm) ?? []).length, 1);
  for (const [kind, title] of [['STORAGE', 'Storage'], ['DOORWAY', 'Doorway']]) {
    assert.ok(edge.includes(`closurePresent('${kind}', 'Quality Gate: ${title}')`));
    assert.ok(edge.includes(`closurePresent('${kind}', 'Build ${title}')`));
    assert.ok(edge.includes(`pushClosureImage('${kind}', 'elohim-${title.toLowerCase()}')`));
    assert.ok(edge.includes(`${kind}_CLOSURE_MATERIAL=`));
  }
  assert.match(edge, /if \(closurePresent\(kind, "Push \$\{imageName\}"\)\) \{[\s\S]*?return/);
  assert.doesNotMatch(edge, /push-harbor-(tracked|untracked)\.sh' elohim-(doorway|storage)/);
  assert.doesNotMatch(edge, /scripts\/ci\/push-harbor-dev-latest-aliases\.sh/);
  assert.match(edge, /def doorwayTag = "closure-\$\{doorwayKey\.substring\(7, 23\)\}"/);
  assert.match(edge, /def storageTag = "closure-\$\{storageKey\.substring\(7, 23\)\}"/);
});

test('held storage readiness is sampled once; only changed inputs wait for rollout', () => {
  const edge = readFileSync(join(root, 'elohim/holochain/Jenkinsfile'), 'utf8');
  const deploy = edge.slice(edge.indexOf('def deployHumanManifest('), edge.indexOf('def deployDoorwayManifest('));
  assert.match(deploy, /podInputsBefore && podInputsBefore == podInputsAfter/);
  assert.match(deploy, /if \(!reportHeldStorageReadiness\(humanConfig\)\) return false\s*\} else \{/);
  assert.match(deploy, /rollout restart statefulset[\s\S]*?waitForRolloutWithEvidence\('statefulset', humanConfig.resourcePrefix, humanConfig.namespace, '600s'\)\s*\}/);
  const helper = edge.match(/def reportHeldStorageReadiness\(Map human\) \{([\s\S]*?)^\}/m)[1];
  assert.equal((helper.match(/kubectl get pod/g) ?? []).length, 1);
  assert.match(helper, /unstable\("\$\{human.resourcePrefix\}: held, NotReady \(pre-existing\)"\)/);
  assert.doesNotMatch(helper, /rollout|timeout|sleep|waitFor/);
  assert.match(edge, /outcomes\[hc.resourcePrefix\] = deployHumanManifest\(hc, envFile, allHumans\)/);
  const held = deploy.match(/if \(!reportHeldStorageReadiness\(humanConfig\)\) return false/)[0];
  assert.ok(deploy.indexOf(held) < deploy.indexOf('CONDUCTOR_DEPLOY_QUEUE['));
  assert.match(deploy, /return true\s*\}/);
  const readiness = new Function('ready', 'human', 'echo', 'unstable', helper.slice(helper.indexOf("    if (ready")).replace(/"(\$\{human.resourcePrefix\}[^"\n]*)"/g, '`$1`'));
  for (const status of ['False', '', 'Unknown', 'True']) {
    let queued = false;
    const outcomes = {};
    const humanConfig = {resourcePrefix: 'peer'};
    const reportHeldStorageReadiness = () => readiness(status, humanConfig, () => {}, () => {});
    const runHeld = new Function('reportHeldStorageReadiness', 'humanConfig', 'enqueue', `${held}; enqueue(); return true`);
    outcomes.peer = runHeld(reportHeldStorageReadiness, humanConfig, () => { queued = true; });
    assert.equal(outcomes.peer, status === 'True');
    assert.equal(queued, status === 'True');
  }
});

test('deploy readers receive the selected closure and preserve unselected images', () => {
  const edge = readFileSync(join(root, 'elohim/holochain/Jenkinsfile'), 'utf8');
  const writer = edge.match(/def writeDeploymentVersions\([^\n]+\) \{([\s\S]*?)^\}/m)[1];
  assert.match(writer, /selected = params.DEPLOY_ONLY != true && shouldRunStep\("cargo-build-\$\{kind.toLowerCase\(\)\}"\)/);
  assert.match(writer, /\['TAG', 'CLOSURE', 'DIGEST'\]/);
  assert.match(writer, /selected \? \(env\[key\] \?: ''\) : ''/);
  assert.equal((edge.match(/^\s+writeDeploymentVersions\(/gm) ?? []).length, 4);
  assert.match(edge, /def storageInputs = versions.STORAGE_CLOSURE \?: ''/);
  assert.match(edge, /def pushClosureImage[^\n]+\{\s*if \(!shouldRunStep\("cargo-build-\$\{kind.toLowerCase\(\)\}"\)\) return/);
});


test('each closure recipe change selects both edge build steps through the installed graph matcher', () => {
  const content = JSON.parse(readFileSync(join(root, 'elohim/holochain/build-manifest.json'), 'utf8'));
  for (const file of ['build-storage-image.sh', 'closure-input-digest.sh', 'harbor-oracle.sh', 'storage-input-digest.sh']) {
    const path = `scripts/ci/${file}`;
    for (const step of ['cargo-build-storage', 'cargo-build-doorway']) assert.ok(content.steps[step].inputs.buildProcess.includes(path));
    const result = walkGraph([{content}], [path]);
    const edge = result.pipelines.find(p => p.name === 'elohim-edge');
    assert.ok(edge, JSON.stringify(result));
    for (const name of ['cargo-build-storage', 'cargo-build-doorway']) assert.ok(edge.steps.some(step => step.name === name));
  }
});

test('two clean checkouts retain each live closure image; DEPLOY_ONLY uses the same source and missing evidence refuses', () => {
  const edge = readFileSync(join(root, 'elohim/holochain/Jenkinsfile'), 'utf8');
  assert.match(edge, /params.DEPLOY_ONLY == true \|\| !shouldRunStep\("cargo-build-\$\{kind.toLowerCase\(\)\}"\)/);
  assert.match(edge, /versions.putAll\(readProperties\(text: live\)\)/);
  assert.match(edge, /workloadImageVersions\('STORAGE', "statefulset\/\$\{humanConfig.resourcePrefix\}"/);
  assert.match(edge, /workloadImageVersions\('DOORWAY', "deployment\/\$\{depName\}"/);
  assert.match(edge, /def deployOnlyEnvFilePreflight[^\n]+\{[\s\S]*?writeDeploymentVersions\(envFile, targetEnv\)/);
  const tmp = mkdtempSync(join(root, 'scripts/ci/.live-image-test-'));
  const state = join(tmp, 'state.json');
  const kubectl = join(tmp, 'kubectl');
  writeFileSync(kubectl, `#!/usr/bin/env node
const fs = require('node:fs');
const a = process.argv.slice(2), path = process.env.LIVE_STATE;
const state = JSON.parse(fs.readFileSync(path));
const row = state[a[1]];
if (!row || process.env.LIVE_FAIL === '1') process.exit(1);
if (a[0] === 'annotate') {
  for (const arg of a) if (arg.startsWith('elohim.host/')) { const i = arg.indexOf('='); row[arg.slice(0, i)] = arg.slice(i+1); }
  fs.writeFileSync(path, JSON.stringify(state));
} else if (a.at(-1).includes('containers')) process.stdout.write(row.image);
else if (a.at(-1).includes('closure-image')) process.stdout.write(row['elohim.host/closure-image'] || '');
else if (a.at(-1).includes('image-closure')) process.stdout.write(row['elohim.host/image-closure'] || '');
else process.exit(2);
`, {mode: 0o755});
  const invoke = (...args) => spawnSync('bash', ['scripts/ci/storage-workload-image.sh', ...args], {cwd: root, encoding: 'utf8', env: {...process.env, KUBECTL: kubectl, LIVE_STATE: state}});
  const props = text => Object.fromEntries(text.trim().split('\n').map(line => {const i = line.indexOf('='); return [line.slice(0, i), line.slice(i+1)];}));
  try {
    const live = {};
    const kinds = [['STORAGE', 'statefulset/peer', 'storage', 'a'], ['DOORWAY', 'deployment/doorway', 'doorway', 'b']];
    // First clean checkout: both selected images deploy, then record their live identity.
    for (const [kind, workload, image, hex] of kinds) live[workload] = {image: `harbor.ethosengine.com/ethosengine/elohim-${image}:closure-${hex.repeat(16)}@sha256:${hex.repeat(64)}`};
    writeFileSync(state, JSON.stringify(live));
    for (const [kind, workload, , hex] of kinds) {
      const recorded = invoke('--record-image', kind, workload, 'alpha', `sha256:${hex.repeat(64)}`, live[workload].image);
      assert.equal(recorded.status, 0, recorded.stderr);
    }
    // Next run starts from the untouched checked-in env again, as git clean would.
    const checkout = () => props(readFileSync(join(root, 'genesis/orchestrator/environments/alpha.env'), 'utf8'));
    const firstStorage = invoke('--live', 'STORAGE', 'statefulset/peer', 'alpha').stdout;
    const secondCheckout = checkout();
    assert.notEqual(secondCheckout.STORAGE_TAG, props(firstStorage).STORAGE_TAG);
    // A doorway-only second deploy changes just the doorway's live pod template.
    const deployed = JSON.parse(readFileSync(state));
    const nextDoorway = `harbor.ethosengine.com/ethosengine/elohim-doorway:closure-${'c'.repeat(16)}@sha256:${'c'.repeat(64)}`;
    deployed['deployment/doorway'].image = nextDoorway;
    writeFileSync(state, JSON.stringify(deployed));
    assert.equal(invoke('--record-image', 'DOORWAY', 'deployment/doorway', 'alpha', `sha256:${'c'.repeat(64)}`, nextDoorway).status, 0);
    assert.equal(invoke('--live', 'STORAGE', 'statefulset/peer', 'alpha').stdout, firstStorage);
    // Both an unselected doorway and DEPLOY_ONLY now retain the second deployment.
    kinds[1][3] = 'c';
    live['deployment/doorway'].image = nextDoorway;
    for (const [kind, workload, , hex] of kinds) {
      const clean = checkout();
      const retained = invoke('--live', kind, workload, 'alpha');
      assert.equal(retained.status, 0, retained.stderr);
      const resolved = {...clean, ...props(retained.stdout)};
      assert.equal(resolved[`${kind}_REF`], live[workload].image);
      assert.equal(resolved[`${kind}_DIGEST`], `sha256:${hex.repeat(64)}`);
      assert.equal(resolved[`${kind}_CLOSURE`], `sha256:${hex.repeat(64)}`);
      assert.notEqual(resolved[`${kind}_TAG`], clean[`${kind}_TAG`]);
      // Selection of the other image has no effect on this workload's identity.
      assert.equal(invoke('--live', kind, workload, 'alpha').stdout, retained.stdout);
    }
    // Deploy-only sidecars also come from the pod template, not computed HEAD tags.
    const sidecars = JSON.parse(readFileSync(state));
    sidecars['deployment/sidecar'] = {image: 'harbor.ethosengine.com/ethosengine/doorway-app:live-ui'};
    writeFileSync(state, JSON.stringify(sidecars));
    const sidecar = invoke('--live-container', 'deployment/sidecar', 'alpha', 'doorway-app', 'doorway-app');
    assert.equal(sidecar.status, 0, sidecar.stderr);
    assert.equal(sidecar.stdout.trim(), 'live-ui');
    assert.notEqual(invoke('--live-container', 'deployment/missing', 'alpha', 'doorway-app', 'doorway-app').status, 0);
    assert.match(edge, /retainDoorwaySidecars\(manifest, "deployment\/\$\{depName\}"/);
    for (const mutation of ['missing', 'mismatch', 'no-digest', 'unknown-closure']) {
      const snapshot = JSON.parse(readFileSync(state));
      const broken = JSON.parse(JSON.stringify(snapshot));
      const row = broken['statefulset/peer'];
      if (mutation === 'missing') delete broken['statefulset/peer'];
      if (mutation === 'mismatch') row['elohim.host/closure-image'] = 'stale';
      if (mutation === 'no-digest') row.image = row.image.split('@')[0];
      if (mutation === 'unknown-closure') delete row['elohim.host/image-closure'];
      writeFileSync(state, JSON.stringify(broken));
      const result = invoke('--live', 'STORAGE', 'statefulset/peer', 'alpha');
      assert.notEqual(result.status, 0, mutation);
      assert.equal(result.stdout, '');
      assert.match(result.stderr, /REFUSED/);
      writeFileSync(state, JSON.stringify(snapshot));
    }
  } finally { rmSync(tmp, {recursive: true, force: true}); }
});
