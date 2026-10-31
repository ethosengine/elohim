// The storage-data claim template states each human's real volume size.
//
// deployments.json `storageDataSize` is the declared size of `storage-data-<prefix>-0`.
// elohim/holochain/Jenkinsfile renders it into STORAGE_DATA_SIZE_PLACEHOLDER (20Gi
// when absent), and scripts/ci/grow-conductor-pvc.sh raises the live claim to it
// before the apply. Until 2026-10-29 the template said a fixed 20Gi while four shem
// claims were resized to 80Gi by hand, so the manifest no longer described the
// machine. Pinned here:
//   1. both manifests carry the placeholder, once, as the claim's request;
//   2. the Jenkinsfile renders it from storageDataSize with the 20Gi default;
//   3. every declared size is a Gi quantity no smaller than 20Gi (the size every
//      claim was minted at; a claim never shrinks).
import { readFileSync } from "node:fs";
import { test } from "node:test";
import assert from "node:assert/strict";

const ROOT = new URL("../../", import.meta.url);
const read = (rel) => readFileSync(new URL(rel, ROOT), "utf8");

const PLACEHOLDER = "STORAGE_DATA_SIZE_PLACEHOLDER";
const manifests = {
  template: read("genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml"),
  adam: read("genesis/orchestrator/manifests/humans/adam-firstman.yaml"),
};
const jenkinsfile = read("elohim/holochain/Jenkinsfile");
const deployments = JSON.parse(read("genesis/orchestrator/data/deployments.json"));

test("both human manifests request the rendered size for storage-data", () => {
  for (const [name, text] of Object.entries(manifests)) {
    assert.equal(text.split(PLACEHOLDER).length - 1, 1, `${name}: one ${PLACEHOLDER}`);
    const claim = text.slice(text.indexOf("volumeClaimTemplates:"));
    assert.match(claim, /name: storage-data[\s\S]*?\n\s+storage: STORAGE_DATA_SIZE_PLACEHOLDER\n/, `${name}: the claim's request`);
  }
});

test("the Jenkinsfile renders storageDataSize with the 20Gi default", () => {
  assert.ok(
    jenkinsfile.includes(`"s|${PLACEHOLDER}|\${humanConfig.storageDataSize ?: '20Gi'}|g",`),
    "sed arm for the storage-data size",
  );
});

test("every declared storageDataSize is a Gi quantity of at least 20Gi", () => {
  const humans = Object.values(deployments).flatMap((v) => (Array.isArray(v) ? v : []));
  const declared = humans.filter((h) => h && typeof h === "object" && "storageDataSize" in h);
  assert.ok(declared.length > 0, "the shem humans declare their size");
  for (const h of declared) {
    const m = /^(\d+)Gi$/.exec(h.storageDataSize);
    assert.ok(m, `${h.name}: ${h.storageDataSize} is not a Gi quantity`);
    assert.ok(Number(m[1]) >= 20, `${h.name}: ${h.storageDataSize} is below the 20Gi a claim was minted at`);
  }
});
