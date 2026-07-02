---
id: "backlog-deprecation-lit-context-upstream-dts-jsdoc-noise"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "@lit/context upstream .d.ts @deprecated JSDoc — scan-noise, elohim uses the current API"
slug: "deprecation-lit-context-upstream-dts-jsdoc-noise"
written: "2026-07-02"
author: "deprecation-triage"
status: "backlog"
priority: "low"
deprecation_status: blocked
severity: low
fingerprints: ["e12d9a25c749", "e25b0da5e7c5", "42b47260d588"]
relatedNodeIds: []
tags: [deprecation, lit, lit-context, node-modules, scan-noise, upstream]
cites:
  - https://lit.dev/docs/data/context/
  - https://github.com/lit/lit/tree/main/packages/context
  - app/elohim-elements/elohim-core/package.json
  - app/elohim-elements/elohim-core/src/capability/mixin.ts
  - app/elohim-elements/elohim-core/src/elohim-epr-link.spec.ts
  - app/elohim-elements/elohim-core/src/capability/context.ts
  - app/elohim-elements/elohim-core/src/navigation/epr-resolution-provider.ts
  - .claude/hooks/deprecation-sentinel.py
---

## What is deprecated

Three `@deprecated` JSDoc lines emitted from the **upstream `@lit/context@1.1.6`
TypeScript declaration files** under `node_modules/`, captured by the sentinel
when an investigative shell command dumped the package's `.d.ts` internals
(`cd /projects/elohim/node_modules/@lit/context; … find . -name
"context-*.d.ts" | …`). The three lines:

```
# node_modules/@lit/context/development/lib/controllers/context-provider.d.ts:51
/** @deprecated Use new ContextProvider(host, options) */          (fp e12d9a25c749)

# node_modules/@lit/context/development/lib/controllers/context-consumer.d.ts:33
/** @deprecated Use new ContextConsumer(host, options) */          (fp e25b0da5e7c5)

# node_modules/@lit/context/development/lib/create-context.d.ts (ContextKey alias)
* @deprecated use Context instead                                  (fp 42b47260d588)
```

These are **library-internal** deprecations: Lit marked the *positional*
constructor overloads (`new ContextProvider(host, context, initialValue)` /
`new ContextConsumer(host, context, callback, subscribe)`) and the old
`ContextKey` type alias as deprecated in favour of the **options-object**
constructor form and the `Context` type. They are not elohim code and not an
in-flight toolchain warning against elohim code — the sentinel fingerprinted
raw `.d.ts` file content that scrolled past in a `cat`/`find` output (the
capturing command carried no `deprecat` token, so the command-string
`GUARD_TOKENS` did not fire, and no path-prefix appeared on the dumped lines,
so the existing anti-echo guards A–E did not match).

## Usage inventory

`@lit/context` is a direct dependency of exactly one workspace
(`app/elohim-elements/elohim-core/package.json:56` — `"@lit/context": "^1.1.6"`,
resolved to `1.1.6`). Every elohim-source touch uses the **current,
non-deprecated** API — none uses a deprecated positional overload and nothing
references `ContextKey`:

- `app/elohim-elements/elohim-core/src/capability/mixin.ts:44` —
  `new ContextConsumer(this, { context, callback, subscribe })` — modern
  **options-object** constructor form.
- `app/elohim-elements/elohim-core/src/elohim-epr-link.spec.ts:351` —
  `new ContextProvider(host, { … })` — modern options-object form.
- `app/elohim-elements/elohim-core/src/capability/context.ts:7` —
  `createContext(...)` (current factory API).
- `app/elohim-elements/elohim-core/src/navigation/epr-resolution-provider.ts:32` —
  `createContext(...)`.
- `app/elohim-elements/elohim-core/src/capability/mixin.spec.ts:3`,
  `app/elohim-elements/elohim-core/src/capability/context.spec.ts:3` —
  `provide` / `consume` decorators (current API).

Repo-wide grep (excluding `node_modules/`, `.angular/cache`, `dist/`, `vite/deps`)
returns **zero** occurrences of `new ContextProvider(host, context, …)` /
`new ContextConsumer(host, context, …)` positional forms and **zero**
`ContextKey` references. There is no elohim deprecation debt behind these
fingerprints.

## Migration path

None required for elohim. The deprecated overloads still exist in
`@lit/context@1.1.6` for backward compatibility; elohim already writes the
recommended options-object form and the `Context` type throughout. The
upstream JSDoc will only disappear from `node_modules/` when `@lit/context`
ships a release that removes the deprecated overloads — an upstream event
that is out of our control and irrelevant to elohim's correctness (nothing in
the tree would need to change).

## Current decision

**Blocked (terminal for automation) — upstream node_modules scan-noise, no
elohim debt.** The elohim tree already uses only the non-deprecated
`@lit/context` API (verification below), so there is nothing in this repository
to fix. Deleting the ledger lines would be wrong: the `.d.ts` content is stable
across the pinned `@lit/context@1.1.6`, so a future `cat`/`find`/`grep` of the
package internals (without a `deprecat` token in the command) would re-mint
these as NEW fingerprints and needlessly re-dispatch a triage agent. Keeping
the three lines present with `status: blocked` makes the sentinel cite this
decision deterministically on every re-encounter and never re-fire; the
deprecation-stasis sweep owns the (no-op) re-check.

**Live trajectory — structural guard if the class recurs.** The permanent-clean
resolution is a `deprecation-sentinel.py` anti-echo guard for **node_modules
library-internal source reads** (a sixth guard alongside A–E), after which
these three ledger lines and this entry can be deleted outright. Deferred, not
taken now, for two reasons: (1) a *single* occurrence does not yet justify the
over-suppression risk — a naive "command contains `node_modules/`" gate would
wrongly silence real deprecation warnings from tools invoked via
`node_modules/.bin/` (e.g. `vitest`, `ng`), and the captured lines carry no
path-prefix to key a line-level guard on, so a *safe* guard needs careful
design; (2) the hook is a shared safety-critical surface that was already
edited on 2026-07-02 (Guard E). If this node_modules-`.d.ts`-dump class recurs,
promote the guard (keying on the command `cd`-ing into / reading a
`node_modules/**` path while excluding `node_modules/.bin/` tool invocations),
then decompose this entry and its fingerprints.

## Verification

- Scope grep (repo, excluding `node_modules/`, `.angular/`, `dist/`,
  `vite/deps`): `new ContextProvider(` / `new ContextConsumer(` appear only in
  the modern options-object form; `ContextKey` has **zero** source references —
  confirms no deprecated-API usage in elohim code.
- The one non-test direct construction (`mixin.ts:44`) and the one test
  construction (`elohim-epr-link.spec.ts:351`) both pass an options object as
  the second argument (the recommended form the upstream JSDoc points to).
- Dependency footprint: `@lit/context` is declared only in
  `elohim-core/package.json`, resolved to `1.1.6` — a single, current, pinned
  version.

No code change was needed or made; this entry records the disposition so the
deterministic layers answer every re-encounter without another agent dispatch.
