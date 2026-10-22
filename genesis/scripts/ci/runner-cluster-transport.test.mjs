import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import {
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  readFileSync,
  existsSync,
  statSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { withClusterTransport } from "./runner-cluster-transport.mjs";

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), "runner-transport-test-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const sa = join(root, "service-account");
  mkdirSync(sa);
  writeFileSync(join(sa, "token"), "private-fixture-token");
  writeFileSync(join(sa, "ca.crt"), "fixture-ca");
  return {
    root,
    sa,
    options: {
      env: { PATH: process.env.PATH },
      serviceAccountDirectory: sa,
      temporaryDirectory: root,
    },
  };
}
test("subprocess receives explicit projected authority without bearer copying; temporary config removed", async (t) => {
  const { root, sa, options } = fixture(t);
  let configPath;
  const output = await withClusterTransport(async (env) => {
    configPath = env.KUBECONFIG;
    assert.equal(statSync(configPath).mode & 0o777, 0o600);
    assert.equal(statSync(dirname(configPath)).mode & 0o777, 0o700);
    // Fake kubectl reads exactly the child configuration contract, with no live call.
    const fake = join(root, "fake-kubectl.mjs");
    writeFileSync(
      fake,
      `import {readFileSync} from 'node:fs'; const c=JSON.parse(readFileSync(process.env.KUBECONFIG)); console.log(JSON.stringify(c));`,
    );
    return execFileSync(process.execPath, [fake], { env, encoding: "utf8" });
  }, options);
  const c = JSON.parse(output);
  assert.equal(c.clusters[0].cluster.server, "https://kubernetes.default.svc");
  assert.equal(
    c.clusters[0].cluster["certificate-authority"],
    join(sa, "ca.crt"),
  );
  assert.deepEqual(c.users[0].user, { tokenFile: join(sa, "token") });
  assert(!output.includes("private-fixture-token"));
  assert(!existsSync(configPath));
  assert.equal(options.env.KUBECONFIG, undefined);
});
test("existing explicit context is authoritative, including missing-file refusal", async (t) => {
  const { root, options } = fixture(t);
  const named = join(root, "explicit");
  writeFileSync(named, "existing context");
  await withClusterTransport((env) => assert.equal(env.KUBECONFIG, named), {
    ...options,
    env: { KUBECONFIG: named },
  });
  let called = false;
  await assert.rejects(
    withClusterTransport(
      () => {
        called = true;
      },
      { ...options, env: { KUBECONFIG: join(root, "missing") } },
    ),
  );
  assert.equal(called, false);
});
test("missing projected credentials refuse before command; async failure cleans private config", async (t) => {
  const { sa, options } = fixture(t);
  let config;
  await assert.rejects(
    withClusterTransport(async (env) => {
      config = env.KUBECONFIG;
      throw new Error("fixture command failure");
    }, options),
    /fixture command failure/,
  );
  assert(!existsSync(config));
  rmSync(join(sa, "token"));
  let called = false;
  await assert.rejects(
    withClusterTransport(() => {
      called = true;
    }, options),
  );
  assert.equal(called, false);
});
