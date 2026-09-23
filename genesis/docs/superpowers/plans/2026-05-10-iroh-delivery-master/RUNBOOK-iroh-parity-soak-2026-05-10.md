# RUNBOOK — Iroh Parity Soak (Gate #6)

**Soak window opened:** 2026-05-10
**Soak window closes when:** 7 consecutive nightly runs with zero divergences
**Gate:** #6 of iroh cutover (see master plan `2026-05-10-iroh-delivery-master.md`)

---

## What this soak measures

Every night at midnight UTC the orchestrator runs all `iroh_*` integration tests
against the `dev` branch in release mode with both transport features enabled:

```
RUSTFLAGS='--cfg getrandom_backend="custom"' \
cargo test --release --features "p2p p2p-iroh" \
    --test 'iroh_*' -- --test-threads=1
```

The stage also fires on any push that touches
`elohim/elohim-storage/src/p2p/**` or `elohim/elohim-storage/src/p2p_iroh/**`.

The artifact `iroh-parity-nightly.log` is archived per run. Each run produces
a pass/fail verdict visible in the Jenkins stage summary.

---

## Divergence definition

A **divergence** is:

> Any `iroh_*_real_backend` or `iroh_*_parity` test that fails in the nightly
> run on a commit where the corresponding libp2p code path is green (i.e., the
> non-iroh tests in the same `cargo test` invocation pass).

A single failure is **not** automatically a divergence — it could be a flake,
a transient network event in CI, or a test ordering issue. Apply this threshold:

| Failures in a 7-day window | Action |
|---|---|
| 0 | No action. Soak progressing. |
| 1 | Investigate the log. If the failure reproduces locally, it is a divergence. If it does not reproduce, mark as flake and continue. |
| 2+ | **Escalate immediately.** Do not close gate #6 until root cause is identified and fixed. |

---

## Monitoring guidance

1. Open Jenkins at `https://jenkins.ethosengine.com/job/elohim-orchestrator/`
2. Each nightly build shows the `Iroh Parity Nightly (Gate #6 Soak)` stage.
3. Stage is **green** = all `iroh_*` tests passed.
4. Stage is **yellow (UNSTABLE)** = at least one `iroh_*` test failed.
   Download `iroh-parity-nightly.log` from the build's artifact list.
5. Stage is **absent** = the cron trigger ran but the `when` guard excluded it
   (this should not happen on cron-triggered builds; investigate if seen).

### Reading the log

Look for the Rust test harness summary line:

```
test result: FAILED. N passed; M failed; ...
```

Each failing test name maps directly to a file in
`elohim/elohim-storage/tests/`. The `iroh_*_parity` tests assert byte-identical
output between iroh and libp2p transports. The `iroh_*_real_backend` tests
exercise the iroh transport end-to-end with actual QUIC connections.

---

## Escalation path

1. **First responder** (on-call engineer): reproduce the failure locally using
   the cargo invocation above; check if it is a flake by re-running 3 times.
2. **If reproducible**: open a `[transport]` issue with the test name, failure
   output, and commit SHA. Assign to the iroh integration owner.
3. **If 2+ divergences in 7 days**: pause the gate #7 alpha-cluster soak
   (do not flip `TRANSPORT_BACKEND=dual-stack` on new peers until parity is
   restored). The alpha cluster can continue running existing dual-stack peers.
4. **Do not** close gate #6 until the 7-consecutive-run criterion is met and
   recorded in the closure section below.

---

## Gate closure

Record each nightly run result below. Once 7 consecutive rows show PASS, close
the gate by committing a note to this file and updating the tracker table in
`2026-05-10-iroh-delivery-master.md`.

| Date (UTC) | Jenkins Build | Result | Notes |
|---|---|---|---|
| 2026-05-10 | — | OPEN | Soak window opened |
| (fill in) | | | |

**Gate #6 closed:** _(date, Jenkins build URL, signed off by)_
