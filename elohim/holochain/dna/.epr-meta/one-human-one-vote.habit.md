---
epr-habit-version: 1
id: one-human-one-vote
invariant: >
  A second vote on the same proposal by the same agent is refused by an integrity zome, where
  every validator runs it. The refusal is a deterministic property of the vote entry plus
  `action.author()` and the author's own prior votes on that proposal (`must_get_agent_activity`
  over the author's chain in this DNA), so every peer refuses it identically. A vote path that
  accepts any number of votes from one key, or that takes the voter's identity from caller
  input, does not have this property no matter what a coordinator or a storage projection
  later de-duplicates.
status: unwired
active: false
refs:
  - genesis/research/weall-protocol-peer-review-2026-10-10.md
  - genesis/research/letter-to-weall-peer-review-2026-10-10.md
  - genesis/data/timeline/backlog/arch-authority-in-integrity-backlog.md
retire-when: >
  when the sweettest that proves decision closure (habit decision-closure) necessarily proves
  this too — one committed outcome computed from one vote per author — so this row carries no
  information that row does not
---
DELTA 2026-10-10: Declared UNWIRED from the WeAll peer review
(epr:weall-protocol-peer-review-2026-10-10 §7 challenge 3, §10 T4). We told the WeAll author in
writing that nothing stops one person's free keys from casting many votes; this row makes the
register say the same. Evidence today: `elohim/holochain/dna/elohim/zomes/content_store/src/governance_action.rs:396-410`
issues a vote as a child attestation with `proof_evidence` the literal `{"class":"witness"}` and
no check against the author's prior votes; `elohim/holochain/dna/mishpat/zomes/mishpat/src/lib.rs:1118`
takes `voter_id` from caller input and `:1159` returns a zeroed sentinel `ActionHash`. No
runnable check exists, so the habit cannot be red or green — it is unwired on purpose. The
check it needs: a sweettest in `elohim/holochain/tests/sweettest/` where agent A casts two votes
on one proposal through a direct source-chain write and the second conductor REFUSES the second
(`two_agent_conductors` + `exchange_peer_info` + `await_consistency` before the assertion; the DNA
suite runs `--run-ignored all`). Cure rides authority-in-integrity row 18 (the mishpat crossing).
Credit: Errol Swaby, author of the WeAll Protocol (github.com/errol1swaby2-bit/WeAll-Protocol). His repository's claim discipline — a first-screen NO-GO table and a 'current allowed claim' sentence pointed at civic claims, not plumbing — and his constitution's 'one verified human, one civic presence' (Art. IV §2) are why this row exists; the question in his frame ('which file stops one person's 100 free keys from casting 100 votes?') is the one this habit answers when it is wired.
