---
epr-habit-version: 1
id: decision-closure
invariant: >
  Every closed governance decision produces exactly one committed outcome record, scoped to the
  decision's reach, that any two peers inside that reach compute byte-identically from the
  signed votes and the proposal they answer, and that is itself a validated entry every
  validator can check. A tally that exists only as a projection each reader computes from the
  vote attestations is not closure: two strangers cannot compare it, and a peer that
  disagrees cannot be shown to be wrong.
status: unwired
active: false
refs:
  - genesis/research/weall-protocol-peer-review-2026-10-10.md
  - genesis/data/timeline/backlog/commons-holonic-stewardship-backlog.md
  - genesis/research/hypha-dao-autonomous-collectives-cross-pollination-2026-06-24.md
retire-when: >
  when the outcome record is the only path by which a decision is read — no projection-side
  tally remains — so the tally code's own test is this habit's check and the row is redundant
---
DELTA 2026-10-10: Declared UNWIRED from the WeAll peer review
(epr:weall-protocol-peer-review-2026-10-10 §7 challenge 1, §8, §10 T2). WeAll's one sentence we
cannot answer: name one finished Elohim decision and the single artifact two strangers compute
and compare byte for byte. Evidence today: votes are child attestations and the tally is a
derived projection (`elohim/holochain/dna/elohim/zomes/content_store/src/governance_action.rs:4,13`;
`content_store/src/lib.rs:17761`); the register already records peers binding the same human to
different agent keys on all three household peers (`genesis/manifests/habits.yaml:839`), which is
the divergence closure must survive. The Hypha survey asked for the same object (a signed
outcome attestation, "whoever controls what reached a vote controls the vote"). Design home:
commons-holonic-stewardship row 32 (which reach tiers may bind; the closure artifact; the
witnessed upward appeal) — p2p-design-gated, because it adds an entry type. The check it needs:
two conductors inside the reach each compute the outcome record for one closed proposal and the
sweettest asserts the two CIDs are equal and the record validates on both.
Credit: Errol Swaby, author of the WeAll Protocol (github.com/errol1swaby2-bit/WeAll-Protocol). His architectural thesis — civic authority as 'open, deterministic, independently verifiable protocol state' with a committed state root any observer reproduces — is the standard this habit holds us to; the question in his frame ('what single artifact can two strangers compute and compare byte for byte?') is the one this habit answers when it is wired.
