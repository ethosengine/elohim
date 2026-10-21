import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, writeFile, rm } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const script = fileURLToPath(
  new URL("./capture-compute-worker-grant.sh", import.meta.url),
);
const provider = `uhCAk${"p".repeat(48)}`;
const recipient = `uhCAk${"r".repeat(48)}`;
const image = `harbor.example/worker@sha256:${"a".repeat(64)}`;
const receipt = {
  declarationProvenance: "operator-approved-config",
  stewardOfRecord: "human:matthew",
  fixtureProvider: "adam",
  provider,
  recipient,
  state: "active",
  grantCid: `uhCEk${"e".repeat(48)}`,
  grantActionHash: `uhCkk${"a".repeat(48)}`,
  consentActionHash: `uhCkk${"c".repeat(48)}`,
  token: "SECRET-RECEIPT-SENTINEL",
};

async function fixture(t) {
  const dir = await mkdtemp(join(tmpdir(), "capture-worker-grant-"));
  t.after(() => rm(dir, { recursive: true, force: true }));
  const output = join(dir, "evidence");
  const artifact = join(output, "test-ns--adam", "activation.json");
  await mkdir(join(dir, "bin"));
  await mkdir(join(output, "test-ns--adam"), { recursive: true });
  await writeFile(artifact, '{"staleSuccess":true}');
  await writeFile(join(dir, "receipt.json"), JSON.stringify(receipt));
  const cgroup = {
    "/proc/self/cgroup": "0::/worker\n",
    "/sys/fs/cgroup/worker/cpu.max": "800000 100000\n",
    "/sys/fs/cgroup/worker/memory.max": "25769803776\n",
    "/sys/fs/cgroup/worker/memory.current": "1073741824\n",
    "/sys/fs/cgroup/worker/memory.peak": "2147483648\n",
    "/sys/fs/cgroup/cpu.max": "max 100000\n",
    "/sys/fs/cgroup/memory.max": "max\n",
  };
  await writeFile(join(dir, "cgroup.json"), JSON.stringify(cgroup));
  await writeFile(
    join(dir, "preload.cjs"),
    `
const fs = require('node:fs');
const read = fs.readFileSync, stat = fs.lstatSync, directoryStat = fs.statSync;
const cgroup = JSON.parse(read(process.env.FIXTURE + '/cgroup.json', 'utf8'));
fs.readFileSync = function(path, ...args) {
  if (path === '/proc/1/cmdline') return process.env.MODE === 'starting'
    ? 'bash\\0/opt/compute/worker-start.sh\\0' : 'node\\0/opt/compute/worker.mjs\\0';
  if (path === '/var/lib/compute/runner-grant.json') path = process.env.FIXTURE + '/receipt.json';
  if (path === '/proc/self/cgroup' || String(path).startsWith('/sys/fs/cgroup')) {
    if (cgroup[path] === undefined) { const error = new Error('missing'); error.code = 'ENOENT'; throw error; }
    return cgroup[path];
  }
  return read.call(this, path, ...args);
};
fs.lstatSync = function(path, ...args) {
  if (path === '/var/lib/compute/runner-grant.json') path = process.env.FIXTURE + '/receipt.json';
  return stat.call(this, path, ...args);
};
fs.statSync = function(path, ...args) {
  if (String(path).startsWith('/sys/fs/cgroup')) return {isDirectory: () => Object.keys(cgroup).some(key => key.startsWith(path.replace(/\\/+$/, '') + '/'))};
  return directoryStat.call(this, path, ...args);
};
`,
  );
  await writeFile(
    join(dir, "bin/kubectl"),
    `#!${process.execPath}
const fs = require('node:fs');
const {spawnSync} = require('node:child_process');
const args = process.argv.slice(2), mode = process.env.MODE;
fs.appendFileSync(process.env.FIXTURE + '/calls', JSON.stringify(args)+'\\n');
if (args.includes('get')) {
  if (mode === 'missing') process.exit(1);
  const counter = process.env.FIXTURE + '/get-count';
  let count = fs.existsSync(counter) ? Number(fs.readFileSync(counter,'utf8')) : 0;
  fs.writeFileSync(counter, String(++count));
  const actual = mode === 'wrong-image' ? 'other@sha256:'+'b'.repeat(64) : process.env.COMPUTE_WORKER_IMAGE;
  const ready = mode === 'unready' ? 'false' : 'true';
  const cid = mode === 'restart' && count > 1 ? 'new' : 'original';
  console.log('01234567-89ab-cdef-0123-456789abcdef||Running|'+actual+'|'+ready+'|containerd://'+cid+'|0');
} else if (args.includes('exec')) {
  if (mode === 'forbidden') { console.error('SECRET-KUBECTL-SENTINEL'); process.exit(1); }
  const i = args.indexOf('/usr/local/bin/node');
  if (i < 0 || args[i+1] !== '-') process.exit(99);
  const env = {...process.env,
    COMPUTE_RUNNER_GRANT_JSON: JSON.stringify({enabled:true, provider: process.env.COMPUTE_WORKER_PROVIDER,
      recipient: process.env.COMPUTE_WORKER_RECIPIENT, stewardOfRecord:'human:matthew', fixtureProvider:'adam',
      token:'SECRET-CONFIG-SENTINEL'}),
    COMPUTE_PERFORMER: process.env.COMPUTE_WORKER_PROVIDER,
    COMPUTE_RUNTIME_IMAGE: process.env.COMPUTE_WORKER_IMAGE.split('@')[1],
    ELOHIM_COMPUTE_LOCAL_TOKEN: 'SECRET-ENV-SENTINEL'};
  const result = spawnSync(process.execPath, ['--require', process.env.FIXTURE+'/preload.cjs', '-', ...args.slice(i+2)],
    {input:fs.readFileSync(0), env});
  process.stdout.write(result.stdout); process.stderr.write(result.stderr); process.exit(result.status ?? 99);
} else process.exit(99);
`,
    { mode: 0o755 },
  );
  const env = {
    ...process.env,
    PATH: `${dir}/bin:${process.env.PATH}`,
    FIXTURE: dir,
    COMPUTE_WORKER_ENABLED: "true",
    COMPUTE_WORKER_GRANT_ENABLED: "true",
    COMPUTE_WORKER_PROVIDER: provider,
    COMPUTE_WORKER_RECIPIENT: recipient,
    COMPUTE_WORKER_IMAGE: image,
    COMPUTE_WORKER_CPU_MILLIS: "8000",
    COMPUTE_WORKER_MEMORY_BYTES: "25769803776",
    // Healthy probes spawn several processes; allow parallel CI contention.
    // Only deadline-refusal cases deliberately use the one-second budget.
    COMPUTE_WORKER_STARTUP_SECS: "10",
    MODE: "ready",
  };
  return {
    dir,
    cgroup,
    artifact,
    run: (extra = {}) =>
      spawnSync("bash", [script, "adam", "test-ns", output], {
        env: { ...env, ...extra },
        encoding: "utf8",
        timeout: 15000,
      }),
  };
}

test("captures only validated public grant fields and the running worker incarnation", async (t) => {
  const f = await fixture(t);
  const result = f.run();
  assert.equal(result.status, 0, result.stderr);
  const raw = await readFile(f.artifact, "utf8");
  const captured = JSON.parse(raw);
  for (const [key, value] of Object.entries(receipt)) {
    if (key !== "token") assert.equal(captured[key], value);
  }
  assert.equal(captured.worker.state, "running");
  assert.equal(captured.worker.command, "/opt/compute/worker.mjs");
  assert.equal(captured.worker.image, image);
  assert.equal(captured.worker.containerId, "containerd://original");
  assert.equal(captured.worker.restartCount, 0);
  assert.equal(captured.capacity.effectiveCpuMillis, 8000);
  assert.equal(captured.capacity.effectiveMemoryBytes, 25769803776);
  assert.equal(captured.capacity.hierarchy[0].memoryCurrent, "1073741824");
  assert.equal(captured.capacity.hierarchy[0].memoryPeak, "2147483648");
  assert.match(captured.capacity.measurement, /not stage peak/);
  assert.doesNotMatch(
    raw + result.stdout + result.stderr,
    /SECRET-|token|staleSuccess/,
  );
  const calls = (await readFile(join(f.dir, "calls"), "utf8"))
    .trim()
    .split("\n")
    .map(JSON.parse);
  assert.deepEqual(
    calls.map((args) => args[1]),
    ["get", "exec", "get"],
  );
  assert.ok(calls.every((args) => args[0].startsWith("--request-timeout=")));
});

test("disabled worker does not query Kubernetes and removes stale success", async (t) => {
  const f = await fixture(t);
  const result = f.run({ COMPUTE_WORKER_ENABLED: "false" });
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /SKIPPED/);
  await assert.rejects(readFile(join(f.dir, "calls")), { code: "ENOENT" });
  await assert.rejects(readFile(f.artifact), { code: "ENOENT" });
});

for (const mode of [
  "missing",
  "unready",
  "wrong-image",
  "starting",
  "forbidden",
  "restart",
]) {
  test(`${mode} cannot produce activation evidence or reuse a retained receipt`, async (t) => {
    const f = await fixture(t);
    const result = f.run({ MODE: mode, COMPUTE_WORKER_STARTUP_SECS: "1" });
    assert.equal(result.status, 1, result.stderr);
    assert.match(result.stderr, /startup budget 1s exhausted/);
    assert.doesNotMatch(result.stdout + result.stderr, /SECRET-/);
    await assert.rejects(readFile(f.artifact), { code: "ENOENT" });
  });
}

for (const patch of [
  { recipient: provider },
  { grantActionHash: "not-native" },
  { state: "revoked" },
]) {
  test(`invalid public receipt ${JSON.stringify(patch)} refuses`, async (t) => {
    const f = await fixture(t);
    await writeFile(
      join(f.dir, "receipt.json"),
      JSON.stringify({ ...receipt, ...patch }),
    );
    const result = f.run();
    assert.equal(result.status, 1, result.stderr);
    assert.match(result.stderr, /differs from approved grant\/image binding/);
    await assert.rejects(readFile(f.artifact), { code: "ENOENT" });
  });
}

test("missing receipt waits within budget and malformed JSON never becomes evidence", async (t) => {
  const f = await fixture(t);
  await rm(join(f.dir, "receipt.json"));
  const missing = f.run({ COMPUTE_WORKER_STARTUP_SECS: "1" });
  assert.equal(missing.status, 1, missing.stderr);
  assert.match(missing.stderr, /startup budget 1s exhausted/);
  await writeFile(join(f.dir, "receipt.json"), "SECRET-MALFORMED-JSON");
  const malformed = f.run();
  assert.equal(malformed.status, 1, malformed.stderr);
  assert.match(malformed.stderr, /differs from approved grant\/image binding/);
  assert.doesNotMatch(malformed.stdout + malformed.stderr, /SECRET-/);
  await assert.rejects(readFile(f.artifact), { code: "ENOENT" });
});

test("effective capacity takes stricter visible ancestors and rounds CPU upward like executor", async (t) => {
  const f = await fixture(t);
  await writeFile(
    join(f.dir, "cgroup.json"),
    JSON.stringify({
      ...f.cgroup,
      "/sys/fs/cgroup/worker/cpu.max": "900000 100000",
      "/sys/fs/cgroup/worker/memory.max": "34359738368",
      "/sys/fs/cgroup/cpu.max": "799999 100000",
      "/sys/fs/cgroup/memory.max": "25769803776",
    }),
  );
  const result = f.run();
  assert.equal(result.status, 0, result.stderr);
  const { capacity } = JSON.parse(await readFile(f.artifact, "utf8"));
  assert.equal(capacity.effectiveCpuMillis, 8000);
  assert.equal(capacity.effectiveMemoryBytes, 25769803776);
  assert.equal(capacity.hierarchy.length, 2);
});

test("namespaced cgroup path fallback observes the mounted root like executor", async (t) => {
  const f = await fixture(t);
  await writeFile(
    join(f.dir, "cgroup.json"),
    JSON.stringify({
      "/proc/self/cgroup": "0::/not-visible/worker",
      "/sys/fs/cgroup/cpu.max": "800000 100000",
      "/sys/fs/cgroup/memory.max": "25769803776",
      "/sys/fs/cgroup/memory.current": "1073741824",
    }),
  );
  const result = f.run();
  assert.equal(result.status, 0, result.stderr);
  const { capacity } = JSON.parse(await readFile(f.artifact, "utf8"));
  assert.equal(capacity.hierarchy[0].path, "/sys/fs/cgroup");
  assert.equal(
    capacity.hierarchy[0].memoryPeak,
    null,
    "older kernel can lack memory.peak",
  );
});

for (const patch of [
  { "/sys/fs/cgroup/worker/cpu.max": "max 100000" },
  { "/sys/fs/cgroup/worker/memory.max": "max" },
  { "/sys/fs/cgroup/worker/cpu.max": "800001 100000" },
  { "/sys/fs/cgroup/worker/memory.max": "25769803777" },
  { "/sys/fs/cgroup/worker/cpu.max": "800000 0" },
]) {
  test(`unenforced or invalid cgroup ${JSON.stringify(patch)} refuses`, async (t) => {
    const f = await fixture(t);
    await writeFile(
      join(f.dir, "cgroup.json"),
      JSON.stringify({ ...f.cgroup, ...patch }),
    );
    const result = f.run();
    assert.equal(result.status, 1, result.stderr);
    assert.match(result.stderr, /enforced capacity differs/);
    await assert.rejects(readFile(f.artifact), { code: "ENOENT" });
  });
}

test("enabled worker without a grant, malformed bindings or an excessive budget refuses before Kubernetes", async (t) => {
  for (const extra of [
    { COMPUTE_WORKER_GRANT_ENABLED: "false" },
    { COMPUTE_WORKER_PROVIDER: "not-agent" },
    { COMPUTE_WORKER_IMAGE: "floating:latest" },
    { COMPUTE_WORKER_STARTUP_SECS: "901" },
  ]) {
    const f = await fixture(t);
    assert.equal(f.run(extra).status, 1);
    await assert.rejects(readFile(join(f.dir, "calls")), { code: "ENOENT" });
  }
});

test("edge checks enabled Adam after conductor rollout and archives only the public artifact", async () => {
  const jenkins = await readFile(
    new URL("../../elohim/holochain/Jenkinsfile", import.meta.url),
    "utf8",
  );
  const start = jenkins.indexOf("def deployHumansInParallel(");
  const call = jenkins.indexOf(
    "captureEnabledComputeWorkerGrant(humans, outcomes)",
    start,
  );
  assert.ok(call > jenkins.indexOf("deployHumanConductor(h,", start));
  assert.ok(call < jenkins.indexOf("emitDeployJunit(", start));
  assert.match(
    jenkins,
    /worker.enabled != true \|\| outcomes\[adam.resourcePrefix\] != true/,
  );
  assert.match(
    jenkins,
    /compute-worker-evidence\/\$\{adam.namespace\}--\$\{adam.resourcePrefix\}\/activation.json,compute-worker-evidence\/\$\{adam.namespace\}--\$\{adam.resourcePrefix\}\/status.txt/,
  );
  const helper = jenkins.slice(
    jenkins.indexOf("def captureEnabledComputeWorkerGrant("),
    jenkins.indexOf("def deployEdgeWithManifest("),
  );
  assert.ok(
    helper.indexOf("worker.enabled != true") <
      helper.indexOf("archiveArtifacts"),
  );
  assert.ok(
    helper.indexOf("capture-compute-worker-grant.sh") <
      helper.indexOf("archiveArtifacts"),
  );
});
