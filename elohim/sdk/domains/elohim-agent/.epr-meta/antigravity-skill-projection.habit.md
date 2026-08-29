---
epr-habit-version: 1
id: antigravity-skill-projection
invariant: >
  Every Elohim SkillPackage materializes as a discoverable Antigravity workspace skill from the
  same package-authoritative body used by Claude and Codex; every relative asset follows it
  byte-identically and receives provenance; unsupported Antigravity surfaces are never claimed.
status: green
active: false
checks:
  - "node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs selftest — synthetic package-master proof for Antigravity frontmatter, .agents pathing, and relative asset placement"
  - "node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs verify — schema + package + fixture + live-runtime freshness for all 41 SkillPackages and their assets"
  - "RUSTFLAGS='' CARGO_TARGET_DIR=/tmp/elohim-agent-antigravity-target cargo test --manifest-path elohim/sdk/domains/elohim-agent/adapter/Cargo.toml -p elohim-agent-adapter — one package produces three skill entrypoint edges plus one edge per runtime asset"
  - "a2o @concern:agent-runtime-projection (genesis/a2o/features/devflow/antigravity-skill-projection.feature — story is @wip until devflow step definitions execute the local projector)"
guard: >
  A SKILL.md-only green is false when its relative references or scripts are absent. A shared
  .agents path is also not evidence that Antigravity supports Codex agent, command, hook, or MCP
  dialects. Add a runtime target only after its documented ABI and adapter both exist.
refs:
  - "elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs"
  - "elohim/sdk/domains/elohim-agent/adapter/src/lib.rs"
  - "/home/user/.gemini/antigravity-cli/builtin/skills/agy-customizations/docs/skills.md (local installed Antigravity 1.1.x contract used during implementation)"
retire-when: >
  when Antigravity resolves Elohim capability packages and their eprfs derivation graph natively,
  making the checked-in .agents materialization an unnecessary compatibility projection.
---
RED WRITTEN 2026-08-28: Antigravity discovers .agents/skills, but only three skills were present,
their provenance called the target codexProject, and packaged assets were not projected or
addressed. The projector's green therefore described entrypoints, not usable capabilities.

GREEN 2026-08-28: projector selftest 43/43, package verify 1697/1697, adapter tests 6/6,
and the live compose graph records 123 skill entrypoints plus 9 asset edges across Claude,
Codex, and Antigravity; 41 Antigravity skills and all 3 packaged assets are present, and the
context-blind a2o review returned READY.

DELTA 2026-08-29: landing commit re-ran the checks on the integrated tree — projector selftest 43/43,
package verify 1697/1697, adapter cargo test 6/6 (EXIT=0); status stays green. a2o story remains @wip.
