---
name: feedback_sweettest_ignore_is_ci_noop
description: "CI runs the DNA sweettests with --run-ignored all, so #[ignore] is a no-op as a CI silencer — adding #[ignore] to quarantine a broken sweettest does nothing in CI; the test still runs and still fails. To remove a sweettest from the CI run you must delete it or change the runner config, not annotate it."
metadata:
  node_type: memory
  type: feedback
  originSessionId: 2026-05-16T05-00-three-pipelines-green
cites:
  - elohim/holochain/tests/sweettest/.config/nextest.toml
  - elohim/holochain/dna/Jenkinsfile
---

**CI runs sweettests with `--run-ignored all` — `#[ignore]` is a no-op as a CI silencer.**

**Why:** every sweettest carries `#[ignore]` so *local* `cargo test` / `cargo nextest run` skips them (they're expensive). CI **deliberately overrides** this — the DNA sweettest runner runs ignored tests anyway (`--run-ignored all`, carried in the nextest `ci` profile config / the DNA Jenkinsfile sweettest stage). Consequence: adding `#[ignore]` to quarantine a broken sweettest accomplishes nothing in CI — the test still runs and still fails. This cost a full ~75-min holochain build cycle when `#[ignore]` on `proposal_round_trips_across_agents` was a no-op and the test had to be **deleted** instead (the corroborating code comment lives in `elohim/holochain/tests/sweettest/src/tests/mishpat.rs`).

**How to apply:**
- To remove a sweettest from the CI run you must **delete it** (or change the runner invocation), not annotate it. `#[ignore]` only affects local default-skip behavior.
- If a sweettest you "quarantined" still appears in a CI failure, this is why — don't re-add `#[ignore]` expecting it to take; it already ran.
- Budget the verification of any sweettest-removal as a full holochain build cycle (~75 min); you can't confirm the silence locally because local already skips it.

Complements [[feedback_cargo_nextest_installed]] (which notes the *local* default-skip behavior but not the CI override). Also captured in the §3a recurring-anti-pattern table (#6) and the CI-orchestrator museum record.
