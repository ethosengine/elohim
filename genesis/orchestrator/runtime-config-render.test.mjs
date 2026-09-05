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
import { readFileSync, writeFileSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

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

test("every declared ELOHIM_RELEASE_CHANNELS follows the one workspace channel id", () => {
  const withChannel = deployments.humans.filter(
    (h) => h.runtimeConfig?.ELOHIM_RELEASE_CHANNELS !== undefined,
  );
  assert.ok(withChannel.length > 0, "at least one alpha human declares the channel");
  for (const human of withChannel) {
    const value = human.runtimeConfig.ELOHIM_RELEASE_CHANNELS;
    assert.match(
      value,
      new RegExp(`^${WORKSPACE_CHANNEL_ID.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}=(observe|canary|apply)$`),
      `${human.name}: ELOHIM_RELEASE_CHANNELS must be the single workspace channel id at a valid mode`,
    );
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
  // active alpha human enrolls at =observe on the FIRST render -- no
  // apply/canary from a workspace channel while nobody is watching. Promotion
  // (james to canary, then matthew/jessica to apply on the bootstrap pair) is
  // a later data change the operator flips after Task 4's first
  // workspace->alpha receipt, not minted here.
  for (const [name, mode] of [
    ["james", "observe"],
    ["matthew", "observe"],
    ["jessica", "observe"],
    ["adam", "observe"],
    ["gertrude", "observe"],
    ["susan", "observe"],
    ["eve", "observe"],
  ]) {
    const human = humanNamed(name);
    const expectedValue = `${WORKSPACE_CHANNEL_ID}=${mode}`;
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
});
