---
id: "backlog-security-declare-carries-record-carried-evidence-bounds"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "declare-carries-Record: the four validation gates prove self-consistency, NOT DHT-existence — a fifth author-membership gate becomes load-bearing the moment the declarer authorization is tightened"
slug: "security-declare-carries-record-carried-evidence-bounds"
written: "2026-07-26"
author: "claude (resiliency-saga sprint-3, red-team adversarial review of b91168724)"
status: "open"
priority: "high"
ci_status: none
jobs: [elohim-edge]
tags: [security, canonical-head, declare, carried-record, authorization, membrane, C5, earned-authority]
cites:
  - elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
  - elohim/elohim-storage/src/http.rs
  - genesis/data/timeline/backlog/security-doorway-auth-required-unenforced.md
---

# The carried record is evidence, not authority — and only in the weaker sense

## The finding (2026-07-26, sprint-3 red-team of commit b91168724)

`declare-carries-Record` lets a `declare_canonical_head` caller carry a
serialized `Record` so a canonical-head link can be created when the target
action is not locally retrievable (full-arc gossip gap). `validate_carried_record`
enforces four gates: recomputed `hash_action` == target; the envelope's claimed
action_address == target; `verify_signature(author, sig, action)`; and
`hash_entry(carried_entry)` == `action.entry_hash()` — plus the shared
`content.id == id` gate. The gates are internally sound; the red-team could not
break any individually. The entry-hash binding (gate 4) genuinely closes the
signed-action-with-substituted-entry forgery, and gate 1's recompute is
necessary because `HoloHashed` deserializes its claimed hash verbatim.

**But the gates prove the carried bytes are self-consistent — not that the
action ever existed in the DHT, nor that its author ever joined the DNA.**
`verify_signature` is a pure ed25519 check over a *caller-supplied* key. An
attacker generates a keypair offline, authors `Action::Create { author: <their
key>, entry_hash: hash_entry(<their Content>), .. }`, self-signs it, and produces
a Record that passes every gate. Nothing calls `must_get_valid_record`, checks
membrane/source-chain, or invokes the integrity zome's `validate_create_entry`
(which is where `attestation:` / `governance-action:` floors live).

The correct bound is the *weaker* one: the carried record cannot name an action
other than the target, cannot pair a foreign entry, cannot be unsigned, cannot
carry a foreign `content.id` — so it cannot widen the declaration beyond **what
the carrier could author for itself**. The stated "cannot widen beyond what a
gossip-healthy conductor could `get`" is false: a `get` result is DHT-existent
and integrity-validated; a carried record is neither.

## Why this is not blocking today, and exactly when it becomes load-bearing

Under the **currently open** `authorize_canonical_head_declarer` (god-mode, see
[[security-doorway-auth-required-unenforced]]), "what the carrier could author
for itself" is nearly everything — most content-injection was already reachable
via commit-then-declare, so the practical delta is small. The four gates read as
"the carried record is safe." **They will read that way at exactly the moment the
declarer gate is tightened to a steward allowlist (the C5 earned-authority
work)** — and they will be wrong. One compromised steward key would then bind
content authored by a key nobody in the fleet knows, at an action hash that
exists nowhere, gossiped fleet-wide as a phantom canonical link that suppresses
legitimate convergence for that id until the next declare.

## Required before the declarer gate is tightened (C5 precondition)

1. **Fifth gate — carried author must be a DNA member.** `must_get_agent_activity`
   on `record.action().author()`, or require a counter-signature from a known
   peer. Without it the C5 authorization gate closes *who* declares but does
   nothing about *what* a carried record may assert.
2. **Add the foreign-key-forgery negative test.** The sweettest negatives (c/d/e)
   all mutate a *genuine* record; the ed25519-keypair-that-never-joined case
   (passes all four gates) is uncovered. Today it would pass positively — encode
   it as the tripwire when gate 5 lands.

## Lower-severity residue (pre-existing classes, newly relevant)

- **Poisoned-row self-heal.** A row whose `declared_head_action_hash` is
  unresolvable locally falls into `GapFill` → `SkippedDeclared`
  (`p2p/projection_reconcile.rs:1896-1899`; `db/content_diesel.rs:1082-1086`) and
  persists until a manual re-declare. Make an unresolvable declared head eligible
  for canonical re-stamp.
- **No request-body size cap on `/db/*` writes** (`http.rs` `into_body().collect()`
  is unbounded) — pre-existing for all POSTs, newly relevant now that
  `carried_record` is the first *designed* multi-MB write field.

## Already fixed in sprint-3 (not residue)

The attacker-controlled `declared_at` primitive (carried action timestamp →
permanent monotonic-heal lockout at `content_diesel.rs:1093-1098`, which the
deploy path depends on) is closed in the b91168724 follow-up: the carried branch
stamps `sys_time()` rather than trusting the action timestamp.
