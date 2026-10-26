---
epr-habit-version: 1
id: device-supervises-its-manifest
invariant: >-
  A developer can declare a resident offline on the device's private manifest chain
  and supervise that head with one ark per data root. A killed permanent resident
  returns ready with a death witness naming its exit reason. A later supervise
  reloads a superseded head by graceful restart under the new CID, carrying the
  incarnation; an unchanged head is a no-op, and stop ends ark and its children.
  With residents declared, an absent or stale supervisor reads Degraded with the
  cure epr device supervise, never Refused; at the head with every child ready it
  reads Ok. Supervision enforces no resource bounds and provides no isolation.
status: red
active: false
checks:
  - "@concern:device-supervises-its-manifest"
  - >-
    cargo test --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli
    --test device_supervise ark_keeps_the_declared_resident_and_reloads_a_moved_head
    -- --exact --nocapture (native CARGO_TARGET_DIR pool slot and RUSTFLAGS=;
    ARK_BIN must name a built ark: without it this test returns successfully
    without asserting the live story)
guard: >-
  Fixture evidence is not evidence of adoption on this workspace's real chain.
  The fixture disables berth leasing with EPR_BERTH_BIN empty, so it cannot prove
  a device-ark lease. With no berth tool or resolvable session, supervise proceeds
  unleased. Ark reports EffectiveTier None: restart and death witnesses do not
  establish resource bounds or process isolation. A conflicting ark started
  outside supervise is refused by process ID; repeating supervise at its own
  unchanged head succeeds without starting another ark.
refs:
  - genesis/a2o/features/devflow/device-runs-its-own-manifest.feature
  - elohim/eprfs/epr-cli/tests/device_supervise.rs
  - elohim/eprfs/epr-cli/src/device/supervise.rs
  - elohim/eprfs/epr-cli/src/device/arrival.rs
  - elohim/ark/core/src/declaration.rs
  - genesis/agentic/pool-policy.json
retire-when: >-
  when a replacement device runtime owns manifest supervision, carries this
  concern's restart, witness, reload, stop and singleton checks against a real
  device in its required gate, and epr device supervise has been removed
---

DELTA 2026-10-26 (born RED, inactive): commit `06dd6c51c` records that the live
story `ark_keeps_the_declared_resident_and_reloads_a_moved_head` passed with
`ARK_BIN` set against a fixture device, NOT this workspace's own chain. The
commit also records `just gate elohim-ark`, `eprfs` and `rust-style` with exit 0.
These are the commit's reported results, not new runs during this authoring pass.
Blind-reader loop (fresh `blind-reader` agent, a2o-story profile, path only):
round 1 READY, 0 correctness · 5 interpretability · 1 preference (EffectiveTier
jargon, chain/CID model, fixture vs product readiness, berth lease vs data-root
exclusivity, degraded vs REFUSED); all five repaired, the preference (split the
stop proof) declined. Round 1 also exposed a defect: a resident declared without
a ready line is never marked ready, so the arrival line would have stayed
degraded; `check()` now counts such a child as waiting only while it has no live
process. Round 2 (new reader) READY, 0 correctness · 5 interpretability · 2
preference; repaired restart-policy gloss, incarnation's purpose, the by-hand
`ark run` behind the singleton scenario, and the lease's role; declined chain
history (out of this story's scope) and a concrete path. Closed by judgment at two
READY rounds with zero correctness findings. Loop cost: 2 reader rounds, about
105k reader tokens.

The live story declares a resident, observes readiness, repeats supervise at the
same head, kills the resident with signal 9, reads its death witness, supersedes
the head, observes reload under the new CID with a greater incarnation, and stops
ark and its child. Its resident is a copy of bash registered as the fixture's
`epr` tool, not `epr lsp serve --device`. With `ARK_BIN` unset the live test returns
before those assertions; an ordinary green eprfs gate alone does not prove them.
The fixture also disables the berth lease. The duplicate-ark refusal is grounded
in `supervise.rs`; this live test does not exercise that refusal. The Gherkin
story remains `@wip` until its steps are wired.

Green requires evidence from this workspace's real device chain: declare the
resident `epr lsp serve --device`, supervise it and observe it ready at the head,
kill that resident and observe its replacement ready with a death witness naming
the killed process and cause, and record `berth status` showing `device-ark`.
Keep the manifest CID, process IDs, witness and lease observation with that delta.
Until that adoption is observed, the fixture pass leaves this habit red.
