---
id: brit-daily-driver-evidence-layer-sprint
title: "Sprint: brit daily-driver — opens the monorepo, every claim fails closed"
status: in-progress
class: architecture
serves: [brit-native-daily-driver, push-delivers-within-budget, push-gate-pays-once]
steward: human:matthew
actor: agent:orchestrator@claude-opus-5-5
date: 2026-10-29
cites:
  - genesis/research/ngit-nostr-git-forge-cross-pollination-2026-10-29.md
  - genesis/data/timeline/backlog/brit-daily-driver-handoff-2026-10-27.md
  - "steward-space-landing-plane-design | The steward space | sha256:53718b5898db3b15 | path: genesis/docs/superpowers/specs/2026-10-27-steward-space-landing-plane-design.md"
---

# Sprint: brit daily-driver — opens the monorepo, every claim fails closed

> Provenance: grounded and challenged by a 41-agent workflow (wf_2a270487-6c2) and designed by a diverse panel (wf_f9cfa72a-80c: Opus, Sonnet and Codex designs; three Codex code probes; two Claude judges, both choosing the Opus integrity-first plan). Code lives on brit branch `feat/daily-driver-evidence-layer` in `.worktrees/brit-daily-driver`. The closure-keyed landing-plane design this composes with is a Draft in another session's worktree and is not cited by path here.

## Context

The ngit survey was challenged by a 41-agent pass (wf_2a270487-6c2) and a diverse design panel (wf_f9cfa72a-80c: Opus / Sonnet / Codex-astra designs, 3 Codex probes, 2 Claude judges; both judges chose the Opus integrity-first plan and grafted the rest). Findings that set this sprint:

- **ngit is daily-driven because it does not replace git** (C12). The daily path is: stock git porcelain + a brit that opens the monorepo + brit-owned evidence verbs that fail closed and work offline/alone + named bridges for everything brit does not own.
- **brit cannot open /projects/elohim.** gix fixes the Reduced-trust slot default (`Slots::Given(32)`, `gix/src/open/options.rs:211`) before `safe.directory` upgrades trust to Full (`gix/src/open/repository.rs:415`), and builds the store from the stale value (`:549`); the repo has 41 packs. Not fixed upstream.
- **brit's evidence layer does not fail closed:** `build find/get` match raw JSON, no signature check, git failures read as absence (`brit-epr/src/elohim/refs.rs:40-97, :71, :359`); note/ref writes have no expected prior (`refs.rs:320-395`); `LocalObjectStore::get` never checks the CID; `reach compute/get` stamps "verified" from ref-name presence; a plain commit with `commit.gpgSign=true` is written unsigned (`gitoxide-core/src/repository/commit.rs`); key generation is non-atomic and `common_git_dir` silently guesses `<repo>/.git`.
- **`brit add` refuses everywhere here** because LFS filter drivers are configured globally, though the monorepo has no `.gitattributes` (`index/authoring.rs:135`).
- Forge-shaped work (landing intents, Change-Id, sibling reuse, device-written `ci:` notes, the bundle carrier, brit off GitHub, the pre-push hook, gate-evidence/brit-attest/delivery-series) is **owned by the closure-keyed landing-plane session** and untouched here.

**Principle established:** *brit is the evidence layer beside stock git, and every claim it makes fails closed.* A note is admitted only if it decodes strictly, its signature verifies and it binds to where it was found; failure is never absence; a head moves only against the prior it read; brit never writes what it would refuse to read; a commit configured signed is never written unsigned; the signer is named by a verb, never by reading a seed. **Verification is not admission** — brit output carries no trusted/admitted flag; standing stays with the reader. Habit: `brit-native-daily-driver` (elohim/brit/.epr-meta, unwired → red with a runnable check), with handoffs to `push-delivers-within-budget` and `push-gate-pays-once`.

## Integrity rulings (Claude, recorded)

**Planes (claim-backed credentials spec §12.1, operator ruling 2026-10-04; final by witness §12.10).** A brit signature lives on the **provenance** plane only: which device produced these bytes. Recognition (whose device: the consent-grant joining record), standing (who may change a subject: held by people and collectives, never by a key; the network holds the root) and reach are separate planes, and a check that reads two at once is the defect. So brit verifies provenance and stops there; admission is the reader's, through recognition and standing.

**Diversity, not headcount (operator, 2026-10-29).** The root is inviolable because a socially-derived holon is diverse (school, neighbour, family, NGO, government, association, workplace, library): independent failure domains make the core improbable to capture. Keys are free to mint, so brit never counts signatures as witness strength and never aggregates rows into a quorum; it returns provenance per row under each row's own agentId, and judging whether holders are diverse, recognized members belongs to recognition and standing. One steward's devices are one failure domain (D5).

**Why the separation matters (operator, 2026-10-29).** A stolen device still holds its brit key, so its notes stay valid provenance. What catches the theft is a contextual witness (an elohim agent that knows who the person really is) noticing the discontinuity; what stops it is recognition being revocable, so readers stop admitting that signer while brit keeps reporting the signature honestly. brit must therefore never turn a valid signature into recognition or standing — that fusion is exactly what would take away the agents' ability to intercede.

Slots follow FINAL trust via `Option<Slots>` (explicit caller wins; no global Reduced relax). A build note proves signer + its ten signed fields, **not** which commit it annotates. Strict decode with `deny_unknown_fields` (safe: all 90 live notes carry exactly the ten fields). `find` keeps its `[{commit, attestation}]` array; `get` keeps the exact `no build ref found for step=` line (the S2d' lane parses it). Notes committer becomes the neutral `brit <notes@brit.invalid>` (judges split; an unsigned field naming a signer is an unverified identity claim). Key custody is settled by D5 (2026-10-27), not reopened here: brit's signer stays its own per-device key domain; no move, no device-key derivation; `binds-signer` is the S2 entry condition. Delete `reach compute/get` outright (supersedes unmerged `06f9f234a4` — recorded as a handoff).

## Citation chain (arch-authority-in-integrity row 19 (c), operator 2026-10-29)

*The citation is the architecture:* an act's EPR links by CIDv1 to the decision it executes, the decision to the charter it was made under; each hop verifies from the bytes alone, and signatures are only evidence of who acted.

- **This sprint dogfoods it.** Every sprint commit carries `Qahal: steward | ref=bafyreifomav2mided4i7g66j47adaxosadm7canhmgf2i7knxpiqm7kl3a` and `Qahal-Node: bafyreifomav2mided4i7g66j47adaxosadm7canhmgf2i7knxpiqm7kl3a` (this session's purpose clause), added at final integration while the branch is local. First brit commits to use the covenant spec's citing trailers.
- **Finding → next sprint, slice 1:** a build attestation does not cite its subject inside its signed bytes (it proves signer + ten fields, not which commit). Under (c) the node must carry its subject's CID; that is a signed-shape change and needs a schema version, which G2's strict decode rightly refuses today.
- **Finding → handoff (brit `covenant.rs`, `epr covenant`):** a purpose clause does not cite the affirmation it is honoured under; `covenant show` derives "declared under an honoured affirmation" from the clause list, not from the clause's bytes.
- **Substrate link:** brit's canonical `EprMeta` / `NodeSeed` work is the natural home for (a) a governance package as data with a CID and (e) `established_by` as a CID citation.

## Slices

Worktree `.worktrees/brit-daily-driver`, branch `feat/daily-driver-evidence-layer` from pin `76ba95d8a`. Slice legs run in scratch worktrees under `/var/tmp/bdd/wt-<slice>` and are cherry-picked in.

| # | Slice | Files | Test first | Implementer → reviewer |
|---|---|---|---|---|
| G1 | Open from final trust (upstream-marked gix commit) | `gix/src/open/{mod.rs,options.rs,repository.rs}`, `gix/src/repository/location.rs`, `gix/tests/gix/repository/open.rs` + 41-pack fixture | Assume(Reduced)+GIT_CONFIG_GLOBAL safe.directory → trust Full, 45 slots (fails today 32≠45); explicit-slot companions; live `brit status --no-write` at /projects/elohim; B2 hooksPath retested separately | Codex sol → Claude |
| G4 | `brit-build-ref signer --json` (load-only, never mints, no keyPath/seed); atomic 0600 key publish (O_EXCL+hard link, loser loads winner); `common_git_dir` refuses instead of guessing; `meta_cmd` uses it | `brit-epr/src/engine/signing.rs`, `brit-build-ref/src/{main.rs,signer_cmd.rs,git_dir.rs,meta_cmd.rs}` | no mint; agentId equals put's; two racers → one key; linked-worktree seal verifies from both | Codex sol → Claude |
| G5 | Plain commit refuses when `commit.gpgSign` truthy, before hooks/writes, naming `git commit -S` | `gitoxide-core/src/repository/commit.rs` | true/yes/on/1 via config and `-c` → refusal, HEAD/index/logs unchanged, hook not run; `false` commits | Codex sol → Claude |
| G7 | Named git bridges: `brit <reset/rm/mv/show/stash/cherry-pick/revert/checkout/notes/submodule update…>` prints `run: git <name> …`, one exit code, no fallback; README owned-vs-bridged table | `src/plumbing/main.rs` (pre-clap table), README | each row prints + leaves repo unchanged; no row shadows an owned verb | Sonnet → Claude |
| G2 | Verified reads: `read_verified<T: Signed>` (original bytes, strict decode, verify, bind step/inputsHash, emit from typed node); covenant CID+sig check; object-store CID check; failure ≠ absence; delete reach | `brit-epr/src/elohim/{refs.rs,attestation/*}`, `engine/object_store.rs`, `brit-build-ref/src/*_cmd.rs`, tests | flipped-success / step-swap / unknown-field dropped; foreign-key returned under own agentId; valid red admitted; JSON-note round-trip pinned; git failure → non-zero; live corpus 90/90 + 24/24 | Opus → Sonnet |
| G3 | CAS writes: scratch ref + `update-ref <new> <old>`, retry only if the target entry is unchanged, else refuse naming expected/observed; covenant create-only; zero oid from object format; put_* accept only verified nodes; Ok only after the published head reads back through the verified path (a write ack is not evidence — Nostr Compass T2); neutral committer | `brit-epr/src/elohim/refs.rs`, `*_cmd.rs` | barrier-controlled races: different commits both survive; same-commit change refuses; blob stale/zero races; tampered node refused pre-git | Opus → Sonnet |
| G6 | LFS refusal scoped per path through gix's attribute stack (fail closed on unreadable/macro filters) at all five sites | `gitoxide-core/src/repository/{index/authoring.rs,worktree_restore.rs,switch/linux/{mod.rs,pull.rs},replay/mod.rs}` | global LFS + no attributes → matches `git add` oracle; `*.bin filter=lfs` → refuses, index unchanged | Opus → Sonnet |
| G8 | Probe ladder `scripts/daily-driver-probe.sh [--json]` (R1 status · R2 timed verify · R3 log==git · R4 scratch add/commit oracle · R5 under LFS · R6 find 0 unverifiable · R7 signer no-mint; every rung prints ok, REFUSED or `unmeasured: <reason>` and the ladder fails on a silent rung — Nostr Compass's "a skip is legitimate, a silent skip is not"); habit DELTA unwired→red; brit CLAUDE.md drift; daily-driver.md LFS boundary | brit `scripts/`, `.epr-meta/brit-native-daily-driver.habit.md`, `CLAUDE.md`, `docs/plans/2026-09-28-daily-driver.md` | probe red on installed 0.1.5, R1–R7 ok on sprint binary | Sonnet → me |
| G9 | Survey + handoff revision; measure monorepo `brit verify` wall time (the pre-push Stage 1a leg will now do real work) and hand it to the CK owner | `genesis/research/ngit-…md`, `genesis/data/timeline/backlog/brit-daily-driver-handoff-2026-10-27.md` | — | me |

**Order:** Wave A = G1, G4, G5, G7 in parallel (Codex/Sonnet write, no cargo), then a serial Claude gate+review per slice. Wave B = G2 → G3 (refs.rs) and G6, after A is integrated. Then G8, G9, final verification.

**Not this sprint, and not open:** key custody. Decision D5 (steward-space landing-plane spec, 2026-10-27) settled it: brit keeps its own per-device signing key; deriving it from the device key was rejected (it would merge two key domains and couple brit-key rotation to device enrolment); at S1 provenance is the admitted device whose signed head carried the record; a `binds-signer` row is the S2 entry condition. G4 names and publishes that key; it does not move it. (The Nostr Compass survey's Q3 is answered by the same decision.) notes transport; proposal/landing-intent notes; anything in the CK/S2d' set; porcelain parity; reach replacement; BritCid JSON form change; register census of submodule habits + a2o feature (needs parent `.claude/scripts/_lib/epr_habits.py` and `genesis/a2o` — handoff).

## Execution rules

- Codex legs via Haiku wrappers: `gpt-5.6-sol`, `-s workspace-write -C <wt>`, `-c sandbox_workspace_write.exclude_slash_tmp=true -c sandbox_workspace_write.exclude_tmpdir_env_var=true`, `TMPDIR=/var/tmp/bdd/<leg>`; codex runs no cargo and no git; check `ls -d /tmp/.git` after.
- Builds: `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/brit/brit/dev`, `RUSTUP_TOOLCHAIN=1.96.1`, `RUSTFLAGS=""`, `CARGO_BUILD_JOBS=4`, plain `cargo test -p <crate>` with `EXIT=$?`, scoped clippy `-D warnings`, nightly fmt scoped to `brit-*` crates only, `cargo +stable fmt --check`. One cargo at a time (the shared target lock serializes).
- Every integrity slice gets a Claude-family adversarial review before its commit lands.

## Verification

- `brit status --no-write`, `brit log`, `brit verify` succeed at /projects/elohim with the sprint binary; timed.
- Per-slice tests; existing suites of touched crates; scoped clippy + fmt.
- Live corpus: 90/90 build notes and 24/24 covenant refs verify through the new read path.
- Probe ladder all ok on the sprint binary.

## Publication (operator)

Commit-only on `feat/daily-driver-evidence-layer`. Push to `run-ci/*` for GitHub "Tests pass", merge to brit main (with producer `2d4acccb08`), parent pin move, and bin-store follow each need your go. Meanwhile the operator can use the local binary via `BRIT_BIN`.
