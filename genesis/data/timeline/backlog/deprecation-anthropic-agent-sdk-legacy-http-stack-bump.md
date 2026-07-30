---
id: "backlog-deprecation-anthropic-agent-sdk-legacy-http-stack-bump"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "@anthropic-ai/sdk@0.39.0 → 0.80.0 in agent-sdk — one manifest bump drops the whole node-fetch/formdata-node stack (node-domexception)"
slug: "deprecation-anthropic-agent-sdk-legacy-http-stack-bump"
written: "2026-07-30"
author: "deprecation-triage"
status: "backlog"
priority: "medium"
deprecation_status: blocked
severity: low
fingerprints: ["e83cd3f2d7e3"]
relatedNodeIds:
  - "backlog-deprecation-first-party-glob-v10-declarations-bump"
tags: [deprecation, anthropic-sdk, elohim-agent-sdk, formdata-node, node-domexception, node-fetch, mirror-blocked, bounded-fix]
cites:
  - https://github.com/anthropics/anthropic-sdk-typescript/blob/main/CHANGELOG.md
  - https://github.com/anthropics/anthropic-sdk-typescript/blob/main/MIGRATION.md
  - elohim/elohim-agent/elohim-agent-sdk/package.json
  - elohim/elohim-agent/elohim-agent-sdk/src/invoke.ts
  - elohim/elohim-agent/elohim-agent-sdk/src/server.ts
---

## What is deprecated

One of the eleven packages in the root-workspace install banner (fingerprint
`e83cd3f2d7e3`). Verbatim lockfile `deprecated:` field:

```
node-domexception@1.0.0
    Use your platform's native DOMException instead
```

The message is exactly right, and it names the root cause: `node-domexception`
exists to polyfill a global that Node has shipped natively since v17. Its presence
is a *proxy signal* for a dependency that is pinned to a pre-native-fetch HTTP
stack — which is the actual concern this entry owns.

## Usage inventory

Single carrier chain, no other parent tree-wide:

```
node-domexception@1.0.0
  ← formdata-node@4.4.1
    ← @anthropic-ai/sdk@0.39.0
      ← IMPORTER:elohim/elohim-agent/elohim-agent-sdk (@anthropic-ai/sdk@^0.39.0)
```

First-party declaration: `elohim/elohim-agent/elohim-agent-sdk/package.json:24`
→ `"@anthropic-ai/sdk": "^0.39.0"` (production `dependencies`; the package is
`"type": "module"`, built with `tsc`, run as `node dist/server.js`).

Call sites — three files, one narrow API surface:

| File | Use |
|---|---|
| `elohim/elohim-agent/elohim-agent-sdk/src/server.ts:15,38` | `import Anthropic from '@anthropic-ai/sdk'` · `new Anthropic({ apiKey: API_KEY })` |
| `elohim/elohim-agent/elohim-agent-sdk/src/invoke.ts:10,67,95,106` | default import · `sdk: Anthropic` param type · `await sdk.messages.create({...})` · `Anthropic.TextBlock` type guard on `block.type === 'text'` |
| `elohim/elohim-agent/elohim-agent-sdk/src/invoke.test.ts:2` | default import (mocked) |

No file upload, no streaming helper, no `toFile`/`FormData` use — nothing that
touches the `formdata-node` code path at all. The polyfill is dead weight in this
consumer.

## Migration path

**`^0.39.0` → `^0.80.0` in one manifest line.** The dependency reduction is
dramatic and verified against the registry packument, not assumed:

| Version | Full `dependencies` |
|---|---|
| `0.39.0` (installed) | `node-fetch@^2.6.7`, `@types/node-fetch@^2.6.4`, `@types/node@^18.11.18`, `formdata-node@^4.3.2`, `form-data-encoder@1.7.2`, `agentkeepalive@^4.2.1`, `abort-controller@^3.0.0` |
| `0.80.0` (`latest`) | `json-schema-to-ts@^3.1.1` |

Seven dependencies → one. The modern SDK uses the platform's native `fetch`,
`FormData`, and `AbortController`, so `formdata-node` — and with it
`node-domexception@1.0.0`, plus `node-fetch`, `abort-controller`,
`agentkeepalive`, and a stale bundled `@types/node@^18` — all leave the tree
together. Because this is the sole carrier chain, the banner line disappears
entirely.

API compatibility for this consumer is expected to be clean but is **not yet
proven** and must be checked at implementation time, not assumed: `new
Anthropic({ apiKey })`, `client.messages.create({...})`, and the `TextBlock`
content-block type are the SDK's stable core across the 0.4x–0.8x line. The two
things to verify in the upstream `MIGRATION.md` when the bump is attempted are
(a) whether the default export is still the client class or has moved to a named
`Anthropic` export, and (b) the exact type path for the text content block
(`Anthropic.TextBlock` vs a nested namespace). Both are compile-time failures, so
`pnpm --filter @elohim/agent-sdk build` catches them immediately.

This is the **cheapest complete win** in the whole `e83cd3f2d7e3` decomposition:
one manifest line, one workspace, a `tsc` build plus a three-test Vitest suite as
the verification surface, and the banner line provably clears.

## Current decision

**Blocked — the target artifact is not fetchable from the configured registry,
and the lockfile is write-locked. Not blocked on difficulty: this is a bounded
fix waiting on infrastructure.**

1. **Mirror-blocked, probed twice this session.** Against
   `https://nexus.ethosengine.com/repository/npm/`: the `@anthropic-ai/sdk`
   packument returns `200` (which is how the `0.80.0` dependency set above was
   read), but `…/@anthropic-ai/sdk/-/sdk-0.80.0.tgz` → **HTTP 404** on two
   consecutive passes, while the already-installed
   `…/sdk-0.39.0.tgz` → **`200`**. The mirror serves **cached artifacts only**;
   it advertises metadata for versions it cannot deliver. `pnpm install` would
   therefore fail at tarball fetch no matter how the manifest is edited. Same
   class as the `uuid` entry's blocker #1 and the campaign's "47 of 49
   mirror-blocked" probe table. Clearing it is a Nexus proxy/remote-cache operator
   action, not a repo change.
2. **Write-lock**: `pnpm-lock.yaml` and `pnpm-workspace.yaml` are owned by
   concurrent in-flight runs this session. Editing
   `elohim/elohim-agent/elohim-agent-sdk/package.json` without the matching
   lockfile re-resolution would strand CI on `--frozen-lockfile` — a half-applied
   migration, which is worse than a documented blocker.

Fingerprint `e83cd3f2d7e3` stays **present with `status: blocked`**. It is a
**shared aggregate banner fingerprint** decomposed across six sibling entries in
`genesis/data/timeline/backlog/`: this entry,
`deprecation-storybook-test-runner-jest-island-retire.md`,
`deprecation-angular19-toolchain-legacy-builder-transitives.md`,
`deprecation-helia-webrtc-native-addon-react-native-subtree.md`,
`deprecation-first-party-glob-v10-declarations-bump.md`, and
`deprecation-uuid-support-window-upgrade-unit.md`.

### Live trajectory — smallest unblock in the set

1. **Operator: make the Nexus npm proxy fetch uncached artifacts.** Re-probe:
   `curl -o /dev/null -w "%{http_code}" https://nexus.ethosengine.com/repository/npm/@anthropic-ai/sdk/-/sdk-0.80.0.tgz`
   → a `200` unblocks step 2 outright.
2. **Land it (bounded, single agent, one sitting)**, once the lockfile write-lock
   has also cleared: bump
   `elohim/elohim-agent/elohim-agent-sdk/package.json:24` to `^0.80.0`, read the
   upstream `MIGRATION.md` for the default-export and `TextBlock` type questions
   above, fix the three call sites if needed, then
   `pnpm --filter @elohim/agent-sdk build && pnpm --filter @elohim/agent-sdk test`.
3. **Close out with full decomposition**: re-run the root install, confirm
   `node-domexception@1.0.0` is **gone from the banner** (it is exclusive to this
   chain, so its absence is the proof), then delete this entry and quote the
   verification in the commit message. Note `e83cd3f2d7e3` must **not** be deleted
   from the ledger at that point — the aggregate banner still carries the other
   ten packages; the fingerprint retires only when the last sibling closes.

## Verification

No fix was applied this run; nothing is claimed fixed. Verified:

- **Registry probes, this session**: `@anthropic-ai/sdk` packument → `200`,
  `dist-tags.latest = 0.80.0`, `versions["0.80.0"].dependencies =
  {"json-schema-to-ts": "^3.1.1"}` vs `versions["0.39.0"].dependencies` = the
  seven-package node-fetch/formdata-node stack listed above. Tarball
  `sdk-0.80.0.tgz` → **`404`** (×2 passes); `sdk-0.39.0.tgz` → **`200`** — the
  cached-only mirror behaviour that is the load-bearing blocker.
- **Reverse-dep trace** over `pnpm-lock.yaml` `snapshots:`:
  `node-domexception@1.0.0` has exactly one parent (`formdata-node@4.4.1`), which
  has exactly one parent (`@anthropic-ai/sdk@0.39.0`), terminating at
  `IMPORTER:elohim/elohim-agent/elohim-agent-sdk`. Single chain — no second
  carrier to chase.
- **Call-site scan** across `elohim/elohim-agent` (excluding `node_modules` and
  `dist/`): three files, four API touch points, table above. Zero
  upload/streaming/`FormData` use, so the removed polyfill is provably unused by
  this consumer.
- **Files touched this run**: this entry (new), five sibling entries, and one
  `.claude/data/deprecations.jsonl` status transition. No lockfile, no
  `package.json`, no `pnpm install`.
