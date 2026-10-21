---
id: placement-c4-brief
title: "C4 — eprfs: slice-1 renames, the tier, reach and view interfaces, eprfs-serve, eprfs-ts, and the eprfs seam governance (E4)"
status: Active
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#27
actor: agent:implementer@claude-opus-5-5
habits: [materialize-what-is-read]
cites:
  - "placement-contract-architecture-plan | The placement contract's architecture | sha256:5417b3363dffdc44 | path: genesis/docs/superpowers/plans/2026-10-20-placement-contract-architecture-plan.md"
  - "placement-contract-interfaces-design | The placement contract's interfaces | sha256:c70878179c5ed5f9 | path: genesis/docs/superpowers/specs/2026-10-20-placement-contract-interfaces-design.md"
  - "what-every-app-inherits-placement-contract-design | What every app inherits | sha256:fe10f9dc966c2e9a | path: genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md"
---

Item C4 (gap `#27`). Build what the interfaces spec's §1.2-§1.4, §2.A, §2.B (and its table of adopted slice-1 types), §2.C types that live in eprfs-core, §2.H for eprfs-ts, §4 name for this lane, true to the spec's
signatures; where the code proves a signature wrong, fix the code to the spec's intent and record the
revision in your report so the spec can absorb it. Write-set for this lane: `elohim/eprfs/**`.

Apply decision 12's slice-1 names (`Coverage`, `NotHeld.tried`, `Withheld`, public constructors) and the `#[non_exhaustive]` marks; add the defaulted `EprfsStorage` methods, `tier.rs`, `reach.rs`, `view_decl.rs` and the R-E1/R-E2/R-E3 rails; create the `eprfs-serve` crate (TierEndpoint, reason labels, `testing` feature with `assert_tier_contract` and `CountingTier`); create the pure `eprfs-ts` package with `fixtures/tree-vectors.json` shared by Rust and TypeScript tests. Then plan item E4: teach the view-walk seam in `eprfs-core/.epr-meta` (extend, do not duplicate, its existing rule), add seam-registry rows for the new decision points, and move `elohim/eprfs/.epr-meta/materialize-what-is-read.habit.md` from unwired to red with a `checks:` line naming the headline walk test, re-projecting with `.claude/scripts/habits-project.py`. Gate: `just gate eprfs`. The slice-1 code's last fix round (diff_trees externals on the spine, result-path metering, Incomplete assertions) has had no reviewer yet; this lane's reviewers cover it.

Rulings in force: the design spec's rulings 1-7 and §7; the orchestrator's R1-R6 and R6a (listed in
`task-placement-interfaces-brief.md`); the interfaces spec §5 resolutions; and the operator's answers of
2026-10-20: the three crate moves are approved (placement-core crate, eprfs-core admits seam-contracts,
elohim-storage may depend on eprfs-core and eprfs-serve); incumbency is scoped to the parameter set in force
and the built-in default set is exempt from ratification; the app declares closure, loss and signing as
needs and the space resolves them; the write-set covers interface code and the first adapters.
Architecture before landing: no staging, commits or pushes. Claim the cargo berth before any cargo run
(`berth claim cargo --class verify --ttl 3600`), release it after, echo `EXIT=$?` on its own line after every
run, and never judge a cargo run from piped output. Never cd into a submodule or worktree. Pin
`RUSTUP_TOOLCHAIN=1.96.1` for manual cargo; prefer `just gate <project>`. Running behavior must not change
except where this item's interfaces spec section says an adapter replaces a mechanism, and then only behind
the same tests passing.

Report: `task-placement-c4-report.md` in this directory with `gap:`, `actor:` and `status:`, listing files
changed, tests added, gate commands with their `EXIT`, and every place the code and the interfaces spec
disagreed.
