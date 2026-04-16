# genesis/agentic

Utility scripts and reference data backing the agentic developer loop
(see `genesis/docs/superpowers/specs/2026-04-16-agentic-developer-loop-design.md`).

## Contents

- `palette.mjs` — pattern-match a bash command against `.claude/settings.json` allowlist entries.
- `generalize.mjs` — cluster near-duplicate allowlist entries into broader patterns under a safety taxonomy.
- `readiness.mjs` — pre-shift environment check (tokens, connections, measure command, git state, palette sanity).
- `data/safety-taxonomy.json` — command family classifications (broadly-safe / subcommand-scoped / never-wildcard).
- `data/anti-patterns.json` — reference catalog of pipeline output patterns that waste Haiku's effort.

## Running

```sh
# From repo root:
node --test genesis/agentic/*.test.mjs
node genesis/agentic/readiness.mjs --objective .claude/shifts/<shift-id>.objective.yaml
```

No `package.json` here — scripts use root-level dev dependencies (`ajv`, `picomatch`).
