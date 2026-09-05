# The version-divergence half of federation-deploy, split out 2026-09-05 because it is an
# ACT I scenario: it is proven on a household mesh this run owns, while its sibling
# features/dataplane/federation-deploy.feature is @act:ii (a fleet story). An act is declared by
# TAG and a feature file is the unit that carries one, so a file whose scenarios span two acts
# cannot state the truth about either — the household lane held this scenario on the Act II
# baseline (jenkins, apex-dns, alpha-cluster-6peer …) it never needed.
@e2e @dataplane @concern:federation-deploy @requires:multi-node @act:i
Feature: Federation version convergence — two doorways that disagree serve the same version again

  A person reading a page on elohim.host and a person reading the same page on the other
  federation doorway should be reading the same words. Sometimes they are not. Both doorways hold
  the page, both answer HTTP 200, neither is broken in any way either visitor could notice — and
  they are serving two different VERSIONS of it. Measured live on 2026-08-31, that state lasted
  days. Nobody arriving at either front door can tell it happened to them, and nobody choosing
  between the two addresses knows they are choosing between two texts.

  This is the SECOND of two failure modes in federation deploy, and it is parallel to the first,
  not downstream of it. The sibling feature (federation-deploy.feature) covers the first: bytes
  and metadata that never ARRIVE on a doorway, so it answers "App not found". This one covers what
  can still go wrong once they have arrived. A reader who wants the deploy-uniformity story starts
  there; a reader who wants version agreement is in the right file.

  Vocabulary, because every assertion below rests on it. A DOORWAY is a gateway node that serves
  content to ordinary browsers on behalf of a federation of peers. An EPR (Elohim Protocol
  Resource) is the addressable content record a doorway resolves in order to serve a page. A HEAD
  is the specific version of an EPR a doorway serves — a pointer to one authored revision,
  distinct from blobHash (which points at bytes; the head names WHICH bytes). A DECLARATION is a
  signed link that names one head as the canonical version of an EPR, carrying a tier: EARNED
  (authored through the protocol's authority path) or STAGING (placed by a deploy/seed scaffold).
  The ELECTION is the rule every peer shares for picking one winner from all declarations it can
  see: earned beats staging, then the newest notarized declaration timestamp, with a deterministic
  tiebreak — so identical inputs give identical winners on every peer. The RECONCILE SWEEP is the
  periodic background process on each peer that measures disagreements and heals the ones it can
  prove.

  Two more terms the mechanism lines rest on. A CONDUCTOR is the peer-to-peer runtime each peer
  runs behind its doorway: the doorway is the web front door, the conductor is what actually holds
  the signed records and the rules for reading them. VERIFIED IN WASM means the check ran inside
  the conductor's own sandboxed protocol code — the same code, byte for byte, on every peer —
  rather than in the doorway or in this test. That distinction is the whole difference between a
  peer PROVING a version won and a peer being TAKEN AT ITS WORD: bytes carried from another peer
  are only ever believed after the receiving peer's own conductor re-derives the claim from them
  and gets the same answer.


  One piece of test vocabulary, because this file's tag line uses it: an ACT names the substrate a
  scenario is measured on. Act I runs against a household mesh the test run OWNS and may write to;
  Act II runs against the deployed fleet, which it may only read. A feature file carries exactly one
  act tag, and the runner uses it to decide whether a lane executes the file at all — which is why a
  scenario that needs a different substrate from its neighbours has to live in a different file.

  The two peers: alpha-A is the author peer (the one the deploy pipeline authors from). Peer
  "elohim.host" is a second federation doorway serving the same content from a different premises.
  On the household lane both are peers of a mesh this run owns.

  WHY THE DIVERGENCE PERSISTED, which is what the cure had to address: each doorway's head only
  ever moved when a deploy or seed wrote to that host DIRECTLY, while the declarations that should
  settle the election live on the conductor DHT — the distributed hash table every peer shares —
  and were not traveling. A peer's STORAGE ARC is the slice of that shared space it volunteers to
  hold and pass on; an arc of Empty means it holds and passes on nothing. Arcs reset to Empty on
  every restart, so election links never gossiped in — one peer's sweep measured 2,619 divergent
  rows and refused 2,603 of them per sweep, correctly, having no election to obey. A cure that
  fixed this by re-uploading to each host would be the disease.

  A GREEN SCENARIO HERE IS NOT YET A HEALED VISITOR, and the difference is not a technicality. The
  convergence capability ships DORMANT: it does nothing until an operator turns it on per fleet.
  The scenario turns it on for the length of its own run and turns it back off after, so what it
  proves is "the cure works where it is enabled" — not "visitors are experiencing the cure".
  Visitors stop seeing two versions of a page only when the operator enables it on the fleet. Read
  a green here as a capability receipt, never as a closed harm.

  Background:
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"

  # BOUND 2026-09-05 (steps/dataplane/federation-deploy.steps.ts).
  #
  # @requires:owned-substrate IS LOAD-BEARING, and it is the ONLY tag here that gates anything.
  # This scenario WRITES: it stages a real disagreement on a real page by authoring a revision on
  # each of two peers, and it flips an operator flag in a peer's runtime config. That is only ever
  # acceptable on a substrate THIS RUN OWNS. `owned-substrate` is the one capability that says so —
  # available: false in genesis/manifests/cluster-state.yaml (the shared fleet), available: true
  # only under cluster-state.act1-household.yaml. So the fleet's Dataplane Validation lane, which
  # selects `@dataplane and not @wip`, skips this scenario instead of staging divergence on the
  # live landing page that visitors are reading.
  #
  # @requires:household-nodes would NOT have gated it: that capability is available: true in BOTH
  # cluster-state files, because a household mesh is normally up. "Peers exist" and "this run may
  # write to them" are different questions, and only the second one is the permission this scenario
  # needs. The feature's own @requires:multi-node is the third, weakest question — "more than one
  # peer" — true of every scenario in the file.
  @requires:owned-substrate
  Scenario: two doorways that disagree about a page converge on the elected version without anyone re-uploading it
    # THE CURE UNDER TEST — the disagreeing peer OBEYS AN ELECTION rather than copying a peer.
    # It merges every declaration it can see under the shared rule and moves its row only to a
    # winner it has itself verified, under the same never-move-backwards guard as any other head
    # move. Where its own conductor cannot see the election at all, a peer that HOLDS the winning
    # declaration can hand over that declaration link's own signed record, and the disagreeing
    # peer's conductor re-derives it in wasm — the bytes hash to the address they claim, the
    # author's signature verifies, the link binds to this EPR's anchor, the tier parses. Which of
    # those two routes carries a given move depends on what the peer can see, and this scenario
    # does not prescribe one; what it asserts is that no doorway credential, no seed and no deploy
    # is involved anywhere in the chain, and that the peer can verify a carried record and refuses
    # a forged one.
    #
    # Ships DORMANT: the capability is enabled per-fleet by the operator flag
    # ELOHIM_OBEY_CARRIED_ELECTION. Scenario green means the capability works where enabled;
    # visitors experience convergence only once the operator turns it on.
    #
    # THE VEHICLE IS A PAGE THIS RUN AUTHORS, and that is a protocol fact, not a test shortcut.
    # An EARNED canonical declaration is restricted to a page's root author, a device it
    # delegated, or the bootstrap steward — measured 2026-09-05, the conductor refuses anyone
    # else in wasm. So no test can ever manufacture this disagreement on a page it does not
    # own; it stages on its own page, and reads the REAL landing page in the two Thens that name it.
    #
    # WHO HOLDS WHICH HEAD, because the assertions below turn on it: the first peer plants the
    # page's one root and authors the revision it will declare EARNED. The second peer then
    # authors its OWN, LATER revision of that same page, which its conductor declares as its head
    # at the STAGING tier. So the peer that must move is holding the NEWER head, and the head it
    # must move TO is older.
    Given peer "alpha-A" and peer "elohim.host" both declare a head for a page this run authored
    And their declared heads DISAGREE
    And an EARNED canonical declaration exists on the page's root author for the head it authored
    And carried elections are enabled on the fleet via the operator flag ELOHIM_OBEY_CARRIED_ELECTION
    When the reconcile sweep runs on the disagreeing peer
    # OUTCOME — the visitor-facing promise this scenario exists to prove:
    Then the peer's served head moves to the earned-tier elected head
    And both doorways serve the SAME head for that page
    # AGREEING ON THE VERSION IS NOT YET SERVING IT. The two lines above compare what each doorway
    # DECLARES. A doorway can declare the elected head and still hand a visitor the bytes of an
    # older one, because the pointer from the head record to the served bundle is projected
    # separately and can drift behind it — heads equal, pages different, which is the same harm
    # wearing a different costume. These two lines close that gap by comparing, on each doorway,
    # what the running process has MATERIALIZED against what its own storage row DECLARES.
    # SCOPE CHANGES HERE, and the next line is what entitles it to. Everything above concerns the
    # page this run authored. The two checks after it read a REAL, seeded page instead — so the
    # first thing asserted about that page is that this run never wrote to it.
    And EPR "elohim-host-landing" was never staged by this run
    And the served head for EPR "elohim-host-landing" matches the declared head on peer "alpha-A"
    And the served head for EPR "elohim-host-landing" matches the declared head on peer "elohim.host"
    # MECHANISM — how that outcome is trustworthy, not a trust-the-peer copy. The peer that moved
    # gave up a head it had authored LATER and took one authored EARLIER, so no rule that prefers
    # the freshest write — and no simple copy of whoever spoke last — can account for the move.
    And the head it moved to is OLDER than the head it gave up, so recency cannot explain the move
    # Earned-beats-staging is the rule this line exercises: one EARNED declaration against the
    # other peer's staging-tier one. The timestamp tiebreak is the election's SECOND rule and is
    # only reached when two EARNED declarations compete — a shape this scenario does not stage, so
    # the line claims only what it proves: the winner carries the earned declaration, and the
    # notarized timestamp the tiebreak would read is present on it. Staging a real two-earned tie
    # is worth its own scenario; asserting it from here would be an overclaim.
    And the elected head carries the EARNED canonical declaration, and that declaration carries a notarized timestamp
    # ANTI-REGRESSION: the move must be an ELECTION OBEYED, never a trust-the-peer copy.
    And a carried declaration link whose signature or binding fails wasm verification moves nothing
