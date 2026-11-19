// The synthetic repository the compute-worker tests render against (shared by the named file and
// its tree-side supervisor file, compute-worker-supervisor.test.mjs).
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

export async function writeFittingFixtureRepo() {
  // A synthetic repo tree with a generous envelope bound and an empty-container
  // conductor manifest, isolating renderer invariants from the deployed configuration.
  const root = await mkdtemp(join(tmpdir(), "compute-worker-fixture-"));
  const orchestratorDir = join(root, "genesis", "orchestrator");
  await mkdir(join(orchestratorDir, "data"), { recursive: true });
  await mkdir(join(orchestratorDir, "manifests", "runtime"), {
    recursive: true,
  });
  await mkdir(join(orchestratorDir, "manifests", "humans"), {
    recursive: true,
  });
  await writeFile(
    join(orchestratorDir, "data", "deployments.json"),
    JSON.stringify({
      humans: [
        {
          name: "adam",
          runtimeManifest: {
            path: "genesis/orchestrator/manifests/runtime/adam.manifest.json",
          },
          conductorManifest:
            "genesis/orchestrator/manifests/humans/adam-firstman-conductor.yaml",
        },
      ],
    }),
  );
  await writeFile(
    join(orchestratorDir, "manifests", "runtime", "adam.manifest.json"),
    JSON.stringify({
      processes: [
        {
          name: "compute-worker",
          kind: "delegated",
          quota: { memory_max_bytes: 512 * 1024 * 1024, cpu_share_millis: 500 },
        },
      ],
      envelope: {
        bound: { memory_bytes: 100 * 1024 * 1024 * 1024, cpu_millis: 100000 },
      },
    }),
  );
  await writeFile(
    join(
      orchestratorDir,
      "manifests",
      "humans",
      "adam-firstman-conductor.yaml",
    ),
    "apiVersion: apps/v1\nkind: StatefulSet\nmetadata:\n  name: adam-conductor-fixture\nspec:\n  template:\n    spec:\n      containers: []\n",
  );
  return root;
}
