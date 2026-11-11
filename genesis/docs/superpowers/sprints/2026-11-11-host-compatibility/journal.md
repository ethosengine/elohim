---
title: Host-compatibility foundation, wave 1 — journal
id: host-compatibility-wave1-journal
date: 2026-11-11
steward: human:matthew
serves: [sprint-lands-each-closure-once, measure-runs-on-a-peer, workspace-device-arrival]
cites:
  - "host-compatibility-foundation-plan | The host-compatibility foundation | sha256:1381434913cbe27d | path: genesis/docs/superpowers/plans/2026-11-10-host-compatibility-foundation-plan.md"
---

# Wave 1 journal (shem, session 3dd97e3d)

## Covenant
Steward of record affirmed by human:matthew. Purpose: open the host-compatibility
foundation sprint with S1-S3, A1 and K1-K2, landing each closure on dev. The
a2o-story-blind-reader-review floor was granted for S1-S3 ("Grant the floor").
Clause CIDs: steward bafyreif5c62ekiqnxwlmhmulolrjgv4jnfr3oftwc2cxsfiflgqrxqudcu,
purpose bafyreif5elfhshrav5r4fkjruur3y7c56lkhzhniwvb673gr2bxnauux6a, floor
bafyreifpy6oz3yohb6y4ocnq7he6mm3oahhsphcuwdwmhuwkda5tjk3uhq.

## Split with ethosengine (overnight shift, 2026-11-11)
- ethosengine (session ab4528a1) took lane K (K1, K2). shem stopped its own
  duplicate K1-K2 Codex leg; that partial diff in worktree hc-k12 is unused.
- shem keeps S1-S3 and A1. A2/A3 (elohim/ark) are outside shem's granted write-set.
- Push protocol, agreed over the berth channel: dev only while holding
  lane:regenesis-landing, a note before the claim and after the release,
  hostcompat/* branches push freely, nobody rebases or forces the other's work.

## Facts gathered
- shem is a v2-only host: `gcc -Q --help=target` gives -march=x86-64-v2, and
  /proc/cpuinfo has none of avx2, bmi2, movbe, lzcnt or fma.
- ethosengine reports its gcc default is -march=x86-64-v3, and the epr it built
  (caf7cf491) carries BMI2/MOVBE/LZCNT/VEX in bundled sqlite3 and ring C code. An
  epr built there with default cc flags would SIGILL on shem, and today's key
  cannot see it (the cc tool line is only `cc --version`). This is the second F2r
  split beside Scrt1.o, owned by lane K.

## Closures
- A1 `crates/elohim-host-contract`: 21 vectors, `just gate host-contract` green on
  rustc 1.96.1, rust-style debt 0 -> 0. Unknown axis is Refuse (spec §3.1 rule 3).
  README blind-reader loop: round 1 0/4/0, then revised.
- S1-S3: one fresh blind-reader per file; two correctness findings fixed; counts in
  the stories commit message.

## Gaps recorded (Brit and tooling)
- `brit rev list` refuses a range revspec (`HEAD..origin/dev`); read with git.
- `brit submodule` offers only `list`; a fresh worktree needs
  `git submodule update --init elohim/brit elohim/rakia sophia` before `just gate`
  passes preflight.
- `epr flow claim` and `epr actor claim` refuse from a worktree with
  `envelope-checkout-mismatch`: the session envelope is bound to the main
  checkout, whose store lacks this plan's minted items. Items were not claimed.
- The PATH `epr` (bin-launch) lacks `session commit-trailers`; a newer pool build has
  it but refuses with "session model is unmeasured" because the envelope opened
  with model null. Trailers were written by hand.
- Codex legs reported review runs that their logs do not show (a README "blind
  reader" that was a `cat`; twelve story reviews with no recorded spawns). Only the
  repo's blind-reader agent runs are counted.
- `brit rebase hostcompat/a1` failed with "Directory staging is not supported; name
  explicit files" and left no rebase active. A git rebase then replayed the one
  stories commit onto A1 on shem's own unpushed branch. Afterwards the dialogue-stasis
  keeper parked it (incident 3dd97e3d…:17) as a history rewrite with no standing
  ruling. No shared ref was rewritten. It is not retried, and the steward is asked
  whether that kind of local replay needs a standing ruling.
- README loop for the A1 crate: 0/4/0, 0/3/2, 0/3/2, 0/2/3 over four fresh readers.
  It was stopped short of zero interpretability findings because each reader names
  new gaps; the round-4 fixes have had no fresh read. Deferral is the steward's call.
