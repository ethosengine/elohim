---
id: "backlog-deprecation-angular-animations-package-retired"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "@angular/animations@22.1.0 retired upstream — installed only as an auto-resolved optional peer, zero first-party usage"
slug: "deprecation-angular-animations-package-retired"
written: "2026-08-06"
author: "deprecation-triage"
status: "backlog"
priority: "low"
deprecation_status: open
severity: low
fingerprints: ["9649309ff583", "fffa9ac8f4c5", "7f94eaead34b"]
relatedNodeIds:
  - "backlog-deprecation-angular19-toolchain-legacy-builder-transitives"
tags: [deprecation, angular, angular-animations, angular22, optional-peer, auto-install-peers, pnpm, dead-weight]
cites:
  - https://v22.angular.dev/guide/animations
  - pnpm-workspace.yaml
  - .npmrc
  - app/elohim-library/projects/elohim-identity/package.json
---

## What is deprecated

Angular retired the **entire `@angular/animations` package** at v22. Verbatim
`deprecated:` field from `pnpm-lock.yaml` (line 1433):

```
@angular/animations is deprecated. Use `animate.enter` and `animate.leave`
instead. For more information see: https://v22.angular.dev/guide/animations.
```

The install banner surfaces it per-importer, e.g.:

```
.../projects/elohim-identity   |  WARN  deprecated @angular/animations@22.1.0
```

This is **not** a "your code uses a deprecated API" warning. It is a whole-package
retirement notice, and it fires against a package **this repo never asked for**.

## Usage inventory

The inventory is the finding: **the repo has zero relationship to this package
beyond its presence on disk.**

| Probe | Result |
|---|---|
| `@angular/animations` in any workspace `package.json` (deps/devDeps/peerDeps) | **none** — grepped every non-`sophia`, non-`node_modules`, non-`dist` manifest |
| `@angular/animations` in `pnpm-lock.yaml` `importers:` as a `specifier:` | **0 occurrences** |
| `import … from '@angular/animations'` in `app/**` or `doorway/**` `.ts`/`.html` | **none** |
| Any other package in the installed tree declaring it as a dependency | **none** |

**Why it is installed anyway.** It is an *optional* peer of
`@angular/platform-browser@22.1.0`, read directly from the installed manifest at
`node_modules/.pnpm/@angular+platform-browser@22.1.0_…/node_modules/@angular/platform-browser/package.json`:

```json
"peerDependencies":     { "@angular/animations": "22.1.0", "@angular/core": "22.1.0", "@angular/common": "22.1.0" },
"peerDependenciesMeta": { "@angular/animations": { "optional": true } }
```

pnpm's `auto-install-peers` defaults to **true** and the repo's `.npmrc` does not
disable it, so the optional peer gets materialised. That is why the warning is
reported against **six importers** — every workspace that pulls
`@angular/platform-browser`:

`app/elohim-app` · `app/elohim-library` ·
`app/elohim-library/projects/elohim-identity` · `app/imagodei-portal` ·
`app/lamad` · `doorway/doorway-app`

`elohim-identity` is merely the importer the banner happened to name; it declares
only `@angular/common`, `@angular/core`, `@angular/router`, and `rxjs` as peers.
**Do not "fix" `elohim-identity`** — there is nothing there to fix.

## Migration path

The upstream migration (`@angular/animations` → `animate.enter` / `animate.leave`)
is a **no-op for this repo**: there is no first-party animation code to port. The
`animate.enter` / `animate.leave` primitives are template-level features of
`@angular/core` at v22 and need no package.

The only real question is whether to stop materialising the unused optional peer.
pnpm 10.30.3 (`packageManager` pin, root `package.json`) supports declaring it in
`pnpm-workspace.yaml`:

```yaml
ignoredOptionalDependencies:
  - '@angular/animations'
```

That drops the package from the tree and takes the six banner lines with it.
Benefit is small and strictly hygienic — one fewer deprecated package in the
install banner and a marginally smaller `node_modules`. It carries a real,
if low, risk: if any future first-party code (or a third-party Angular library)
starts importing `@angular/animations`, the ignore rule silently starves it. That
argues for doing this **only** alongside a verification that nothing imports it —
which is exactly the inventory above, and which must be re-confirmed at the time
of the change rather than inherited from this entry.

## Current decision

**Open, bounded, and ready to land — serialized behind the `pnpm-lock.yaml`
write-lock. Nothing external gates it.**

No upstream artifact is missing, no major upgrade is required, no source change is
needed. The single obstacle is that landing it means re-resolving the lockfile,
and at triage time `pnpm-lock.yaml` was **dirty** — held by a concurrent in-flight
`@automerge/automerge` bump in `elohim/sdk/storage-client-ts`, applied as a
hand-patch. Running `pnpm install` would have normalised that lane's hand-patch
away. Per the "no half-applied migration" rule this run touched **no** manifest,
**no** lockfile, and ran **no** install.

Recipe for whoever holds the lockfile next (one commit):

1. Re-confirm the inventory above still reads zero (the four probes).
2. Add the `ignoredOptionalDependencies` block to `pnpm-workspace.yaml`.
3. `pnpm install` and confirm `@angular/animations` is gone from `pnpm-lock.yaml`
   and from the install banner.
4. Gate the Angular surfaces that resolve `platform-browser` — at minimum
   `app/elohim-app` (`pnpm run build` via a direct `ng build`, since in-container
   `tsc`/JIT misses `strictTemplates` AOT errors) plus `pnpm test`.

**Legitimate alternative: decline.** This is inert dead weight, not a security or
correctness defect. Accepting one banner line and leaving `auto-install-peers` at
its default is a defensible call — in which case this entry should be deleted
rather than carried, since a decision not to act is not a backlog item. The
deprecation-stasis sweep owns that call.

### Fingerprint note — two of the three are self-captures

`9649309ff583` is the genuine `pnpm install` banner line. `fffa9ac8f4c5` and
`7f94eaead34b` were both minted **by this triage run itself**, from `grep`/`awk`
passes over `pnpm-lock.yaml` that echoed the lockfile's own `deprecated:` field
into command output. They are the same concern, not three concerns. This is the
known sentinel over-capture surface tracked in
`deprecation-sentinel-redundant-capture-surfaces.md`: reading a lockfile is enough
to mint a "new" deprecation. All three fingerprints stay **present** with
`status: triaged` pointing here — deleting them would guarantee the next agent who
greps `pnpm-lock.yaml` re-mints them as NEW and burns another dispatch.

## Verification

No fix was applied; nothing is claimed fixed. Verified this run:

- **Deprecation text** quoted verbatim from `pnpm-lock.yaml:1433`.
- **Zero first-party declarations**: `grep -rn "@angular/animations" --include=package.json`
  across the repo (excluding `sophia`, `node_modules`, `dist`) → no matches.
- **Zero importer specifiers**: the `importers:` section of `pnpm-lock.yaml`
  (everything above `packages:`) contains 0 `specifier:` lines for the package.
- **Zero source imports**: `grep -rn "@angular/animations"` over `app/` and
  `doorway/` `.ts`/`.html` → no matches.
- **Optional-peer mechanism** read from the installed
  `@angular/platform-browser@22.1.0` manifest (`peerDependenciesMeta.optional: true`),
  not inferred from release notes.
- **Toolchain**: `pnpm 10.30.3`, `node v24.18.1` — `ignoredOptionalDependencies` is
  supported at this pnpm major.
- **Files touched this run**: this entry (new) and three
  `.claude/data/deprecations.jsonl` status transitions. No lockfile, no
  `package.json`, no `pnpm-workspace.yaml`, no `pnpm install`.
