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
import {
  WORKER_REPOSITORY,
  JSON_LIMIT,
  exporterDigest,
  registryImage,
  registryBlob,
} from "./package-compute-worker-registry.mjs";
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
let packagingPhase = "inputs";
function main() {
  process.umask(0o077);
  // GNU timeout fences the process group and escalates after five seconds.
  const run = (cmd, args, { seconds = 30, capture = false, ...options } = {}) =>
    execFileSync(
      "timeout",
      ["--signal=TERM", "--kill-after=5", String(seconds), cmd, ...args],
      {
        stdio: capture ? ["ignore", "pipe", "pipe"] : "inherit",
        maxBuffer: JSON_LIMIT,
        ...options,
      },
    );
  const read = (cmd, args) =>
    run(cmd, args, { capture: true }).toString().trim();
  const root = read("git", ["rev-parse", "--show-toplevel"]);
  const output = join(root, "compute-worker-image");
  mkdirSync(output, { recursive: true });
  for (const name of ["image.json", "inputs.json", "registry-preflight.json"])
    rmSync(join(output, name), { force: true });
  const [flag, path, extra] = process.argv.slice(2);
  assert(flag === "--inputs" && path && !extra);
  const requested = validateInputs(JSON.parse(readFileSync(path)));
  const inputs = { ...requested };
  const assemblyCommit = read("git", ["rev-parse", "HEAD"]);
  assert(/^[a-f0-9]{40}$/.test(assemblyCommit));
  assert(
    process.env.DOCKER_CONFIG,
    "explicit private registry config required",
  );
  const registryConfig = join(process.env.DOCKER_CONFIG, "config.json");
  // Private build/export metadata cannot enter Jenkins workspace archives, even on abort.
  const ctx = mkdtempSync("/tmp/compute-worker-package-");
  const addr =
    process.env.BUILDKIT_HOST || "unix:///run/buildkit/buildkitd.sock";
  const remaining = (deadline, ceiling = 30) => {
    const seconds = Math.ceil((deadline - Date.now()) / 1000);
    assert(seconds > 0, "registry preflight deadline exceeded");
    return Math.min(seconds, ceiling);
  };
  const oras = (args, seconds = 30) =>
    run("oras", [...args, "--registry-config", registryConfig], {
      capture: true,
      seconds,
    });
  const build = (context, filename, args = [], seconds = 120) =>
    run(
      "buildctl",
      [
        "--addr",
        addr,
        "build",
        "--frontend",
        "dockerfile.v0",
        "--local",
        `context=${context}`,
        "--local",
        `dockerfile=${context}`,
        "--opt",
        `filename=${filename}`,
        "--opt",
        "platform=linux/amd64",
        "--progress",
        "plain",
        ...args,
      ],
      { seconds },
    );
  const exported = (metadata, labels, readBlob = oras) =>
    registryImage(readBlob, exporterDigest(readFileSync(metadata)), labels);
  try {
    // Exercise auth, publication and digest verification before the cold build.
    // This tiny scratch image contains only the explicit public probe payload.
    const deadline = Date.now() + 120_000;
    const probe = join(ctx, "preflight");
    mkdirSync(probe);
    const proof = `compute-worker registry export preflight\nassembly=${assemblyCommit}\n`;
    writeFileSync(join(probe, "proof.txt"), proof);
    writeFileSync(
      join(probe, "Dockerfile"),
      "FROM scratch\nCOPY proof.txt /export-proof.txt\n",
    );
    const probeMetadata = join(ctx, "preflight-metadata.json");
    packagingPhase = "preflight-export";
    build(
      probe,
      "Dockerfile",
      [
        "--opt",
        `label:org.elohim.export-preflight=${assemblyCommit}`,
        "--output",
        `type=image,name=${WORKER_REPOSITORY}:export-preflight-${assemblyCommit},push=true`,
        "--metadata-file",
        probeMetadata,
      ],
      remaining(deadline, 120),
    );
    packagingPhase = "preflight-verify";
    const probeRead = (args) => oras(args, remaining(deadline));
    const checked = exported(
      probeMetadata,
      { "org.elohim.export-preflight": assemblyCommit },
      probeRead,
    );
    assert.equal(checked.layers.length, 1);
    assert(
      [
        "application/vnd.oci.image.layer.v1.tar+gzip",
        "application/vnd.docker.image.rootfs.diff.tar.gzip",
      ].includes(checked.layers[0].mediaType),
    );
    const layerPath = join(ctx, "preflight-layer.tgz");
    writeFileSync(layerPath, registryBlob(probeRead, checked.layers[0]));
    const actualProof = run("tar", ["-xOzf", layerPath, "export-proof.txt"], {
      seconds: remaining(deadline),
      capture: true,
    });
    assert.equal(actualProof.toString(), proof);
    remaining(deadline);
    writeFileSync(
      join(output, "registry-preflight.json"),
      JSON.stringify(
        {
          outcome: "passed",
          ...checked.evidence,
          proofSha256: createHash("sha256").update(actualProof).digest("hex"),
        },
        null,
        2,
      ) + "\n",
    );

    if (inputs.toolsBase) {
      const base = inputs.toolsBase;
      const baseCheck = join(ctx, "Dockerfile.base-check");
      writeFileSync(
        baseCheck,
        `FROM ${base}\nUSER 0:0\nRUN bash --version && node --version && /usr/local/bin/mongod --version\nRUN set -eu; for compiler in gcc g++; do $compiler -Q --help=target | grep -E '^[[:space:]]+-march=[[:space:]]+x86-64(-v2)?[[:space:]]*$'; done\n`,
      );
      // No output: successful compiler checks stay in BuildKit's own cache.
      packagingPhase = "base-check";
      build(ctx, "Dockerfile.base-check", [], 1200);
      const toolsTag = `${WORKER_REPOSITORY}:tools-rebuild-${assemblyCommit}`;
      const metadata = join(ctx, "tools-metadata.json");
      packagingPhase = "tools-export";
      run(
        "bash",
        [
          join(root, "scripts/ci/build-compute-worker.sh"),
          "--source",
          base,
          toolsTag,
          inputs.sourceCommit,
        ],
        {
          seconds: 4800,
          env: {
            ...process.env,
            BUILDKIT_HOST: addr,
            COMPUTE_WORKER_BUILDER: "buildctl",
            COMPUTE_WORKER_METADATA_FILE: metadata,
          },
        },
      );
      packagingPhase = "tools-verify";
      inputs.tools = exported(metadata, {
        "org.opencontainers.image.revision": inputs.sourceCommit,
      }).image;
      delete inputs.toolsBase;
      inputs.base = base;
    } else {
      packagingPhase = "tools-verify";
      registryImage(oras, inputs.tools.split("@")[1], {
        "org.opencontainers.image.revision": inputs.sourceCommit,
      });
    }
    packagingPhase = "happ";
    // Embedded checkout and runtime checks still execute in Dockerfile.mesh.
    run(
      "oras",
      [
        "pull",
        inputs.happArtifact,
        "--output",
        ctx,
        "--registry-config",
        registryConfig,
      ],
      { seconds: 120 },
    );
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
    const metadata = join(ctx, "image-metadata.json");
    const args = [
      "--opt",
      `label:org.elohim.assembly.revision=${assemblyCommit}`,
    ];
    for (const [name, key] of Object.entries({
      COMPUTE_TOOLS_IMAGE: "tools",
      STORAGE_IMAGE: "storage",
      DOORWAY_IMAGE: "doorway",
      RELAY_IMAGE: "relay",
      BEACON_IMAGE: "beacon",
    }))
      args.push("--opt", `build-arg:${name}=${inputs[key]}`);
    args.push(
      "--output",
      `type=image,name=${WORKER_REPOSITORY}:package-${assemblyCommit},push=true`,
      "--metadata-file",
      metadata,
    );
    packagingPhase = "final-export";
    build(ctx, "Dockerfile", args, 1200);
    packagingPhase = "final-verify";
    const image = exported(metadata, {
      "org.opencontainers.image.revision": inputs.sourceCommit,
      "org.elohim.assembly.revision": assemblyCommit,
    }).image;
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
    console.error(
      `REFUSED compute worker packaging phase=${packagingPhase}; no success artifact`,
    );
    process.exitCode = 1;
  }
}
