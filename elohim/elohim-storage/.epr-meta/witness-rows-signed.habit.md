---
epr-habit-version: 1
id: witness-rows-signed
invariant: >
  Every observation row a peer keeps either carries a signature the peer verified against the
  observer's key (device key at a repo node, agent key at a peer) or says plainly that it is
  unsigned — in the write ack, in every view, and in graduation, where an unsigned row counts
  toward no diversity threshold and never graduates.
status: red
active: false
checks:
  - "a2o @concern:witness-rows-signed (genesis/a2o/features/observation/witness-rows-signed.feature — signed ack, unsigned-labelled-and-excluded, browser-signs-with-session-key; and genesis/a2o/features/observation/observed-without-a-doorway.feature — the local-first station: a reading lands signed on a peer with both doorways stopped; all @wip; default profile: just test mesh features/observation/witness-rows-signed.feature)"
  - "epr flow report --bound observation-rows-unsigned-ceiling@1 (hard 0 over observation-rows-unsigned@1: rows with an empty signature per peer; reads skipped until folded)"
refs:
  - "canon: genesis/docs/content/elohim-protocol/observability/epic.md §3 (witness plane), §10 (witness with diversity)"
  - "first move: genesis/data/timeline/backlog/observation-signing-graduation.md (device key at the repo node, agent key at the peer, one observer_cid per person)"
  - "today: elohim/elohim-storage/src/observation/stream.rs ~38–40 ('observations are unsigned until the signing graduation'); src/api/observations.rs ('the row is unsigned and the ack says so'); src/observation/projector.rs"
  - "design: genesis/docs/content/elohim-protocol/architecture/2026-05-11-observation-event-layer-design.md §6.3 trust-then-verify"
  - "siblings: attention-witnessed-privately owns privacy (never gossiped, never joined); this atom owns provenance; acts-attributed-to-participants owns the observer-namespace split"
retire-when: >
  when the observation log is the signed, persisted, agent-private-encrypted iroh log named in
  attention-witnessed-privately's retire-when — a row that cannot be appended unsigned leaves
  nothing to say "absent" about.
---
DELTA 2026-10-08 (BORN red): `POST /api/v1/observations` accepts an empty `signature_b64` and the stream emits a signature-absent omission line; nothing verifies a signature against the observer's key, and the graduation evaluator does not yet exclude unsigned rows (it is referenced by nothing outside its module). Cargo tests the first code pass adds: `cargo test --test api -- observations::signing` (a stored signature verifies against the X-Agent-Cid key; an unsigned post acks signed: absent) and `cargo test --lib graduation::evaluator -- unsigned_rows_never_graduate`. FIRST MOVE: the signing graduation backlog atom's first step, then the evaluator exclusion. Planned in the Observability epic §7 step 3. NO status change.
