// Assemble immutable inputs; an explicit toolsBase rebuilds the pinned source tools.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import {
  readFileSync,
  writeFileSync,
  mkdirSync,
  mkdtempSync,
  copyFileSync,
  rmSync,
} from "node:fs";
import { join, resolve } from "node:path";
import { createHash } from "node:crypto";
import { pathToFileURL } from "node:url";
export function validateInputs(input) {
  assert(input && typeof input === "object" && !Array.isArray(input));
  const rebuilding = Object.hasOwn(input, "toolsBase");
  assert.deepEqual(
    Object.keys(input).sort(),
    [
      "sourceCommit",
      rebuilding ? "toolsBase" : "tools",
      "storage",
      "doorway",
      "relay",
      "beacon",
      "happArtifact",
    ].sort(),
  );
  assert(/^[a-f0-9]{40}$/.test(input.sourceCommit));
  if (rebuilding)
    assert(
      /^harbor\.ethosengine\.com\/devspaces\/udi-plus-mem-rust-nix@sha256:[a-f0-9]{64}$/.test(
        input.toolsBase,
      ),
    );
  const repos = {
    ...(rebuilding ? {} : { tools: "elohim-compute-worker" }),
    storage: "elohim-storage",
    doorway: "elohim-doorway",
    relay: "iroh-relay",
    beacon: "relay-addr-beacon",
    happArtifact: "elohim-happ",
  };
  for (const [key, repo] of Object.entries(repos))
    assert(
      new RegExp(
        `^harbor\\.ethosengine\\.com/ethosengine/${repo}@sha256:[a-f0-9]{64}$`,
      ).test(input[key]),
    );
  return input;
}
function main() {
  const run = (cmd, args, options = {}) =>
    execFileSync(cmd, args, { stdio: "inherit", ...options });
  const read = (cmd, args) =>
    run(cmd, args, {
      stdio: ["ignore", "pipe", "inherit"],
      encoding: "utf8",
    }).trim();
  const root = read("git", ["rev-parse", "--show-toplevel"]);
  const output = join(root, "compute-worker-image");
  mkdirSync(output, { recursive: true });
  rmSync(join(output, "image.json"), { force: true });
  const [flag, path, extra] = process.argv.slice(2);
  assert(flag === "--inputs" && path && !extra);
  const requested = validateInputs(JSON.parse(readFileSync(path)));
  const inputs = { ...requested };
  const assemblyCommit = read("git", ["rev-parse", "HEAD"]);
  assert(/^[a-f0-9]{40}$/.test(assemblyCommit));
  const ctx = mkdtempSync(join(output, ".package-"));
  const nc = (...args) => run("nerdctl", ["-n", "k8s.io", ...args]);
  try {
    if (inputs.toolsBase) {
      const base = inputs.toolsBase;
      // Refuse a compiler whose default would recreate the target's SIGILL.
      const baseCheck = join(ctx, "Dockerfile.base-check");
      writeFileSync(
        baseCheck,
        `FROM ${base}\nUSER 0:0\nRUN bash --version && node --version && /usr/local/bin/mongod --version\nRUN set -eu; for compiler in gcc g++; do $compiler -Q --help=target | grep -E '^[[:space:]]+-march=[[:space:]]+x86-64(-v2)?[[:space:]]*$'; done\n`,
      );
      process.env.BUILDKIT_HOST ||= "unix:///run/buildkit/buildkitd.sock";
      nc("build", "--file", baseCheck, ctx);
      const toolsTag = `harbor.ethosengine.com/ethosengine/elohim-compute-worker:tools-rebuild-${assemblyCommit}`;
      run("bash", [
        join(root, "scripts/ci/build-compute-worker.sh"),
        "--source",
        base,
        toolsTag,
        inputs.sourceCommit,
      ]);
      nc("push", toolsTag);
      nc("pull", toolsTag);
      const rebuilt = JSON.parse(
        read("nerdctl", ["-n", "k8s.io", "image", "inspect", toolsTag]),
      )[0];
      assert.equal(
        rebuilt.Config.Labels["org.opencontainers.image.revision"],
        inputs.sourceCommit,
      );
      inputs.tools = rebuilt.RepoDigests.find((ref) =>
        /^harbor\.ethosengine\.com\/ethosengine\/elohim-compute-worker@sha256:[a-f0-9]{64}$/.test(
          ref,
        ),
      );
      assert(inputs.tools);
      delete inputs.toolsBase;
      inputs.base = base;
    }
    nc("pull", inputs.tools);
    const info = JSON.parse(
      read("nerdctl", ["-n", "k8s.io", "image", "inspect", inputs.tools]),
    )[0];
    assert.equal(
      info.Config.Labels["org.opencontainers.image.revision"],
      inputs.sourceCommit,
    );
    // Embedded checkout identity is checked inside BuildKit by Dockerfile.mesh.
    // nerdctl run cannot mount the host containerd snapshots from this CI pod.
    run("oras", ["pull", inputs.happArtifact, "--output", ctx]);
    const happSha256 = createHash("sha256")
      .update(readFileSync(join(ctx, "elohim.happ")))
      .digest("hex");
    const provenance = { ...inputs, assemblyCommit, happSha256 };
    writeFileSync(
      join(ctx, "inputs.json"),
      JSON.stringify(provenance, null, 2) + "\n",
    );
    copyFileSync(
      join(root, "genesis/agentic/compute/Dockerfile.mesh"),
      join(ctx, "Dockerfile"),
    );
    copyFileSync(join(ctx, "inputs.json"), join(output, "inputs.json"));
    const tag = `harbor.ethosengine.com/ethosengine/elohim-compute-worker:package-${assemblyCommit}`;
    const args = [
      "build",
      "--label",
      `org.elohim.assembly.revision=${assemblyCommit}`,
    ];
    for (const [name, key] of Object.entries({
      COMPUTE_TOOLS_IMAGE: "tools",
      STORAGE_IMAGE: "storage",
      DOORWAY_IMAGE: "doorway",
      RELAY_IMAGE: "relay",
      BEACON_IMAGE: "beacon",
    }))
      args.push("--build-arg", `${name}=${inputs[key]}`);
    process.env.BUILDKIT_HOST ||= "unix:///run/buildkit/buildkitd.sock";
    nc(...args, "--tag", tag, ctx);
    nc("push", tag);
    nc("pull", tag);
    const built = JSON.parse(
      read("nerdctl", ["-n", "k8s.io", "image", "inspect", tag]),
    )[0];
    assert.equal(
      built.Config.Labels["org.opencontainers.image.revision"],
      inputs.sourceCommit,
    );
    assert.equal(
      built.Config.Labels["org.elohim.assembly.revision"],
      assemblyCommit,
    );
    const image = built.RepoDigests.find((ref) =>
      /^harbor\.ethosengine\.com\/ethosengine\/elohim-compute-worker@sha256:[a-f0-9]{64}$/.test(
        ref,
      ),
    );
    assert(image);
    writeFileSync(
      join(output, "image.json"),
      JSON.stringify({ image, ...provenance }, null, 2) + "\n",
    );
    console.log(`compute worker packaged: ${image}`);
  } finally {
    rmSync(ctx, { recursive: true, force: true });
  }
}
if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  try {
    main();
  } catch {
    console.error("REFUSED compute worker packaging; no success artifact");
    process.exitCode = 1;
  }
}
