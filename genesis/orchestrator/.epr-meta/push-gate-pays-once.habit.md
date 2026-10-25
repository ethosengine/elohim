---
epr-habit-version: 1
id: push-gate-pays-once
invariant: >
  A local gate pays for a closure once. A push whose selected gate already carries this
  workspace's green attestation for the same input closure runs no recipe and says so; an
  eprfs push tests only the crates the change can reach; every gate states its expected
  cost and local fail rate before it runs; a long gate that has never failed locally may
  defer to CI on a feature-branch push, and never on a push to dev or main.
status: red
active: false
checks:
  - "node --test genesis/orchestrator/gate-evidence.test.mjs (closure key v2 material, selfAgentId, every reusableHit refusal reason and the happy path)"
  - "node --test genesis/orchestrator/gate-input-audit.test.mjs (no Rust test of a reuse-enabled project reads a repo path its step does not declare)"
  - "node --test genesis/orchestrator/cargo-affected.test.mjs (reverse-dependency narrowing and every full-workspace fallback)"
  - "node --test genesis/orchestrator/gate-cost.test.mjs (expected-cost fold over gate-cycle-seconds@1 and the defer decision's refusals)"
  - "a2o genesis/a2o/features/devflow/native-gate-evidence.feature reuse rule (this workspace's own green stands in; a red or an unadmitted signer never does)"
guard: >
  Regression risks: (1) reusing on a key that omits something the recipe reads — the recipe
  body, the project tree, a submodule pin, a repo file a test include_str!s; the input audit
  is the probe, never trust; (2) admitting a signer brit has not verified — roster and
  integrator keys wait for `build find` to verify signatures; (3) a narrowed green standing
  in for a full run — the package set is key material; (4) counting reused or deferred rows
  in the cost fold, which would pull the p90 under the defer threshold.
refs:
  - "plan: /projects/.claude-config/plans/sure-let-s-go-with-toasty-noodle.md"
  - "prior: /projects/.claude-config/plans/brit-is-greenfield-we-elegant-journal.md (F1 slice 1c)"
  - "measure: .claude/epr-meta/measures.yaml gate-cycle-seconds@1 and its two ceilings"
retire-when: >
  when gate verdicts are read from the dataplane (brit-native integrate) rather than from a
  pre-push hook, so a push no longer runs local gates at all.
---
DELTA 2026-10-25b (RED preserved; one reuse path, live probe green): dev had landed a parallel `gate-receipt@1` path (eprfs only), keyed on epr's own source closure. That key (epr's source closure, which does cover the whole eprfs workspace, plus toolchain and manifest) missed the `_gate-eprfs` recipe body and the repo files eprfs tests read outside it (`.epr-meta/`, `.claude/epr-meta/`, `genesis/docs/`, `.eprfs/status/`). An earlier claim that it missed `eprfs-cli` and the adapter was wrong: those were read off the dependency graph, and the evaluator's real closure includes them. Operator ruling: one path. Receipts are folded into brit evidence: `nativeGateKey` is now eprfs key material in closure key v3, the receipt short-circuit, its writes, tests and opt-in are removed, `gate-receipt@1` is superseded, and seam-registry.yaml cites `reusableHit` and `reuseSelector`. The rebase took dev's process-group renew-loop fix over this branch's. Probe (worktree, elohim-epr): run 1 miss, recipe ran, green recorded; run 2 `reused` in 5 s against an expected ~2 min (p90 5); a dirtied read path refused reuse and recorded nothing.
DELTA 2026-10-25 (RED preserved; built, not yet proven on a push): reuse lands behind GATE_EVIDENCE=reuse for six projects (pool-policy `gate_reuse`), self-signed only. The closure key now covers the recipe file, run-local-gate.sh, the project tree, `.cargo/config.toml`, `rustc -vV`, submodule gitlinks under the inputs, and every repo path the code reads (gate-input-audit.mjs readSet: storage 134, doorway 31, steward-node 118, elohim-epr 9 paths, 0 unresolved; eprfs 26 unresolved, so its key is the whole tree). Declaring those reads in manifests was tried and reverted, because step sources drive CI dispatch. eprfs narrowing runs in shadow (eprfs-meta change → 2 of 11 members). The real cost fold reads `eprfs: expect ~14 min (p90 28, max 32; 10 runs) · failed locally 6 of last 20`, so eprfs never defers. The harness leg drops 77 → 36 s at HARNESS_JOBS=4 with identical pass sets. Tests: 175/175 across the six touched files; full orchestrator suite 810 pass, 0 fail, with prepush-berth.test.mjs hanging identically on dev (pre-existing; `prepush_releases_on_gate_failure` hangs only when its runs share one process). Story `native-gate-evidence.feature` gains the reuse rule and two read-set scenarios. Blind-reader loop: round 1 2C/3I/2P (REVISE), round 2 0C/4I/3P (READY); open interpretability: the cause of "the recipe now succeeds" is unnamed, "enabled for reuse" is not concrete, the scan is described only in prose, and the cycle observation's reader is implicit.
DELTA 2026-10-24 (born RED): pre-push re-ran every gate from the top; gate evidence was report-only and dark on shem (no brit, no epr resolved). Measured: elohim-storage 1875 s mean (pre-push peak 9383 s), eprfs 1228–1894 s, orchestrator 1260 s, agentic-library harnesses 81 s serial. 16 of the last 18 dev CI builds were red, on dataplane or environment, not on code a local gate catches. brit 76ba95d8a0 landed in the device store by bootstrap (local build 29m36s). Flips green on a real push log showing `reused` with seconds saved.
