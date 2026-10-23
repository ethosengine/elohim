---
epr-habit-version: 1
id: rule-variance-under-covenant
invariant: >
  A governance rule's obligation is lowered only where the steward of record granted it, in their
  own words, for one session and named paths, to a floor the rule itself declares. Every agent the
  session spawns carries what is held, and none can grant, extend or record one. An unreadable
  covenant lowers nothing, and an edited rule voids the variances granted over its old bytes.
status: green
active: false
checks:
  - "cargo test -p elohim-epr-cli --test covenant — the authority half: record refusals (no standing, no affirmation, no declared variance, ask/deny class, paths outside the manifest, another party's revocation), check across paths, window, revocation and edited manifest bytes, offer and show. Stub brit."
  - "python3 -m unittest discover -s .claude/hooks/__tests__ -p 'covenant_hooks_test.py' — the hook legs: loop vs floor line, an unreadable covenant keeps the full rule and says so, a subagent is not told to ask, the SubagentStart stamp (blind-reader never stamped), and a subagent's record attempt is refused."
  - "python3 -m unittest discover -s .claude/hooks/__tests__ -p 'participant_standing_test.py' — the bundled ask: a pending covenant replaces the bare steward-of-record ask with ONE AskUserQuestion bundle; an unreadable covenant keeps the steward ask."
  - "brit CI on run-ci/session-covenant (ethosengine/brit ci.yml) — clause node, append-only refs, `brit-build-ref covenant put|list|show`, the linked-worktree agent-key path."
  - "LIVE (brit-build-ref and a covenant-aware epr on PATH): a session records its covenant through `epr covenant record` against the real brit-build-ref, a spawned agent's scoped `.feature` write receives the floor line, and `git for-each-ref refs/notes/brit/covenant/` lists the clauses."
guard: >
  Risk 1 — a self-granted floor. Rung 1 signs a clause with the repository's brit key, not the
  human's, and anything with a shell on this device can write one straight into brit; `epr` re-tests
  every clause it reads, but a forged affirmation by the standing human passes that re-test. The
  subagent guard stops honest drift, not a forger. Rung 2 (the human's own key co-signs) is what
  closes it; until then the covenant is a recorded promise, not a proof.
  Risk 2 — silence. A held floor swaps the loop's advice for a floor line and a witness row; it
  must never make the obligation vanish. Grep the ledger for `variance:` rows to see what a floor
  saved, and what it might have let through.
  Risk 3 — decision fatigue reborn. The covenant exists so the asks arrive as one bundle before the
  work; if hooks start re-asking per edit, the instrument has inverted.
evidence: >
  DELTA 2026-10-19a (RED; built, not yet live). Session 182f26d8 (operator: "I matthew am the
  steward of record here. in the affirmative"). Authority half: `cargo test -p elohim-epr-cli --test
  covenant` 16 passed EXIT=0; live_manifest_integrity 1 passed and governance_parity_vectors 1 passed
  (both EXIT=0, evaluators unchanged); clippy -D warnings EXIT=0. Hooks: covenant_hooks_test 9 OK,
  participant_standing_test 21 OK. brit half committed 5072dcec5 on run-ci/session-covenant, pushed
  with --no-verify by operator decision (the installed epr's reach gate requires
  `.epr-meta/elohim/packages`, which brit main has never carried); not compiled locally (no Nexus
  read credential in the workspace, operator chose CI). brit CI run 39320297567 on 5072dcec5:
  lint (clippy -D warnings + fmt), test, test-journey, test-doc, cargo-deny and every check job
  green; test-fast (windows-11-arm) and test-fixtures-windows red on one test of mine
  (git_dir linked-worktree: `git worktree add` refuses a canonicalized `\\?\` path), fixed in
  8627773c9 (unpushed); cargo-deny-advisories red on rustls advisories in existing dependencies,
  not a `Tests pass` need. `just gate eprfs` EXIT=0 (75 test binaries ok).
  DELTA 2026-10-19b: independent review (code-reviewer) found three ways `check` could hold an
  ungranted clause (`..`/symlink/outside-repo spellings; a nearer same-id manifest; same-tick
  revocation ordering), fixed in 32e317832 with two new tests (covenant 18 ok). brit CI run
  39333130342 on 8627773c9 (fix pushed with --no-verify, operator's second ruling): `Tests pass`
  success, only cargo-deny-advisories red. Landed on dev at e3e0af5cb (pre-push ALL CLEAR).
  Still red: the brit pin does not carry feat/session-covenant yet (batch with the pending
  evidence pin move), and the LIVE check has not run.
  DELTA 2026-10-20 (GREEN). brit main fast-forwarded to 76ba95d8a (operator: "yes push to main
  please"); pin moved on dev at 1b2deee4c (pre-push: attested `Tests pass` success). LIVE, session
  182f26d8: brit-build-ref built --locked at the pin against the Nexus crates and put on PATH; the
  steward's three answers recorded (affirmation bafyreibtyq4…, purpose bafyreieu2kus…, variance
  bafyreidktlxh… over the probe path only, manifestCid bafkreihhv5zo…). `epr covenant check`
  held on the probe path, not-held one file outside it; `git for-each-ref
  refs/notes/brit/covenant/` lists all three. A spawned general-purpose agent carried the
  [SESSION COVENANT] stamp; its in-scope `.feature` write got the resolver advisory and the
  [BLIND-READER REVIEW · COVENANT] floor line, its out-of-scope write got the full loop, and its
  `epr covenant record` was refused by the subagent guard. governance-findings.jsonl row 11945
  carries `variance:`, `party: human:matthew`, `agent: general-purpose`. Probe files deleted;
  the covenant refs stay local (pushing them is the named frontier). Risk 1 stands: rung 1 is a
  recorded promise, not a proof.
  DELTA 2026-10-20b (GREEN kept). LIVE re-check, session 4718bf00, with a `storyteller` (the agent
  whose principal concern is plain storytelling): stamp carried; in-scope `.feature` write → floor
  line + witness row (`agent: storyteller`); control write → full loop; its record refused. Second
  variable rule born: `genesis/data/stories/.epr-meta` `experience-story-blind-reader-review`
  (`*--*--*.md`, new blind-reader profile `experience-story`, package 1.3.0); `epr covenant offer`
  lists it, the hook fires on a story and stays silent on INDEX/CONVENTIONS. Hook tests 193 ok.
refs:
  - "genesis/docs/superpowers/specs/2026-10-19-session-covenant-epr-design.md — the authority half (offer, record, check, show)"
  - "elohim/brit/docs/specs/2026-10-19-session-covenant-design.md (branch feat/session-covenant) — the mechanism half: a Layer 2 blessing over the rule's manifest CID"
  - "genesis/a2o/.epr-meta — the first rule to declare `parameters.variance` (a2o-story-blind-reader-review)"
  - "genesis/data/stories/.epr-meta — the same rule over the prose experience-story catalog (experience-story-blind-reader-review)"
  - ".claude/hooks/blind-reader-review.py, .claude/hooks/covenant-subagent-stamp.py, .claude/hooks/participant-standing.py, .claude/hooks/epr-meta-resolver.py"
retire-when: >
  when the hooks read a variance as a notarized governance commitment the network validates (rung 3,
  a `varies-rule` Commitment co-signed by the human's own key), so that a lowered obligation is a
  property of the substrate rather than a practice under watch.
---

The session covenant: the steward of record's bundled pre-flight asks, and the rule variances they
can grant. Plan: `/projects/.claude-config/plans/yes-plan-to-build-misty-flurry.md`.
