import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
  rmSync,
  utimesSync,
  chmodSync,
} from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { test } from "node:test";

import {
  packageApp,
  packageDistribution,
  loadAdapter,
  verifyPackageRuntime,
} from "./package-app.mjs";

function fixture(t) {
  const dir = mkdtempSync(join(tmpdir(), "sdk-app-package-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const browser = join(dir, "dist/demo/browser");
  const server = join(dir, "dist/demo/server");
  mkdirSync(browser, { recursive: true });
  mkdirSync(server, { recursive: true });
  mkdirSync(join(dir, "src/app"), { recursive: true });
  writeFileSync(
    join(dir, "angular.json"),
    JSON.stringify({
      projects: {
        demo: {
          projectType: "application",
          architect: {
            build: {
              options: {
                outputPath: "dist/demo",
                server: "src/main.server.ts",
              },
            },
          },
        },
      },
    }),
  );
  writeFileSync(
    join(dir, "src/main.server.ts"),
    'export default function bootstrap() {}\nexport function renderApplication() { return "<app-root>Ready</app-root>"; }',
  );
  writeFileSync(join(dir, "src/app/app.component.ts"), "selector: 'app-root'");
  writeFileSync(
    join(browser, "index.csr.html"),
    '<base href="/demo/"><app-root></app-root><script src="main.js"></script><link rel="stylesheet" href="styles.css">',
  );
  writeFileSync(
    join(browser, "main.js"),
    'document.querySelector("app-root").textContent="Ready"',
  );
  writeFileSync(join(browser, "styles.css"), "app-root { display:block }");
  writeFileSync(
    join(browser, "version.json"),
    '{"commit":"abc","version":"1"}',
  );
  writeFileSync(join(server, "version.json"), '{"commit":"abc","version":"1"}');
  writeFileSync(
    join(server, "main.server.mjs"),
    'export default function bootstrap() {}\nexport function renderApplication() { return "<app-root>Ready</app-root>"; }',
  );
  return { dir, browser, server, out: join(dir, "packages/browser.zip") };
}

test("pair packages checked bytes without mutating dist; mtime does not change addresses", (t) => {
  const f = fixture(t);
  const before = readFileSync(join(f.browser, "index.csr.html"));
  const pair = packageApp({ appDir: f.dir });
  assert.equal(pair.length, 2);
  const first = readFileSync(pair[0].path);
  utimesSync(join(f.browser, "main.js"), new Date(), new Date());
  const second = packageApp({ appDir: f.dir });
  assert.equal(pair[0].hash, second[0].hash);
  assert.deepEqual(first, readFileSync(second[0].path));
  assert.deepEqual(before, readFileSync(join(f.browser, "index.csr.html")));
  assert.throws(() => readFileSync(join(f.browser, "index.html")), /ENOENT/);
});

for (const [name, change, expected] of [
  [
    "missing asset",
    (f) => rmSync(join(f.browser, "main.js")),
    /asset is missing/,
  ],
  [
    "missing version",
    (f) => rmSync(join(f.browser, "version.json")),
    /version.json/,
  ],
  [
    "missing server stamp",
    (f) => rmSync(join(f.server, "version.json")),
    /SSR build stamp missing/,
  ],
  [
    "stale archive",
    (f) => writeFileSync(join(f.browser, "spa-bundle.zip"), "old"),
    /stale package archive/,
  ],
  [
    "wrong SSR export",
    (f) => writeFileSync(join(f.server, "main.server.mjs"), "export default 1"),
    /export renderApplication/,
  ],
  [
    "mismatched pair",
    (f) => writeFileSync(join(f.server, "version.json"), '{"commit":"old"}'),
    /version.json disagree/,
  ],
  [
    "wrong source root",
    (f) =>
      writeFileSync(
        join(f.dir, "src/app/app.component.ts"),
        "selector: 'different-root'",
      ),
    /Command failed/,
  ],
  [
    "external asset",
    (f) =>
      writeFileSync(
        join(f.browser, "index.csr.html"),
        '<script src="https://cdn.invalid/main.js"></script>',
      ),
    /same doorway/,
  ],
]) {
  test(`package refuses ${name}`, (t) => {
    const f = fixture(t);
    change(f);
    assert.throws(() => packageApp({ appDir: f.dir }), expected);
  });
}

test("SDK API refuses an output archive nested in input dist", (t) => {
  const f = fixture(t);
  assert.throws(
    () =>
      packageDistribution({ dist: f.browser, out: join(f.browser, "new.zip") }),
    /outside source dist/,
  );
});

test("stage script uploads the exact SDK-checked archive and preserves input", (t) => {
  const f = fixture(t);
  const expected = packageDistribution({
    dist: f.server,
    kind: "server",
    out: join(f.dir, "expected.zip"),
  });
  const bin = join(f.dir, "bin");
  mkdirSync(bin);
  const curl = join(bin, "curl");
  writeFileSync(
    curl,
    '#!/bin/bash\nfor value in "$@"; do case "$value" in @*) cat "${value#@}" > "$CAPTURE_UPLOAD";; esac; done\nprintf "{}\\n"\n',
  );
  chmodSync(curl, 0o755);
  const captured = join(f.dir, "uploaded.zip");
  const script = new URL(
    "../../../scripts/ci/stage-spa-blob.sh",
    import.meta.url,
  );
  const output = execFileSync(
    "bash",
    [script.pathname, f.server, "fixture", "http://unused.invalid", "server"],
    {
      encoding: "utf8",
      env: {
        ...process.env,
        EPR_APP_ADAPTER: join(f.dir, "unrelated-local-adapter.mjs"),
        PATH: `${bin}:${process.env.PATH}`,
        CAPTURE_UPLOAD: captured,
        DO_PATCH: "0",
      },
    },
  );
  assert.ok(output.includes(expected.hash));
  assert.deepEqual(readFileSync(captured), readFileSync(expected.path));
  assert.throws(() => readFileSync(join(f.server, "spa-bundle.zip")), /ENOENT/);
});

for (const succeeds of [true, false]) {
  test(`build option stamps only after ${succeeds ? "successful" : "failed"} app build`, (t) => {
    const f = fixture(t);
    rmSync(join(f.browser, "version.json"));
    const bin = join(f.dir, "build-bin");
    mkdirSync(bin);
    writeFileSync(
      join(bin, "git"),
      '#!/bin/bash\ncase "$1" in rev-parse) echo abc123;; status) echo " M src/main.ts";; esac\n',
    );
    writeFileSync(
      join(bin, "pnpm"),
      `#!/bin/bash\ntest "$1 $2" = "run build" || exit 9\nexit ${succeeds ? 0 : 7}\n`,
    );
    chmodSync(join(bin, "git"), 0o755);
    chmodSync(join(bin, "pnpm"), 0o755);
    const previous = process.env.PATH;
    process.env.PATH = `${bin}:${previous}`;
    t.after(() => {
      process.env.PATH = previous;
    });
    if (succeeds) {
      assert.equal(packageApp({ appDir: f.dir, build: true }).length, 2);
      const browser = readFileSync(join(f.browser, "version.json"), "utf8");
      assert.equal(
        browser,
        readFileSync(join(f.server, "version.json"), "utf8"),
      );
      assert.equal(JSON.parse(browser).dirty, true);
      assert.equal(JSON.parse(browser).commit, "abc123");
    } else {
      assert.throws(
        () => packageApp({ appDir: f.dir, build: true }),
        /Command failed/,
      );
      assert.throws(
        () => readFileSync(join(f.browser, "version.json")),
        /ENOENT/,
      );
    }
  });
}

test("SSR distribution requires a build stamp even outside pair API", (t) => {
  const f = fixture(t);
  rmSync(join(f.server, "version.json"));
  assert.throws(
    () => packageDistribution({ dist: f.server, kind: "server", out: f.out }),
    /version.json/,
  );
});

test("stylesheet-only browser shell is refused before publishing", (t) => {
  const f = fixture(t);
  writeFileSync(
    join(f.browser, "index.csr.html"),
    '<link rel="stylesheet" href="styles.css">',
  );
  assert.throws(() => packageApp({ appDir: f.dir }), /no executable script/);
});

test("explicit BYO adapter packages a toy executable and launches the archived bytes", async (t) => {
  const dir = mkdtempSync(join(tmpdir(), "sdk-byo-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const adapterFile = join(dir, "toy-adapter.mjs");
  writeFileSync(
    adapterFile,
    String.raw`
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync, chmodSync, mkdtempSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
export default {
  id: 'toy-executable-example',
  build({appDir}) {
    mkdirSync(join(appDir,'dist/toy'), {recursive:true});
    writeFileSync(join(appDir,'dist/toy/runner'), '#!/usr/bin/env node\nconsole.log("READY from archived executable")\n');
    chmodSync(join(appDir,'dist/toy/runner'), 0o755);
    writeFileSync(join(appDir,'dist/toy/browser.zip'), 'opaque asset; not an Angular package');
  },
  layout({appDir}) { return [{kind:'executable',dist:join(appDir,'dist/toy')}]; },
  check({archive,target}) {
    if (target !== 'toy-node-' + process.platform + '-' + process.arch) throw new Error('incompatible toy runtime target');
    execFileSync('python3',['-c', 'import sys,zipfile; z=zipfile.ZipFile(sys.argv[1]); assert z.read("runner").startswith(b"#!/usr/bin/env node"); assert z.getinfo("runner").external_attr >> 16 & 0o111', archive]);
    return true;
  },
  runtimeCheck({packages,target}) {
    if (target !== 'toy-node-' + process.platform + '-' + process.arch) throw new Error('incompatible toy runtime target');
    const dest = mkdtempSync(join(tmpdir(),'toy-readiness-'));
    try {
      execFileSync('python3',['-c', 'import os,sys,zipfile; z=zipfile.ZipFile(sys.argv[1]); p=z.extract("runner",sys.argv[2]); os.chmod(p,z.getinfo("runner").external_attr >> 16 & 0o777)', packages[0].path,dest]);
      assert.match(execFileSync(join(dest,'runner'),{encoding:'utf8'}), /READY from archived executable/);
      return true;
    } finally { rmSync(dest,{recursive:true,force:true}); }
  }
};
`,
  );
  const adapter = await loadAdapter(adapterFile);
  const target = `toy-node-${process.platform}-${process.arch}`;
  const packages = packageApp({ appDir: dir, build: true, adapter, target });
  assert.equal(packages.length, 1);
  assert.equal(packages[0].kind, "executable");
  assert.equal(packages[0].runtimeVerified, false);
  assert.equal(verifyPackageRuntime({ adapter, packages, target }), true);
  assert.throws(
    () => packageApp({ appDir: dir, adapter, target: "wrong-runtime" }),
    /incompatible/,
  );
  writeFileSync(join(dir, "dist/toy/runner"), "not an executable");
  assert.throws(
    () => packageApp({ appDir: dir, adapter, target }),
    /Command failed/,
  );
  // The CLI selects the same explicit local module and runs its real build hook.
  const cli = new URL("./package-app.mjs", import.meta.url).pathname;
  const output = execFileSync(
    process.execPath,
    [cli, dir, "--adapter", adapterFile, "--target", target, "--build"],
    { encoding: "utf8" },
  );
  assert.match(output, /executable:/);
  assert.match(output, /runtime not verified/);
});

test("adapter cannot omit a contract hook, declare no outputs, or escape its output path", (t) => {
  const f = fixture(t);
  const adapter = {
    id: "fixture",
    build() {},
    layout() {
      return [];
    },
    check() {
      return true;
    },
    runtimeCheck() {
      return true;
    },
  };
  for (const hook of ["build", "layout", "check", "runtimeCheck"]) {
    assert.throws(
      () =>
        packageApp({
          appDir: f.dir,
          adapter: { ...adapter, [hook]: undefined },
        }),
      new RegExp(hook),
    );
  }
  assert.throws(() => packageApp({ appDir: f.dir, adapter }), /at least one/);
  assert.throws(
    () =>
      packageApp({
        appDir: f.dir,
        adapter: {
          ...adapter,
          layout() {
            return [{ kind: "../outside", dist: f.browser }];
          },
        },
      }),
    /invalid artifact/,
  );
  assert.throws(
    () =>
      packageApp({
        appDir: f.dir,
        adapter: {
          ...adapter,
          layout() {
            return [
              { kind: "same", dist: f.browser },
              { kind: "same", dist: f.browser },
            ];
          },
        },
      }),
    /Duplicate artifact/,
  );
});

test("runtime proof refuses changed archive bytes before invoking the adapter", (t) => {
  const f = fixture(t);
  const packages = packageApp({ appDir: f.dir });
  const wrong = {
    id: "other",
    build() {},
    layout() {
      return [];
    },
    check() {
      return true;
    },
    runtimeCheck() {
      throw new Error("must not run");
    },
  };
  assert.throws(
    () => verifyPackageRuntime({ adapter: wrong, packages }),
    /adapter differs/,
  );
  writeFileSync(packages[0].path, "replaced after package check");
  let called = false;
  const adapter = {
    id: "angular",
    build() {},
    layout() {
      return [];
    },
    check() {
      return true;
    },
    runtimeCheck() {
      called = true;
      return true;
    },
  };
  assert.throws(
    () => verifyPackageRuntime({ adapter, packages }),
    /bytes changed/,
  );
  assert.equal(called, false);
});

test("Angular build prepares declared workspace dependencies before app compilation", (t) => {
  const f = fixture(t);
  writeFileSync(
    join(f.dir, "package.json"),
    JSON.stringify({
      name: "demo",
      dependencies: { "my-elements": "workspace:*" },
    }),
  );
  const bin = join(f.dir, "dependency-bin");
  mkdirSync(bin);
  const log = join(f.dir, "build-order");
  writeFileSync(
    join(bin, "git"),
    '#!/bin/bash\ncase "$1" in rev-parse) echo abc123;; esac\n',
  );
  writeFileSync(
    join(bin, "pnpm"),
    '#!/bin/bash\nprintf "%s\\n" "$*" >> "$PACKAGE_BUILD_LOG"\n',
  );
  chmodSync(join(bin, "git"), 0o755);
  chmodSync(join(bin, "pnpm"), 0o755);
  const previousPath = process.env.PATH,
    previousLog = process.env.PACKAGE_BUILD_LOG;
  process.env.PATH = `${bin}:${previousPath}`;
  process.env.PACKAGE_BUILD_LOG = log;
  t.after(() => {
    process.env.PATH = previousPath;
    if (previousLog === undefined) delete process.env.PACKAGE_BUILD_LOG;
    else process.env.PACKAGE_BUILD_LOG = previousLog;
  });
  packageApp({ appDir: f.dir, build: true });
  assert.deepEqual(readFileSync(log, "utf8").trim().split("\n"), [
    "-r --filter demo^... --if-present run build",
    "run build",
  ]);
});
