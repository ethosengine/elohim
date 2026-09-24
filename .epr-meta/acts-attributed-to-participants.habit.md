---
epr-habit-version: 1
id: acts-attributed-to-participants
invariant: >
  Every act on a collective-memory contribution — author, edit, review, retract, graduate — is an
  append-only event by a first-class participant who claimed for themselves, human or agent, and
  the shared (tracked) store carries those events, not a rewritable snapshot of the last actor.
  Authorship is derived from the first act and frozen; a re-import appends an edit act and never
  rewrites identity; reviewer independence is judged against the set of everyone who shaped the
  body, not one field; and the substrate never copies a human's email or any cross-namespace key
  into fruit — a human appears only as the handle they claimed, or as an honest `(unclaimed)`.
status: red
active: false
checks:
  - "grep -q 'human:' elohim/epr-rea/src/actor.rs (the grammar admits a human participant; MEASURED 2026-09-23: exit 1 — the one legal prefix is `agent:`)"
  - "test $(grep -LE '[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\\.[A-Za-z]{2,}' .eprfs/status/memory/contributions/*.json | wc -l) -eq $(ls .eprfs/status/memory/contributions/*.json | wc -l) (no tracked contribution carries an email of ANY shape — tightened 2026-09-24 from the bracketed `<x@y>` form, finding M7; MEASURED 2026-09-23: 250 of 251 carry one; MEASURED 2026-09-24: 0 of 254 under the bracketed form; MEASURED 2026-09-24 under any email shape: 1 of 254 carries one — project_civic_ai_outreach_thread.json quotes a correspondent's address in its claim, at HEAD too — exit 1)"
  - "test $(ls .eprfs/status/memory/contributions/*.acts.jsonl 2>/dev/null | wc -l) -gt 0 (the per-concern acts projection exists in the tracked store; MEASURED 2026-09-23: 0 files)"
first_move: >
  Widen the grammar. `parse_agent_ref` (elohim/epr-rea/src/actor.rs) becomes
  `parse_participant_ref` accepting `human:<handle>` beside `agent:<role>@<model>`, born with the
  test `participant_ref` (accepts both; refuses `agent:<person>@human`, an email, and any
  substrate-minted form); `ActorClaim.definition_cid` becomes optional and is absent for a human.
  Then the human is witnessed once per device by a present agent (`epr actor witness`), or
  claims for themselves (`epr actor claim --as human:<handle>`). The later stations
  each arrive with their own test and replace a grep check above with it: derive-and-freeze author
  + `<id>.acts.jsonl` projection (`memory_import_freezes_author`: re-import under a different
  session leaves `author` byte-identical and appends exactly one act; an identical re-run appends
  nothing), the independence predicate (`graduate_refuses_shaping_actor`: a review by any actor
  who authored or edited the body is refused naming C1; no acts file is refused naming honest
  absence), then the identity-reserve migration.
retire-when: >
  when a rewritten author or an unattributed act is UNREPRESENTABLE rather than refused — the
  contribution record has no writable author slot at all (author is a read over the acts), the
  acts store is the only writer path, and the actor grammar has graduated to the household mesh
  where the participant IS an AgentPubKey with a source chain (imagodei Human/Agent). At that
  point attribution is a property of the substrate, not a practice under watch.
refs:
  - "genesis/docs/superpowers/specs/2026-09-23-human-participant-actor-plane-design.md — the gate output, the four measured defects, the §3 attribution relationship and the §5 checks"
  - "genesis/docs/architecture/private-thought-governed-fruit.md §2, §4 (boundaries 5 and 6)"
  - "genesis/docs/architecture/stewardship-over-sovereignty.md §3"
  - "genesis/docs/content/elohim-protocol/manifesto.md — section Witnessed Humanity: Attunement, Not Filters; the canon the witness leg (epr actor witness) rehearses at this node"
  - "elohim/epr-rea/src/actor.rs — the claim record and the one-prefix grammar this widens"
  - "elohim/eprfs/epr-cli/src/flow/memory/import.rs — `acting_author` and the rewrite-on-change path"
  - "elohim/elohim-storage/.epr-meta/identity-cross-signed.habit.md — the sibling: this habit makes attribution REPRESENTABLE; that one makes it ADMISSIBLE for economic joins"
---
2026-09-23: DECLARED `unwired`. Operator observed `author` flipping between agents across
re-imports in the tracked contributions store and named it: stewardship and affinity to objects
(author, editor, reviewer) were being buried in git history. Measured: one record's author
rewritten four times; the six-actor event history for it lives only in the gitignored
flows.jsonl; the graduate independence check reads the rewritable field; 250 of 251 tracked
contributions carry the operator's email. Operator's second observation: the human must be a
participant too — "a load-bearing imagodei". Gate run, spec written, commitment minted. The
commitment itself is attributed to an agent with the human in the steward slot, because the
grammar does not yet admit the human: the defect recorded once more, on purpose, as the last
time it has to be.

DELTA 2026-09-23 (STAYS RED; check 1 of 3 green). Station 1 landed: `parse_participant_ref`
admits `human:<handle>` beside `agent:<role>@<model>`, `agent:<person>@human` is refused in the
agent parser itself, a human claim refuses a definition address, `epr actor claim --as
human:<handle>` works, note `--as` and contribution `author` accept either kind; hashed atom
unchanged. Evidence: `just gate elohim-epr` EXIT 0, `just gate eprfs` EXIT 0, independent
review approved (agent:reviewer@claude-sonnet-5, verdict note on the station-1 commitment),
seam-audit spot-check trust. Check 1 now passes; checks 2 (email-free store) and 3 (acts
projection) still fail — stations 3 and 5. Next: station 2, the operator claims
`human:<handle>` once, standing per workspace.

DELTA 2026-09-24 (iterated in flight during governed-discovery station 4; STAYS RED; check 1 of 3 green). Every act of the station carried a participant who claimed for themselves: the orchestrator seat was claimed as `agent:orchestrator@claude-fable-5-1`, re-claimed as `@claude-opus-5-5` when the operator switched the session model and back again — attribution follows the model actually running, never a uniform line; implementers signed their own trailers; eight intents claimed and fulfilled under that seat; the new `FoldAttestation.attested_by` carries the sidecar's participant ref or the honest literal `(unclaimed)` (never inferred, never an email) and is APPENDED to `attestations.jsonl` — a new act-shaped surface born under this habit's rule. Observed on every one of those acts: the claim/fulfil/note records still print the operator's EMAIL in the steward slot — check 2 (identity-reserve migration) is visible on each, the next station's work. Station 2 (the human's own `human:<handle>` claim) remains the operator's act.

2026-09-24: operator ruling — the elohim witness the human (R-P9 of the 2026-09-25 plan); station 2 becomes a witness act.

DELTA 2026-09-24 (STAYS RED; checks 1 and 2 of 3 green). Station 5 landed: `imported.gitAuthor` → `imported.gitName` (display name only; the retired field is refused, not aliased), import freezes authorship on unchanged bytes, and the contribution store was rewritten by ONE act, `epr flow memory migrate-identity-reserve`, attributed through the standard arm order (R-P11): provider `agent:orchestrator@claude-fable-5-1` (the executing session's claim; run by the Opus 5.5 implementer on the orchestrator's behalf), `steward:human:matthew`, record `bafyreibs2quaq2ricl3refixx7f6caqc2zzzknwwmrpheq5rtsliacjrki`, lineage `bafkreigvcsogqvi24k5qmajmpnvxqdtvkkxc22t7veqyee4pj3zqyv6nqe`. It corrects the first act `bafyreidabgoctch4e5c7v4w5yn54hhizpswnoqgvvg5zhnepq2wbi5efq4`, which forced the standing human as provider; the correction moved zero bytes. 246 tracked files each changed in exactly one line and no other byte, all still attributed to their original authors through the lineage. Evidence: check 2 exit 0; `flow_memory_import` 20 passed incl. `memory_import_freezes_author` (its appends-one-act half is station 3's), `migration_is_idempotent`, `migration_provider_is_the_session_claim_when_one_exists` and `a_misattributed_migration_act_is_corrected_by_a_re_run`. Check 3 (`*.acts.jsonl`) is station 3's and stays red.

DELTA 2026-09-24 (Lane P review follow-up, R-P14..R-P19; STAYS RED; check 1 of 3 green — check 2 re-reddened by its own tightening). Executed by agent:implementer@claude-opus-5-5. Only a roster member's contest counts (W1); the chain root is pinned per device at `<config>/elohim/device/rosters/<handle>.root` and a differing root reads `contested` (W2); a fresh checkout on a witnessed device stands by the tracked roster, and `--again` reads it (W3); `ELOHIM_SESSION_ID` leads the session env order and an inferred session stamps `source:session-env` (W4); import freezes authors on the tracked bytes alone (W5); witness writes its roster genesis before its signed records under both locks (M1); the migration act checks its provider before appending (M2); a note's `steward:` names `repo:ethosengine/elohim` when no human stands, never an email (M3); `ELOHIM_DEVICE_KEY_FILE` is refused relative or in-repo (M4); a re-witness after a contest names it with `--answers` (M5); the SessionStart hook reads `current --device` (M8). Check 2 tightened to any email shape (M7) and re-measured: 1 of 254 — a correspondent's address quoted in project_civic_ai_outreach_thread's claim, copied from its memory entry's description; the cure is an edit act on that entry, not a grep exemption. Revocation (M6) is backlog: genesis/data/timeline/backlog/participant-roster-binding-revocation.md.
