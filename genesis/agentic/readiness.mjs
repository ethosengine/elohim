import { execFile, spawn } from "node:child_process";
import { promisify } from "node:util";
import { matchesPalette } from "./palette.mjs";
import { loadObjective } from "./objective.mjs";
import fs from "node:fs";

const execFileP = promisify(execFile);

/**
 * Run `git status --porcelain`; empty output means clean.
 *
 * Submodule-internal working-tree changes are ignored (--ignore-submodules=dirty).
 * The parent's recorded submodule pointer is still tracked normally — only
 * intra-submodule diff is filtered, since e.g. brit's gitoxide working tree
 * is not the parent shift's concern.
 */
export async function checkGitClean({ cwd, observerPaths = [] }) {
  try {
    const { stdout } = await execFileP(
      "git",
      ["status", "--porcelain", "--ignore-submodules=dirty"],
      { cwd, env: process.env },
    );
    const rows = stdout.split("\n").filter(Boolean);
    // Only known unstaged generated observer state may coexist with clean source.
    // Staged state remains an error: a later git commit would publish it.
    const sourceRows = rows.filter(
      (row) => !row.startsWith(" M ") || !observerPaths.includes(row.slice(3)),
    );
    if (sourceRows.length > 0) {
      return {
        ok: false,
        reason: `git has untracked, staged, or uncommitted source changes:\n${sourceRows.join("\n")}`,
      };
    }
    return {
      ok: true,
      retainedObserverChanges: rows.filter((row) => !sourceRows.includes(row)),
    };
  } catch (err) {
    return { ok: false, reason: `git status failed: ${err.message}` };
  }
}

/**
 * Run the Objective's measure command; parse stdout as a number.
 * Returns the baseline for iteration 1's delta tracking.
 */
export async function checkMeasureRuns({
  cmd,
  cwd = process.cwd(),
  deadline = Date.now() + 60_000,
}) {
  try {
    const remaining = deadline - Date.now();
    if (!Number.isFinite(remaining) || remaining <= 0)
      throw new Error("measurement deadline expired");
    const stdout = await new Promise((resolve, reject) => {
      const child = spawn("sh", ["-c", cmd], {
        cwd,
        env: process.env,
        detached: true,
      });
      let output = "";
      let stderr = "";
      let failure;
      const kill = (reason) => {
        failure = reason;
        try {
          process.kill(-child.pid, "SIGKILL");
        } catch {
          /* already reaped */
        }
      };
      const timer = setTimeout(
        () => kill("measurement deadline expired"),
        remaining,
      );
      child.stdout.on("data", (data) => {
        output += data;
        if (output.length > 1024 * 1024)
          kill("measurement output exceeds 1 MiB");
      });
      child.stderr.on("data", (data) => {
        stderr = (stderr + data).slice(-4096);
      });
      child.on("error", (error) => {
        clearTimeout(timer);
        reject(error);
      });
      child.on("close", (code) => {
        clearTimeout(timer);
        if (failure || code !== 0)
          reject(new Error(failure || `exit ${code}: ${stderr}`));
        else resolve(output);
      });
    });
    const value = Number(stdout.trim());
    if (!stdout.trim() || !Number.isFinite(value)) {
      return {
        ok: false,
        reason: `measure command did not return a numeric value (got: ${JSON.stringify(stdout.trim().slice(0, 120))})`,
      };
    }
    return { ok: true, baseline: value };
  } catch (err) {
    return { ok: false, reason: `measure command failed: ${err.message}` };
  }
}

/**
 * Compare planned commands against the palette. Returns the set that would
 * trigger permission prompts.
 */
export function checkPaletteGaps({ palette, planned }) {
  const missing = [];
  for (const cmd of planned) {
    if (!matchesPalette(cmd, palette)) missing.push(cmd);
  }
  return { ok: missing.length === 0, missing };
}

/**
 * CLI entry point. Reads an Objective YAML, runs all applicable checks,
 * emits a structured JSON readiness report to stdout. Exit 0 if ready,
 * 1 if any check failed.
 */
export async function runReadiness({
  objectivePath,
  cwd = process.cwd(),
  deadline,
  observerPaths = [],
}) {
  const obj = loadObjective(objectivePath);
  const reports = {};

  reports.measure = await checkMeasureRuns({
    cmd: obj.measure.run,
    cwd,
    deadline,
  });
  reports.git = await checkGitClean({ cwd, observerPaths });

  const ok = Object.values(reports).every((r) => r.ok);
  const out = {
    ready: ok,
    checks: reports,
    baseline: reports.measure.ok ? reports.measure.baseline : null,
  };
  process.stdout.write(JSON.stringify(out, null, 2) + "\n");
  process.exit(ok ? 0 : 1);
}

// Invoked directly?
if (import.meta.url === `file://${process.argv[1]}`) {
  const idx = process.argv.indexOf("--objective");
  const objectivePath = idx >= 0 ? process.argv[idx + 1] : null;
  if (!objectivePath) {
    console.error(
      "Usage: node genesis/agentic/readiness.mjs --objective <path>",
    );
    process.exit(2);
  }
  const arg = (name) => process.argv[process.argv.indexOf(name) + 1];
  const cwd = process.argv.includes("--worktree")
    ? arg("--worktree")
    : process.cwd();
  const deadline = process.argv.includes("--deadline")
    ? Date.parse(arg("--deadline"))
    : undefined;
  const observerPaths = process.argv.includes("--preserve-observers")
    ? JSON.parse(
        fs.readFileSync(
          new URL("./shift-observer-paths.json", import.meta.url),
          "utf8",
        ),
      )
    : [];
  runReadiness({ objectivePath, cwd, deadline, observerPaths }).catch((e) => {
    console.error(e);
    process.exit(2);
  });
}
