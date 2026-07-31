---
id: "backlog-ci-sophia-node-engine-skew-ci-pod"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "sophia declares engines.node ^24.18.0 but its CI pod builds and publishes sophia-element on node:20 — WARN-only, so nothing will ever flag it"
slug: "ci-sophia-node-engine-skew-ci-pod"
written: "2026-07-31"
author: "ci-failure-triage"
status: "backlog"
priority: "medium"
ci_status: open
fingerprints: []
jobs: [elohim-sophia]
relatedNodeIds: []
tags: [ci, elohim-sophia, node24, engines, pnpm, sophia-element, nexus, publish, toolchain-skew]
cites:
  - https://jenkins.ethosengine.com/job/elohim-sophia/job/dev/146/
  - sophia/Jenkinsfile
  - sophia/package.json
  - genesis/data/timeline/backlog/ci-sophia-mathquill-git-dep-build-allowlist.md
---

# sophia's CI pod is two majors behind the Node it declares — and the signal is a WARN

## The observation

Discovered while triaging fingerprint `88e5f1135510`
(`ci-sophia-mathquill-git-dep-build-allowlist`). Not itself a ledger finding:
**no fingerprint, because nothing is red**. Every `elohim-sophia/dev` run emits,
repeatedly (quoted from #146, lines 201 / 390 / 409 / 16549 / 16927 / 17240):

```
 WARN  Unsupported engine: wanted: {"node":"^24.18.0"} (current: {"node":"v20.20.2","pnpm":"10.30.3"})
```

`pnpm` also appears at two different versions across the same build
(`10.30.3` and `9.15.9` at line 16554) — a second, smaller skew inside the same
warning family.

## Verdict — **real drift, currently latent**

The two declarations genuinely disagree:

- `sophia/package.json` — `"engines": { "node": "^24.18.0" }`, set by sophia
  `1db6b2f32e` (*"security(deps): bump node engines pin to ^24.18.0 (port of
  upstream Khan/perseus b062f818ef, reduced scope)"*).
- `sophia/Jenkinsfile` — the `node` container is `image: node:20`.

That container is not a corner of the pipeline. Of the twelve stages, **eleven**
run in `container('node')` — `Install`, `Lint`, `Type Check`, `Unit Tests`,
`Build`, `Build UMD`, `Publish to npm`, `Publish to Nexus`, `Archive Artifacts`
— only `SonarQube Analysis` uses the `builder` container.

## Why it matters (the part the WARN hides)

1. **`@ethosengine/sophia-element` is built and published from an unsupported
   runtime.** `Build UMD` and both publish stages run on node:20 while the
   package says it requires `^24.18.0`. Build 146 published 1.1.0 to npm and
   Nexus this way. The artifact the whole monorepo consumes is produced on a
   Node the producing package declares it does not support.
2. **The consumer moved and the producer did not.** `sophia-element`'s UMD
   bundle is a hard prebuild dependency of elohim-app, and elohim-app is on the
   Angular 22 / Node 24 toolchain. Producer-on-20 / consumer-on-24 is exactly
   the seam where a Node-version-conditional emit or a native-binding mismatch
   lands as an unexplained downstream failure.
3. **`engines` is advisory here, so the skew is self-silencing.** pnpm warns and
   proceeds. Nothing escalates, no fingerprint is ever minted, and the harvester
   will never surface it. Absent this entry the drift has no home.

## Current decision

**Open — deliberately not fixed drive-by.** Flipping `image: node:20` →
`node:24` is a one-line edit with a real shakeout surface behind it: sophia's
test stack is jest 29 / jsdom 20 (see
`deprecation-sophia-jsdom20-jest29-test-stack`), and jsdom 20 on Node 24 is not
a combination this pipeline has ever run. Doing it inside a CI-triage run would
risk trading a WARN for a red across eleven stages, in a submodule that pushes
independently of the parent.

What unblocks it: run the sophia suite once on Node 24 (locally or on a
throwaway branch) to size the jest/jsdom fallout, then either move the pod to
node:24 in the same change as the test-stack bump, **or** — if the test stack
cannot move yet — walk `engines.node` back to a range that admits the runtime CI
actually uses, so the declaration stops asserting something untrue. Either
direction is acceptable; leaving the two homes disagreeing is not.

Sequence it with the `deprecation-sophia-jsdom20-jest29-test-stack` work rather
than as an isolated pipeline edit.

## Trail

No fix attempted. Recorded during the `88e5f1135510` triage so the drift
survives that entry's decomposition.
