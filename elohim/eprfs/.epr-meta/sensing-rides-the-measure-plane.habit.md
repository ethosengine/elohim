---
epr-habit-version: 1
id: sensing-rides-the-measure-plane
invariant: >
  What the development system declares worth measuring is OBSERVED: every declared measure that a
  bound reads is folded by a declared probe through `epr flow sense`, never by a private ledger
  or a bespoke judge. Every level is judged by one limit model (`elohim_epr_rea::Bound`, read by
  the native report). A breach names its dispatch through `epr flow report --breaches`. A
  detector proposes candidates, a ruling on the candidate's own file classifies each one
  (duplication | intentional | canonical | unresolved | false-positive), and an excepted
  candidate is counted and named rather than silently skipped. A concern is a declared entity:
  a reference to one is valid when it resolves in the home its relation names, never because its
  spelling matches a grammar.
status: red
active: false
checks:
  - "cargo test -p elohim-epr-cli --test flow_sense (manifest cargo target): a count probe folds on change only; a candidates probe opens each new subject with 1 and heals each vanished one with 0, so `distinct-subjects-since-reset` reads the open set; a deleted subject drains through the declared `<id>-reset@1`; an excepting ruling closes and names its subject while duplication/unresolved keep it open; a crashed, over-budget or malformed probe writes nothing and exits 1; `report --breaches` carries the bound's concern, dispatch and outstanding subjects"
  - "epr flow report --bound limit-models-open@1 (hard 1, at-or-above, derive distinct-subjects-since-reset over limit-models@1): open candidates that judge a level outside epr-rea's Bound"
  - "epr flow report --bound concern-joins-open@1 (hard 1, at-or-above, derive distinct-subjects-since-reset over concern-joins-unresolved@1): files holding a concern reference that does not resolve in its relation's home (evidence-of → story, prices → habit, governed-by → canon, pain-of → habit or canon); spelling is never the test"
  - "epr flow report --headline `sense:` line: bounds observed of declared, breached, and numb (skipped for want of a fold) — numbness is the red mass this habit drains"
guard: >
  Risk 1 — closing candidates by ruling them away. A `false-positive` or `intentional` ruling must
  name the contract difference (inputs, direction, errors, authority, persistence, trust boundary,
  latency); a ruling whose why is "not worth it" is an unresolved one wearing an exception.
  Risk 2 — centralizing on similarity. A candidate is a question; consolidation needs the notes'
  experiment (behavioral oracle anchored outside the code being replaced, plus a restraint case
  that must stay allowed). A large line-count drop that adds indirection is not progress.
  Risk 3 — the organ becoming a bypass of itself. Probes are polyglot implementations of a prose
  procedure; the FOLD and the JUDGE stay native (note::observe, report). A probe that writes its
  own state file, or a consumer that re-judges a lens instead of reading `--breaches`, is the
  defect this habit names.
  Risk 4 — a precision number becoming a ranking score. `sense-classes@1` is shown beside the
  reasons; it never orders candidates (notes §5).
refs:
  - "spec: genesis/docs/superpowers/specs/2026-10-22-sense-organ-design.md"
  - "operator notes: autonomous-loop-refactoring-notes.tmp.txt §4–§10 (detection proposes, contract inspection decides, intentional variations are negative examples, no second backlog)"
  - "siblings: ledgers-fold-once (signature 3: private ledgers, its ledger-planes@1 now probed), governance-plane-single-evaluator (declared twins), dev-system-equilibrium (memory/drift lenses priced here)"
  - "the census this habit's first parts came from: habit-progress spec §8 (genesis/docs/superpowers/specs/2026-10-22-habit-progress-scalar-design.md, branch feat/habit-progress-scalar)"
retire-when: >
  when no declared bound reads `skipped — no fold` for want of a probe, no development-system code
  judges a level outside elohim_epr_rea::Bound, and every remaining candidate carries a ruling.
---
DELTA 2026-10-22 (RED preserved, first live reading, branch feat/sense-organ, unlanded): `cargo test -p elohim-epr-cli --lib --test flow_sense --test flow_report --test live_manifest_integrity --test governance_parity_vectors` 332 + 9 + 111 + 1 + 1 passed; `just gate eprfs` exit 0 (fmt, workspace clippy `-D warnings`, 1343 tests passed, 0 failed); the restraint oracle (genesis/agentic/fixtures/sense-restraint) holds all four legs against the built `epr`. `epr flow sense --due session` on the worktree root: ledger-planes@1 = 5, limit-models@1 = 12 open (unruled in this store; the 12 rulings live in the host store), concern-joins-unresolved@1 = 0, concern-evidence-unadopted@1 = 69 — the native port reads exactly what the Python sketch read, and a host dry-run agrees. The first headline read `sense: ⚠ 2 of 59 bounds observed · 2 breached · numb 57`, and that reading found the organ's own defect. A clean candidates probe wrote nothing, so concern-joins-open@1 read `skipped — no fold` though its probe had just seen 0: numbness posing as health. A clean first reading now opens the window with the declared reset, and flow_sense's `a_clean_first_reading_opens_the_window…` test holds this. After the fix, concern-joins-open@1 reads passed (0), and the headline reads `sense: ⚠ 4 of 59 bounds observed · 2 breached · numb 55`. Red holds: limit-models-open@1 fails at 12 ≥ 1, and 55 bounds stay numb.

DELTA 2026-10-22 (BORN red, sense-organ S1, branch feat/sense-organ): the organ is built and
unlanded. Measured on the host store before this habit existed: 57 bounds, 46 skipped for want of a
fold. First probes: ledger-planes@1 (count), limit-models@1 and concern-tag-parsers@1
(candidates). The live readings and the first rulings are recorded in the next delta.
