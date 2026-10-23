// Serialized into the CI-only fixed Node exec. Never reads a live key or backup.
export function readRecoveryProof(
  base = "/var/lib/compute/mesh",
  expected = {
    originalLog:
      "490ba1c27615728aab776a6e25fa2afeca7e92e30ed9157e7573d232ee324c4c",
    inventory:
      "8d5e87b8824ee9d89d640bb4e71128b3a4abfb04dc6a4a46e73c6a0f6805ab65",
    baseline:
      "19153b700d3c9b9bdf9a7c58aa2d229cb55fd0303717d5e1ad8305fe0324f7ce",
  },
) {
  const fs = require("node:fs");
  const path = require("node:path");
  const crypto = require("node:crypto");
  const peers = ["matthew", "jessica", "james"];
  const roles = [
    "imagodei",
    "infrastructure",
    "lamad",
    "mishpat",
    "node_registry",
  ];
  const assert = (condition) => {
    if (!condition) throw new Error("invalid proof");
  };
  const hash = (value) =>
    crypto.createHash("sha256").update(value).digest("hex");
  const digest = (value) =>
    typeof value === "string" && /^[a-f0-9]{64}$/.test(value);
  const keys = (value, names) => {
    assert(
      value !== null && typeof value === "object" && !Array.isArray(value),
    );
    assert(
      JSON.stringify(Object.keys(value).sort()) ===
        JSON.stringify([...names].sort()),
    );
  };
  const canonical = (value) => {
    if (Array.isArray(value)) return "[" + value.map(canonical).join(",") + "]";
    if (value !== null && typeof value === "object")
      return (
        "{" +
        Object.keys(value)
          .sort()
          .map((k) => JSON.stringify(k) + ":" + canonical(value[k]))
          .join(",") +
        "}"
      );
    return JSON.stringify(value);
  };
  const read = (relative) => {
    const file = path.resolve(base, "initialization-continuation", relative);
    for (let current = file; ; current = path.dirname(current)) {
      const stat = fs.lstatSync(current);
      assert(!stat.isSymbolicLink());
      assert(current === file ? stat.isFile() : stat.isDirectory());
      if (current === path.dirname(current)) break;
    }
    const fd = fs.openSync(
      file,
      fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW,
    );
    try {
      const stat = fs.fstatSync(fd);
      assert(stat.isFile() && stat.size > 0 && stat.size <= 65536);
      const buffer = Buffer.alloc(65537);
      let count = 0;
      while (count < buffer.length) {
        const n = fs.readSync(fd, buffer, count, buffer.length - count, null);
        if (!n) break;
        count += n;
      }
      assert(count > 0 && count <= 65536);
      return buffer.subarray(0, count);
    } finally {
      fs.closeSync(fd);
    }
  };
  const json = (bytes) => {
    const value = JSON.parse(bytes.toString("utf8"));
    assert(bytes.equals(Buffer.from(canonical(value) + "\n")));
    return value;
  };
  const raw39 = (value, prefix) => {
    assert(typeof value === "string" && value.startsWith(prefix));
    const bytes = Buffer.from(value.slice(1), "base64url");
    assert(bytes.length === 39 && "u" + bytes.toString("base64url") === value);
    return bytes.toString("hex");
  };
  try {
    keys(expected, ["originalLog", "inventory", "baseline"]);
    assert(Object.values(expected).every(digest));
    const originalSeal = read("complete").toString("utf8");
    assert(originalSeal === expected.originalLog + "\n");
    const before = read("apps-before.jsonl");
    const afterApps = read("apps-after.jsonl");
    assert(before.equals(afterApps) && hash(before) === expected.baseline);
    const rows = before
      .toString("utf8")
      .trimEnd()
      .split("\n")
      .map((line) => {
        const row = JSON.parse(line);
        assert(line === canonical(row));
        return row;
      });
    assert(rows.length === peers.length);
    const allowed = new Set();
    const initial = {};
    rows.forEach((row, index) => {
      keys(row, ["peer", "agent", "cells"]);
      assert(row.peer === peers[index]);
      const agent = raw39(row.agent, "uhCAk");
      keys(row.cells, roles);
      for (const role of roles) {
        const cells = row.cells[role];
        assert(Array.isArray(cells) && cells.length === 1);
        const cell = cells[0];
        keys(cell, ["agent_pub_key", "dna_hash"]);
        assert(cell.agent_pub_key === row.agent);
        const name =
          row.peer +
          "/closed-chain-fence/credentials/" +
          raw39(cell.dna_hash, "uhC0k") +
          "-" +
          agent +
          ".json";
        allowed.add(name);
        if (role === "infrastructure") initial[row.peer] = name;
      }
    });
    const inventoryBytes = read("resume-existing/inventory.json");
    assert(hash(inventoryBytes) === expected.inventory);
    const inventory = json(inventoryBytes);
    keys(inventory, ["version", "peers", "files"]);
    assert(
      inventory.version === 1 &&
        JSON.stringify(inventory.peers) === JSON.stringify(peers),
    );
    const paths = [
      "initialization-continuation/start.log",
      "initialization-continuation/apps-before.jsonl",
      "logs/start.log",
    ];
    for (const peer of peers)
      paths.push(
        peer + "/content.db",
        peer + "/identity.key",
        peer + "/iroh.key",
        peer + "/closed-chain-fence/closed-cells.json",
        initial[peer],
      );
    keys(inventory.files, paths);
    const originals = {};
    for (const [name, value] of Object.entries(inventory.files)) {
      if (name.endsWith("/iroh.key") || name.endsWith("/closed-cells.json"))
        assert(value === null);
      else if (name.endsWith("/content.db")) {
        keys(value, ["size"]);
        assert(Number.isSafeInteger(value.size) && value.size > 0);
      } else {
        keys(value, ["sha256"]);
        assert(digest(value.sha256));
        originals[name] = value.sha256;
      }
    }
    assert(
      originals["initialization-continuation/start.log"] ===
        expected.originalLog,
    );
    assert(
      originals["initialization-continuation/apps-before.jsonl"] ===
        expected.baseline,
    );
    const afterBytes = read("resume-existing/after-state.json");
    const after = json(afterBytes);
    keys(after, [
      "version",
      "inventorySha256",
      "originalFiles",
      "iroh",
      "credentials",
      "appsBeforeSha256",
      "appsAfterSha256",
    ]);
    assert(after.version === 1 && after.inventorySha256 === expected.inventory);
    assert(canonical(after.originalFiles) === canonical(originals));
    assert(
      after.appsBeforeSha256 === expected.baseline &&
        after.appsAfterSha256 === expected.baseline,
    );
    keys(after.iroh, peers);
    for (const value of Object.values(after.iroh)) {
      keys(value, ["sha256", "size"]);
      assert(value.size === 32 && digest(value.sha256));
    }
    assert(
      after.credentials &&
        typeof after.credentials === "object" &&
        !Array.isArray(after.credentials),
    );
    for (const [name, value] of Object.entries(after.credentials))
      assert(allowed.has(name) && digest(value));
    for (const peer of peers)
      assert(after.credentials[initial[peer]] === originals[initial[peer]]);
    const seal = json(read("resume-existing/complete"));
    keys(seal, ["version", "inventorySha256", "afterSha256"]);
    assert(
      seal.version === 1 &&
        seal.inventorySha256 === expected.inventory &&
        seal.afterSha256 === hash(afterBytes),
    );
    return {
      status: "verified",
      kind: "historical-initialization-recovery-proof",
      originalLogSha256: expected.originalLog,
      baselineSha256: expected.baseline,
      inventory,
      afterState: after,
      seal,
    };
  } catch {
    // No exception text, paths, failed JSON, or secret-bearing unknown fields.
    return {
      status: "refused",
      reason: "recovery-proof-unavailable-or-invalid",
    };
  }
}
