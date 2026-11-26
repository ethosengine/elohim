---
epr-habit-version: 1
id: research-takes-have-a-home
invariant: >
  Every take a research note declares has a home that can be checked: a backlog
  row that names the note, a file that exists, or a stated release. A take the
  note still means and that has no home is declared as owed, and counted. A note
  that declares nothing is unmeasured; it is not read as a note with no takes.
status: red
active: false
checks:
  - "python3 .claude/scripts/research-mint-check.py --strict --max-undeclared 46 (exit 0 = no declared take is owed or broken and no more than 46 notes are undeclared; 46 is the count of 2026-11-26 and is lowered as notes declare)"
  - "python3 .claude/scripts/_lib/__tests__/research_mint_check_test.py"
guard: >
  The check verifies what a note declares and does not read prose for takes, so
  a note can under-declare; the reader of its mint pass is the guard for that.
  Greening this habit by rewriting an owed take as released without the note's
  own reason is the over-claim the invariant names. No gate project covers the
  script (backlog arch-workspace-discipline row 85), so a green here is a run
  someone made, with its date.
refs:
  - "rule: genesis/research/.epr-meta/manifest.md, mint-pass-at-close (this check is the drain that rule's retire-when waits for)"
  - "plan: genesis/docs/superpowers/plans/2026-11-26-research-disciplines-exercised.md, flight F1"
  - "lesson: genesis/research/README.md, What more than one pass reached, row 4 (nothing is dropped silently)"
retire-when: >
  when the mint pass is mechanical: a survey's verdicts are folded into cluster
  rows by tooling at its close, so a take cannot be written without its home.
---

DELTA 2026-11-26 (BORN red; flight F1 of sprint 2026-11-26-research-disciplines, on ethosengine at dev baeb32247 plus the flight's changes): `python3 .claude/scripts/research-mint-check.py` reads 54 notes, 8 declared, 46 undeclared and reported as unmeasured (10 of them have no frontmatter). The 8 declare 66 takes: 23 on a backlog row that names the note, 18 on a file, 3 released with the note's reason, and 22 owed in five notes (hypha-dao 6, memorilla 6, hypha-distributed-press 5, radicle 3, freenet 2). The strict run exits 1 on those 22. Four rows of agentic-harness-borrows-backlog (14 to 17) did not name the survey they came from and now do. The unit test passes (18 tests). Red until the owed takes are given a home or released by their notes.

DELTA 2026-11-26b (RED preserved; the flight's judge, a seat that did not do the work): two Radicle declarations were wrong, because the declarer read the note's take table and not its later status table. T5 was declared owed though the note marks it done; it now points at its code. T4 was declared on a row though the note records it as not taken, by ruling; it is now released with that ruling. Counts after the fix: 22 on a row, 19 on a file, 4 released, 21 owed (hypha-dao 6, memorilla 6, hypha-distributed-press 5, radicle 2, freenet 2). The check was hardened on the judge's reading: a `file:` home must stay inside the repository, a row must name the note as a whole slug, a row number carried by two table rows is ambiguous, and an entry that runs onto a second line fails. The unit test passes (22 tests; 23 after one more hardening, a link that leaves the repository, made after the verdict).
