import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import crypto from "node:crypto";
import { createRequire } from "node:module";
import vm from "node:vm";
import { readRecoveryProof } from "./runner-recovery-diagnostics.mjs";

const canonical = (value) => {
  if (Array.isArray(value)) return "[" + value.map(canonical).join(",") + "]";
  if (value && typeof value === "object")
    return (
      "{" +
      Object.keys(value)
        .sort()
        .map((key) => JSON.stringify(key) + ":" + canonical(value[key]))
        .join(",") +
      "}"
    );
  return JSON.stringify(value);
};
const bytes = (value) => canonical(value) + "\n";
const hash = (value) => crypto.createHash("sha256").update(value).digest("hex");
const req = createRequire(import.meta.url);
function fixture(t) {
  const base = fs.mkdtempSync(path.join(os.tmpdir(), "recovery-reader-"));
  t.after(() => fs.rmSync(base, { recursive: true, force: true }));
  const journal = path.join(base, "initialization-continuation");
  const write = (name, value) => {
    const p = path.join(journal, name);
    fs.mkdirSync(path.dirname(p), { recursive: true });
    fs.writeFileSync(p, value);
  };
  const peers = ["matthew", "jessica", "james"];
  const roles = [
    "imagodei",
    "infrastructure",
    "lamad",
    "mishpat",
    "node_registry",
  ];
  const initial = {};
  const rows = peers.map((peer, i) => {
    const agentRaw = Buffer.concat([
      Buffer.from("842024", "hex"),
      Buffer.alloc(36, i + 1),
    ]);
    const agent = "u" + agentRaw.toString("base64url");
    const cells = {};
    roles.forEach((role, index) => {
      const dnaRaw = Buffer.concat([
        Buffer.from("842d24", "hex"),
        Buffer.alloc(36, index + 4),
      ]);
      cells[role] = [
        { agent_pub_key: agent, dna_hash: "u" + dnaRaw.toString("base64url") },
      ];
      if (role === "infrastructure")
        initial[peer] =
          peer +
          "/closed-chain-fence/credentials/" +
          dnaRaw.toString("hex") +
          "-" +
          agentRaw.toString("hex") +
          ".json";
    });
    return { peer, agent, cells };
  });
  const baseline = rows.map(bytes).join("");
  const expected = {
    originalLog: hash("reviewed original"),
    baseline: hash(baseline),
  };
  const files = {
    "initialization-continuation/start.log": { sha256: expected.originalLog },
    "initialization-continuation/apps-before.jsonl": {
      sha256: expected.baseline,
    },
    "logs/start.log": { sha256: hash("failed start") },
  };
  for (const peer of peers) {
    files[peer + "/content.db"] = { size: 2072576 };
    files[peer + "/identity.key"] = { sha256: hash(peer) };
    files[peer + "/iroh.key"] = null;
    files[peer + "/closed-chain-fence/closed-cells.json"] = null;
    files[initial[peer]] = { sha256: hash("private credential " + peer) };
  }
  const inventory = { version: 1, peers, files };
  expected.inventory = hash(bytes(inventory));
  const after = {
    version: 1,
    inventorySha256: expected.inventory,
    originalFiles: Object.fromEntries(
      Object.entries(files)
        .filter(([, v]) => v?.sha256)
        .map(([k, v]) => [k, v.sha256]),
    ),
    iroh: {},
  };
  after.iroh = Object.fromEntries(
    peers.map((p) => [p, { size: 32, sha256: hash("iroh " + p) }]),
  );
  after.credentials = Object.fromEntries(
    peers.map((p) => [initial[p], files[initial[p]].sha256]),
  );
  after.appsBeforeSha256 = expected.baseline;
  after.appsAfterSha256 = expected.baseline;
  const seal = {
    version: 1,
    inventorySha256: expected.inventory,
    afterSha256: hash(bytes(after)),
  };
  write("complete", expected.originalLog + "\n");
  write("apps-before.jsonl", baseline);
  write("apps-after.jsonl", baseline);
  write("resume-existing/inventory.json", bytes(inventory));
  write("resume-existing/after-state.json", bytes(after));
  write("resume-existing/complete", bytes(seal));
  const observed = [];
  const reader = vm.runInNewContext("(" + readRecoveryProof.toString() + ")", {
    Buffer,
    require: (name) =>
      name === "node:fs"
        ? new Proxy(fs, {
            get(target, key) {
              if (key === "openSync")
                return (p, ...args) => {
                  observed.push(p);
                  return target.openSync(p, ...args);
                };
              return target[key];
            },
          })
        : req(name),
  });
  return {
    base,
    journal,
    write,
    expected,
    inventory,
    after,
    seal,
    observed,
    run: () => reader(base, expected),
    production: () => reader(base),
  };
}

test("serialized self-contained reader verifies only public proof files", (t) => {
  const f = fixture(t);
  const result = f.run();
  assert.equal(result.status, "verified");
  assert.deepEqual(JSON.parse(JSON.stringify(result.inventory)), f.inventory);
  assert.equal(f.observed.length, 6);
  assert.ok(
    f.observed.every(
      (p) =>
        !p.includes("/backup/") &&
        !p.endsWith("/identity.key") &&
        !p.includes("/credentials/"),
    ),
  );
  assert.equal(f.production().status, "refused"); // Production pins cannot accept fixture hashes.
});
for (const kind of [
  "unknown-secret",
  "malformed",
  "symlink",
  "oversize",
  "digest-drift",
  "iroh-length",
]) {
  test(kind + " refuses without disclosing failed bytes", (t) => {
    const f = fixture(t);
    const marker = "DO_NOT_DISCLOSE_SECRET";
    const target = "resume-existing/after-state.json";
    if (kind === "unknown-secret") {
      f.after.secret = marker;
      f.write(target, bytes(f.after));
      f.seal.afterSha256 = hash(bytes(f.after));
      f.write("resume-existing/complete", bytes(f.seal));
    } else if (kind === "malformed") f.write(target, "{" + marker);
    else if (kind === "symlink") {
      const p = path.join(f.journal, target);
      fs.unlinkSync(p);
      fs.symlinkSync(path.join(f.journal, "apps-before.jsonl"), p);
    } else if (kind === "oversize") f.write(target, marker.repeat(5000));
    else if (kind === "digest-drift")
      f.write("apps-after.jsonl", "changed " + marker);
    else {
      f.after.iroh.matthew.size = 31;
      f.write(target, bytes(f.after));
      f.seal.afterSha256 = hash(bytes(f.after));
      f.write("resume-existing/complete", bytes(f.seal));
    }
    const result = f.run();
    assert.equal(result.status, "refused");
    assert.equal(
      JSON.stringify(result),
      '{"status":"refused","reason":"recovery-proof-unavailable-or-invalid"}',
    );
  });
}
