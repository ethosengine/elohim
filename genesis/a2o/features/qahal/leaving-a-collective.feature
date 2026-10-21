# The held status, the slice this waits on, the missing nodes it proves and why it needs an owned
# mesh are stated in the Feature description below, so they survive a reader or tool that skips
# comments.
# @concern:leave-is-clean names this proof for the habit register; no habit checks it yet.
# @act:i: this story belongs to Act I, the household chapter of the test suite.
# @requires:household-nodes: the household's three peers must be running.
# @requires:owned-substrate: the scenarios end a membership and delete copies, so they run only on a
# mesh this run owns outright.
@e2e @qahal @act:i @wip @concern:leave-is-clean @requires:household-nodes @requires:owned-substrate
Feature: When Jessica leaves the garden co-op, her peer stops keeping the co-op's things, the co-op stops keeping hers, and her household is untouched

  Matthew, Jessica and James Dowell share a household. Each of them keeps a peer: an always-on
  machine at home that holds their data and does work for them. Each peer is named after its
  person: matthew, jessica and james.

  All three also belong to the Elm Street garden co-op, a collective: a group of people who keep
  things in common under rules they agreed together. The co-op keeps a seed library and its
  catalogue. It has other members who live elsewhere, but in this story the co-op's things are
  kept only on the three Dowell peers, and no other member's machine takes part.

  A group asks its members' peers to keep copies of what it shares, so that losing one machine
  loses nothing. Such a request is called a need, and it is made in one group: the co-op, or the
  household. A peer that keeps a copy for a need signs a commitment: a record saying that it holds
  this copy, for this person or group, for this need. Signing a commitment only records a promise.

  Committed is not honoured. A peer that has signed a commitment is committed. It is honoured only
  once the copy is actually on that peer and the peer's own list of what it holds names it. A
  holder is a peer whose commitment is honoured. Every step below that says "honoured" means the
  copy has arrived, not only that the commitment was signed, because a signed commitment whose copy
  has not arrived keeps nothing safe.

  Nobody hands out this work: every peer runs the same fixed rules, which decide from the records it
  holds which peers should keep each copy. A peer takes up a copy when it signs a commitment to keep
  that copy and is then honoured. Only the first scenario turns on who takes up a copy, and there one
  more rule decides it: each peer has an offer, its person's signed statement of how much room it
  gives to keeping copies for each group, and a peer with no room for the co-op never takes up a
  co-op copy. When a commitment is ended but the need it served still stands, so that the need has
  fewer honoured holders than it asks for, those fixed rules have a peer with room sign a new
  commitment to keep the copy, and that peer takes it up. In this story Matthew's offer has room for
  one more co-op copy, so his peer is the one that can take up Jessica's. Leaving changes no one's
  offer.

  Being a member of the co-op works two ways. Jessica's peer carries for the co-op: it keeps a copy
  of the co-op's "Seed catalogue" under a commitment made in the co-op. And Jessica draws on the
  co-op: other members' peers keep copies of her "Plot notes", which she shared with the co-op, so
  that they survive the loss of her machine. Her own copy of "Plot notes" on her own peer is the one
  she made. It is not kept for any need, so no commitment, in the co-op or in the household, holds it
  there, and nothing her leaving ends can touch it.

  A member leaves by signing a withdrawal of her membership. The withdrawal revokes each commitment
  her own peer made in the co-op: a revocation of that commitment is recorded, a signed record that
  the withdrawal took it back, and the revoked commitment is ended. A revocation is how a commitment
  her peer made is ended; a commitment another peer made to keep a copy for her is ended when that
  peer lets the copy go. To let a copy go always ends the commitment. The copy kept under it is
  deleted too, unless another commitment in force still keeps the same copy on that peer. Her leaving has
  settled once every peer has finished acting on her withdrawal: each commitment it must end is
  recorded as ended, each copy it must let go is deleted, and each copy her peer kept for the co-op
  is kept by another peer instead. Every outcome below is
  read once her leaving has settled. Leaving is clean when, once it has settled, no commitment made
  in the co-op names her peer as the one keeping a copy or her as the one a copy is kept for, every
  peer that kept something for her under the co-op has let it go, and nothing that is hers is lost.
  Once it has settled, she is told what the co-op no longer keeps for her.
  Clean does not mean forgotten: the signed record that she was a member, and the withdrawal she
  signed, both stay after she leaves.

  Her own peer stops carrying in a safe order: another member's peer takes up each copy first,
  and only then does her peer let its copy go, so the co-op is never left short.
  This story does not prove that order. The same rule is proven, for a member leaving the household,
  by its own story, genesis/a2o/features/resilience/member-leaves-replaced-first.feature, which is
  itself held: written as a specification, but not yet runnable. Here the story checks only where
  the co-op's copies end up. That end state is the same whichever order the peers acted in: the
  order decides only whether the co-op was short of a copy for a moment along the way, and
  every assertion below is read after settling, not during.

  Leaving the co-op is not leaving the household. A copy kept for Jessica under a household need
  stays kept when she leaves the co-op.

  This story is held in the same sense, written as a specification but not yet runnable, and so
  every scenario is marked @wip, the tag a test run uses to skip what cannot run yet, until slice 5
  of the placement-contract design lands: "the floor controller", section 17 of
  genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md,
  which writes and releases the commitments this story reads. It proves two of that design's
  missing nodes (section 15): a withdrawn membership revokes the commitments made under it, which
  is specified but not built, and the peers that kept copies for a leaver let them go, which is not
  yet specified. The read-back that Jessica and each holder use in the last scenario does not exist
  yet either (section 9, "Leave and withdrawal"). No step of this story has a definition yet.

  Because these scenarios end a membership and delete copies, they run only on a mesh of peers that
  the test run owns outright, never on machines that other people use.

  Background:
    Given the household's peers "matthew", "jessica" and "james" are running
    And Matthew, Jessica and James are members of the Elm Street garden co-op
    And the co-op needs "Seed catalogue" kept on two peers, and peers "jessica" and "james" are its two honoured holders, each holding the copy it committed to keep
    And Matthew's offer gives room on peer "matthew" for one more copy kept for the co-op
    And Jessica has shared "Plot notes" with the co-op, and peers "matthew" and "james" keep copies of it for her under the co-op
    And peer "jessica" holds Jessica's own copy of "Plot notes", which no commitment in any group keeps there

  Scenario: Leaving revokes what Jessica's peer kept for the co-op, and the peer with room keeps it instead
    When Jessica signs a withdrawal of her membership of the garden co-op
    And her leaving has settled
    Then a revocation of peer "jessica"'s commitment to keep "Seed catalogue" for the co-op is recorded
    And peer "matthew", whose offer gives it room, is an honoured holder of "Seed catalogue" under a commitment made in the co-op
    And peers "matthew" and "james" are the co-op's two honoured holders of "Seed catalogue"
    And no commitment made in the co-op names peer "jessica" as the one keeping a copy

  Scenario: Leaving stops what Jessica drew, and her own copy of "Plot notes" stays on her peer
    Given no commitment other than the co-op's keeps "Plot notes" on peer "matthew" or peer "james"
    When Jessica signs a withdrawal of her membership of the garden co-op
    And her leaving has settled
    Then peers "matthew" and "james" each let go of the copy of "Plot notes" they kept for her under the co-op
    And neither peer "matthew" nor peer "james" still holds a copy of "Plot notes"
    And no commitment made in the co-op names Jessica as the one a copy is kept for
    And peer "jessica" still holds Jessica's own copy of "Plot notes", unchanged
    And Jessica is told that the co-op no longer keeps copies of "Plot notes" for her

  Scenario: Leaving the co-op ends only the co-op's commitments, and the household's stay in force
    Given the household also needs "Plot notes" kept for Jessica on two peers, and peers "matthew" and "james" keep it under commitments made in the household
    When Jessica signs a withdrawal of her membership of the garden co-op
    And her leaving has settled
    Then the commitments made in the co-op to keep "Plot notes" for Jessica are ended
    And the commitments made in the household to keep "Plot notes" for Jessica are still in force
    And peers "matthew" and "james" still hold their copies of "Plot notes"
    And Jessica is still a member of the household

  Scenario: Jessica and each peer that kept something for her can read back that her leaving settled
    When Jessica signs a withdrawal of her membership of the garden co-op
    And her leaving has settled
    Then Jessica can read, for each commitment made in the co-op that named her or her peer, that it has ended
    And peers "matthew" and "james" can each read that they let go of what they kept for her under the co-op, and, for each copy they let go, which commitment they ended by letting it go
    And the co-op's record of Jessica's membership still exists and shows her withdrawal
