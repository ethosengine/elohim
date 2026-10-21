#!/usr/bin/env node
// Alpha sponsorship reuses an existing operator credential at declared dev stakes.
// Storage still verifies the token and grant actor on every operation.
import { readFile, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import YAML from "yaml";

const pools = {
  matthew: "elohim-doorway-alpha-secrets",
  adam: "elohim-doorway-alpha-b-secrets",
};
export function renderPoolCompute(source, human, environment) {
  if (environment !== "alpha" || !pools[human]) return source;
  const documents = YAML.parseAllDocuments(source);
  const workload = documents.find((doc) => doc.get("kind") === "StatefulSet");
  if (!workload) throw new Error("Pool storage StatefulSet missing");
  const manifest = workload.toJS();
  const storage = manifest.spec.template.spec.containers.find(
    (c) => c.name === "elohim-node",
  );
  if (!storage) throw new Error("Pool storage container missing");
  const names = new Set([
    "POOL_COMPUTE_BOOTSTRAP_KEY",
    "ELOHIM_COMPUTE_LOCAL_API",
    "ELOHIM_COMPUTE_LOCAL_TOKEN",
  ]);
  storage.env = (storage.env ?? []).filter((v) => !names.has(v.name));
  storage.env.push(
    {
      name: "POOL_COMPUTE_BOOTSTRAP_KEY",
      valueFrom: { secretKeyRef: { name: pools[human], key: "api-key-admin" } },
    },
    { name: "ELOHIM_COMPUTE_LOCAL_API", value: "1" },
    {
      name: "ELOHIM_COMPUTE_LOCAL_TOKEN",
      value: "$(POOL_COMPUTE_BOOTSTRAP_KEY):elohim-pool-compute",
    },
  );
  workload.contents = workload.createNode(manifest);
  return documents.map((doc) => doc.toString()).join("---\n");
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  const [file, human, environment] = process.argv.slice(2);
  if (!file || !human || !environment)
    throw new Error(
      "usage: render-pool-compute.mjs manifest human environment",
    );
  const source = await readFile(file, "utf8");
  await writeFile(file, renderPoolCompute(source, human, environment));
}
