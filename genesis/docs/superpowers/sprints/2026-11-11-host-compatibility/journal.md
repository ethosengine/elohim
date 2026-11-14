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
- A push from a fresh worktree runs the MAIN checkout's `.husky/pre-push`: the
  hooksPath shim resolves the script next to itself. Copying only `h` and `pre-push`
  into `<worktree>/.husky/_` and pushing with `-c core.hooksPath=<worktree>/.husky/_`
  runs the worktree's own hook. This is a candidate gate-preflight check.
- A TMPDIR under `/projects` (mode 2775) inherits setgid, so the orchestrator's
  `hc-mesh-initialization-continuation.test.sh` sees 2700, not 700, on its private
  directory and fails silently inside `test-suite`. `chmod g-s` on the TMPDIR fixes
  it. Another candidate preflight check, beside the in-repo TMPDIR one.
- The first gate run also exposed an A1 defect: the host-contract manifest did not
  watch its Cargo path-dependency closure (elohim/epr, crates/elohim-address,
  elohim/Cargo.toml). Fixed in 6667cc748.
- The bin-store `epr` (bafyreibrqq…) predates abd66bdb1, so the orchestrator's
  gate-evidence test fails with it. Gates ran with EPR_BIN at the pool gate build.

## Steward asks for the morning
1. A parked local rebase (stasis 3dd97e3d…:17): one unpushed shem commit was
   replayed onto hostcompat/a1 after `brit rebase` failed. No shared ref was rewritten.
   Does this kind of local replay need a standing ruling?
2. The README blind-reader loop (above) stopped at round 4. Defer, or reopen?
3. Sysroot grant (lane K6): shem serving its v2 sysroot to ethosengine needs a new
   `berth.artifacts.serves` grant. It was not added, so the fetch refuses as not granted.
4. Compiler closure (lane K): `cc1` and `as` differ between shem (an AlmaLinux 10.2
   x86_64_v2 rebuild) and ethosengine; `crtbeginS.o` is the same. Option A: one
   CID-pinned gcc closure mounted beside each native toolchain. Option B: distinct
   variants per compiler, sharing a family but never a key.
5. Lane K review (Codex astra, quotes grep-verified, relayed to ethosengine): a fixed
   LINK_INPUTS list; host tools in the key input rather than the method; the family
   keeps host tools; Mismatch on tied holder counts. ethosengine judges each.

## Open-items ledger, agreed between shem and ethosengine (2026-11-11)

At the steward's request the two devices assigned every open item an owner and a
working posture, so none is left open. A steward ruling overrides any posture.

| # | Item | Owner | Posture or decision | State |
|---|---|---|---|---|
| 1 | Parked local rebase (stasis :17) | shem | No standing ruling asked; shem integrates by merge only, never a local rebase | adopted |
| 2 | A1 README loop close | shem | Round 5 fresh read: 0 correctness, 3 interpretability; its glosses applied (psABI, DAG-CBOR, `compile_fail`, toolchain), with no fresh read of them; closed under the 2026-10-20 no-fixed-point ruling. Rounds: 0/4/0, 0/3/2, 0/3/2, 0/2/3, 0/3/0 | closed |
| 3 | Who may draw a floor (was: sysroot serves grant) | steward rules; ethosengine designs | Changed at the steward's direction: no grant row. Spec §7.8 (6e8c1e2a1) puts reach on the object: every Produce carries `env:reach`, the peer gate serves when the requester's relationship meets it, and `berth.artifacts.serves` becomes a narrowing list. The sysroot-pack grant proposal is withdrawn. New rulings **H12** (reach on the object) and **H13** (compiler posture B). Implementation is lane K, held until H12 is ruled | ruled 2026-11-12 (H12 option B, H13 posture B; `rulings-2026-11-12.md`); K12 may start, not yet built |
| 4 | Compiler closure, A or B | ethosengine (lane K designer) | **B**: per-compiler variants share a family, never a key. A (a CID-pinned gcc closure as a seed member, X1) is a later optimisation | ratified 2026-11-12 (H13) |
| 5 | Lane K review findings | ethosengine | (1) HIGH, fixed by deriving keyed libs from rustc's native-static-libs; (2) host tool lines stay in the key input until K6 slice 2; (3) the family carries rustc/cargo identity plus target.* only; (4) a tie refuses rather than guess, open to the steward; (6) the daemon serves its own CLI, which derived the key | dispositions in the K DELTA |
| 6 | ethosengine's extracted vectors | shem | Raw extraction kept under `tests/vectors/extracted/`; two derived vectors: epr on a synthetic v3 host (permit) and on a synthetic v2 host (refuse; missing avx avx2 bmi1 bmi2 f16c fma lzcnt movbe osxsave). A mutation check confirmed both are evaluated | landed with this commit |
| 7 | cc -march second F2r split (per-build CFLAGS) | ethosengine | K6 slice 2 runs the `-dM` probe with the realised per-build CFLAGS | planned |
| 8 | A2-A4 (ark) | unassigned | Neither covenant covers `elohim/ark`. Whoever first holds a covenant with that write-set takes it and notes the other | taken by ethosengine 2026-11-14 (covenant "Lane A, ark write-set"); A2, A3 and A4 part 1 on dev at landing 35 |
| 9 | K3 part 2, K6, K7 | ethosengine | Lane K continues | in flight |
| 10 | Preflight checks: worktree hook path, setgid TMPDIR | next covenant covering `genesis/orchestrator` | Backlog row below; neither device's write-set covers it tonight | row ready |
| 11 | `epr flow claim` refused from a worktree | next covenant covering the backlog | Backlog row below | row ready |
| 12 | shem's leftover worktrees | shem | hc-k12 (unused K partial, superseded) and the merged hostcompat, hc-a1, hc-stories and hc-review are removed after this lands | after landing |
| 13 | DELTA label collisions | both | Read the atom's top label just before writing; land DELTAs only in the landing commit | adopted |
| 14 | Residual findings in device-declared-target scenario 2 | shem | Folded into the S2 step-binding work when lane V/P binds the steps | deferred to binding |
| 15 | ethosengine's merged worktrees | ethosengine | Remove the merged ones; keep hc-k6 and hc-k1c until they land | in hand |
| 16 | Explicit `-C linker` plus bare `-fuse-ld=lld` may lose the rustc gcc-ld shim | ethosengine | K6 slice 2 | planned |
| 17 | Sibling notes dropped by an exact-id filter | ethosengine | Watcher now matches by prefix | done |

### Finding after the ledger: build notes do not cross device versions

The vectors gate run could not reuse genesis, steward-node or elohim-storage evidence:
"unverifiable note under this step … does not decode: unknown field `body`". The
notes at those commits are epoch-2 records (`{"body":…,"epoch":2,…}`) that the
decoder this device's gate ran with does not accept, so a green result recorded under
one format cannot stand in for the other. All three gates ran in full and passed.
Owner: lane K (ethosengine), as the brit build-note reader is its seam; noted to it.

### Backlog-ready rows (to be filed by whoever holds the backlog write-set)

- **gate-preflight: two worktree traps.** (a) A push from a fresh worktree runs the main
  checkout's `.husky/pre-push`, because the shim resolves the script next to itself.
  A check should refuse when `core.hooksPath` resolves outside the pushing worktree, or
  name the cure (copy `h` and `pre-push` into `<wt>/.husky/_` and push with
  `-c core.hooksPath=<wt>/.husky/_`). (b) A setgid TMPDIR (anything under `/projects`,
  mode 2775) makes `hc-mesh-initialization-continuation.test.sh` see 2700 and fail
  silently inside the orchestrator `test-suite`. The check is `stat -c %a "$TMPDIR"`
  having no setgid bit. Habit `careless-reds-cost-seconds`.
- **epr flow claim from a worktree.** `epr flow claim` and `epr actor claim` refuse with
  `envelope-checkout-mismatch` when run from a linked worktree, because the session
  envelope is bound to the main checkout. That checkout's flow store may also lack items
  minted only on the worktree's tree. Plan items in a sprint that runs in worktrees go
  unclaimed. Habit `sprint-lands-each-closure-once`.

## K6 slice 2 on shem: the v2 floor, and a second-device K8 extraction (2026-11-12)

Steward direction, relayed by ethosengine at 23:50Z and confirmed by the steward ("ethosengine
sent you what you can pickup now"); recorded as a purpose clause, covenant
`bafyreicxdtrpc4m24rijpndwqjurerftwxayka4o5pbbwa63nawz6vvcl4`. ethosengine took K9 then K11.

**The floor.** Only shem can supply it: ethosengine's RHEL 10 glibc declares v3. The file set
comes from the installed packages' own lists (`rpm -ql`), not a hand list: glibc-devel,
kernel-headers, the shared libraries of glibc and libgcc, gcc 14.3.1's support objects under
`usr/lib/gcc/x86_64-redhat-linux/14/`, and `lib64/` copies of what the linker scripts name
(`evidence/k6-v2-floor-assemble.sh.txt`). Packed with an epr built natively at dev d32131dfa:
1,655 members, three linker scripts, libc 2.39. ISA notes: 28 members need baseline plus v2,
20 carry none, none is above v2. Two packs were byte-identical. Recorded with
`epr artifact sysroot put --cross` and a declared requirement (arch x86_64, os linux,
`abi.libc` at least glibc 2.39, `isa.x86` within the v2 set): output
`bafkr4ic3gyme6ajwr7ctshk4jbarcby7vss75fhviiugl477rv4weqbhxq`, key
`bafyreih25tuk4bubq7672wf44zi3ee63hp72elmjiv3ldo3gznorshh4qi`, `env:requires`
`bafkr4igfy7nkuttfkd2qq7dofxipermlhcykqvv7txlhyq74s4ptmx2cv4` (`evidence/k6-v2-floor.json`).
The first put omitted `--requires`, so the output has two Produces from shem, one without the
slot. Serving the floor to ethosengine waits on H12. gcc's support objects do not move with
`--sysroot` alone; a builder points `-B` at them.

**The extraction.** K8's measurement of that natively built epr (136,592,296 bytes, 9.6 s)
yields Requires `bafkr4ic76hvp5ozpc2izyuhug6fqiwrdxcpndalzemmiygdzakucicfccy`, byte-identical
to ethosengine's (`evidence/k8-extraction-shem-built.json`). The equality hides different
causes. shem's build has no `lzcnt` and no `f16c` anywhere: its C code (sqlite) was compiled
for v2, where ethosengine's carried v3 sqlite evidence. The v3 level here is selected by
features that live in runtime-dispatched code: chacha20's AVX2 backend and `core::arch`
intrinsics (`avx`, `avx2`, `osxsave`), ring (`adx`, `bmi2`, `aes`, `pclmulqdq`, `vaes`,
`vpclmulqdq`, `movbe`, `sha_ni`), sha2 (`sha_ni`), and compiler_builtins (`fma`, `fma4`).

**Two findings for the K8 owner.** (1) `bmi1` is counted from 1,281 sites, and 1,277 of them
are `tzcnt` in hashbrown and similar code: LLVM's baseline `rep bsf`, which executes as BSF on
a CPU without BMI1 and is emitted only where the two agree. The remaining 4 genuine BMI1
operations are in ring's `__bn_postx4x_internal`, a dispatched path. Like ENDBR, the encoding
alone is not a requirement. (2) This device's CPU (Xeon E5-2667, Sandy Bridge) has `avx`,
`aes` and `pclmulqdq` but no `avx2`, `bmi1`, `bmi2`, `fma`, `adx` or `sha_ni`. The epr built
here ran the pack above on this CPU, yet admit would refuse it against this host's observed
features (by the clause; no Provides observer exists until A2), until K10 proves the
dispatch. That is the strongest case for K10 the sprint has measured. These are K8 and K10
facts; neither changes the host-contract crate.

**Branch push without the hook (steward ruling, 2026-11-12).** ethosengine was waiting on
these docs, so at the steward's direction the docs-only branch `hostcompat/k6-v2-sysroot` was
pushed to origin at 89e42f17e with `--no-verify`. Two hooked attempts failed first. One saw
`EPR_BIN` vanish from this device's own new cargo-pool slot (`family/hc-k6v2`) between the
build and the gate. The other had one orchestrator red, `gate-runner.test.mjs` "gate evidence
rejects dirty external dependency…" (`recordEvidence` returned 1). That test passes alone,
with either epr build, and under each of the hook's variables tried (`GIT_CONFIG_PARAMETERS`,
`GATE_EVIDENCE`, `CARGO_TARGET_DIR`, `GATE_RUN_KIND`), while Codex 01a1cbeb's gates ran
alongside. Re-gating was blocked by the one-build guard during that session's elohim-storage
gate. Cause not found; the dev landing runs the full gate. The bypass covers only that branch.

## Shem handoff (2026-11-12)

State, open decisions and the traps found on 2026-11-12 are in `handoff-2026-11-12-shem.md` beside this journal. The K9 receipt is one serve-side manifest defect from done.

## Lane A on ethosengine: A2, A3, A4 part 1 (2026-11-14)

Steward direction (2026-11-14, ~04:40Z): while shem shakes out the pipeline, carry the sprint
forward outside eprfs; lane A chosen. Covenant recorded this session: steward of record, and
purpose "Lane A, ark write-set" (`epr covenant` clauses bafyreihdpelj… and bafyreidbdfux…, the
second superseding a wrong preflight-script glob). Overnight the steward made this an in-session
shift with shem as integrator. The shift launcher (`just dev shift start`) was not used: it needs
native flow commitments for these items, and this sprint's items were never claimed (see the
worktree-envelope note above).

- **Spec first.** c7560b026 states the own-build rule in spec §3.2 (ruling condition 6) before any
  code relies on it.
- **A2 and A3** (81605d283, 179ef207b, implemented by Codex astra, reviewed by a Claude reviewer):
  ark observes `Provides` at supervisor start (uname, CPU features from std detection and CPUID
  with OS state checks, the running glibc, loader paths for x86, aarch64 and riscv64) and keeps
  it in memory. `ark host provides` prints it locally. Candidate admission replaces the
  platform-string comparison with `admit` over a caller-resolved `Requires`. A missing
  requirement, or one without `arch` and `os`, refuses. The own-build rule waives only unmet,
  observed, extracted `isa.x86` clauses, only with an `OwnProduce` naming this device's Produce
  CID for that exact executable, and never for a withdrawn artifact. Recordable verdicts carry
  required clauses only (golden test). 33 extractor-emittable features ark cannot observe soundly
  (virtualisation, SGX, AMX, CET, MSR-gated) refuse as `unobserved`; a test reads the
  extractor's table (`// gate-reads:`) so a new name cannot slip past. `just gate elohim-ark`
  EXIT 0 (227 tests plus 3 doctests), `just gate rust-style` EXIT 0, ark debt 0 to 0. The review's
  low findings were fixed except one, kept by design: an admitted verdict lists the satisfied
  clauses (spec §3.3). For the contract crate's owner: `Provides` derives `Debug`, so a `{:?}`
  would print the inventory.
- **A4 part 1** (238528a6a): `ark host admit --requires <file>` prints one admitted or REFUSED
  line naming the axis and the required value, never the host's; a declared `avx512f`
  requirement is refused here. The first A4 attempt wired preflight by re-implementing the flow
  log and payload-store layout in Python inside `hc-mesh.sh`; refused and removed, since it
  duplicates eprfs's format outside its owner. **Part 2 is blocked** on an epr verb returning a
  recorded output's requirement; asked of shem as eprfs owner.
- **Gaps met.** `brit worktree add` panics "Entry too large to fit in memory" (it creates the
  branch first; `git worktree add` attached it). `brit fetch origin dev` finds no configured
  remote in a worktree. `brit commit` still refuses the worktree hooksPath. `just dev prep`
  clones submodules only from the main checkout, whose brit, rakia and sophia lag dev's pins;
  the pins were fetched from the rg2-rust worktree instead (an E4 follow-up for prep). Codex
  astra hit its usage limit (until 2026-11-18) during the A4 rework; a Claude agent finished it.
- **ark-core stays contract-free** (240693e34). The first branch push refused at the orchestrator's
  closure tests: ark-core depending on `elohim-host-contract` pulled the contract crate into the
  closures of every workspace that reaches ark-core (storage and its Docker context, the k8s bridge,
  eprfs, steward/node). Host admission moved into `ark-supervisor`, which nothing outside
  `elohim/ark` depends on. `authenticate_candidate` takes the supervisor's `CompatibilityEffect`
  and refuses it fail-closed (no effect, an effect for other bytes, a refusal). No other
  workspace's lockfile, manifest or image changed.

## Steering review (2026-11-14, coordinating session a5b5548c)

Asked by the steward: where the two devices stand against the plan, a brief review of lane A,
verdicts on the open questions, and steering. Read at dev b1c788353. The plan carries the
resulting edits (the corrected sibling-verdict ruling, A3's state, and items K13, A5, G1, G2).

**Where the plan stands.** S1 to S3, A1, A3 and K9 are closed. A2 waits on shem's
`ark host provides` print. A4 part 2 waits on K13. K1 to K8, K10 to K12 are unticked in the plan
although the plan's own note says K1, K2, K3 part 1, K4, K5 and K6 slice 1 are on dev: their
owner ticks each one against its verify line, or restates the line. K1's verify still says the
two devices' keys are equal, which H13 ruled they are not. Lanes V, P, F, R and C are unstarted.

**Open questions, answered.**

| Question | Answer | Evidence |
|---|---|---|
| Does gate reuse accept a sibling's green yet? | Yes in code since 2026-10-27, behind five fences. It has never fired on a real push | `gate-runner.mjs` `siblingVerdicts` call; habit `push-gate-pays-once` DELTA 2026-10-27; no `independence: sibling` line in any push log on shem |
| Are H12 and H13 open? | No. Both ruled 2026-11-12. K12 may start and is not built: `env:reach` appears only as a declared gap | `rulings-2026-11-12.md`; spec §12 |
| Do the declared gaps block daily use? | No. `reach` closes with K12; build-scripts and dependency-sources close with K6's pinned closure; compiler-inputs closes with a build receipt (K11). Each prints as pending | plan K11 |
| Is one landing lane the bottleneck? | The lane is right; the hold is too long. The author gates the exact commit before claiming the lane, and the lane then pays only what reuse cannot cover. G1 measures the hold | plan G1 |
| Which gates run as placed tasks? | Only the orchestrator gate and `just measure`. No more are moved until P2 puts admission inside `ark task run`; then one cargo gate at a time, `elohim-ark` first | plan P2 |
| Rust 1.96.1 | The gate pin on dev is already 1.100.0 (`pool-policy.json`). What remains is the holochain fork's own toolchain file and brit's pin. It retires with those pins and blocks nothing here | `pool-policy.json` `cargo_env_overrides` |
| Reviews while Codex is out (until 2026-11-18) | Claude seats, with the reviewer given the code and not the author's conclusion. Blind-reader rounds are unaffected | memory note, shem |

**Review of lane A (d12a28be, 4277de3e, f468a0ba, 240693e3).** Read by a separate reviewer and
then checked in the source. Sound: the CPU probes (OS state gating for AVX and AVX-512, the
leaf limit, the unprobed list held to the extractor's table by a test), the waiver logic (only
an unmet, observed, extracted `isa.x86` clause waives, and any other failed check clears every
waiver), and the shape of `admit_artifact` (a pure function over CIDs, a requirement and the
observed host, with no checkout, mesh or CI assumption, so placement and a devspace launcher
can call it). Not sound yet, and latent because nothing calls it outside tests: the effect and
the own-build claim are forgeable by a caller (plan A3 state, item A5). Keeping ark-core free of
the contract crate was the right call for the closures, and its cost is that the trust boundary
now rests on those two types, so they must be closed by construction. `ark host admit` prints
`ok` for any requirement file handed to it; with K13 it takes the requirement from the record
by output CID, and the file form stays a diagnostic.

**Steering.**

1. shem: K13 first (its `self` answer is what A5 needs), then the A2 print. ethosengine: A5
   beside A4 part 2. Neither waits on the other after K13's interface is agreed.
2. G1 on the next landing, at no cost: whichever device lands next has the other gate the same
   commit first. Record the hold time either way.
3. No running path consumes an admission before A5 lands. P1, P2, V4 and R1 each name A5 as a
   precondition in their brief.
4. P1 takes the verdict by injection: placement-core declares the compatibility stage and an
   admitter trait, and ark's supervisor supplies it. Placement never holds `Provides`, and
   placement-core does not gain the contract crate, for the same closure reason as ark-core.
5. Under H13 the only reuse of native builds between the two devices is lane V. With K3's fetch
   by family landed, V1 is the next item on the critical path, ahead of more lane K polish.
6. Owners tick or restate the lane K items this week, so `epr flow project` mints a true plate.

**Work for a Fable seat.** Two items are design-heavy, cross several owners' seams, and are read
by most of what follows. Each is better designed once by one seat holding the whole picture,
then handed out as packets.
- *The record seam of lanes V and P, as interfaces only*: the `VariantSet` document (V1), the
  recordable admission verdict (A3, A5), and the commitment and fulfilment written as epr-rea
  flow events on the plane `RecordPlane` chooses (P2 notes the placement drafts reach no store
  today). V1 to V4, P1, P2, F3, R1 and C1 all read these three shapes, across ark-core,
  eprfs fetch, placement-core and epr-rea. Deliverable: types, seam-registry rows and vector
  tests, no behaviour. It unblocks both devices to build V and P in parallel.
- *K12*: the serving gate gains a second path (reach on the object beside the method rows),
  with a derivation of the narrowest input reach and an exit criterion for the old path. It is
  the one item here where a mistake widens who can draw bytes, and it unblocks the K6 floor
  draw, K7 and K11's `reach` gap.
K10 step 3 (the dispatch certificate) is the hardest item in the plan and is correctly not
started: its trigger has not been measured.

