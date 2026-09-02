# Asserts uniform all-peer EPR resolution to kill the per-host stageSpaBlob crutch. Authored
# RED-FIRST (2026-06-29): scenario 2 failed until blobHash-pointer propagation landed. Re-probed
# 2026-09-02: propagation landed — scenario 2 now passes live on both doorways. The remaining red
# is a second, parallel failure mode (version divergence, scenario 4, @wip).
@e2e @dataplane @concern:federation-deploy @requires:multi-node @act:ii
Feature: Federation deploy uniformity — landing EPR resolves on all federation doorways
  A person who types elohim.host into a browser should arrive at the Elohim landing page. Today
  some of them get "App not found" instead — not because the page is gone, but because the
  doorway they happened to reach cannot find it. Which of two addresses a visitor uses decides
  whether the protocol appears to exist at all, and nobody arriving at the front door can tell
  that is what happened to them. That is the harm this feature exists to close.

  Vocabulary, because every assertion below rests on it: a DOORWAY is a gateway node that serves
  content to ordinary browsers on behalf of a federation of peers — the web2 front door to a
  peer-to-peer substrate. An EPR (Elohim Protocol Resource) is the addressable content record a
  doorway resolves in order to serve an application; its blobHash is the pointer from that record
  to the actual bytes, so a null blobHash means the doorway holds the record but cannot find what
  it refers to. The EprRouter is the component on each doorway that follows that pointer and
  serves the bytes — it is what answers "App not found" when the pointer is null. stageSpaBlob is
  the CI stage that uploads the app bundle to each doorway individually — the crutch named below.

  The two peers: alpha-A is the author peer (the one the deploy pipeline authors from) — internally
  also called alpha-a. Peer "elohim.host", internally also called alpha-b, is a second federation
  doorway serving the same content from a different premises. This feature names each peer by the
  identifier its steps resolve against (alpha-A, elohim.host); alpha-a/alpha-b are mentioned only so
  a reader who meets those internal labels elsewhere recognizes the same two peers. Uniform deploy
  means a visitor cannot tell which one they reached.

  A second vocabulary pair, needed only by the final scenario: a HEAD is the specific version of an
  EPR a doorway serves — a pointer to one authored revision, distinct from blobHash (which points at
  bytes; the head names WHICH bytes). A DECLARATION is a signed link that names one head as the
  canonical version of an EPR, carrying a tier: EARNED (authored through the protocol's authority
  path) or STAGING (placed by a deploy/seed scaffold). The ELECTION is the rule every peer shares for
  picking one winner from all declarations it can see: earned beats staging, then the newest
  notarized declaration timestamp, with a deterministic tiebreak — so identical inputs give identical
  winners on every peer. The RECONCILE SWEEP is the periodic background process on each peer that
  measures disagreements and heals the ones it can prove.

  Resolution has a PRECONDITION the first two scenarios do not cover: the deploy pipeline must be
  permitted to place bytes on a doorway it did not author from at all. Scenario 3 covers that link
  in the same chain — it is not a separate concern that happens to share a file.

  The landing EPR (elohim-host-landing) must resolve correctly on EVERY federation doorway,
  not just the deploy-time author peer. Two conditions must hold on each doorway: the root path
  returns HTTP 200 (not "App not found"), and the EPR content record carries a non-null blobHash.
  Both conditions hold today on both doorways (dated probes in the Evidence log below) — the
  pointer-propagation gap this feature was authored against is closed. What the evidence also
  shows is the second failure mode below: the doorways now each resolve the page, but serve two
  different versions of it.

  Uniformity has TWO failure modes, and this feature carries both: bytes/metadata that never
  ARRIVE on a doorway (the first three scenarios — closed for the landing EPR as of 2026-09-02,
  though scenario 3's staging-authority precondition remains @wip pending its own deploy), and
  doorways that each hold the page but serve different VERSIONS of it because the version
  election never reached them (the final scenario — its own vocabulary block sits above it, and
  it is the fleet's live red today).

  This feature is the acceptance gate that makes the per-host Jenkinsfile stageSpaBlob stage
  un-shippable: a uniform deploy MUST mean every doorway can independently resolve the landing
  EPR, not just the peer that ran the CI blob-upload step. blobHash metadata propagation from the
  author peer to all federation peers has now landed (backlog: dataplane-peer-fallback-and-
  blob-replication, item D5) — the crutch itself is not yet retired, since retiring it needs the
  staging-authority precondition (scenario 3) and the version-divergence cure (scenario 4) settled
  first.

  Evidence log (dated probes against the live fleet — history, not the specification above):

  2026-06-29:
    alpha-A      GET /                                          → HTTP 200  SPA bundle served (passing)
    alpha-A      /db/content/elohim-host-landing.blobHash      = sha256-1c3451873f57… (non-null, passing)
    elohim.host  GET /                                          → HTTP 404  {"error":"App not found: elohim-host-landing"}
    elohim.host  /db/content/elohim-host-landing.blobHash      = null  (the gap — causes App-not-found)

  2026-09-02 (re-probed — scenario 2's two conditions now hold on BOTH doorways):
    alpha-A      GET /                                          → HTTP 200  SPA bundle served (passing)
    alpha-A      /db/content/elohim-host-landing.blobHash      = sha256-04ae43109b676984a77aa15e01fc83a6f11068f2cd742e61100753f80adccd5c (non-null, passing)
                 updatedAt 2026-08-31T02:53:06Z
    elohim.host  GET /                                          → HTTP 200  SPA bundle served (passing — the App-not-found gap is closed)
    elohim.host  /db/content/elohim-host-landing.blobHash      = sha256-f0f0e637816611570740a56221c7b9900d27a324f2bb038b1b3b8d76cc7940ad (non-null, passing)
                 updatedAt 2026-08-31T02:47:27Z
    Both peers report the SAME dhtAnchorHash uhCkkvfsTSJ-Ti3fscryEhYv7Nmdwy_LpMEjTFg8PKYv5FIoSuotF — the pointer-propagation
    gap this feature was written to close is closed. What remains open is the SECOND failure mode below: the two
    blobHashes differ, so the doorways hold two different VERSIONS of the same EPR — version divergence, not a missing
    pointer. Both doorways' `/health` report `p2p.caughtUp: false`, `p2p.converged: false`, with `divergentAnchor` counts
    of 2131 (alpha-A) and 1011 (elohim.host) — the reconcile sweep has not settled the fleet.

  # `peer "<name>" at "<alias>"` registers a peer under a local name, resolving the second
  # parameter (an environment-alias key, not a duplicate label) to that peer's live base URL —
  # "alpha-A" resolves to the doorway-alpha environment, "elohim.host" to the public elohim.host
  # endpoint. Every step below addresses peers by the name registered here.
  Background:
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"

  Scenario: alpha-A fully resolves the landing EPR on both conditions (baseline — green)
    # Green baseline. alpha-A is the deploy-time author peer. The Jenkinsfile stageSpaBlob
    # stage wrote the non-null blobHash into the EPR content record and the SPA blob was
    # uploaded locally, so the EprRouter resolves the landing app correctly. Both assertions
    # pass today and confirm the author-peer surface is healthy.
    Given EPR "elohim-host-landing" was uploaded to peer "alpha-A" at deploy time
    When I query "/" on peer "alpha-A"
    Then resolving "/" on peer "alpha-A" does NOT return App-not-found
    And EPR "elohim-host-landing" blobHash is non-null on peer "alpha-A"

  Scenario: elohim.host fully resolves the landing EPR on both conditions (uniform deploy — federation peer)
    # Authored RED-FIRST 2026-06-29. elohim.host is the alpha-b federation peer. At that time the
    # EPR content record on elohim.host had blobHash: null — the blob metadata was never written
    # on the peer even though the blob bytes were placed there at deploy time by the per-host
    # stageSpaBlob CI crutch. Because blobHash was null, the EprRouter on elohim.host could not
    # find the blob to serve and returned HTTP 404 {"error":"App not found: elohim-host-landing"}.
    #
    # It failed then with:
    #   AssertionError: Resolving "/" on elohim.host returned 404 — route may not be registered
    #   (the 404 check fires before the body-text check; both conditions fail independently)
    #   AssertionError: EPR "elohim-host-landing" on elohim.host:
    #     blobHash is null — blob not yet attached
    #
    # Re-probed live 2026-09-02: both conditions now hold — blobHash metadata has propagated to
    # the federation peer, so elohim.host independently serves the landing app. The per-host
    # stageSpaBlob CI stage is not yet retired (see the feature description above); this scenario
    # is the receipt that its replacement — pointer propagation — is working. This is the first
    # fleet-lane run of this scenario: the feature carried @act:i until 2026-09-02, and the fleet
    # lane does not execute Act I, so nothing before today's re-tag ever measured it there.
    #
    # No "was uploaded to peer" Given here, unlike scenario 1 — that absence IS the point: this
    # scenario proves elohim.host resolves the page WITHOUT ever having received a direct
    # deploy-time upload of its own. Whatever made blobHash non-null here arrived by propagation
    # from alpha-A, not by a second per-host stageSpaBlob run.
    When I query "/" on peer "elohim.host"
    Then resolving "/" on peer "elohim.host" does NOT return App-not-found
    And EPR "elohim-host-landing" blobHash is non-null on peer "elohim.host"

  @wip
  Scenario: the deploy authority can stage bytes on a NON-author doorway (precondition for uniform resolution)
    # THE MISSING PRECONDITION NODE. The two scenarios above assert the END of the chain —
    # elohim.host resolves the landing EPR. They are silent on the step immediately before it:
    # whether the deploy pipeline is ALLOWED to put bytes on a doorway it did not author from.
    # It was not. From app #1672 every `seed elohim.host` leg answered 403, so the two
    # assertions above could not have gone green no matter how well blobHash propagated —
    # the bytes never arrived.
    #
    # THE ACTOR: the "fleet deploy authority" is the CI pipeline's own seed credential
    # (API_KEY_SEED) — one credential shared across every doorway in the fleet, and NOT any
    # single doorway's admin identity, which is deliberately different on each one.
    #
    # The cause was a conflation, not a credential: `require_seed_authority` asked "is this
    # caller MY admin?" while one pipeline drives several doorways whose own admin identities
    # are deliberately distinct. It was masked for months because alpha-b.yaml set
    # DEV_MODE:"true" expressly to bypass this gate ("so the CI admin key lands the SPA blob
    # without a per-host credential. Remove when doorway-B federation auth hardens"), until the
    # a later fix correctly required that an uncredentialed caller be connecting from the machine
    # itself. That closed the bypass — and, because the deploy pipeline calls in over the network
    # like anyone else, left it with no way in at all.
    #
    # The fleet seed authority (API_KEY_SEED) answers the right question — "may this caller
    # seed?" — and is scoped to the seed/admin-cache routes only, so it can never stand in for
    # an operator identity. @wip until a deployed build carries the gate — the same way the blob
    # byte-route gate scenario landed ahead of its own deploy.
    Given the fleet deploy authority is the CI pipeline's shared seed credential, distinct from any single doorway's own admin identity
    When the fleet deploy authority stages a blob on peer "elohim.host"
    Then staging the blob on peer "elohim.host" is accepted
    # ANTI-REGRESSION: this must NOT be satisfiable by reopening the hole the gate closed.
    # A credential-free remote caller stays refused; a green here with this line failing means
    # the fix was a bypass, not an authority.
    And staging a blob on peer "elohim.host" with NO credential is refused

  # ── Second failure mode: VERSION DIVERGENCE (parallel to the deploy gap, not downstream of it) ──
  #
  # Everything above is about bytes and metadata ARRIVING. This scenario is about something that
  # can go wrong even when they have: the two doorways each HOLD the page but serve two different
  # VERSIONS of it. (HEAD / DECLARATION / ELECTION / RECONCILE SWEEP are defined in the feature
  # description above, alongside the first vocabulary block.)
  #
  # This failure mode EXISTS TODAY, alongside (not after) the null-blobHash gap: measured live
  # 2026-08-31, both alpha doorways held the SAME bytes for the manifesto but served two
  # DIFFERENT declared heads, for days — because each doorway's head only ever moved when a
  # deploy or seed wrote to that host directly, while the declarations that should settle the
  # election live on the conductor DHT and were not traveling (storage arcs reset to Empty on
  # every restart, so election links never gossiped in; adam's sweep measured 2,619 divergent
  # rows and refused 2,603 per sweep — correctly, having no election to obey).
  @wip
  Scenario: two doorways that disagree about a page converge on the elected version without anyone re-uploading it
    # THE CURE UNDER TEST — carry the election: a peer that HOLDS the winning declaration
    # serves the declaration link's own signed record alongside its head; the disagreeing
    # peer's OWN conductor re-derives it in wasm — the link's bytes hash to the address they
    # claim, the author's signature verifies, the link binds to this EPR's anchor, the tier
    # parses — and merges it with every declaration it can already see under the shared
    # election rule. Only a verified win moves the row, under the same never-move-backwards
    # guard as every other head move. No doorway credential, no seed, no deploy is involved
    # anywhere in this chain — that is the assertion, not an implementation detail.
    #
    # Ships DORMANT: the capability is enabled per-fleet by the operator flag
    # ELOHIM_OBEY_CARRIED_ELECTION. Scenario green means the capability works where enabled;
    # visitors experience convergence only once the operator turns it on.
    Given peer "alpha-A" and peer "elohim.host" both declare a head for EPR "elohim-host-landing"
    And their declared heads DISAGREE
    And an EARNED canonical declaration exists for the newer head on its declaring peer
    And carried elections are enabled on the fleet via the operator flag ELOHIM_OBEY_CARRIED_ELECTION
    When the reconcile sweep runs on the peer holding the older head
    # OUTCOME — the visitor-facing promise this scenario exists to prove:
    Then the peer's served head moves to the earned-tier elected head
    And both doorways serve the SAME head for EPR "elohim-host-landing"
    # MECHANISM — how that outcome is trustworthy, not a trust-the-peer copy:
    And that peer's conductor verified the carried declaration link in wasm before moving it
    And the election obeyed earned-beats-staging, with ties broken on the notarized declaration timestamp
    # ANTI-REGRESSION: the move must be an ELECTION OBEYED, never a trust-the-peer copy.
    And a carried declaration link whose signature or binding fails wasm verification moves nothing
