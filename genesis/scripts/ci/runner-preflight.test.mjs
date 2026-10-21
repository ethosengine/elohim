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

function fixture({ ensure, grants, providers, health, healthExit = 0 }) {
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
  if (health !== undefined) {
    const fakeCurl = join(dir, "curl");
    writeFileSync(
      fakeCurl,
      `#!/usr/bin/env bash
[[ "$*" == *"--max-time 15"* && "$*" != *Authorization* && "$*" != *compute-token* ]] || exit 9
printf '%s' "$FAKE_HEALTH"
exit ${healthExit}
`,
    );
    chmodSync(fakeCurl, 0o755);
  }
  const run = spawnSync("bash", [PREFLIGHT], {
    env: {
      ...process.env,
      ...(health !== undefined
        ? {
            PATH: `${dir}:${process.env.PATH}`,
            COMPUTE_PROVIDER_HEALTH_URL: "http://adam.fixture:8090/health",
            COMPUTE_PROVIDER_NAME: "adam",
            FAKE_HEALTH: JSON.stringify(health),
          }
        : {}),
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
const active = (days) => ({
  state: "active",
  daysRemaining: days,
  recipient: RUNNER,
  runner: RUNNER,
  forThisRunner: true,
});

test("all standing holds: one ok line per check, exit 0, the JWT never printed", () => {
  const run = fixture({
    ensure: STANDING_OK,
    grants: { uhCkkJessica: active(29) },
    providers: {
      jessica: { api: 8091, runnerGrant: "uhCkkJessica" },
      adam: { agentKey: null },
    },
  });
  assert.equal(run.status, 0, run.stdout + run.stderr);
  assert.match(
    run.stdout,
    /^ok standing: agent:ci-runner-jenkins-matthew sponsored under uhCEkG, 364 days remaining/m,
  );
  assert.match(
    run.stdout,
    /^ok provider:jessica: measure-stage grant active, 29 days remaining$/m,
  );
  assert.doesNotMatch(
    run.stdout,
    /provider:adam/,
    "a provider that declares no runnerGrant is not checked",
  );
  assert.ok(
    !run.stdout.includes(TOKEN) && !run.stderr.includes(TOKEN),
    "the JWT leaked",
  );
});

test("a missing admin key refuses standing and every grant read, naming the cure", () => {
  const run = fixture({
    ensure:
      "#!/usr/bin/env bash\necho 'runner-ensure: ADMIN_KEY unset' >&2\nexit 0\n",
    grants: {},
    providers: { jessica: { runnerGrant: "uhCkkJessica" } },
  });
  assert.equal(run.status, 1);
  assert.match(
    run.stdout,
    /^REFUSED standing: ADMIN_KEY is unset.* — export ADMIN_KEY/m,
  );
  assert.match(
    run.stdout,
    /^REFUSED provider:jessica: the runner has no session.* — clear the standing refusal first$/m,
  );
});

test("a lapsed, withdrawn, foreign or unrecorded grant is REFUSED with its cure", () => {
  const run = fixture({
    ensure: STANDING_OK,
    grants: {
      uhCkkLapsed: { ...active(-2), state: "lapsed" },
      uhCkkWithdrawn: { ...active(5), state: "withdrawn" },
      uhCkkForeign: {
        ...active(5),
        forThisRunner: false,
        recipient: "uhCAkOther",
      },
    },
    providers: {
      p1: { runnerGrant: "uhCkkLapsed" },
      p2: { runnerGrant: "uhCkkWithdrawn" },
      p3: { runnerGrant: "uhCkkForeign" },
      p4: { runnerGrant: null },
    },
  });
  assert.equal(run.status, 1);
  assert.match(
    run.stdout,
    /^REFUSED provider:p1: the grant lapsed — measure\.sh grant --on p1 --recipient <runner agent key> --renew/m,
  );
  assert.match(
    run.stdout,
    /^REFUSED provider:p2: the provider withdrew the grant — ask p2 for a new grant/m,
  );
  assert.match(
    run.stdout,
    /^REFUSED provider:p3: the grant is active but names a different recipient/m,
  );
  assert.match(
    run.stdout,
    /^REFUSED provider:p4: no runner grant recorded — measure\.sh grant --on p4 --recipient/m,
  );
  assert.match(run.stdout, /^ok standing:/m);
});

test("runner ensure prints only the JWT on stdout and the validated public key on its ok line", () => {
  const dir = mkdtempSync(join(tmpdir(), "runner-ensure-key-"));
  const fakeCurl = join(dir, "curl");
  writeFileSync(
    fakeCurl,
    `#!/usr/bin/env bash
cat >/dev/null
case "$*" in
  */auth/register*) printf '{}\\n201' ;;
  */auth/login*) printf '{"token":"${TOKEN}"}\\n200' ;;
  */auth/account*) jq -nc --arg key "$FAKE_RUNNER_KEY" '{agencyPhase:"service-agent",permissionLevel:"AUTHENTICATED",hostedCellGrantCid:"uhCEkGrant",hostedCellValidUntil:"2099-01-01T00:00:00Z",agentPubKey:$key}'; printf '200' ;;
  *) exit 1 ;;
esac
`,
  );
  chmodSync(fakeCurl, 0o755);
  const key = "uhCAk" + "A".repeat(48);
  const execute = (publicKey) =>
    spawnSync("bash", [resolve(HERE, "runner-ensure.sh")], {
      encoding: "utf8",
      env: {
        ...process.env,
        PATH: `${dir}:${process.env.PATH}`,
        ADMIN_KEY: "private-bootstrap-test",
        DOORWAY_URL: "http://fixture.invalid",
        FAKE_RUNNER_KEY: publicKey,
      },
    });
  try {
    const valid = execute(key);
    assert.equal(valid.status, 0, valid.stderr);
    assert.equal(valid.stdout, TOKEN);
    assert.ok(
      valid.stderr
        .split("\n")
        .some(
          (line) => line.startsWith("runner-ensure: ok") && line.includes(key),
        ),
    );
    assert.ok(
      !valid.stderr.includes(TOKEN) &&
        !valid.stderr.includes("private-bootstrap-test"),
    );
    for (const malformed of [
      "",
      "12D3KooTransport",
      "uhCAkShort",
      key + "\nforged-log",
    ]) {
      const rejected = execute(malformed);
      assert.equal(rejected.status, 1);
      assert.equal(rejected.stdout, "");
      assert.match(rejected.stderr, /no canonical runner agent key/);
      assert.ok(!rejected.stderr.includes(TOKEN));
      assert.ok(!rejected.stderr.includes("forged-log"));
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("optional provider identity uses only canonical lamad health and bounds failed reads", () => {
  const key = "uhCAk" + "B".repeat(48);
  const run = (options) =>
    fixture({
      ensure: STANDING_OK,
      grants: { known: active(3) },
      providers: { adam: { runnerGrant: "known" } },
      ...options,
    });
  const valid = run({
    health: {
      dhtParticipation: { agentKeys: { lamad: key, imagodei: "wrong-role" } },
    },
  });
  assert.equal(valid.status, 0, valid.stdout + valid.stderr);
  assert.ok(
    valid.stdout.includes(`ok provider-identity:adam: lamad agent key ${key}`),
  );
  assert.ok(!valid.stdout.includes(TOKEN) && !valid.stderr.includes(TOKEN));
  for (const health of [
    {},
    { dhtParticipation: { agentKeys: { imagodei: key } } },
    { dhtParticipation: { agentKeys: { lamad: key + "\nforged" } } },
  ]) {
    const invalid = run({ health });
    assert.equal(invalid.status, 1);
    assert.match(
      invalid.stdout,
      /REFUSED provider-identity:adam: health has no canonical lamad actor/,
    );
    assert.ok(!invalid.stdout.includes("forged"));
  }
  const timeout = run({ health: {}, healthExit: 28 });
  assert.equal(timeout.status, 1);
  assert.match(
    timeout.stdout,
    /REFUSED provider-identity:adam: provider health unavailable within 15 seconds/,
  );
});
