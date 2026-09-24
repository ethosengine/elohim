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
  - "test $(grep -L '<[^>]*@[^>]*>' .eprfs/status/memory/contributions/*.json | wc -l) -eq $(ls .eprfs/status/memory/contributions/*.json | wc -l) (the identity-reserve migration landed: no tracked contribution carries an email; MEASURED 2026-09-23: 250 of 251 carry one)"
  - "test $(ls .eprfs/status/memory/contributions/*.acts.jsonl 2>/dev/null | wc -l) -gt 0 (the per-concern acts projection exists in the tracked store; MEASURED 2026-09-23: 0 files)"
first_move: >
  Widen the grammar. `parse_agent_ref` (elohim/epr-rea/src/actor.rs) becomes
  `parse_participant_ref` accepting `human:<handle>` beside `agent:<role>@<model>`, born with the
  test `participant_ref` (accepts both; refuses `agent:<person>@human`, an email, and any
  substrate-minted form); `ActorClaim.definition_cid` becomes optional and is absent for a human.
  Then the human claims: `epr actor claim --as human:<handle> --session <id>`. The later stations
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
