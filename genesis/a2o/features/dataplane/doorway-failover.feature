# Doorway failover — the "two doorways, one name" invariant the resiliency saga
# ends on. Chapter 4 of that saga (features/dataplane/resiliency-saga/) proves
# hosting is real in steady state (GET / serves the shell); THIS concern asserts
# the stronger thing: hosting SURVIVES — a person hitting the apex name gets the
# landing shell even while one doorway is dead or in its post-deploy catch-up
# shed window.
#
# The vocabulary the scenarios run on — serving/shedding/dead, green/amber,
# knowledge/authority — is defined in the Feature block below, NOT up here: a
# scenario a reader cannot follow without its comments is not yet a story. What
# stays in comments is provenance and wire detail.
#
# Spine node: doorway-failover (genesis/manifests/habits.yaml). Sprint plan:
# genesis/docs/superpowers/plans/2026-07-31-doorway-federation-failover-sprint-plan.md.
#
# Measurement philosophy (v1): NO synthetic chaos. The post-deploy
# arc-convergence window on the peer named adam reopens for hours after every
# edge deploy (self-heal-adam-projection-catchup-exhaustion-full-arc.md), so the
# live pair organically supplies real shed windows; scenarios assert the
# invariants that must hold THROUGH those windows. v2 escalation (not built):
# the kill-on-purpose shape from features/federation/peer-loss-failover.feature
# applied at the doorway layer.
#
# Wire detail behind the vocabulary (steps/dataplane/failover.steps.ts):
#   shedding is confirmed by the 503's own {"status":"catching-up"} body, or by
#     an open upstream circuit / rising admission.shedTotal in status.json —
#     spec: 2026-07-19-doorway-catching-up-page-design
#   freshness rides response headers — x-elohim-freshness (green|amber) and
#     x-elohim-freshness-class (knowledge|value|authority) on every proxied GET;
#     on amber also x-elohim-stocked-at + x-elohim-served-head; on a shed,
#     x-elohim-freshness-required. The advertised policy is the `freshness`
#     block on status.json / /health/serving.
#
# Born red 2026-07-31: scenario "The apex name survives its doorway's shed"
# live-probed 503 (elohim.host mid-catch-up) while doorway-alpha served 200.
# The assertion IS the specification for the cure (apex multi-A + client
# fallback + warm-boot shell cache + the adam-peer provisioning ceiling) — do
# not weaken it to pass.
# Born red 2026-08-21: the three freshness scenarios. No deploy carries the
# graded-freshness cure yet, so today these reads shed instead of declaring a
# colour, and no doorway emits a freshness header at all.
@e2e @dataplane @concern:doorway-failover @act:i
Feature: Doorway failover — two doorways, one name, one truth
  A person reaching for elohim.host should never inherit a single doorway's bad
  hour. The doorway pair must be honestly classifiable, someone must always be
  serving, the apex name must ride through a sibling's shed, and whoever serves
  must serve the same declared truth.

  THE PAIR. "alpha-A" and "elohim.host" are two doorways holding the same
  converged content, each reachable at its own address. "elohim.host" is also
  the APEX NAME — the address a person actually types — and it is pinned to its
  own doorway, so that doorway's bad hour becomes the public name's bad hour
  unless failover holds. "alpha-A" is the sibling that keeps serving through it.
  The DECLARED HEAD both must agree on is the notarized identity of the current
  version of a piece of content: same head, same answer. The content these
  scenarios read, "elohim-host-landing", is the landing page itself — the first
  thing a stranger typing the apex name ever sees, and so the read whose failure
  is most expensive.

  HOW A DOORWAY IS CLASSIFIED. Serving = it answers GET / with 200. Shedding =
  it answers 503 carrying the specified catching-up contract — behind, and
  saying so out loud. Dead = it answers neither / nor /health at all. Shedding
  is honest degradation and clears the classification bar; dead does not.

  HOW AN ANSWER DECLARES ITS FRESHNESS. Green = the doorway answered from its
  live upstream, the current DHT-witnessed projection. Amber = it answered from
  bytes it stocked earlier — true and content-addressed, just not re-witnessed
  since the moment the answer names. Which colours are acceptable depends on
  what the read is FOR. A KNOWLEDGE read (the landing page, content, blobs) may
  be amber: a page twenty minutes old costs a person far less than a locked
  door. An AUTHORITY read (the head-record a peer's declared truth rests on,
  and anything that changes state) must be green: an answer that is merely
  probably-still-true cannot be told apart from one that is current, so serving
  it would spend trust the doorway has not earned. A doorway that cannot meet
  the required colour sheds, and says which colour it needed.

  All of that is said ON THE WIRE, not inferred: every proxied GET carries its
  colour and its read class as response headers, an amber answer additionally
  carries when it was stocked and the content-address of the bytes it served,
  and a shed carries the colour it required. The policy itself is published on
  the doorway's own status surface, so it can be read before a request is spent.

  Each doorway also declares the STAGE it believes the network is in —
  simulacra, bootstrap, coordinated or enforced — the maturity setting the
  freshness policy is read against. The two rows that never move with the
  stage are the ones these scenarios probe: authority is green-only and
  knowledge is amber-ok in every stage.

  BORN RED means a scenario specifies behaviour that does not exist yet: it
  fails on purpose, its assertion is the specification for the cure, and it is
  never weakened to make a run green.

  Background:
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"

  Scenario: Every doorway is honestly classifiable — shed is not death
    # A doorway mid-catch-up is DEGRADED, not down: it must answer its /health
    # and present the specified shed contract, never a silent connection void.
    # This is the readiness contract any routing layer (multi-A client
    # fallback today, LB health checks later) gets to route on.
    Then doorway "alpha-A" classifies as serving or shedding, not dead
    And doorway "elohim.host" classifies as serving or shedding, not dead

  Scenario: The pair floor holds — at least one doorway is serving
    # Both-shedding (or worse) means no human can reach the commons at all —
    # the floor this arc exists to keep. Green today via whichever sibling is
    # outside its deploy window; a correlated outage turns it red honestly.
    Then at least one of doorways "alpha-A" and "elohim.host" is serving

  Scenario: The apex name survives its doorway's shed
    # THE BORN-RED. elohim.host (the apex) is pinned to its own doorway — the B
    # side of the pair; when that doorway enters its hours-long post-deploy
    # catch-up, the NAME sheds even though the healthy sibling "alpha-A" holds
    # the identical converged content. Saga chapter 4 accepts
    # steady-state green; this scenario does not — the name, not the pod, is
    # what a person trusts.
    When I query "/" on peer "elohim.host" expecting raw text
    Then the raw response status is 200
    And the raw response body contains "app-root"

  Scenario: Whoever serves, serves the same declared truth
    # Failover that changes the answer is worse than an outage. Every doorway
    # currently classified as serving must resolve the declared head for the
    # landing content — and when both serve, their heads must be identical
    # (the ch10 "two doorways, one truth" bar, held through failover).
    Then every serving doorway among "alpha-A" and "elohim.host" resolves the same declared head for content "elohim-host-landing"

  Scenario: Being behind is a signal, not an outage
    # The landing page is knowledge, not authority: someone reading it is not
    # casting a vote or moving a head. Refusing that read because a background
    # circuit is open teaches them the commons is DOWN when it is merely BEHIND —
    # and they cannot tell those apart from a closed door, so they leave. So the
    # read is answered, and the answer declares what it is: green (live from the
    # upstream) or amber (bytes this doorway stocked earlier). Amber must carry
    # its own receipts — WHEN it was stocked and WHICH head it served — because
    # an undated last-good answer is indistinguishable from a lie.
    # BORN RED 2026-08-21: today this read answers 503 inside a catch-up window
    # and carries no freshness header at all.
    Then the knowledge read "/db/content/elohim-host-landing" on doorway "alpha-A" is answered and declares its freshness
    And the knowledge read "/db/content/elohim-host-landing" on doorway "elohim.host" is answered and declares its freshness

  Scenario: Authority reads never ride amber
    # A head-record read is how one peer learns what another peer swears is
    # current — the same answer the "one truth" scenario above compares across
    # the pair. Serving that from stocked bytes would let a doorway swear to
    # something it has not re-witnessed, which is the one failure a notary chain
    # exists to prevent: everything downstream would look converged while being
    # quietly wrong. So authority has exactly one honest colour. When the doorway
    # cannot be green it must shed and NAME the colour it needed, so the caller
    # knows to come back rather than to accept what it got.
    # BORN RED 2026-08-21 with the scenario above — same missing cure.
    Then the authority read "/db/content/elohim-host-landing/head-record" on doorway "alpha-A" is green or honestly shed
    And the authority read "/db/content/elohim-host-landing/head-record" on doorway "elohim.host" is green or honestly shed

  Scenario: What the doorway advertises is what it serves
    # A trust signal nobody can read ahead of time is not a contract. The doorway
    # publishes its stage and the freshness each class of read requires, so an
    # operator — or a sibling doorway, or a client deciding whether to retry —
    # can predict the answer before spending a request on it, and can catch a
    # doorway whose advertised policy has drifted from what it actually serves.
    # BORN RED 2026-08-21 with the two scenarios above — same missing cure.
    Then doorway "alpha-A" advertises a freshness stage, with authority green-only and knowledge amber-ok
    And doorway "elohim.host" advertises a freshness stage, with authority green-only and knowledge amber-ok
