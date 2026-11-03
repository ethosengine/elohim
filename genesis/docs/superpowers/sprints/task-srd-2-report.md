---
id: task-srd-2-report
status: DONE_WITH_CONCERNS
gap: plans__2026-11-02-shared-rust-discipline#2
actor: agent:implementer@codex
class: implementation-report
created: 2026-11-02
---

# Repository-authored imports of portable policy content

The existing version-1 registry accepts opt-in `content: { cid, path }` rows.
Each row retains its local id/version, class, scope and established_by. Its
existing contentHash pins the authored pointer and standing before content is
read. No source-space standing is inherited. This is local repository configuration,
not independently witnessed adoption, receipt or cross-space authority.

The path is repository-relative, rejects parent/absolute traversal and symlinks,
and reads a bounded regular file. Raw CID, canonical format, semantic identity,
field support and predicate shapes verify before expansion. Initial supported
portable predicates are validator and require-frontmatter. Local shadowing,
unknown semantics, bad scope, missing/tampered pins or content, missing bound
rows and duplicate identities produce Ask verdicts. Valid legacy row parsing
is unchanged; missing/duplicate/malformed bound rows no longer silently permit.
No validator executes merely because bytes were imported.

Public format: eprfs_meta::PolicyContentRef. The type's documentation gives the
minimal authoring contract. Shared content remains unchanged when repositories
choose different scope or establishment provenance.

## Evidence

Base d7dc62fdd328f3c50b87ded46b34f599802a97bb. No commits or publication.
Native commands use Rust 1.96.1, empty RUSTFLAGS, CARGO_BUILD_JOBS=1 and the
existing eprfs development pool. Cargo berth is claimed for each invocation.

Gate-evidence: context selected `just gate eprfs`; parent reports it refuses
before recipes because an existing local proof checkout duplicates a gate
project. The fixture is unchanged. Required final gate remains pending;
focused evidence does not claim full delivery or network acceptance.

Package version: changed publishable eprfs-meta is explicitly 0.3.3. The rest
of the workspace remains 0.3.2; epr-cli is publish=false. Local path consumers'
locks must reflect that version before final verification.

Focused final evidence (2026-11-02):

- `cargo test --manifest-path elohim/eprfs/Cargo.toml -p eprfs-meta --lib`: 34 passed, `EXIT=0`.
- `cargo test --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli canon_lift::portable --lib`: 3 passed, `EXIT=0`.
- `cargo fmt --manifest-path elohim/eprfs/Cargo.toml --all --check`: `EXIT=0`.
- Seam registries validate against the existing JSON schema.
- Cargo offline targeted lock updates for rust-style, cargo bridge and Jenkins bridge: each `EXIT=0`; each diff changes only eprfs-meta 0.3.2 to 0.3.3. The eprfs lock updated during tests.
- Initial compilation caught unavailable BlobCid FromStr; corrected to existing BlobCid::parse before passing tests.

The initial 33-test run exposed no failure; subsequent source review added a
34th regression proving raw YAML parse failure cannot let shadowed local
semantics materialize. Repeated verification was justified by that final fix.

Final review additions and proof:

- Successor discovery regression uses two separately authored local provenance rows and the existing recording validator provider. Placing and verifying a version-2 object changes neither pin and invokes no validator; an explicit repin in one repository activates version 2 only there, while the other retains version 1.
- Final meta library suite: 35 passed, `EXIT=0`.
- Final portable CLI suite: 4 passed, `EXIT=0`, including atomic index replacement refusing a preexisting symlink without changing its legacy-registry target.
- `cargo clippy --manifest-path elohim/eprfs/Cargo.toml -p eprfs-meta -p elohim-epr-cli --all-targets -- -D warnings`: `EXIT=0` after the final fixes. Existing dependency ts-rs/vendor warnings remain nonfatal; no suppressions added.
- Both updated seam registries validate against their schema.

Resource coordination residual: child agents share the runtime berth identity, so one claim renewed another child's lease instead of detecting concurrency. A short overlap with the rust-style recipe was reported to the parent; subsequent checks were explicitly serialized by message. No result was inferred from the lease. All actual process exits above were read directly.

The pooled native epr executable predates the importer and final index guard;
consumers must rebuild it before exercising those features outside test binaries.
No publication, remote adoption, or independent acceptance is claimed here.

Final workspace `cargo fmt --manifest-path elohim/eprfs/Cargo.toml --all --check`: `EXIT=0`. Cargo berth released; no build remains in this seat.

Parent integration follow-up: rebuilt the pooled native `debug/epr` from the final
source with `cargo build --locked -p elohim-epr-cli --bin epr`, `EXIT=0`.
That binary verifies the offered raw policy CID and reports legacy canon fresh
(42 atoms / 31 heads). It now includes the importer and index guard; the shared
installed PATH binary was not replaced. Independent source review approved these
bounded changes. Publisher precheck is green; full owning gate remains unresolved.

## Continued implementation: complete checker artifact closure

Following the steward-authorized micro council, the existing native checker Policy
schema now accepts optional `discipline: { policy: { cid, path }, guide: { cid,
path }, method: { cid, path } }` and `scope_roots`. Existing version-1 policy
syntax remains accepted. This is repository-authored standing; it is neither a
network receipt nor delegated peer-verifiable scope authority.

The portable policy addresses the full agreed discipline: guide, method,
automatic RS-GOV-04 coverage, no-new enrollment, later per-touch paydown, strict
scope persistence, meaningful-rationale review and limits of automatic review.
The installed checker compares the verified method to its own canonical method
descriptor before scanning. No object supplies executable code. The method
identifies checker sources/manifests/lock, first-party eprfs-meta/core and
seam-contracts source/manifests, root Cargo configuration, and the verbatim
workspace inheritance tables preserved by standalone source distribution.
The std-only build script embeds that inventory at compile time; evaluation
needs no source checkout or generator. A source archive separately identifies
its complete distribution bytes. Registry publication is explicitly unsupported
for this source-layout-dependent method and remains disabled.

`--export-discipline --guide FILE --out DIR` writes an offer, not enrollment.
The exported `discipline.json` contains paths relative to the export directory;
local adoption explicitly places those held bytes and authors repository-relative
paths. A guide may stay at its visible documentation path. `--head` reads the
policy and closure objects exclusively from that Git tree, rejects committed
symlinks and refuses contradictory prospective stdin mode. Local object reads
are bounded, regular-file-only and refuse symlink traversal.

Additive legacy-to-addressed enrollment is allowed only with identical existing
rules/exclusions/scopes. Pointer addition alone does not make every debt-bearing
crate touched, but ordinary source changes still owe paydown. Removing/replacing
adopted content or narrowing existing standing is refused as a reviewed migration.
A refused scope change still reports previously governed sources. Matching parsed
rule/scope/attribute debt moved into changed excluded source cannot count as
paydown; unrelated additions and unparseable excluded compile-fail fixtures remain
outside the adopted scope. This bounded semantic-tuple check does not claim to
prove arbitrary renamed or rewritten code migration. `Report.effective_policy`
names the exact locally selected closure and standing measured.

Final focused proof on frozen sources:

- `cargo test --manifest-path genesis/orchestrator/rust-style/Cargo.toml --locked`: 18 ratchet + 12 actual-Git/CLI reach tests passed, `EXIT=0`.
- `cargo clippy --manifest-path genesis/orchestrator/rust-style/Cargo.toml --locked --all-targets -- -D warnings`: `EXIT=0`.
- `cargo fmt --manifest-path genesis/orchestrator/rust-style/Cargo.toml --check`: `EXIT=0`.
- Native export against the real guide: `EXIT=0`.
- Pool: `/projects/.cargo-target-pool/family/dev/genesis__orchestrator__rust-style/dev`, Rust 1.96.1, empty RUSTFLAGS, one build job. Cargo berth released after checks.

Gate-evidence: `just gate rust-style` is the context-selected owning gate;
NOT RUN in this resumed seat because the parent explicitly schedules it once
after root/Brit adoption and standalone installation. These focused green exits
do not substitute for that integration gate or claim the Brit trial has run.

Stable acceptance names in `tests/reach.rs`:

- `addressed_discipline_is_offline_scoped_and_never_auto_upgrades`
- `committed_closure_cannot_borrow_worktree_repair`
- `missing_or_malformed_method_refuses_before_source_evaluation`

Export held at `genesis/local-dev/shared-rust-discipline/full/discipline.json`.
Policy: `bafkreidd2ije344x7jldsfitomqcc6smrnsp6bhuinn2sblfsiuth6yf6i`.
Guide: `bafkreidv7yjl7bp5gls3uwbldjp5rdni3robbnbdfkxwsjvcnjn2wpt2j4`.
Method: `bafkreidmczh47krdmia4qsv4sgmovx6cyxj4d4bjvtybpot6qb2ft36h2y`.

Integration residual: targeted offline Cargo lock refresh added checker core/yaml
dependencies as intended, but also normalized some existing dependency edges
(same package versions) in eprfs and bridge locks. The parent was told to inspect
those changes before its full eprfs gate; none was hidden or manually reversed.
The checker's own lock contains only the intended direct dependencies and earlier
meta version change. No commits or publication were made by this seat.

## Parent integration closure — 2026-11-02

The local delivery is now verified: normal `just gate rust-style`, `just gate
orchestrator` and `just gate eprfs` all returned `EXIT=0`. EPRFS has 1,970 passing
Rust tests across 130 suite summaries, zero failures and zero ignored tests, plus
workspace formatting and all-target Clippy. Six Rust-style a2o scenarios / 18 steps
pass. Brit's real offline installation and 14-file zero-debt trial pass. Earlier
pending-gate statements above describe the seat's original checkpoint; they are
resolved by this integration evidence. Canonical delivery details and limitations
are in `task-srd-5-report.md`. Network authority/witnessing and later scope/checker
migration remain open; no commit, push, remote CI or appointed acceptance is claimed.
