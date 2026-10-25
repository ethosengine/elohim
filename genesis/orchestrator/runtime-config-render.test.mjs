// Per-human runtime-config render (the fleet canary knob, rung 4/5).
//
// deployments.json `runtimeConfig` (KEY -> string) renders as TOML lines into
// the human's <prefix>-runtime-config ConfigMap through ONE sed expression
// built by `runtimeConfigSedExpr` in elohim/holochain/Jenkinsfile:
//   absent/empty  -> `/RUNTIME_CONFIG_BODY_PLACEHOLDER/d`   (line deleted)
//   present       -> `s|RUNTIME_CONFIG_BODY_PLACEHOLDER|K = "v"\n    K2 = "v2"|`
//
// Three things are pinned here, on the REAL template and adam manifest and with
// the REAL sed the pipeline runs:
//   1. the Jenkinsfile still carries both arms and the sed list still calls the helper;
//   2. a human WITHOUT the field renders the ConfigMap byte-identical to the
//      comment-only body (no blank line, no leftover placeholder);
//   3. a human WITH the field renders each key as an indented TOML line inside
//      `runtime-config.toml: |`, and the rest of the manifest is untouched.
// The substitution strings are rebuilt here from the documented rule so the test
// fails if the Groovy drifts from it (the static pin) or if sed's semantics do.
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync, mkdtempSync, readdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

import { requireEpr } from "./runtime-declaration-render.mjs";

const EPR_BIN = requireEpr();
const ROOT = new URL("../../", import.meta.url);
const read = (rel) => readFileSync(new URL(rel, ROOT), "utf8");

const jenkinsfile = read("elohim/holochain/Jenkinsfile");
const template = read("genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml");
const adam = read("genesis/orchestrator/manifests/humans/adam-firstman.yaml");
const deployments = JSON.parse(read("genesis/orchestrator/data/deployments.json"));

const PLACEHOLDER = "RUNTIME_CONFIG_BODY_PLACEHOLDER";

// The one mint site for the workspace release channel id (rung5 workspace
// orchestration Task 3, 2026-09-05) — deployments.json's alpha humans all
// follow THIS channel, at a per-human adoption mode. Kept as a single JS
// constant here so a future rename only moves one line; deployments.json's
// own $releaseChannelIdComment is the human-readable mint record.
const WORKSPACE_CHANNEL_ID = "runtime:coordinators:elohim:workspace";

// The app-bundle channel the alpha serving pair follows (native-delivery
// sprint Lane N, elected-content spec §12.8 slice 2). Enrolled at =observe:
// an app-bundle head MOVES serving rows, so canary/apply on the fleet waits
// until the storage binary carrying the class is deployed and the household
// proof is green (C10 ordering) — a later data change, not minted here.
const APP_BUNDLE_CHANNEL_ID = "runtime:app-bundle:alpha:dev";

// `ELOHIM_RELEASE_CHANNELS` is a LIST (`release_adoption::state::
// parse_followed_channels` splits on `,` `;` and newlines): one
// `<channelId>=<mode>` entry per followed channel.
const CHANNEL_ENTRY = /^runtime:[a-z0-9][a-z0-9-]*:[a-z0-9][a-z0-9-]*:[a-z0-9][a-z0-9-]*=(observe|canary|apply)$/;

function followedChannels(value) {
  return value
    .split(",")
    .map((entry) => entry.trim())
    .filter((entry) => entry.length > 0)
    .map((entry) => {
      assert.match(entry, CHANNEL_ENTRY, `entry "${entry}" is <runtime channel id>=<mode>`);
      const [channelId, mode] = entry.split("=");
      return { channelId, mode };
    });
}

// The documented rule, mirrored (see runtimeConfigSedExpr).
function sedExprFor(runtimeConfig) {
  if (!runtimeConfig || Object.keys(runtimeConfig).length === 0) {
    return `/${PLACEHOLDER}/d`;
  }
  const lines = Object.entries(runtimeConfig).map(([k, v]) => {
    assert.match(k, /^[A-Z][A-Z0-9_]*$/, `key ${k} must be env-var shaped`);
    assert.doesNotMatch(String(v), /[|&\\'"\r\n]/, `value for ${k} carries a forbidden char`);
    return `${k} = "${v}"`;
  });
  return `s|${PLACEHOLDER}|${lines.join("\\n    ")}|`;
}

function renderWithSed(text, expr) {
  const dir = mkdtempSync(join(tmpdir(), "rc-render-"));
  const input = join(dir, "in.yaml");
  writeFileSync(input, text);
  return execFileSync("sed", ["-e", expr, input], { encoding: "utf8" });
}

function configMapBody(rendered, prefixName) {
  // The runtime-config ConfigMap document, from its `kind:` to the next `---`.
  const start = rendered.indexOf(`name: ${prefixName}-runtime-config`);
  assert.notEqual(start, -1, "runtime-config ConfigMap present");
  const end = rendered.indexOf("\n---", start);
  return rendered.slice(start, end) + "\n";
}

test("Jenkinsfile carries both arms and the human sed list calls the helper", () => {
  assert.match(jenkinsfile, /def runtimeConfigSedExpr\(Map humanConfig\)/);
  assert.match(jenkinsfile, /'\/RUNTIME_CONFIG_BODY_PLACEHOLDER\/d'/, "delete arm");
  assert.match(jenkinsfile, /s\|RUNTIME_CONFIG_BODY_PLACEHOLDER\|\$\{lines\.join\('\\\\n    '\)\}\|/, "substitution arm joins with \\n + 4-space indent");
  assert.match(jenkinsfile, /runtimeConfigSedExpr\(humanConfig\),/, "sed list element");
  assert.match(jenkinsfile, /\[A-Z\]\[A-Z0-9_\]\*/, "keys are env-var shaped");
});

test("both manifests carry exactly one placeholder line inside runtime-config.toml", () => {
  for (const [name, text] of [["template", template], ["adam", adam]]) {
    const lines = text.split("\n").filter((l) => l.includes(PLACEHOLDER));
    assert.equal(lines.length, 1, `${name}: one placeholder line`);
    assert.equal(lines[0], `    ${PLACEHOLDER}`, `${name}: placeholder sits at the TOML body indent`);
    const tomlIdx = text.indexOf("runtime-config.toml: |");
    const phIdx = text.indexOf(PLACEHOLDER);
    const nextDoc = text.indexOf("\n---", tomlIdx);
    assert.ok(tomlIdx < phIdx && phIdx < nextDoc, `${name}: placeholder is inside the ConfigMap body`);
  }
});

test("a human without runtimeConfig renders the ConfigMap byte-identical to the comment-only body", () => {
  for (const [name, text, prefix] of [
    ["template", template, "RESOURCE_PREFIX_PLACEHOLDER"],
    ["adam", adam, "elohim-adam-alpha"],
  ]) {
    const rendered = renderWithSed(text, sedExprFor(undefined));
    const expected = text.split("\n").filter((l) => l !== `    ${PLACEHOLDER}`).join("\n");
    assert.equal(rendered, expected, `${name}: only the placeholder line is gone`);
    assert.doesNotMatch(rendered, /RUNTIME_CONFIG_BODY/, `${name}: no leftover placeholder`);
    const body = configMapBody(rendered, prefix);
    assert.doesNotMatch(body, /\n    \n/, `${name}: no blank line left in the TOML body`);
  }
});

test("a human with runtimeConfig renders each key as an indented TOML line in the ConfigMap", () => {
  const cfg = {
    ELOHIM_RELEASE_CHANNELS: "runtime:coordinators:elohim:receipt-x=canary",
    PROJECTION_RECONCILE_SECS: "30",
  };
  const rendered = renderWithSed(template, sedExprFor(cfg));
  const body = configMapBody(rendered, "RESOURCE_PREFIX_PLACEHOLDER");
  assert.match(body, /\n    ELOHIM_RELEASE_CHANNELS = "runtime:coordinators:elohim:receipt-x=canary"\n/);
  assert.match(body, /\n    PROJECTION_RECONCILE_SECS = "30"\n/);
  assert.doesNotMatch(rendered, /RUNTIME_CONFIG_BODY/, "placeholder consumed");
  // Everything outside the placeholder line is untouched.
  const stripped = rendered
    .split("\n")
    .filter((l) => !/^    [A-Z][A-Z0-9_]* = "/.test(l))
    .join("\n");
  const expected = template.split("\n").filter((l) => l !== `    ${PLACEHOLDER}`).join("\n");
  assert.equal(stripped, expected);
});

test("every runtimeConfig declared in deployments.json is renderable", () => {
  for (const human of deployments.humans) {
    if (human.runtimeConfig === undefined) continue;
    assert.equal(typeof human.runtimeConfig, "object", `${human.name}: runtimeConfig is a map`);
    sedExprFor(human.runtimeConfig); // asserts key/value shape
  }
});

function humanNamed(name) {
  const human = deployments.humans.find((h) => h.name === name);
  assert.ok(human, `deployments.json declares a human named ${name}`);
  return human;
}

test("every declared ELOHIM_RELEASE_CHANNELS is a list that follows the one workspace channel id", () => {
  const withChannel = deployments.humans.filter(
    (h) => h.runtimeConfig?.ELOHIM_RELEASE_CHANNELS !== undefined,
  );
  assert.ok(withChannel.length > 0, "at least one alpha human declares the channel");
  for (const human of withChannel) {
    const channels = followedChannels(human.runtimeConfig.ELOHIM_RELEASE_CHANNELS);
    const ids = channels.map((c) => c.channelId);
    assert.equal(new Set(ids).size, ids.length, `${human.name}: no channel is followed twice`);
    assert.equal(
      ids.filter((id) => id === WORKSPACE_CHANNEL_ID).length,
      1,
      `${human.name}: the workspace coordinator channel is followed exactly once`,
    );
    for (const { channelId } of channels) {
      assert.ok(
        channelId === WORKSPACE_CHANNEL_ID || channelId === APP_BUNDLE_CHANNEL_ID,
        `${human.name}: ${channelId} is one of the minted channel ids`,
      );
    }
  }
});

test("a shem-hosted alpha human (no field before rung5 Task 3) without runtimeConfig would render byte-identical", () => {
  // terrance is a real deployments.json human that still carries no runtimeConfig
  // (suspended shem fixture) -- proves the "field absent" arm of the per-human
  // contract on real data, not just the abstract sed mechanism above.
  const human = humanNamed("terrance");
  assert.equal(human.runtimeConfig, undefined, "terrance carries no runtimeConfig");
  const rendered = renderWithSed(template, sedExprFor(human.runtimeConfig));
  const expected = template.split("\n").filter((l) => l !== `    ${PLACEHOLDER}`).join("\n");
  assert.equal(rendered, expected, "no field -> byte-identical to the comment-only body");
  assert.doesNotMatch(rendered, /RUNTIME_CONFIG_BODY/);
});

test("alpha humans WITH the field render the real workspace channel line at their declared mode", () => {
  // Fix round 1 (plan run:ruling "Alpha enrolment modes, resolved"): every
  // 2026-09-06: james promoted to =canary after the first workspace→alpha receipts (Task 4).
  // Every OTHER active alpha human enrolls at =observe on the FIRST render -- no
  // apply/canary from a workspace channel while nobody is watching. Promotion
  // (james to canary, then matthew/jessica to apply on the bootstrap pair) is
  // a later data change the operator flips after Task 4's first
  // workspace->alpha receipt, not minted here.
  // 2026-09-26/27 (one-head-delivered sprint Lane D, Task D3, fix round 1):
  // the alpha serving pair (doorway A -> matthew, doorway B -> jessica) flips
  // the app-bundle channel to =canary, NOT =apply -- =apply adopts only an
  // EARNED (promoted) head (release_adoption/state.rs:86-100) and CI never
  // promotes one; =canary applies a verified STAGING head, which is what the
  // household proof exercises (app-bundle-elected-delivery.steps.ts
  // mode: 'canary', 6/6 twice, 2026-09-26) and what "alpha IS the canary"
  // means. The deployed storage (edge #1486) lists app-bundle among its
  // applyVehicles. See $appBundleChannelComment.
  const appBundleCanary = `,${APP_BUNDLE_CHANNEL_ID}=canary`;
  for (const [name, mode, extra] of [
    ["james", "canary", ""],
    ["matthew", "observe", appBundleCanary],
    ["jessica", "observe", appBundleCanary],
    ["adam", "observe", appBundleCanary],
    ["gertrude", "observe", ""],
    ["susan", "observe", ""],
    ["eve", "observe", ""],
  ]) {
    const human = humanNamed(name);
    const expectedValue = `${WORKSPACE_CHANNEL_ID}=${mode}${extra}`;
    assert.equal(
      human.runtimeConfig?.ELOHIM_RELEASE_CHANNELS,
      expectedValue,
      `${name}: declared channel/mode`,
    );
    const rendered = renderWithSed(template, sedExprFor(human.runtimeConfig));
    const body = configMapBody(rendered, "RESOURCE_PREFIX_PLACEHOLDER");
    assert.match(
      body,
      new RegExp(`\\n    ELOHIM_RELEASE_CHANNELS = "${expectedValue.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}"\\n`),
      `${name}: renders the workspace channel line at ${mode}`,
    );
  }

  // 2026-10-08: the follow set is DERIVED from the doorway manifests, not a
  // hand list. Every storage a doorway reads (its STORAGE_URL) must follow the
  // app-bundle channel: after the slug-pointer cure (5ead7671a) the channel's
  // vehicle is the ONLY writer of a bound slug, so a serving peer that follows
  // no channel is pinned to its last pointer forever. That is exactly what
  // happened on alpha -- this test's previous rule ("adam never follows") was
  // built on doorway B reading jessica, but alpha-b.yaml has named adam since
  // 2026-05-27, and elohim.host served the 2026-09-22 lamad bundle while
  // doorway-alpha served the new one.
  const canaryEntry = new RegExp(`(^|,)${APP_BUNDLE_CHANNEL_ID.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}=canary(,|$)`);
  for (const [doorway, backing] of doorwayBackingStorages()) {
    const channels = humanNamed(backing).runtimeConfig?.ELOHIM_RELEASE_CHANNELS ?? "";
    assert.match(
      channels,
      canaryEntry,
      `${backing} (the storage doorway ${doorway} reads) follows the app-bundle channel at canary`,
    );
  }
});

// [doorway manifest, human name] for every doorway manifest that names a
// STORAGE_URL: the human whose storage that doorway serves from.
function doorwayBackingStorages() {
  const dir = new URL("genesis/orchestrator/manifests/doorway/", ROOT);
  const pairs = [];
  for (const file of readdirSync(dir).filter((f) => f.endsWith(".yaml")).sort()) {
    const text = readFileSync(new URL(file, dir), "utf8");
    const m = text.match(/-\s*name:\s*STORAGE_URL\s*\n\s*value:\s*"http:\/\/elohim-([a-z0-9-]+)-alpha\./);
    if (m) pairs.push([file, m[1]]);
  }
  assert.ok(pairs.length >= 2, `both alpha doorway manifests name a STORAGE_URL (found ${pairs.length})`);
  return pairs;
}

const MANIFEST_PLACEHOLDER = "RUNTIME_MANIFEST_CID_PLACEHOLDER";
const conductorTemplate = read("genesis/orchestrator/manifests/humans/_edgenode-conductor.template.yaml");

function manifestSedExpr(pin) {
  if (pin?.cid == null) return `/${MANIFEST_PLACEHOLDER}/d`;
  assert.match(pin.cid, /^bafyre[a-z2-7]+$/);
  return `s|${MANIFEST_PLACEHOLDER}|${pin.cid}|`;
}

test("runtime manifest Jenkins helper validates CID and renders both arms in the shared sed list", () => {
  assert.match(jenkinsfile, /def runtimeManifestSedExpr\(Map humanConfig\)/);
  assert.match(jenkinsfile, /'\/RUNTIME_MANIFEST_CID_PLACEHOLDER\/d'/);
  assert.ok(jenkinsfile.includes('return "s|RUNTIME_MANIFEST_CID_PLACEHOLDER|${cid}|"'));
  assert.match(jenkinsfile, /bafyre\[a-z2-7\]\+/);
  assert.match(jenkinsfile, /runtimeManifestSedExpr\(humanConfig\),/);
});

test("runtime manifest annotation is inside each pod template; absence preserves all other bytes", () => {
  for (const text of [template, conductorTemplate]) {
    assert.equal(text.split(MANIFEST_PLACEHOLDER).length - 1, 1);
    assert.ok(text.includes(`  template:\n    metadata:\n      annotations:\n        elohim.protocol/runtime-manifest: ${MANIFEST_PLACEHOLDER}\n`));
    const expected = text.split("\n").filter((line) => !line.includes(MANIFEST_PLACEHOLDER)).join("\n");
    assert.equal(renderWithSed(text, manifestSedExpr(undefined)), expected);
  }
});

test("every active pin renders the exact CID in both pod templates without altering other bytes", () => {
  for (const human of deployments.humans.filter((h) => !h.suspended)) {
    assert.ok(human.runtimeManifest, `${human.name}: pinned`);
    for (const text of [template, conductorTemplate]) {
      assert.equal(
        renderWithSed(text, manifestSedExpr(human.runtimeManifest)),
        text.replace(MANIFEST_PLACEHOLDER, human.runtimeManifest.cid),
      );
    }
  }
});

test("runtime manifest annotation rejects sed and shell injection", () => {
  for (const cid of ["", "abc", "bafyre&bad", "bafyre|bad", "bafyre'bad", "bafyre\nbad"]) {
    assert.throws(() => manifestSedExpr({ cid }));
  }
});

test("fleet follow strings and pinned CIDs are fresh projections of runtime manifests", async () => {
  const { renderDeclarations } = await import('./runtime-declaration-render.mjs');
  const { fileURLToPath } = await import('node:url');
  const projected = renderDeclarations(fileURLToPath(ROOT), deployments, EPR_BIN);
  assert.deepEqual(projected, deployments);
});

test("manifest-backed fleet nodes receive their base and a writable watched projection", () => {
  assert.match(jenkinsfile, /readFile\(file: path, encoding: 'Base64'\)/);
  assert.match(jenkinsfile, /runtimeManifestBodySedExpr\(humanConfig\),/);
  assert.match(jenkinsfile, /\/data\/device\/runtime-config\.toml/);
  for (const text of [template, adam]) {
    assert.match(text, /binaryData:\n  runtime-manifest.json: RUNTIME_MANIFEST_BODY_PLACEHOLDER/);
    assert.match(text, /name: ELOHIM_RUNTIME_MANIFEST_PATH\n\s+value: "RUNTIME_MANIFEST_PATH_PLACEHOLDER"/);
    assert.match(text, /mountPath: \/etc\/elohim\/runtime-manifest.json\n\s+subPath: runtime-manifest.json/);
    assert.match(text, /name: ELOHIM_RUNTIME_CONFIG_PATH\n\s+value: "RUNTIME_CONFIG_PATH_PLACEHOLDER"/);
  }
  for (const human of deployments.humans.filter(h => h.runtimeManifest)) {
    const source = read(human.runtimeManifest.path);
    const encoded = Buffer.from(source).toString('base64');
    const rendered = renderWithSed(template, `s|RUNTIME_MANIFEST_BODY_PLACEHOLDER|${encoded}|`);
    const body = rendered.match(/runtime-manifest.json: (\S+)/)[1];
    assert.equal(Buffer.from(body, 'base64').toString(), source);
  }
});

test("deployment writes lock the transaction and replace atomically; hand edits refuse", async () => {
  const { updateDeclarations } =
    await import("./runtime-declaration-render.mjs");
  const { openSync, closeSync, readSync, rmSync } =
    await import("node:fs");
  const { fileURLToPath } = await import("node:url");
  const scratch = mkdtempSync(join(tmpdir(), "runtime-atomic-"));
  const file = join(scratch, "deployments.json");
  const changed = structuredClone(deployments);
  changed.humans.find(
    (h) => h.runtimeManifest,
  ).runtimeConfig.ELOHIM_RELEASE_CHANNELS = "hand-edit";
  const before = JSON.stringify(changed);
  writeFileSync(file, before);
  try {
    assert.throws(
      () => updateDeclarations(fileURLToPath(ROOT), file, EPR_BIN),
      /projection drift/,
    );
    // An open descriptor retains the complete old document across atomic replacement.
    const fd = openSync(file, "r");
    try {
      updateDeclarations(fileURLToPath(ROOT), file, EPR_BIN, true);
      const bytes = Buffer.alloc(Buffer.byteLength(before));
      assert.equal(readSync(fd, bytes, 0, bytes.length, 0), bytes.length);
      assert.equal(bytes.toString(), before);
    } finally {
      closeSync(fd);
    }
    assert.deepEqual(JSON.parse(readFileSync(file)), deployments);
    assert.deepEqual(readdirSync(scratch).sort(), ["deployments.json", "deployments.json.lock"]);
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
});

test("SIGKILL releases the deployment kernel lock for the next renderer", async () => {
  const { spawn } = await import("node:child_process");
  const { existsSync, rmSync, chmodSync } = await import("node:fs");
  const { once } = await import("node:events");
  const { setTimeout: delay } = await import("node:timers/promises");
  const { updateDeclarations } = await import("./runtime-declaration-render.mjs");
  const scratch = mkdtempSync(join(tmpdir(), "runtime-killed-"));
  const file = join(scratch, "deployments.json");
  const ready = join(scratch, "ready");
  const release = join(scratch, "release");
  const stub = join(scratch, "epr-stub");
  writeFileSync(stub, `#!/usr/bin/env node
const fs = require('fs');
fs.writeFileSync(${JSON.stringify(ready)}, 'held');
while (!fs.existsSync(${JSON.stringify(release)})) Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 10);
console.log(process.argv.includes('--json') ? '{"manifestCid":"new"}' : 'ELOHIM_RELEASE_CHANNELS = "bin:epr=apply"');
`);
  chmodSync(stub, 0o755);
  writeFileSync(file, JSON.stringify({ humans: [{ name: "fixture",
    runtimeManifest: { path: "unused", cid: "old" } }] }));
  const child = spawn(process.execPath, ["--input-type=module", "-e",
    `import { updateDeclarations } from ${JSON.stringify(new URL("./runtime-declaration-render.mjs", import.meta.url).href)};
     updateDeclarations(...process.argv.slice(1), true);`, scratch, file, stub],
    { stdio: "ignore" });
  try {
    for (let i = 0; i < 200 && !existsSync(ready); i++) await delay(10);
    assert.ok(existsSync(ready), "holder must acquire before being killed");
    const exited = once(child, "exit");
    child.kill("SIGKILL");
    await exited;
    assert.ok(existsSync(`${file}.lock`), "lock inode must remain stable");
    writeFileSync(release, "go"); // release the killed holder's native-render child too
    updateDeclarations(scratch, file, stub, true);
    assert.equal(JSON.parse(readFileSync(file)).humans[0].runtimeManifest.cid, "new");
  } finally {
    child.kill("SIGKILL");
    writeFileSync(release, "go");
    await delay(50);
    rmSync(scratch, { recursive: true, force: true });
  }
});


test("renderer refuses an unnamed EPR_BIN even when epr is on PATH", async () => {
  const { renderDeclarations, updateDeclarations } = await import("./runtime-declaration-render.mjs");
  assert.throws(() => requireEpr("epr"), /REFUSED EPR_BIN/);
  const previous = process.env.EPR_BIN;
  delete process.env.EPR_BIN;
  try {
    assert.throws(() => renderDeclarations('.', { humans: [] }), /REFUSED EPR_BIN/);
    assert.throws(() => updateDeclarations('.', '/not-a-file'), /REFUSED EPR_BIN/);
  } finally {
    process.env.EPR_BIN = previous;
  }
});

test("orchestrator gate supplies its declared native epr instead of a PATH launcher", async () => {
  const { chmodSync, rmSync } = await import("node:fs");
  const { fileURLToPath } = await import("node:url");
  const run = JSON.parse(read("genesis/orchestrator/build-manifest.json")).gate.projects.orchestrator.run;
  assert.equal(run.cargo.workspace, "elohim/eprfs");
  assert.equal(run.cargo.rustflags, "");
  const scratch = mkdtempSync(join(tmpdir(), "orchestrator-epr-"));
  try {
    // Stand in only for build and suite execution: exercise the actual typed
    // executor and recipe, including slot export and absence of EPR_BIN.
    for (const [name, body] of Object.entries({
      cargo: `if [ "$1" = metadata ]; then pwd > "$M_METADATA_DIR"; printf '{"packages":[]}\n'; else printf '%s\n' "$*" > "$M_BUILD_ARGS"; fi`,
      pnpm: 'printf "%s\\n" "$EPR_BIN" > "$M_SELECTED_EPR"',
      epr: 'echo "PATH launcher must not run" >&2; exit 99',
    })) {
      writeFileSync(join(scratch, name), `#!/bin/sh\n${body}\n`);
      chmodSync(join(scratch, name), 0o755);
    }
    const env = { ...process.env, PATH: `${scratch}:${process.env.PATH}`, EPR_BIN: "",
      GATE_CARGO_ENV: "{}", CARGO_TARGET_POOL_ROOT: join(scratch, "pool"), POOL_PARENT_REPO: fileURLToPath(ROOT),
      M_METADATA_DIR: join(scratch, "metadata-dir"), M_BUILD_ARGS: join(scratch, "build"), M_SELECTED_EPR: join(scratch, "selected") };
    execFileSync("bash", [fileURLToPath(new URL("run-local-gate.sh", import.meta.url)),
      fileURLToPath(ROOT), "orchestrator", "genesis/orchestrator", run.kind, "test-suite",
      run.cargo.workspace, "", "dev", run.cargo.rustflags], { env });
    const selected = readFileSync(env.M_SELECTED_EPR, "utf8").trim();
    assert.ok(selected.startsWith(env.CARGO_TARGET_POOL_ROOT + "/"));
    assert.ok(selected.endsWith("/elohim__eprfs/dev/debug/epr"));
    assert.equal(readFileSync(env.M_METADATA_DIR, "utf8").trim(), fileURLToPath(new URL("elohim/eprfs", ROOT)));
    assert.match(readFileSync(env.M_BUILD_ARGS, "utf8"), /build .*--locked -p elohim-epr-cli --bin epr/);
    env.EPR_BIN = "/explicit/lane/epr";
    rmSync(env.M_BUILD_ARGS);
    execFileSync("bash", [fileURLToPath(new URL("run-local-gate.sh", import.meta.url)),
      fileURLToPath(ROOT), "orchestrator", "genesis/orchestrator", run.kind, "test-suite",
      run.cargo.workspace, "", "dev", run.cargo.rustflags], { env });
    assert.equal(readFileSync(env.M_SELECTED_EPR, "utf8").trim(), env.EPR_BIN);
    assert.ok(!readdirSync(scratch).includes("build"), "explicit lane does not rebuild");
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
});
