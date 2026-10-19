// runner-preflight.sh — one ok/REFUSED line per check, exit 1 on any REFUSED, and the
// runner's JWT never printed. Fakes stand in for runner-ensure.sh and compute-runner.
import test from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, writeFileSync, chmodSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const PREFLIGHT = resolve(HERE, "runner-preflight.sh");
const TOKEN = "eyJ.secret-runner-jwt.sig";
const RUNNER = "uhCAkRunner";

function fixture({ ensure, grants, providers }) {
  const dir = mkdtempSync(join(tmpdir(), "runner-preflight-"));
  const ensureScript = join(dir, "ensure.sh");
  writeFileSync(ensureScript, ensure);
  chmodSync(ensureScript, 0o755);
  const runner = join(dir, "runner.mjs");
  writeFileSync(
    runner,
    `const grants = ${JSON.stringify(grants)};
if (process.env.RUNNER_JWT !== ${JSON.stringify(TOKEN)}) { console.error("REFUSED session: no jwt — x"); process.exit(1); }
const g = grants[process.argv[3]];
if (!g) { console.error("REFUSED runner: unreadable — x"); process.exit(1); }
process.stdout.write(JSON.stringify(g) + "\\n");`,
  );
  const providersJson = join(dir, "providers.json");
  writeFileSync(providersJson, JSON.stringify(providers));
  const run = spawnSync("bash", [PREFLIGHT], {
    env: {
      ...process.env,
      RUNNER_ENSURE: ensureScript,
      COMPUTE_RUNNER: `node ${runner}`,
      PROVIDERS_JSON: providersJson,
    },
    encoding: "utf8",
  });
  rmSync(dir, { recursive: true, force: true });
  return run;
}

const STANDING_OK = `#!/usr/bin/env bash
echo "runner-ensure: ok — agent:ci-runner-jenkins-matthew sponsored under uhCEkG, 364 days remaining (until 2027-10-19T00:00:00Z)" >&2
printf '%s' '${TOKEN}'
`;
const active = (days) => ({ state: "active", daysRemaining: days, recipient: RUNNER, runner: RUNNER, forThisRunner: true });

test("all standing holds: one ok line per check, exit 0, the JWT never printed", () => {
  const run = fixture({
    ensure: STANDING_OK,
    grants: { uhCkkJessica: active(29) },
    providers: { jessica: { api: 8091, runnerGrant: "uhCkkJessica" }, adam: { agentKey: null } },
  });
  assert.equal(run.status, 0, run.stdout + run.stderr);
  assert.match(run.stdout, /^ok standing: agent:ci-runner-jenkins-matthew sponsored under uhCEkG, 364 days remaining/m);
  assert.match(run.stdout, /^ok provider:jessica: measure-stage grant active, 29 days remaining$/m);
  assert.doesNotMatch(run.stdout, /provider:adam/, "a provider that declares no runnerGrant is not checked");
  assert.ok(!run.stdout.includes(TOKEN) && !run.stderr.includes(TOKEN), "the JWT leaked");
});

test("a missing admin key refuses standing and every grant read, naming the cure", () => {
  const run = fixture({
    ensure: "#!/usr/bin/env bash\necho 'runner-ensure: ADMIN_KEY unset' >&2\nexit 0\n",
    grants: {},
    providers: { jessica: { runnerGrant: "uhCkkJessica" } },
  });
  assert.equal(run.status, 1);
  assert.match(run.stdout, /^REFUSED standing: ADMIN_KEY is unset.* — export ADMIN_KEY/m);
  assert.match(run.stdout, /^REFUSED provider:jessica: the runner has no session.* — clear the standing refusal first$/m);
});

test("a lapsed, withdrawn, foreign or unrecorded grant is REFUSED with its cure", () => {
  const run = fixture({
    ensure: STANDING_OK,
    grants: {
      uhCkkLapsed: { ...active(-2), state: "lapsed" },
      uhCkkWithdrawn: { ...active(5), state: "withdrawn" },
      uhCkkForeign: { ...active(5), forThisRunner: false, recipient: "uhCAkOther" },
    },
    providers: {
      p1: { runnerGrant: "uhCkkLapsed" },
      p2: { runnerGrant: "uhCkkWithdrawn" },
      p3: { runnerGrant: "uhCkkForeign" },
      p4: { runnerGrant: null },
    },
  });
  assert.equal(run.status, 1);
  assert.match(run.stdout, /^REFUSED provider:p1: the grant lapsed — measure\.sh grant --on p1 --recipient <runner agent key> --renew/m);
  assert.match(run.stdout, /^REFUSED provider:p2: the provider withdrew the grant — ask p2 for a new grant/m);
  assert.match(run.stdout, /^REFUSED provider:p3: the grant is active but names a different recipient/m);
  assert.match(run.stdout, /^REFUSED provider:p4: no runner grant recorded — measure\.sh grant --on p4 --recipient/m);
  assert.match(run.stdout, /^ok standing:/m);
});
