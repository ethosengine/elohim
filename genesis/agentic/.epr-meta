---
epr-meta-version: 1
id: genesis-agentic-loop-tooling
covers: subtree
purpose: >
  Utility scripts and reference data behind the agentic developer loop: the command-palette
  matcher and allowlist generalizer, the pre-shift readiness check, the safety taxonomy and
  anti-pattern catalogues, the cargo target pool and its resource guards (bin/: cargo-pool,
  ram-guard, io-guard, berth, pool pre- and post-flight), pool-policy.json, and the delegated
  compute worker (compute/).
---
# genesis/agentic — agentic-loop tooling

What belongs here: the scripts the `agentic-developer` skill and the cargo pool call, the data
they read, and a `*.test.mjs` (or `*_test.py`) beside each script, run with
`node --test genesis/agentic/*.test.mjs`. There is no `package.json`; the scripts use the root
workspace's dev dependencies. `pool-policy.json` is read by the pre-push hook and the disk-guard
PreToolUse hook, so a change to its watermarks or cargo overrides changes what every gate does.

`covers: subtree` is a considered ownership claim with no edit-time rule. Shift working state
(objectives, journals, sprint results) belongs in `.claude/shifts/`, not here.
