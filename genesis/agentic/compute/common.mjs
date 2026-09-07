import { createHash } from "node:crypto";
import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { spawn } from "node:child_process";

export const key = (value) => createHash("sha256").update(value).digest("hex");
export const readJson = async (path) =>
  JSON.parse(await readFile(path, "utf8"));
export async function save(path, value) {
  await mkdir(dirname(path), { recursive: true, mode: 0o700 });
  const temporary = `${path}.${process.pid}.tmp`;
  await writeFile(temporary, JSON.stringify(value, null, 2) + "\n", {
    mode: 0o600,
  });
  await rename(temporary, path);
}

export function api(
  base,
  performer,
  fetcher = fetch,
  localToken = process.env.ELOHIM_COMPUTE_LOCAL_TOKEN || "",
) {
  const origin = new URL(base);
  if (
    origin.protocol !== "http:" ||
    !["localhost", "127.0.0.1", "[::1]"].includes(origin.hostname)
  ) {
    throw new Error(
      "Compute adapter must use its own loopback storage; peers communicate through P2P",
    );
  }
  return async (path, body) => {
    const response = await fetcher(new URL(path, origin), {
      method: body === undefined ? "GET" : "POST",
      headers: {
        "content-type": "application/json",
        "x-elohim-verified-performer": performer,
        "x-elohim-compute-token": localToken,
      },
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: AbortSignal.timeout(30_000),
      redirect: "error",
    });
    if (!response.ok)
      throw new Error(`Compute API ${response.status} for ${path}`);
    return response.json();
  };
}

export function run(
  program,
  args,
  {
    input,
    env = process.env,
    cwd,
    timeoutMs = 120000,
    maxBytes = 2097152,
  } = {},
) {
  return new Promise((resolve, reject) => {
    const child = spawn(program, args, {
      env,
      cwd,
      detached: true,
      stdio: ["pipe", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    let failure;
    const stop = (reason) => {
      failure = new Error(reason);
      try {
        process.kill(-child.pid, "SIGKILL");
      } catch {
        child.kill("SIGKILL");
      }
    };
    const timer = setTimeout(() => stop(`${program} timed out`), timeoutMs);
    child.stdout.on("data", (value) => {
      stdout += value;
      if (Buffer.byteLength(stdout) > maxBytes)
        stop(`${program} output exceeded limit`);
    });
    child.stderr.on("data", (value) => {
      stderr = (stderr + value).slice(-32768);
    });
    child.on("error", (error) => {
      clearTimeout(timer);
      reject(error);
    });
    child.on("close", (code) => {
      clearTimeout(timer);
      if (failure) {
        reject(failure);
        return;
      }
      code === 0
        ? resolve(stdout)
        : reject(new Error(`${program} exited ${code}: ${stderr}`));
    });
    child.stdin.end(input);
  });
}

export const taskPath = (root, ref) => join(root, "inbox", `${key(ref)}.json`);
export const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
