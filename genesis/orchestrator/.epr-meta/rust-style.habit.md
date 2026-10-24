---
epr-habit-version: 1
id: rust-style
invariant: >
  The native Rust style gate refuses new unexplained lint suppressions, requires one
  finding of paydown from each touched indebted crate after enrollment, and preserves
  zero-debt strict module scopes. A pushed commit cannot borrow a dirty working-tree repair.
status: green
active: false
checks:
  - "just gate rust-style (native syntax, Git ratchet, strict scopes, checker fmt/Clippy/tests and agent adapter contracts)"
  - "a2o @concern:rust-style: cd genesis/a2o && pnpm exec cucumber-js --profile rust-style (three isolated-Git acceptance scenarios)"
  - "node --test genesis/orchestrator/rust-toolchain-pin.test.mjs (CI/local checker toolchain agreement)"
guard: >
  Never equate equal counts with no new findings; never use dirty source for a pushed object;
  never call a missing baseline or a stale checker clean. Scope and owner moves cannot transfer
  debt, and lint/configuration edits must select the gate. Attribute reason presence does not
  establish justification or certify all guide rules. Runtime trust and remote CI execution
  remain separate from local contract evidence.
refs:
  - "guide: genesis/docs/rust-style-guide.md"
  - "checker: genesis/orchestrator/rust-style/"
retire-when: >
  when all first-party crates have zero active-rule debt and the ordinary compiler/governance
  gates enforce these same properties without a separate Git debt ratchet
---

DELTA 2026-10-22 (GREEN, local enforcement contract): `just gate rust-style` passed with
1,901 Rust files in 79 crates and 411 inherited RS-GOV-04 findings, zero new findings;
the checker itself has six strict files and zero debt. Eight new attributes in current work
were supplied with their existing design rationale, with broader decisions recorded locally.
Native checker: 18 tests passed, fmt and Clippy `-D warnings` passed. Shared agent adapter:
seven tests passed, including Codex freeform events and first-build CI baseline refusal.
Live Write probes denied an unexplained suppression and permitted an explained one; the
native `epr govern` provider independently returned `refuse` then `permit`. A2O: three
scenarios/nine steps passed. Policy pins: 25 verified; validator accounting: 11 assertions;
package projections: 2,194 checks passed. The first policy enrollment preserves no-new checks;
ordinary per-touch paydown begins when enrollment is in the integration baseline. CI is wired
before downstream dispatch; this delta does not claim a remote Jenkins run.

Story review cost: three fresh blind-reader rounds; correctness findings 2, 1, 0;
interpretability findings 0, 1, 0; preference findings 0, 0, 0. All findings resolved;
final reader READY. The stories now distinguish new strict-module debt from inherited
crate debt and explicitly establish the accepted repair as the next integration baseline.

DELTA 2026-10-22 (final hardening): `just gate rust-style` remains GREEN with 19 native
tests, including new-child strict-scope shadowing refusal, plus seven adapter/baseline tests.
All 15 repository-validator tests and the native host's Clippy `-D warnings` pass. The tested
`epr` executable is installed on the shared PATH; live strict refuse/permit probes pass through
that installed host. Build-manifest validation reports 24 manifests, 45 steps, zero errors.
Durable local receipts: `genesis/a2o/reports/recovery/rust-style/`. The feature-branch CI checkout
now explicitly fetches its integration ref; no history or hook-availability gap is called clean.

DELTA 2026-10-24 (ownership diagnostic): `python3 -m unittest discover -s genesis/orchestrator/rust-style -p hook_test.py` passed 8 tests; normal and `--hook` entrypoints now name the safe.directory cure and exit 2 before pool/Cargo on Git ownership refusal. No Cargo build or full native gate was run in this bounded fix; habit status unchanged.
