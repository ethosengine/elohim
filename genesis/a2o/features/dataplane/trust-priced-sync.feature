# RED-FIRST: born red. Scenario 1 runs on today's steps and FAILS today because every
# sync edge on every mesh is priced "public" — the trust handshake is a stub that asserts
# verification without performing it. Scenarios 2-5 are @wip until the implementation
# increment each one names has landed (the increments are called stations and are ordered
# in the spec: genesis/docs/superpowers/specs/2026-09-01-trust-priced-sync-edge-design.md).
# A red concern here surfaces as UNSTABLE in the dataplane rollup; it never blocks a build.
@e2e @dataplane @regression @requires:household-nodes @concern:trust-priced-sync @act:i
Feature: A household's own peers are trusted edges, and trusted edges catch up first
  Matthew, Jessica and James each run a peer in the same house. When one of their machines
  restarts, it should catch up from the other two before it spends any effort on strangers —
  because those two are the peers this household actually depends on. And if a tornado takes
  the house, the doorway the household registered with should already hold their latest
  writings, because its edge to them was treated as a household edge, not a stranger's.

  Vocabulary, because every assertion below rests on it. A PEER is one running storage node.
  An EDGE is one peer's view of one other peer — the same two machines are two edges, one in
  each direction. The DHT (distributed hash table) is the shared ledger every peer holds a
  validated copy of; membership, relationship and replication-commitment records live there.
  A DOORWAY is a gateway a household registers with so its content stays reachable over the
  ordinary web; it runs a storage peer of its own, and that peer holds the household's content
  under a replication commitment recorded on the DHT.

  When two peers connect they perform a TRUST HANDSHAKE: each presents its agent key and the
  membership, relationship and commitment records it holds, and the OTHER peer checks each
  record against its own copy of the DHT. From what it can verify, the receiving peer derives
  a CLASS for the edge: "unverified" (nothing could be verified), "public" (a real agent, but
  no relationship with us), "community" (we share a consented membership outside the
  household), or "trusted" (a household membership, a trusted relationship, or an active
  replication commitment between us). The household mesh has no shared non-household
  collective, so "community" is defined here but not exercised on this substrate; nor is
  "unverified", because every peer on a healthy household mesh presents verifiable records.
  Both classes are exercised where such peers exist — the alpha fleet lane (a shem peer with
  no household record) — not in this household act. A REPLICATES-DWELLING COMMITMENT is the
  named record on the DHT by which one peer agrees to hold a copy of a household's content.

  A peer's sync-request counter is labeled by that class, so the peer itself reports what it
  believes about each edge. The class then sets that edge's BUDGET: how many documents to ask
  for per round, how long to wait for an answer, how soon to retry after a failure, and how
  much of each catch-up round the edge is admitted to. Trust WIDENS a budget; it never gates
  one — a stranger still gets served, only after the household is caught up.

  Station map (the implementation increments the spec orders, and the scenario each one
  unlocks): station 1, the honest handshake — scenarios 1 and 5; station 2, the budget
  predicate — scenario 4; station 3, provider order and admission — scenarios 2 and 3.

  Why it matters: today all of this is a costume. Every edge is labeled "public" whether the
  peer on the other end is Jessica or someone nobody knows, so a restarting household peer
  treats its own family exactly like strangers, and the sync plane cannot give the household
  the cheaper, faster convergence the protocol promises it.

  Background:
    # On the household mesh these three aliases are the local storage peers — the whole household.
    Given peer "matthew" at "E2E_STORAGE_MATTHEW"
    And peer "jessica" at "E2E_STORAGE_JESSICA"
    And peer "james" at "E2E_STORAGE_JAMES"

  # Temporal precondition: the counters below are cumulative since each peer started and are
  # read after the mesh lane's readiness gate — the test harness waits until every household
  # peer is connected and at least one 60 s sync round has completed before any scenario runs,
  # so the handshake has run on every edge. Nothing here triggers traffic.
  Scenario: On a household mesh every edge is family, so no peer may honestly price any edge as public
    # THE FALSIFIER. The household mesh has exactly three peers and all three share one
    # household membership on the DHT, so after a truthful handshake there is NO edge that
    # can honestly be "public": every edge each peer holds is to family. Read from all three
    # peers, because an edge is one peer's view — a handshake that verifies in one direction
    # and asserts in the other would pass a one-peer read.
    #
    # FAILS today with: peer_class="trusted" is absent (measured 0) and peer_class="public"
    # ok reads ~48 on each peer — the handshake stub asserts "public" for everyone.
    Then labeled metric "elohim_sync_request_outcomes_total" with label "peer_class" "trusted" on peer "matthew" >= 1
    And labeled metric "elohim_sync_request_outcomes_total" with label "peer_class" "public" on peer "matthew" == 0
    And labeled metric "elohim_sync_request_outcomes_total" with label "peer_class" "trusted" on peer "jessica" >= 1
    And labeled metric "elohim_sync_request_outcomes_total" with label "peer_class" "public" on peer "jessica" == 0
    And labeled metric "elohim_sync_request_outcomes_total" with label "peer_class" "trusted" on peer "james" >= 1
    And labeled metric "elohim_sync_request_outcomes_total" with label "peer_class" "public" on peer "james" == 0
    # ANTI-REGRESSION: "trusted" must not become a new universal default the way "public"
    # was. The honest answer for an edge the peer could not verify is "unverified", and that
    # label must stay reachable. There is no unverifiable peer on a healthy household mesh,
    # so this scenario cannot assert unverified > 0 (a stranger with verifiable records is
    # "public", scenario 2; "unverified" is exercised on the fleet lane, see the vocabulary).

  @wip
  Scenario: A stranger who joins the running household mesh is still served, and is priced as a stranger
    # Trust widens; it never gates. A fourth peer with no membership, relationship or
    # commitment with the household joins the RUNNING mesh (no household peer restarts). It
    # must converge on a household document — retrievability is never class-gated — and
    # Matthew's peer must price that edge "public", not "trusted".
    # @wip: the late-joiner step definition does not exist yet (the mesh verb does).
    Given a late-joining peer "stranger" has joined the running household mesh
    When a content sync doc is selected from peer "matthew"
    Then the selected sync doc has converged onto peer "stranger"
    And labeled metric "elohim_sync_request_outcomes_total" with label "peer_class" "public" on peer "matthew" >= 1
    # ANTI-REGRESSION: the stranger's arrival must not demote the family.
    And labeled metric "elohim_sync_request_outcomes_total" with label "peer_class" "trusted" on peer "matthew" >= 1

  @wip
  Scenario: A restarted household peer catches up from its family before it spends a round on the stranger
    # The promise in the title, measured as ORDER. With the stranger still present, Jessica's
    # peer is warm-restarted (its data kept, its process restarted). Its sync-request counter
    # starts from zero, so the first sync round after restart shows exactly whom it asked
    # first. Admission is weighted by class — household edges get more of the round — so the
    # first round must contain household requests and must complete the household catch-up
    # (caught-up against Matthew and James) before the stranger edge is granted its share.
    # @wip until the admission ring lands (station 3) and the warm-restart step exists.
    Given a late-joining peer "stranger" has joined the running household mesh
    When peer "jessica" is warm-restarted
    And peer "jessica" completes its first sync round after restart
    Then labeled metric "elohim_sync_request_outcomes_total" with label "peer_class" "trusted" on peer "jessica" >= 1
    And peer "jessica" reports caught-up against peer "matthew" and peer "james"
    # The stranger is served in the SAME round or the next, never starved — the floor.
    And labeled metric "elohim_sync_request_outcomes_total" with label "peer_class" "public" on peer "jessica" >= 1

  @wip
  Scenario: Budget decisions are counted through a typed class and reason, and no decision goes uncounted
    # Observability per decision: a peer that widens a window for family, or shrinks one for
    # an edge that keeps timing out, counts that decision through a typed reason. Nothing is
    # inferred from log silence. The universal half is the third line: the counter has no
    # "unknown" reason bucket to hide in. @wip until the budget predicate lands (station 2).
    Then labeled metric "elohim_sync_edge_budget_total" with label "peer_class" "trusted" on peer "matthew" >= 1
    And labeled metric "elohim_sync_edge_budget_total" with label "reason" "class_row" on peer "matthew" >= 1
    And labeled metric "elohim_sync_edge_budget_total" with label "reason" "unknown" on peer "matthew" == 0

  @wip
  Scenario: The doorway a household registered with is a trusted edge, not a stranger's
    # The tornado case. A household registers with a doorway by holding an active
    # replicates-dwelling commitment with that doorway's storage peer. That commitment is a
    # record on the DHT, so the doorway's peer can verify it in the handshake and derive
    # "trusted" for its edge to the household — which is what puts the household's writings
    # at the front of that peer's catch-up, before the house is gone.
    # @wip until the commitment input to the class lands (station 1) and the local mesh
    # doorway's storage peer is addressable as a peer alias.
    Given the household holds an active "replicates-dwelling" commitment with the doorway's storage peer
    Then labeled metric "elohim_sync_request_outcomes_total" with label "peer_class" "trusted" on peer "doorway-storage" >= 1
