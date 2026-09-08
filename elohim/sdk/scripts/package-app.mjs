#!/usr/bin/env node
/** Local package engine with explicit framework adapters. Packaging never publishes. */
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  renameSync,
  rmSync,
} from "node:fs";
import {
  basename,
  dirname,
  isAbsolute,
  join,
  relative,
  resolve,
} from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import angular from "./package-angular.mjs";

const here = dirname(fileURLToPath(import.meta.url));

/**
 * Adapter hooks are synchronous and explicitly return true from check/runtimeCheck.
 * @typedef {object} PackageAdapter
 * @property {string} id Local adapter identity; no protocol authority.
 * @property {function(object): void} build Run the real build and record its context.
 * @property {function(object): object[]} layout Nonempty [{kind, dist, aliases?, ...adapterContext}].
 * @property {function(object): boolean} check Inspect the exact archive; throw on refusal.
 * @property {function(object): boolean} runtimeCheck Execute the target runtime's readiness proof.
 */
export function validateAdapter(adapter) {
  if (!adapter || typeof adapter.id !== "string" || !adapter.id.trim())
    throw new Error("Adapter needs an id");
  for (const hook of ["build", "layout", "check", "runtimeCheck"]) {
    if (typeof adapter[hook] !== "function")
      throw new Error(`Adapter ${adapter.id} needs ${hook} hook`);
  }
  return adapter;
}

function syncResult(value, hook) {
  if (value && typeof value.then === "function")
    throw new Error(`${hook} must be synchronous`);
  return value;
}

/** Explicit local module only: no registry lookup, download or automatic discovery. */
export async function loadAdapter(path) {
  return path
    ? validateAdapter((await import(pathToFileURL(resolve(path)).href)).default)
    : angular;
}

function archiveHash(path) {
  return (
    "sha256-" + createHash("sha256").update(readFileSync(path)).digest("hex")
  );
}

/** Archive once, check those exact bytes, then atomically expose the checked artifact. */
export function packageDistribution({
  dist,
  kind = "browser",
  out,
  version,
  appDir,
  adapter = angular,
  ...options
}) {
  validateAdapter(adapter);
  if (typeof kind !== "string" || !/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(kind))
    throw new Error("Unsafe artifact kind");
  dist = resolve(dist);
  out = resolve(out);
  const inside = relative(dist, out);
  if (
    !inside ||
    (!inside.startsWith(`..${process.platform === "win32" ? "\\" : "/"}`) &&
      inside !== ".." &&
      !isAbsolute(inside))
  )
    throw new Error("archive output must be outside source dist");
  mkdirSync(dirname(out), { recursive: true });
  const temporary = mkdtempSync(join(dirname(out), ".package-"));
  const archive = join(temporary, "artifact.zip");
  try {
    const aliases = options.aliases ?? adapter.aliases?.(kind) ?? {};
    const args = [
      join(here, "package-app.py"),
      "--dist",
      dist,
      "--out",
      archive,
      "--aliases",
      JSON.stringify(aliases),
      "--reject-name",
      basename(out),
    ];
    const packed = JSON.parse(
      execFileSync("python3", args, { encoding: "utf8" }),
    );
    const bundle = { ...options, dist, kind, version };
    if (
      syncResult(
        adapter.check({ archive, bundle, appDir, target: options.target }),
        "check",
      ) !== true
    )
      throw new Error(`Adapter ${adapter.id} did not affirm package checks`);
    const checkedHash =
      "sha256-" +
      createHash("sha256").update(readFileSync(archive)).digest("hex");
    if (checkedHash !== packed.hash)
      throw new Error("Adapter modified the archive while checking it");
    renameSync(archive, out);
    return {
      ...packed,
      path: out,
      kind,
      adapter: adapter.id,
      runtimeVerified: false,
    };
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
}

/** Default Angular adapter or caller-supplied adapter object; no runtime proof implied. */
export function packageApp({
  appDir,
  out,
  build = false,
  adapter = angular,
  target,
}) {
  validateAdapter(adapter);
  appDir = resolve(appDir);
  const context = {
    appDir,
    target,
    out: resolve(out ?? join(appDir, "dist/packages")),
  };
  if (build) syncResult(adapter.build(context), "build");
  const bundles = syncResult(adapter.layout(context), "layout");
  if (!Array.isArray(bundles) || bundles.length === 0)
    throw new Error("Adapter layout must name at least one artifact");
  const names = new Set();
  for (const bundle of bundles) {
    if (
      !bundle ||
      typeof bundle.dist !== "string" ||
      !bundle.dist ||
      typeof bundle.kind !== "string" ||
      !/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(bundle.kind)
    )
      throw new Error("Adapter layout has an invalid artifact path or kind");
    if (names.has(bundle.kind))
      throw new Error(`Duplicate artifact kind: ${bundle.kind}`);
    names.add(bundle.kind);
  }
  return bundles.map((bundle) =>
    packageDistribution({
      ...bundle,
      dist: resolve(appDir, bundle.dist),
      out: join(context.out, `${bundle.kind}.zip`),
      appDir,
      adapter,
      target,
    }),
  );
}

/** Separate operation: invoke the adapter's actual runtime proof, never inferred from packaging. */
export function verifyPackageRuntime({ adapter = angular, ...context }) {
  validateAdapter(adapter);
  if (!Array.isArray(context.packages) || context.packages.length === 0)
    throw new Error("Runtime check needs packages");
  for (const bundle of context.packages) {
    if (bundle.adapter !== adapter.id)
      throw new Error("Runtime adapter differs from package adapter");
    if (archiveHash(bundle.path) !== bundle.hash)
      throw new Error("Package bytes changed before runtime check");
  }
  if (syncResult(adapter.runtimeCheck(context), "runtimeCheck") !== true)
    throw new Error(`Adapter ${adapter.id} did not affirm runtime readiness`);
  return true;
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const args = process.argv.slice(2);
  if (args.includes("--help")) {
    console.log(
      "Usage: package-app.mjs <app-directory> [--out <directory>] [--build] [--adapter <local-module.mjs>] [--target <adapter-target>]\nDefault adapter: Angular. EPR_APP_ADAPTER also selects an explicit local adapter.\nPackage checks never imply runtime readiness or publish anything.",
    );
    process.exit(0);
  }
  const option = (name) => {
    const i = args.indexOf(name);
    return i < 0 ? undefined : args[i + 1];
  };
  try {
    const adapter = await loadAdapter(
      option("--adapter") ?? process.env.EPR_APP_ADAPTER,
    );
    let bundles;
    if (option("--dist")) {
      const kind = option("--kind") ?? "browser";
      bundles = [
        packageDistribution({
          dist: option("--dist"),
          kind,
          version: option("--version"),
          appDir: option("--app-dir"),
          target: option("--target"),
          out: join(resolve(option("--out") ?? "."), `${kind}.zip`),
          adapter,
        }),
      ];
    } else {
      if (!args[0] || args[0].startsWith("--"))
        throw new Error(
          "Usage: package-app.mjs <app-directory> [--build] [--adapter <local-module.mjs>]",
        );
      bundles = packageApp({
        appDir: args[0],
        out: option("--out"),
        build: args.includes("--build"),
        adapter,
        target: option("--target"),
      });
    }
    for (const bundle of bundles)
      console.log(
        `${bundle.kind}: ${bundle.path}\n${bundle.hash} (${bundle.size} bytes); package checks passed; runtime not verified`,
      );
  } catch (error) {
    console.error(`Package refused: ${error.message}`);
    if (
      /version.json|build output directory missing|build stamp/i.test(
        error.message,
      )
    )
      console.error(
        "Run package-app.mjs <app-directory> --build to build and stamp outputs first.",
      );
    process.exitCode = 2;
  }
}
