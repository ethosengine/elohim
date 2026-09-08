/** Built-in Angular adapter for the existing browser + AngularRenderer runtime. */
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../../..");

function layout({ appDir }) {
  const workspace = JSON.parse(
    readFileSync(join(appDir, "angular.json"), "utf8"),
  );
  const apps = Object.entries(workspace.projects ?? {}).filter(
    ([, project]) => project.projectType === "application",
  );
  if (apps.length !== 1)
    throw new Error("Expected exactly one Angular application in angular.json");
  const [name, project] = apps[0];
  const options = (project.architect ?? project.targets)?.build?.options;
  const output = options?.outputPath;
  const base = resolve(
    appDir,
    typeof output === "string" ? output : (output?.base ?? `dist/${name}`),
  );
  const browser = join(
    base,
    typeof output === "object" ? (output.browser ?? "browser") : "browser",
  );
  const bundles = [
    {
      kind: "browser",
      name,
      dist: browser,
      aliases: { "index.html": "index.csr.html" },
    },
  ];
  if (options.server || options.ssr)
    bundles.push({
      kind: "server",
      name,
      dist: join(
        base,
        typeof output === "object" ? (output.server ?? "server") : "server",
      ),
      version: join(browser, "version.json"),
    });
  return bundles;
}

function build(context) {
  const { appDir } = context;
  const commit = execFileSync("git", ["rev-parse", "HEAD"], {
    cwd: appDir,
    encoding: "utf8",
  }).trim();
  const packageFile = join(appDir, "package.json");
  if (existsSync(packageFile)) {
    const manifest = JSON.parse(readFileSync(packageFile, "utf8"));
    const dependencies = {
      ...manifest.dependencies,
      ...manifest.devDependencies,
    };
    if (
      Object.values(dependencies).some((version) =>
        version.startsWith("workspace:"),
      )
    ) {
      if (!manifest.name)
        throw new Error("Workspace app package.json needs a name");
      // pnpm's declared dependency graph owns ordering and transitive build inputs.
      execFileSync(
        "pnpm",
        [
          "-r",
          "--filter",
          `${manifest.name}^...`,
          "--if-present",
          "run",
          "build",
        ],
        { cwd: appDir, stdio: "inherit" },
      );
    }
  }
  execFileSync("pnpm", ["run", "build"], { cwd: appDir, stdio: "inherit" });
  const after = execFileSync("git", ["rev-parse", "HEAD"], {
    cwd: appDir,
    encoding: "utf8",
  }).trim();
  if (commit !== after)
    throw new Error(
      "Checkout changed during build; rebuild from a stable checkout",
    );
  const dirty = !!execFileSync(
    "git",
    ["status", "--porcelain", "--untracked-files=no"],
    {
      cwd: appDir,
      encoding: "utf8",
    },
  ).trim();
  const bundles = layout(context);
  const stamp =
    JSON.stringify(
      {
        commit,
        dirty,
        buildTime: new Date().toISOString(),
        service: bundles[0].name,
        environment: "local",
      },
      null,
      2,
    ) + "\n";
  for (const bundle of bundles)
    writeFileSync(join(bundle.dist, "version.json"), stamp);
}

function check({ archive, bundle, appDir, target }) {
  if (target && target !== "doorway-angular")
    throw new Error(
      "Angular adapter supports the doorway-angular runtime target",
    );
  if (!["browser", "server"].includes(bundle.kind))
    throw new Error(`Unknown Angular bundle kind: ${bundle.kind}`);
  if (
    bundle.kind === "server" &&
    appDir &&
    existsSync(join(appDir, "angular.json"))
  ) {
    execFileSync(
      process.execPath,
      [join(root, "app/scripts/lint-ssr-entry.mjs"), appDir],
      {
        stdio: "inherit",
      },
    );
  }
  const args = [
    join(here, "package-angular-check.py"),
    "--archive",
    archive,
    "--kind",
    bundle.kind,
  ];
  if (bundle.version) args.push("--version", bundle.version);
  execFileSync("python3", args, { encoding: "utf8" });
  return true;
}

function runtimeCheck({
  packages,
  doorway,
  slug,
  mount = "/",
  ssrPath,
  ssrHeading = "",
}) {
  if (!doorway || !slug)
    throw new Error(
      "Angular runtime check needs the published doorway and slug",
    );
  if (
    packages.some((bundle) => bundle.kind === "server") &&
    (typeof ssrPath !== "string" ||
      !ssrPath.startsWith("/") ||
      ssrPath.startsWith("//"))
  )
    throw new Error(
      "Angular SSR runtime check needs an explicit local ssrPath for a renderable route",
    );
  for (const bundle of packages) {
    const script =
      bundle.kind === "browser"
        ? "verify-served-shell.sh"
        : "verify-projected-head.sh";
    const args =
      bundle.kind === "browser"
        ? [doorway, mount, slug, bundle.hash]
        : [doorway, slug, bundle.hash, "", ssrPath, ssrHeading];
    execFileSync("bash", [join(root, "scripts/ci", script), ...args], {
      stdio: "inherit",
    });
  }
  return true;
}

export default {
  id: "angular",
  build,
  layout,
  check,
  runtimeCheck,
  aliases: (kind) =>
    kind === "browser" ? { "index.html": "index.csr.html" } : {},
};
