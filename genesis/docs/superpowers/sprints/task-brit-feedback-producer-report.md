---
id: task-brit-feedback-producer-report-20261111
gap: plans__2026-11-11-brit-feedback-operation#6
actor: agent:orchestrator@gpt-6-astra
status: DONE_WITH_CONCERNS
commits: [6f203b22b59104c0a2708d7181637715069845f0, 4575ab2eea9d2967de1bb6225971e42d3e2ce0e3]
---

# Brit producer and bounded-history repair

Brief: `genesis/docs/superpowers/sprints/task-brit-feedback-producer-brief.md`.
Steward: `human:matthew`. Root owns the native promise and its terminal flow act;
this report supplies technical evidence only. The repair was proposed from retained
occurrence `bafyreieytypsusld4whxkx7af7vfqg2hsofsyuv2ikftu2unm7ozawoe3e`, through Intent
`bafyreidrn3wqc3yj3qbw6ts6f54v56ejzn2qlac5o5ekavgmgrh3x2kz6m`.

## Result

The implementation is isolated in `/projects/elohim/.worktrees/brit-feedback-release`
on local branch `fix/feedback-operation-20261111`, based on installed published source
`f8ef2fb6b40d3678fce36f114259d80e2bf331ee`. The six-file implementation was committed as
`6f203b22b59104c0a2708d7181637715069845f0`; append-only release metadata repair
`4575ab2eea9d2967de1bb6225971e42d3e2ce0e3` is published on Brit main. The complete
four-executable release has passed CPU qualification and downloaded-byte verification.
The original dirty `/projects/elohim/elohim/brit` checkout was not changed by this seat.

The implementation changes exactly six source files:

- `brit-cli/src/main.rs`
- `brit-cli/src/feedback.rs`
- `brit-cli/tests/feedback.rs`
- `gitoxide-core/src/repository/log.rs`
- `src/plumbing/main.rs`
- `src/plumbing/options/mod.rs`

Brit now captures successful commands, parser refusals and intentional bridge
declarations through optional local EPR recording. Capture retains an allowlisted
operation, outcome, invocation token and elapsed duration; it excludes raw arguments,
command output and environment. Missing, refusing or stalled recorders preserve the
command's exit code. The recorder wait is bounded at two seconds.

Bounded history supports `-N`, `-n N` and `--max-count N`, zero, history exhaustion
and root options. Parser-aware prefix discovery identifies the actual canonical
command for both shorthand normalization and refusal capture. Thus `-r . log --bad`
records a log refusal, while a repository value named `log` in `-r log status --bad`
does not misclassify status. The latter cases resolve the reviewer's Minor finding.
Existing path-based history remains explicitly unsupported; limits do not silently
turn it into an unbounded query, and arguments after `--` retain their path meaning.

The newer base's CPU compatibility guards, retained helper and delivery workflows
are unchanged. The release repair changes four additional metadata files:
`gitoxide-core/Cargo.toml` (0.63.5 → 0.63.6), `Cargo.toml` (gitoxide 0.60.5 → 0.60.6,
core floor ^0.63.6), `brit-cli/Cargo.toml` (0.1.12 → 0.1.13, gitoxide floor ^0.60.6),
and `Cargo.lock` (only those three local package version entries). The old `find_commit(...).unwrap()` in
the touched log renderer now returns an I/O error instead of panicking.

## Gate evidence

All evidence paths below are relative to
`genesis/a2o/reports/recovery/brit-feedback-20261111/` in the parent repository.
Native Cargo used the computed worktree pool slot
`/projects/.cargo-target-pool/family/feedback/projects__elohim__.worktrees__brit-feedback-release/dev`,
one compiler worker, `RUSTFLAGS='-C target-cpu=x86-64-v2'`, and
`CFLAGS='-march=x86-64-v2'`. The shared session cargo lease was coordinated with the
integrator. Existing provisioned registry credentials were used through `cargo:token`;
no credential or Cargo configuration was committed.

Mandatory gate-evidence lines for the reviewed implementation later committed as
6f203; these local runs preceded the four-file release metadata repair:

```text
cargo +1.96.1 test --locked -p brit-cli --test feedback --test unified --test bridge
EXIT=0
```

`brit-review-fix-tests.log`: 17 passed, zero failed or skipped: four feedback/history
tests, three bridge tests and ten unified CLI tests. Cases include numeric/short/long
limits, zero, exhaustion, order, root options, explicit path refusal, option terminators,
invalid counts, private argument exclusion, correlated capture, and recorder absence,
refusal and timeout preserving both success and parser-refusal exits.

```text
cargo +1.96.1 clippy --locked -p brit-cli --all-targets --no-deps -- -D warnings
EXIT=0
cargo +stable fmt --check
EXIT=0
BRIT_RUST_STYLE_PREFIX=/projects/elohim/elohim/brit/.eprfs/tools/rust-style bash scripts/ci/rust-style.sh --base f8ef2fb6b40d3678fce36f114259d80e2bf331ee
EXIT=0
bash scripts/ci/test-brit-delivery-source.sh
EXIT=0
```

Evidence: `brit-review-fix-clippy.log`, `brit-format-final.log`,
`brit-rust-style-final.log`, and `brit-delivery-source.log`. Rust-style's verified
checker was reused from the existing installation; its declared scope is the
standalone policy's scope, not a claim of expanded coverage. Clippy emits the
pre-existing command-line unknown-lint warning for `clippy::assert_is_empty` on
toolchain 1.96.1; no new suppression was introduced.

The installed baseline independently reproduced `brit log -5` with `EXIT=2` and
`unexpected argument '-5'`; retained in `brit-installed-log-before.log`. Early
credential-provider failure and prior successful runs remain in
`brit-producer-focused.log`; the review-fix logs above verify the implementation before the metadata repair.
Exact corrected-source CI execution is recorded below. `brit-producer-source.sha256` fingerprints the frozen six-file patch.

## Corrected release gates and publication

The first main publication at 6f203 passed its required tests but failed the immutable
Nexus package comparison: changed source had retained published versions. This was
our delivery regression, repaired by the four-file version/floor change above. No
published history or registry payload was overwritten. Independent release-delta
review found no actionable issue. Detailed rationale and evidence remain in
`brit-release-delta-review.md` under the proof directory.

```text
cargo +1.96.1 update --offline -p gitoxide-core -p gitoxide -p brit-cli
EXIT=0
bash scripts/ci/cargo-publish-brit.sh --audit-index
EXIT=0
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/ci -p 'test_publish_brit.py'
EXIT=0
cargo +stable fmt --check
EXIT=0
BRIT_RUST_STYLE_PREFIX=/projects/elohim/elohim/brit/.eprfs/tools/rust-style bash scripts/ci/rust-style.sh --base 6f203b22b59104c0a2708d7181637715069845f0
EXIT=0
bash scripts/ci/test-brit-delivery-source.sh
EXIT=0
```

Evidence: `brit-release-lock-refresh.log`, `brit-release-index-audit.log`,
`brit-release-publisher-tests.log` (25 passed), `brit-release-fmt.log`,
`brit-release-rust-style.log`, and `brit-release-delivery-source.log`. The index audit
proved 70 packages: 67 present and three new versions absent; it did not prove payload
equality. A subsequent read-only audit invoked the existing publisher's package and
comparison functions for all 67 existing versions: **67 skip-equivalent, zero failed
or unverified, EXIT=0**. This compared packaged source, manifests, modes and generated
Cargo.lock, excluding only `.cargo_vcs_info.json` by publisher policy. It uploaded
nothing and built nothing. See `brit-release-existing-payload-audit.log` and `.json`.

Corrected feature CI run **43112209546** executed exact 4575 source. Linux nextest
job **130830930412** ran `cargo nextest run --workspace --no-fail-fast --exclude gix-error --`:
**5683 passed, 11 unrelated skips**, including all **17/17** feedback, unified and
bridge tests, none skipped. The job conclusion is success; this is forge execution
evidence, not a fabricated local EXIT line. See `brit-release-nextest-linux.log`,
`brit-release-nextest-linux-job.json`, and `brit-release-nextest-focused-evidence.txt`.
This actual corrected-source test run satisfies dependency-work testing; no local
heavy build was duplicated while the integrator's cargo lanes were occupied.

| Exact 4575 evidence | Run / job | Verdict |
| --- | --- | --- |
| Feature Tests pass | 43112209546 / 130842630496 | success, 22:54:28 UTC |
| Main Tests pass | 43115801332 / 130853911517 | success, 23:19:12 UTC |
| Nexus publisher and isolated consumer | 43115801332 / 130854550121 | success, 23:33:04 UTC |
| Four-bin artifact and CPU qualification | 43120684425 / 130860059134 | success |

Dates are 2026-11-11. Main fast-forward publication used Brit with hooks enabled,
`EXIT=0`; remote main was re-read at exact 4575 (`brit-release-push-main.log`,
`brit-release-main-before.txt`, `brit-release-main-after.txt`). No force, amend or
hook bypass was used.

The publisher verified immutable uploads of gitoxide-core 0.63.6, gitoxide 0.60.6,
and brit-cli 0.1.13, then built an isolated consumer from published packages outside
the checkout with locked fork sources. Evidence: `brit-release-publisher-consumer.log`,
`brit-release-publisher-consumer-job.json`, and `brit-release-publisher-consumer-evidence.txt`.
That consumer installs the single public Brit binary; full helper delivery is proven
separately by the artifact workflow.

## Verified release handoff

Release: https://github.com/ethosengine/brit/releases/tag/brit-build-4575ab2eea9d
(published 2026-11-11 23:48:38 UTC). API and tag-ref evidence bind it to exact 4575;
it is a non-draft prerelease. The artifact job explicitly verified triggering main
run **43115801332, attempt 1**, and its required Tests pass before building:

```text
cargo build --locked --profile release-github -p brit-cli --bin brit --bin brit-rebase-retained -p brit-build-ref --bin brit-build-ref -p brit-verify --bin brit-verify --no-default-features --features brit-cli/max-pure
```

CI job conclusion: success. Build flags preserve the x86-64-v2 floor. The exact
binaries passed native, QEMU Nehalem (x86-64-v2) and Haswell (AVX2) startup, functional
and TLS qualification, including a control proving baseline rejects AVX2 and the
newer CPU executes it. The matrix passed at 23:48:33 UTC. Proof:
`brit-release-artifact-job.log`, `brit-release-artifact-job.json`,
`brit-release-artifact-run.json`, and `brit-release-cpu-evidence.txt`.

Downloaded bundle directory, relative to the proof directory:
`artifacts/4575ab2eea9d2967de1bb6225971e42d3e2ce0e3/`.
Its exact inventory is four executables plus `SHA256SUMS`; no release.json is expected.
Strict inventory, four unique checksum rows, all five API asset digests and sizes,
and the executable's version/source output were verified **EXIT=0**. All four binary
hashes also match the CPU matrix's final exact-byte hashes, **EXIT=0**
(`brit-release-cpu-byte-match.txt`):

```text
brit                  120399180e6499aa87c51b7ddedac2bd1e0f86e080a57102eadd77616a06d7fb
brit-build-ref        56b19a0cb8b2db311282c192fe0c2b452a5ab0dbd590da3ca2cc3aa8293bb0d0
brit-rebase-retained  6b0133ce23c9f6e5756830ad84c27da939a02233ab71d50d2324c14d6a8c2f08
brit-verify           c0b59c65ad8e327e7d56333574f8f08af23be2de4a5adc16eaf54cc0394b8922
SHA256SUMS            b20d3d5fb943e832446a1eabfabe8ff6d0d3c4052b9dd2ff34608a09f4f4d652
```

`downloaded brit --version` ran in a disposable cwd outside feedback-policy ancestry,
with feedback root/invocation overrides removed, and returned **EXIT=0**, Brit 0.1.13,
source HEAD 4575ab2eea9d2967de1bb6225971e42d3e2ce0e3 and max-pure. Its source state
honestly says unverified: HEAD alone does not attest working-tree bytes. CI build
provenance and artifact hashes supply that separate evidence. See
`brit-release-downloaded-verification.txt`, `brit-release-manifest.json`, and
`brit-release-tag.json`. The old download process handle did not survive resumption;
its empty log is not asserted as an observed EXIT=0. Downloaded bytes were independently
verified as above.

## Remaining concerns and authority boundaries

The main workflow's overall conclusion is failure despite its required aggregate,
publisher and release all passing. Two declared nonblocking jobs remain red:

- Inherited `cargo-deny-advisories` findings remain. The repair introduces no external
  dependency version change or advisory suppression.
- Main arm32 job 130843342203 passed 5677 tests and failed fixture setup in
  `cli-journey::worktree_restore::publication_failure_preserves_failed_file_neighbors_and_index_with_partial_progress`:
  “restore never exposed its temporary write.” The exact same 4575 feature arm32 job
  130830930364 passed that fixture and all 5678 tests. The polling fixture observes a
  temporary-write window; setup timing variability is an inference, not a proven
  product diagnosis or repaired defect. Both logs are retained as
  `brit-release-main-arm32.log` and `brit-release-ci-arm32.log`; no retry or source
  change concealed this result.

The original source CI run 43095581178 stranded queued with zero jobs. GitHub incident
https://www.githubstatus.com/incidents/dvy0t3mvh1lg overlapped the delay, but the queue
started before the incident announcement, so it does not prove the whole delay's cause.
After recovery, one normal cancel and one authorized force-cancel both returned 409;
the run was preserved. One authorized replacement branch at the unchanged 6f203 SHA
produced recovery run 43103343604 and green Tests pass before the first main promotion.
The earlier 6f203 artifact is not the final accepted bundle.

The corrected feature push encountered Brit's send-pack closed-channel failure after
green hooks. Absent remote destination and clean local state were verified before an
explicit Git fallback with hooks enabled succeeded, EXIT=0. Evidence:
`brit-release-push-ci.log` and `brit-release-push-ci-git-fallback.log`. The shell guard
recorded the declared fallback but reported outcome capture unmeasured. Corrected main
publication subsequently succeeded with Brit. Unsupported shell adapter coverage is
unmeasured, never a clean governance verdict.

Standalone Brit lacks prepare-commit-msg. Corrected commit 4575 carries exact trailers
from fresh native `session commit-trailers --json`, plus an append-only attribution
correction for original 6f203. The provenance is the shared root native session
01a1cbeb-b491-7603-9bc7-13a93dcfe7c2 and measured gpt-6-astra model, not an invented
independent subagent identity. `brit-release-commit-trailers.json` and
`brit-release-commit-message.txt` retain the evidence.

The original retained occurrence's participant is **human:matthew**, not anonymous;
root's correction is `bafyreic3z4i5dgg4wijevtfl5ryohlh5twvcj7maxql2hv55fmueh4igku`.
Technical review may establish verified-fix against it, but does not establish
Matthew's experienced recovery. The agent reproduction remains its own recovery target.

This seat changed no installed tool heads. Native installation, release provenance
ingest, monorepo pin attestation, attributable installed replay, appointed acceptance,
and the terminal native promise act remain integrator-owned. This report supplies
published technical evidence, not an installed acceptance or fulfilment claim.
