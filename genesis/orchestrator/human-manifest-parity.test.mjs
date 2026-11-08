// A per-human manifest that forks a shared template must keep the template's shape.
//
// Adam renders his storage and conductor workloads from his own manifests
// (adam-firstman.yaml, adam-firstman-conductor.yaml) instead of the shared
// templates every other alpha human renders from. Re-genesis 1 (2026-11-07/08)
// found both pairs had drifted: a conductor runtime fix edited only the template,
// and nine storage env vars added to the template never reached adam — among them
// ALLOW_SEED_NETWORK_STAKES, so every Genesis run seeded stakes on "2/3 peers —
// failed (stay Bootstrap): adam". Pinned here: for every workload kind and
// container, the env-var NAMES and the container set agree between template and
// fork. Values may differ (that is what a per-human manifest is for); a name that
// is meant to differ goes in ALLOWED with the reason.
import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";
import yaml from "js-yaml";

const ROOT = new URL("../../", import.meta.url);
const read = (rel) => readFileSync(new URL(rel, ROOT), "utf8");

const PAIRS = {
  storage: {
    template: "genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml",
    fork: "genesis/orchestrator/manifests/humans/adam-firstman.yaml",
  },
  conductor: {
    template: "genesis/orchestrator/manifests/humans/_edgenode-conductor.template.yaml",
    fork: "genesis/orchestrator/manifests/humans/adam-firstman-conductor.yaml",
  },
};

// `<pair>:<Kind>/<container>:<ENV_NAME>` → why the fork intentionally differs.
const ALLOWED = new Map();

/** Map of `<Kind>/<container>` → Set of env-var names, init containers prefixed `init:`. */
function shape(rel) {
  const out = new Map();
  for (const doc of yaml.loadAll(read(rel))) {
    const pod = doc?.spec?.template?.spec;
    if (!pod) continue;
    const containers = [
      ...(pod.initContainers ?? []).map((c) => ({ ...c, name: `init:${c.name}` })),
      ...(pod.containers ?? []),
    ];
    for (const c of containers) {
      out.set(`${doc.kind}/${c.name}`, new Set((c.env ?? []).map((e) => e.name)));
    }
  }
  return out;
}

for (const [pair, { template, fork }] of Object.entries(PAIRS)) {
  const t = shape(template);
  const f = shape(fork);

  test(`${pair}: the fork carries the template's workloads and containers`, () => {
    assert.ok(t.size > 0, `${template} declares no pod-template workloads`);
    assert.deepEqual([...f.keys()].sort(), [...t.keys()].sort(), `${fork} vs ${template}`);
  });

  test(`${pair}: every container's env-var names match the template`, () => {
    const drift = [];
    for (const [key, names] of t) {
      const forked = f.get(key) ?? new Set();
      for (const n of names) {
        if (!forked.has(n) && !ALLOWED.has(`${pair}:${key}:${n}`)) drift.push(`${key}: ${n} only in template`);
      }
      for (const n of forked) {
        if (!names.has(n) && !ALLOWED.has(`${pair}:${key}:${n}`)) drift.push(`${key}: ${n} only in ${fork}`);
      }
    }
    assert.deepEqual(drift, [], `mirror the change into both files, or allowlist it with a reason`);
  });
}
