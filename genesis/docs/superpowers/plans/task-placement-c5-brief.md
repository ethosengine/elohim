---
id: placement-c5-brief
title: "C5 — elohim-storage adapters: the ranking shim, the blob-store tier and the root reach gate"
status: Active
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#28
actor: agent:implementer@claude-opus-5-5
habits: [materialize-what-is-read]
cites:
  - "placement-contract-architecture-plan | The placement contract's architecture | sha256:5417b3363dffdc44 | path: genesis/docs/superpowers/plans/2026-10-20-placement-contract-architecture-plan.md"
  - "placement-contract-interfaces-design | The placement contract's interfaces | sha256:c70878179c5ed5f9 | path: genesis/docs/superpowers/specs/2026-10-20-placement-contract-interfaces-design.md"
  - "what-every-app-inherits-placement-contract-design | What every app inherits | sha256:fe10f9dc966c2e9a | path: genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md"
---

Item C5 (gap `#28`). Build what the interfaces spec's §1.3 (R-E3, R-P8), §2.A adapters, §2.D adapters, §3.1 and §3.2 name for this lane, true to the spec's
signatures; where the code proves a signature wrong, fix the code to the spec's intent and record the
revision in your report so the spec can absorb it. Write-set for this lane: `elohim/elohim-storage/**`.

Re-home ranking: `reconcile/placement.rs`'s `PlacementStrategy` becomes a shim over placement-core's `Ranking`, with the existing salvage tests unchanged and green; add the blob-store tier implementing `EprfsStorage` and the `RootReachGate` as §2.A specifies; add rail R-P8. elohim-storage is the heaviest build in the repo (`CARGO_BUILD_JOBS=1`, peaks near 5.4 GB): run it alone, after C2 and C4 have landed in the working tree, and run `just gate elohim-storage`.

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

Report: `task-placement-c5-report.md` in this directory with `gap:`, `actor:` and `status:`, listing files
changed, tests added, gate commands with their `EXIT`, and every place the code and the interfaces spec
disagreed.
