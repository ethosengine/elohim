# Local EPR-app packages and adapters

Build and package the current Angular app before staging bytes or publishing a head:

```sh
just dev package app/lamad
# SDK CLI with an explicit destination:
node elohim/sdk/scripts/package-app.mjs app/lamad --build --out /tmp/lamad-package
# Check existing, stamped outputs without rebuilding:
node elohim/sdk/scripts/package-app.mjs app/lamad --out /tmp/lamad-package
```

Packaging needs Node and Python 3.9+. It does not publish, contact a doorway, or mint an app head.
`--build` executes the selected adapter's build hook; that build can fetch its normal dependencies.
`--help` lists the CLI options. Both the command and the Node API use the same archive/check operation
that `scripts/ci/stage-spa-blob.sh` uses before upload.

## The common package engine

`package-app.mjs` owns orchestration; `package-app.py` owns archive creation. Neither knows whether an
artifact is browser JavaScript, an SSR server, or a different executable. The engine requires a
nonempty artifact list with distinct safe names, rejects symlinks and unsafe paths, and refuses an
output archive inside the source distribution. It rejects its own output filename in the input; the Angular adapter additionally rejects leftover web packaging archives.

Archives contain sorted paths, fixed timestamps, and normalized permissions: executable files keep
0755, other files use 0644. Changing source mtimes does not change the hash. Each adapter checks the
**exact archive**, and the engine refuses a checker that modifies it. Only after checks pass does an
atomic rename expose the destination archive. Staging uploads those same checked bytes on every
retry; the source dist is not changed by packaging.

Results name the archive path, byte count, existing `sha256-…` storage key, adapter identity, and
`runtimeVerified: false`. This storage key is not advertised as a CID. These local artifacts add no
notarized schema, DHT entry, head, hosting agreement, or per-visitor peer work.

## Angular adapter

`package-angular.mjs` owns Angular build/layout/runtime behavior. `package-angular-check.py` owns its
archive checks, using Python's HTML parser for the shell. The supported runtime target is
`doorway-angular` (the default); passing another target is refused.

The adapter reads the single application's default `build.options.outputPath` from `angular.json`.
With `--build`, it builds declared workspace dependencies in pnpm's dependency order, then runs the
app's `pnpm run build`. Only after success does it write the same build stamp to browser/server
outputs. The stamp names the checkout commit and whether the checkout was dirty. This prepares local
workspace dependencies such as `elohim-core/register`; it does not hard-code an app-specific build
list. Build-hook failure never creates a new stamp.

Without `--build`, outputs must already be stamped. The browser requires `version.json` with a
nonempty commit, `index.html` (or `index.csr.html`, aliased inside the archive), an executable script,
and every named local script/stylesheet. HTML base paths are respected; external asset hosts are
refused under the single-doorway delivery contract.

An SSR artifact requires its own matching build stamp and `main.server.mjs` exporting `default` and
`renderApplication`. Source conformance reuses `app/scripts/lint-ssr-entry.mjs`. Compiled-export checks
are textual, including regex limitations: they do not prove callable exports. Packaging never copies
a fresh browser stamp over an unstamped or mismatched server build.

A local Angular package does not prove application boot, hydration, rendering, routing, peer
propagation, or doorway caching. The separate Angular `runtimeCheck` calls the existing
`verify-served-shell.sh` and `verify-projected-head.sh` against a named published doorway/slug and the
packages' exact hashes. Browser `mount` and SSR `ssrPath` are separate: `/lamad` boots the
browser app, while `/lamad/concept/elohim-host-landing` is a declared SSR route for an existing
published resource. SSR packages require an explicit `ssrPath`; it is never inferred from the slug.
For example, after publishing the packages:

```js
verifyPackageRuntime({
  packages,
  doorway: "http://localhost:8888",
  slug: "lamad-spa",
  mount: "/lamad",
  ssrPath: "/lamad/concept/elohim-host-landing",
});
```

Household Act I and deployed Act II remain the delivery proof. Peers witness
the existing Content snapshot; doorways render and cache locally to absorb visitor traffic.

## Bring your own adapter

Select an explicit local module. No package registry discovery or download runs:

```sh
node elohim/sdk/scripts/package-app.mjs /path/to/app --adapter ./my-adapter.mjs \
  --target linux-x64 --build
EPR_APP_ADAPTER=./my-adapter.mjs just dev package /path/to/app
```

`--target` is an adapter-defined string, not a protocol-wide enum. The module must default-export an
object with an `id` and four synchronous functions: `build`, `layout`, `check`, `runtimeCheck`. Missing
hooks, empty/duplicate artifact lists, unsafe names, and a checker that does not explicitly return
`true` all fail. A Promise is refused; use synchronous local build/probe commands in this first API.

This skeleton shows the boundary. Its compiler, compatibility checker, and runtime probe are supplied
by **your app**; it does not claim those runtime implementations already exist:

```js
import { execFileSync } from "node:child_process";
import { join } from "node:path";

export default {
  id: "my-app-runtime",
  build({ appDir, target }) {
    execFileSync("node", [join(appDir, "tools/build.mjs"), target], {
      stdio: "inherit",
    });
  },
  layout({ appDir }) {
    return [{ kind: "executable", dist: join(appDir, "dist/runtime") }];
  },
  check({ archive, appDir, target }) {
    // Inspect archived bytes: entry kind, OS/architecture, ABI/capabilities,
    // libraries and resources required by this target. Nonzero means refused.
    execFileSync("node", [
      join(appDir, "tools/check-package.mjs"),
      archive,
      target,
    ]);
    return true;
  },
  runtimeCheck({ packages, appDir, target }) {
    // Extract with recorded executable permissions, launch the packaged entry,
    // and assert readiness on the intended execution host, with bounded time.
    execFileSync("node", [
      join(appDir, "tools/probe-package.mjs"),
      packages[0].path,
      target,
    ]);
    return true;
  },
};
```

The required hook does not make a weak custom checker trustworthy. An adapter owner must define the
actual target compatibility and readiness assertions. Packaging always leaves runtime evidence unset;
readiness is an explicit separate operation:

```js
import {
  loadAdapter,
  packageApp,
  verifyPackageRuntime,
} from "./package-app.mjs";

const adapter = await loadAdapter("/absolute/path/my-adapter.mjs");
const options = { appDir: "/path/to/app", adapter, target: "linux-x64" };
const packages = packageApp({ ...options, build: true, out: "/tmp/package" });
verifyPackageRuntime({ ...options, packages });
```

Before calling the runtime hook, the common engine checks adapter identity and recomputes each
archive's hash. Replaced bytes cannot inherit the original package check. The test suite includes a
runnable **toy executable adapter**: it builds a Node executable, checks the target and entry kind,
extracts the produced archive with its executable mode, and actually launches it to observe readiness.
This proves the extension mechanism, not production native-app support.

## Contract evidence

Run `just gate epr-app-package` or `node --test elohim/sdk/scripts/package-app.test.mjs`. Tests cover
bad/missing assets and stamps, incompatible SSR exports/root selector, pair mismatch, deterministic
bytes, source immutability, exact staging upload, failed builds, dependency build ordering, executable
permissions, BYO selection, incompatible targets, missing hooks, invalid layouts, and changed archives.

Concern answers follow `.claude/epr-meta/{concerns,policies}.yaml`:

| Concern | Answer                                                                                       |
| ------- | -------------------------------------------------------------------------------------------- |
| C0      | answered: local artifacts/checks; no authority mutation                                      |
| C1      | n-a: no head election                                                                        |
| C2      | n-a: no canonical-head update                                                                |
| C3      | n-a: local invocation, no converging runtime                                                 |
| C4      | answered: missing inputs/hooks fail explicitly                                               |
| C5      | answered: package success implies no remote authority; runtime rechecks bytes                |
| C6a     | partial: finite file walk; no total-byte/wall-time cap, adapter probes own their bound       |
| C6b     | answered: deterministic bytes; mtime replay test                                             |
| C7      | partial: archive checks bound; delivery requires explicit runtime proof                      |
| C8      | answered: refusal identifies the failed contract and exits nonzero                           |
| C9      | n-a: no agent/transport identity                                                             |
| C10     | partial: Angular current runtime plus explicit BYO hooks, no universal compatibility promise |
| C11     | n-a: no remote admission in common packaging                                                 |
| C12     | n-a: explicit local module/source/output; no publish                                         |
| C13     | n-a: no trust-tier changes                                                                   |
| C14     | n-a: no residual economic obligation minted                                                  |

The current `seam-registry.schema.json` requires Rust `.rs` source locations and Cargo crate identity;
it cannot represent these Node/Python points. This document records their test bindings without
introducing an invalid registry or broadening the protocol schema.
