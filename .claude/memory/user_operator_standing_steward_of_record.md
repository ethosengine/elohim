---
name: user-operator-standing-steward-of-record
title: Matthew is the standing steward of record
description: "Operator 2026-09-26 — Matthew stands as steward of record for unattributable memory entries, affirmed once per session at startup."
metadata:
  node_type: memory
  title: Matthew is the standing steward of record
  type: user
  originSessionId: 4750a644-a508-47ec-9220-8a193f383cf6
  modified: 2026-09-27T01:05:58.758Z
---

"I'm basically the steward of record pretty much all the time until another developer shows up..
so mark me down as such." Then: "maybe a session startup just ask who's the steward of record, and
we'll affirm that." (2026-09-26)

**Why:** the memory-index hook leaves an entry out of MEMORY.md while its author is unattributable
(no harness witness, no claiming session) and used to ask per entry. With one developer the answer
is always yes, but the operator wants it affirmed, not assumed — one question per session.

**How to apply:** the `participant-standing` SessionStart hook prints a `steward of record:
human:<h>?` line after the participant line. Ask it ONCE at the first natural point in the
session; on yes, import each unattributable entry with
`epr flow memory import <entry> --session steward-of-record --steward-of-record`, then
`epr flow memory project --index --budget memory-index-bytes@1 --out .claude/memory/MEMORY.md`
(the `steward-of-record` session label is already claimed by human:matthew). On no, leave the entry
for its author. This never overrides a real author: `steward_of_record()` in
`elohim/eprfs/epr-cli/src/flow/memory/attribution.rs` refuses whenever a claiming agent session
could have written the bytes. It is standing for entries, not identity: witnessing
`human:<handle>` still needs a present agent and a basis ([[feedback_elohim_witness_human_identity]]).
Retire when a second developer joins.
