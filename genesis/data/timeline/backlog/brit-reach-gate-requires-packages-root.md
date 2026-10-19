---
id: "backlog-brit-reach-gate-requires-packages-root"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "brit's reach gate refuses every push from this workspace: the installed epr requires .epr-meta/elohim/packages, which brit has never carried"
slug: "brit-reach-gate-requires-packages-root"
written: "2026-10-19"
author: "claude (session-covenant session 182f26d8, operator-directed)"
status: "open"
priority: "medium"
ci_status: none
jobs: []
tags: [brit, epr, reach-gate, governance, evaluator-skew, submodule]
cites:
  - genesis/docs/superpowers/specs/2026-10-19-session-covenant-epr-design.md
---

# brit's reach gate refuses every push: an evaluator/repo skew

## What happened (2026-10-19)

Pushing `run-ci/session-covenant` to brit, the `.githooks/pre-push` adapter ran the installed
`epr` readiness check and refused:

```
✗ governance.package-root [reach]: missing .epr-meta/elohim/packages
    fix: restore `.epr-meta/elohim/packages` from the repository projection
NOT READY: resolve reach-blocking findings before push
```

brit `main` has never carried `.epr-meta/elohim/packages`. Its `.epr-meta/` holds `manifest.md`,
`repository.yaml`, `recipes.yaml`, `measures.yaml`, `readiness.json`, `git-boundaries.json` and its
habits. The finding therefore fires on every brit push from this workspace, whatever the change.

The operator ruled a one-time `--no-verify` for the `run-ci/**` branch only, so brit CI could
compile the covenant work. Nothing reached brit `main` that way.

## The skew

The installed `epr` applies the monorepo's package-root expectation (a reach-blocking finding) to
a repository whose own manifest terminates the parent's governance cascade. Either:

- brit should carry a package root (perhaps an empty, declared one), so the gate's question has an
  answer there; or
- the readiness check should read the target repository's own declaration of which roots it owes
  (brit's `readiness.json` / `repository.yaml`), and not assume the monorepo's layout.

The second keeps brit's boundary ("do NOT port the parent's Python enforcement tooling"; brit owns
its own governance) and is likely the right home.

## Close when

A brit push from this workspace passes the reach gate with no bypass, and the readiness finding
names only roots that brit itself declares.
