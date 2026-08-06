---
id: "backlog-deprecation-helia-webrtc-native-addon-react-native-subtree"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "@helia/verified-fetch drags a React Native + native-addon subtree into the browser app — prebuild-install (unmaintained), rimraf@3, and the @babel mirror wall"
slug: "deprecation-helia-webrtc-native-addon-react-native-subtree"
written: "2026-07-30"
author: "deprecation-triage"
status: "backlog"
priority: "medium"
deprecation_status: blocked
severity: medium
fingerprints: ["e83cd3f2d7e3", "41d6b28f9cb6"]
relatedNodeIds:
  - "backlog-deprecation-angular19-toolchain-legacy-builder-transitives"
tags: [deprecation, helia, verified-fetch, libp2p, webrtc, node-datachannel, prebuild-install, react-native, babel, elohim-app, supply-chain, mirror-blocked, upstream-blocked]
cites:
  - https://github.com/prebuild/prebuild-install
  - https://github.com/libp2p/js-libp2p/tree/main/packages/transport-webrtc
  - https://github.com/murat-dogan/node-datachannel
  - app/elohim-app/package.json
  - app/elohim-app/src/app/elohim/services/helia-fetch.service.ts
---

## What is deprecated

Two of the eleven packages in the root-workspace install banner (fingerprint
`e83cd3f2d7e3`) enter through a single first-party dependency of the **browser**
Angular app. Verbatim lockfile `deprecated:` fields:

```
prebuild-install@7.1.3
    No longer maintained. Please contact the author of the relevant native
    addon; alternatives are available.

rimraf@3.0.2
    Rimraf versions prior to v4 are no longer supported
```

`prebuild-install` is the interesting one. It is an **install-time** tool that
downloads prebuilt native binaries from a remote host during `pnpm install` — an
unmaintained fetcher on the supply-chain path, which is why this entry carries
`severity: medium` while its siblings sit at `low`. It is not exploitable-as-such,
but "unmaintained code that fetches and unpacks binaries at install time" is a
different risk class than a leaky `glob@7`.

## Usage inventory

One first-party declaration: `app/elohim-app/package.json` →
`"@helia/verified-fetch": "^7.0.3"` (a production `dependencies` entry).

It is **genuinely used**, not stranded — lazily dynamic-imported at
`app/elohim-app/src/app/elohim/services/helia-fetch.service.ts:107`:

```ts
const { verifiedFetch } = await import(/* webpackIgnore: true */ '@helia/verified-fetch');
```

behind the CID-blob read path (`content.service.ts:361-375`, "Helia
verified-fetch (CID blobs only, 5s timeout)" with HTTP fallback). So dropping the
dependency is a capability decision, not cleanup.

Reverse-dep trace over `pnpm-lock.yaml` `snapshots:`:

| Deprecated package | Chain |
|---|---|
| `prebuild-install@7.1.3` | `@helia/verified-fetch@7.0.3` → `@libp2p/webrtc@6.0.11` → `node-datachannel@0.29.0` → `prebuild-install@7.1.3` (also via `helia@6.0.20` → `@helia/verified-fetch`) — **2 chains, no other parent tree-wide** |
| `rimraf@3.0.2` | `@helia/verified-fetch@7.0.3` → `@libp2p/webrtc@6.0.11` → `react-native-webrtc@124.0.7` → `react-native@0.84.1` → `@react-native/community-cli-plugin@0.84.1` → `@react-native/dev-middleware@0.84.1` → `chromium-edge-launcher@0.2.0` → `rimraf@3.0.2` (shared with the Angular unit's `karma@6.4.4`) |

The second chain is the finding worth naming: **an entire React Native 0.84.1
toolchain** — community CLI plugin, dev middleware, Metro-adjacent tooling, an
Edge/Chromium launcher — is resolved into the dependency graph of an Angular
**web** application, because `@libp2p/webrtc` declares `react-native-webrtc` as a
plain `dependencies` entry for its React Native environment support.

**This subtree is also the wall that blocks every wide npm update in the
repository.** `react-native@0.84.1` → `babel-jest` → `@babel/core@^7.29.7`, while
the Nexus npm mirror tops out at `@babel/helpers@7.29.2`; any broad
`pnpm update` re-resolution therefore dies with `ERR_PNPM_NO_MATCHING_VERSION`.
Every other entry in this decomposition inherits that constraint from here.

## Migration path

There is **no version that fixes this** — verified against the registry, not
assumed:

- `@libp2p/webrtc` `latest` = **6.0.14** (installed: 6.0.11). Both `6.0.11` and
  `6.0.14` declare `node-datachannel: ^0.29.0` **and**
  `react-native-webrtc: ^124.0.6` as regular `dependencies`, with
  `peerDependenciesMeta` absent — so neither is optional at any available
  version.
- `node-datachannel` `latest` = **0.32.1** (installed: 0.29.0). Both versions'
  full dependency set is exactly `{"prebuild-install": "^7.1.3"}` — the
  unmaintained fetcher is present in the newest release too.

So the levers, in increasing cost:

1. **Upstream fix (the correct one)**: get `@libp2p/webrtc` to move
   `react-native-webrtc` (and ideally `node-datachannel`) to *optional peer
   dependencies*, so browser and Node consumers stop resolving the React Native
   toolchain. This is a small, well-precedented upstream change (Analog does
   exactly this for its builder peers) and it would simultaneously remove the
   `@babel` mirror wall from this repository's update path.
2. **Local override**: a `pnpm-workspace.yaml` / `pnpm.overrides` entry excluding
   `react-native-webrtc` from `@libp2p/webrtc`'s resolution. Feasible in
   principle, but overrides that *remove* a declared dependency are fragile —
   `@libp2p/webrtc` imports it under a runtime environment check, so a wrong cut
   surfaces as a bundler resolution failure in `app/elohim-app`'s production
   build rather than a clean error.
3. **Drop `@helia/verified-fetch`** and serve CID blobs exclusively over the
   existing HTTP fallback. This is a *protocol* decision (it removes the
   client-side cryptographic verification path), not a dependency-hygiene one —
   route it through the substrate/dataplane owner, not through deprecation
   triage.

## Current decision

**Blocked — upstream-pinned at every available version, and the repository's one
lockfile writer is another agent.**

1. **Upstream-blocked, probed this run.** `@libp2p/webrtc@6.0.14` (`latest`) and
   `node-datachannel@0.32.1` (`latest`) both still carry the offending
   dependencies as non-optional. No bump, override-free, clears
   `prebuild-install@7.1.3`. The remedy is an upstream PR/issue, which is outside
   a background triage run's mandate.
2. **Mirror-blocked for anything wider.** The Nexus npm mirror serves
   **cached artifacts only** — re-probed twice this session:
   `rimraf-6.1.3.tgz`, `tar-7.5.13.tgz`, `uuid-11.1.1.tgz`,
   `@anthropic-ai/sdk-0.80.0.tgz` all `404`, while already-cached
   `@anthropic-ai/sdk-0.39.0.tgz` returns `200`. Combined with the
   `@babel/helpers@7.29.2` ceiling documented above, a wide re-resolution cannot
   even be attempted.
3. **Write-lock**: `pnpm-lock.yaml`, `pnpm-workspace.yaml`, and
   `app/elohim-app/package.json` are owned by concurrent in-flight runs this
   session; this triage was scoped to touch none of them, and did not.

Fingerprint `e83cd3f2d7e3` stays **present with `status: blocked`**. It is a
**shared aggregate banner fingerprint** decomposed across six sibling entries in
`genesis/data/timeline/backlog/`: this entry,
`deprecation-storybook-test-runner-jest-island-retire.md`,
`deprecation-angular19-toolchain-legacy-builder-transitives.md`,
`deprecation-anthropic-agent-sdk-legacy-http-stack-bump.md`,
`deprecation-first-party-glob-v10-declarations-bump.md`, and
`deprecation-uuid-support-window-upgrade-unit.md`.

### Live trajectory

1. **File upstream on `js-libp2p`** (highest leverage, cheapest): ask for
   `react-native-webrtc` → optional peer in `@libp2p/webrtc`. Carry the concrete
   evidence: an Angular web app resolves React Native 0.84.1 +
   `chromium-edge-launcher` + `rimraf@3`, and RN's `babel-jest` →
   `@babel/core@^7.29.7` constraint blocks unrelated dependency updates
   downstream. Do the same on `node-datachannel` for `prebuild-install`
   (its own deprecation notice literally says "contact the author of the relevant
   native addon").
2. **Re-probe on each libp2p minor**: `@libp2p/webrtc` `latest` dependency shape
   is a two-second check (`curl` the packument, read
   `dependencies['react-native-webrtc']` and `peerDependenciesMeta`). The
   deprecation-stasis sweep owns this; the moment either moves to an optional
   peer, this becomes a bounded lockfile-only fix.
3. **Do not** attempt the override (lever 2) as a background action — its failure
   mode is a production-build resolution error in `app/elohim-app`, and the
   verification surface (a full `ng build` plus a live CID-blob read) is
   heavier than the debt.

## Verification

No fix was applied this run; nothing is claimed fixed. Verified:

- **Upstream dependency-shape probes, this session**: `@libp2p/webrtc`
  `dist-tags.latest = 6.0.14`; for both `6.0.11` and `6.0.14`,
  `dependencies["node-datachannel"] = "^0.29.0"` and
  `dependencies["react-native-webrtc"] = "^124.0.6"`, `peerDependenciesMeta =
  None`. `node-datachannel` `dist-tags.latest = 0.32.1`; both `0.29.0` and
  `0.32.1` have `dependencies = {"prebuild-install": "^7.1.3"}`.
- **Reverse-dep trace** over `pnpm-lock.yaml` `snapshots:` — chains in the table
  above; `prebuild-install@7.1.3` resolves to exactly two chains, both
  terminating at `IMPORTER:app/elohim-app (@helia/verified-fetch@^7.0.3)`, with
  no other parent tree-wide.
- **Usage confirmation** (that the dependency is load-bearing, so removal is a
  capability decision): dynamic import at
  `app/elohim-app/src/app/elohim/services/helia-fetch.service.ts:107`, wired into
  the CID-blob read path at
  `app/elohim-app/src/app/elohim/services/content.service.ts:361` and `:375`.
- **Mirror probes, two consecutive passes**: `@anthropic-ai/sdk-0.80.0.tgz`,
  `uuid-11.1.1.tgz`, `uuid-13.0.1.tgz`, `rimraf-6.1.3.tgz`, `tar-7.5.13.tgz` →
  `404`; control `@anthropic-ai/sdk-0.39.0.tgz` → `200`.
- **Files touched this run**: this entry (new), five sibling entries, and one
  `.claude/data/deprecations.jsonl` status transition. No lockfile, no
  `package.json`, no `pnpm install`.


## 2026-08-06 — `41d6b28f9cb6` attached; mirror blocker void

The 2026-08-06 successor banner (`41d6b28f9cb6`, 10 packages) still names
`prebuild-install@7.1.3`, and this entry is its sole owner. Reverse-dep trace over
`pnpm-lock.yaml` `snapshots:`:

```
app/elohim-app  ->  @helia/verified-fetch@7.0.3
                ->  @libp2p/webrtc@6.0.11
                ->  node-datachannel@0.29.0
                ->  prebuild-install@7.1.3
```

Deprecation text: `No longer maintained. Please contact the author of the relevant
native addon; alternatives are available.` — i.e. the notice is addressed to
`node-datachannel`'s maintainers, not to us; there is no first-party lever short
of the `@libp2p/webrtc` subtree itself.

This subtree also contributes the **third** independent root of `rimraf@3.0.2`
(`react-native` -> `@react-native/community-cli-plugin` ->
`@react-native/dev-middleware` -> `chromium-edge-launcher@0.2.0` -> `rimraf@3`),
alongside the storybook and stale-`build-angular@19` roots. Relevant when judging
whether `rimraf@3` can leave the banner: **three** roots must clear, not two.

**Any "mirror-blocked" reasoning in this entry is void** — including the recorded
`@babel/helpers@7.29.2` ceiling that made a wide `pnpm update` die with
`ERR_PNPM_NO_MATCHING_VERSION`. That was a Nexus artifact; commit `ecc65384f`
(2026-07-30) repointed `.npmrc` `registry=` to `https://registry.npmjs.org/`.
Re-probe before inheriting any 404 or version-ceiling claim from this entry.

Not landed this run: `pnpm-lock.yaml` was dirty (hand-patched
`@automerge/automerge` bump owned by a concurrent lane); nothing was touched.
