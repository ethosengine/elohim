// Fleet runtimeConfig and CID are projections of the committed runtime manifest.
import { execFileSync } from "node:child_process";
import {
  accessSync,
  constants,
  statSync,
  readFileSync,
  writeFileSync,
  mkdtempSync,
  rmSync,
  renameSync,
  openSync,
  closeSync,
} from "node:fs";
import { randomUUID } from "node:crypto";
import { tmpdir } from "node:os";
import { resolve, join, isAbsolute } from "node:path";
import { fileURLToPath } from "node:url";

export function requireEpr(binary = process.env.EPR_BIN) {
  try {
    if (!binary || !isAbsolute(binary) || !statSync(binary).isFile()) throw new Error();
    accessSync(binary, constants.X_OK);
  } catch {
    throw new Error("REFUSED EPR_BIN: name an absolute path to the lane's executable epr file");
  }
  return binary;
}

export function renderDeclarations(
  root,
  deployments,
  binary = process.env.EPR_BIN,
) {
  binary = requireEpr(binary);
  const result = structuredClone(deployments);
  const scratch = mkdtempSync(join(tmpdir(), "runtime-render-"));
  const env = {
    ...process.env,
    EPR_PROC_ROOT: join(scratch, "proc"),
    ELOHIM_DEVICE_KEY_FILE: join(scratch, "key"),
    ELOHIM_SESSION_ID: "runtime-render",
    BIN_STORE: join(scratch, "bins"),
  };
  delete env.EPR_SESSION_LAZY;
  try {
    for (const human of result.humans) {
      if (!human.runtimeManifest) continue;
      const args = [
        "device",
        "show",
        "--root",
        scratch,
        "--base",
        resolve(root, human.runtimeManifest.path),
      ];
      const view = JSON.parse(
        execFileSync(binary, [...args, "--json"], {
          cwd: root,
          env,
          encoding: "utf8",
        }),
      );
      const line = execFileSync(
        binary,
        [...args, "--render", "runtime-config"],
        { cwd: root, env, encoding: "utf8" },
      ).trim();
      const match = /^([A-Z_]+) = "(.*)"$/.exec(line);
      if (!match)
        throw new Error(`invalid native runtime projection for ${human.name}`);
      human.runtimeManifest.cid = view.manifestCid;
      human.runtimeConfig = { ...human.runtimeConfig, [match[1]]: match[2] };
    }
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
  return result;
}

// flock locks the inherited open-file description, which remains open here after
// the helper exits. scope.rs uses File::lock on this same persistent inode.
// Never unlink it: the kernel releases ownership on close or process death.
export function updateDeclarations(
  root, file, binary = process.env.EPR_BIN, write = false,
) {
  binary = requireEpr(binary);
  const fd = openSync(`${file}.lock`, "a+", 0o600);
  const temporary = `${file}.${process.pid}.${randomUUID()}.tmp`;
  try {
    // Node has no flock API; util-linux acquires it through descriptor 3, shared
    // with fd in this process. No recursive worker or lock-bypass entry point.
    execFileSync("flock", ["--exclusive", "3"], {
      stdio: ["ignore", "inherit", "inherit", fd],
    });
    const before = JSON.parse(readFileSync(file, "utf8"));
    const after = renderDeclarations(root, before, binary);
    if (write) {
      writeFileSync(temporary, JSON.stringify(after, null, 2) + "\n", {
        flag: "wx",
      });
      renameSync(temporary, file);
    } else if (JSON.stringify(before) !== JSON.stringify(after)) {
      throw new Error(
        "Runtime declaration projection drift; run runtime-declaration-render.mjs --write",
      );
    }
  } finally {
    closeSync(fd);
    rmSync(temporary, { force: true });
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const root = resolve(fileURLToPath(new URL("../../", import.meta.url)));
  const file = join(root, "genesis/orchestrator/data/deployments.json");
  updateDeclarations(root, file, undefined, process.argv.includes("--write"));
}
