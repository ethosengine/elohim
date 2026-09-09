# Doorway-failover household observations and one controlled storage-blob fault.
# The full concern spans this file, app deliverability, fixture readiness,
# apex transition and deployed served-shell proof. This file does not alone
# prove public-name continuity during a doorway loss or catch-up shed.
#
# Historical origin: on 2026-07-31 elohim.host returned503 while alpha served.
# The former apex scenario only sampled a root GET; its actual induced-shed
# promise now lives in doorway-apex-transition.feature. Freshness/startup checks
# below inspect encountered states, preserving the original wire assertions.
# The blob scenario explicitly induces and restores a primary-storage stall.
#
# Wire detail: steps/dataplane/failover.steps.ts and
# src/framework/dataplane/surfaces.ts own serving/shedding/dead classification,
# freshness receipts and bounded probes. Doorway-failover's habit atom owns
# graduation; no single observed-state pass supplies missing transition proof.
@e2e @dataplane @concern:doorway-failover @act:i
Feature: Doorway failover — two doorways, one name, one truth
  A person reaching for elohim.host should never inherit a single doorway's bad
  hour. The doorway pair must be honestly classifiable, someone must always be
  serving, the apex name must ride through a sibling's shed, and whoever serves
  must serve the same declared truth. A doorway's bad hour is not always its
  own, either: it can be inherited from the storage peer it reads from, so the
  same question — must this bad hour reach the person? — is asked twice, once
  between the doorways and once behind one of them.

  THE PAIR. "alpha-A" and "elohim.host" are two doorways holding the same
  converged content, each reachable at its own address. "elohim.host" is also
  the APEX NAME — the address a person actually types — and it is pinned to its
  own doorway in the current broken topology, so that doorway's bad hour
  becomes the public name's bad hour unless failover holds. "alpha-A" is the sibling that keeps serving through it.
  The DECLARED HEAD both must agree on is the notarized identity of the current
  version of a piece of content: same head, same answer. The content most of these
  scenarios read, "elohim-host-landing", is the landing page itself — the first
  thing a stranger typing the apex name ever sees, and so the read whose failure
  is most expensive. One scenario reads "manifesto" instead, for a reason given
  where the difference is introduced: it needs content that more than one peer
  holds.

  STEP VOCABULARY. A peer named "alpha-A" or "elohim.host" in these steps is
  a doorway endpoint. Storage peers are the separately named holders behind it.
  FAULT SAFETY. The harness registers unconditional restoration before inducing
  the blob fault. It restores the primary even when a later assertion fails;
  the scenario's final recovery assertion checks that restoration was observed.
  SAME-HEAD SCOPE. The same-head assertion refuses zero serving doorways. With
  one serving doorway it checks that doorway can resolve the declared head;
  with two it also requires their heads to agree. A single survivor does not
  prove that the unavailable sibling has converged.

  HOW A DOORWAY IS CLASSIFIED. Serving = it answers GET / with 200. Shedding =
  it answers 503 carrying the specified catching-up contract — behind, and
  saying so out loud. Dead = it answers neither / nor /health at all. Shedding
  is honest degradation and clears the classification bar; dead does not.

  MEASUREMENT SCOPE. The knowledge, authority and startup-consistency scenarios
  below inspect the state they encounter. They check the wire contract for
  that state; they do not manufacture catch-up, force an amber response or
  prove that a closed breaker later opens correctly. The blob scenario alone
  induces a storage fault here. Public-name shed and recovery are separate
  transition promises in doorway-apex-transition.feature. A steady-state pass
  in this file cannot fulfil those unmeasured transitions.

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

  HOW A DOORWAY GETS READY TO SERVE. A doorway does not open cold: at
  startup it WARMS UP — streams its content, humans and relationships from
  its storage pool before it will call itself ready. A doorway also holds a
  BREAKER per storage peer, the live open/closed circuit that decides
  whether a request to that peer is honoured or shed — the same decision
  behind every "shedding" answer above. Both are self-reported on the
  doorway's STARTUP SURFACE, its own status page, so an operator (or this
  suite) can read "am I actually ready?" and "am I actually refusing
  requests?" without guessing from outside.

  WHERE A DOORWAY'S ANSWERS COME FROM, AND WHY IT MATTERS WHICH PEER GIVES
  THEM. A doorway does not hold content; it reads from a POOL of storage peers,
  and it declares one of them its PRIMARY — the peer it asks first for
  everything. A peer ANSWERS or it STALLS — the peer-level counterpart of a
  doorway serving or shedding. Stalling is the quiet one: still there, still
  accepting the connection, answering nothing. Enough stalled answers and the doorway's breaker for that
  peer opens, and from then on the doorway sheds every read it would have sent
  there. That is how a doorway inherits a bad hour it did not have.

  Whether it SHOULD inherit it depends on what is being read. A PROJECTION —
  content, humans, a head-record — is one peer's own worked-out answer, so
  asking a different peer is asking a different question and may get a
  different answer; the primary owns it, stall or no stall. A BLOB is the
  opposite: it is named by the hash of its own bytes, so every peer in the pool
  that holds it — every HOLDER — holds the byte-identical thing, and any holder
  answering is the same answer. Refusing a blob read because the primary
  stalled tells a person "unavailable" about bytes the pool had all along.
  So for blobs, and only for blobs, the doorway may choose a different single
  peer to spend its one request on — one request, one peer, still; it just
  stops spending it on a peer it already knows is refusing.

  Which is why the blob-failover scenario reads "manifesto" rather than the
  landing page: a blob only has somewhere to ride to if a SECOND holder exists,
  and "manifesto" is the commons content this pool is known to replicate. It is
  also the one scenario below that names a single doorway, because that is the
  altitude it works at — everything before and after it asks the question
  between the pair.

  BORN RED means a scenario specifies behaviour that does not exist yet: it
  fails on purpose, its assertion is the specification for the CURE — whatever
  implementation would make it green — and it is never weakened to make a run
  green. A `@requires:` tag says something else entirely: not that the cure is
  missing, but that this lane lacks the substrate the scenario needs, so the
  scenario is SKIPPED rather than failed. `@requires:owned-substrate` names the
  strongest of those: control of a live peer — permission to stop one on
  purpose and the standing to promise it comes back.

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

  Scenario: The apex name serves the landing page before a fault
    # Steady-state prerequisite only. The actual induced-shed, sibling-selection
    # and recovery contract lives in doorway-apex-transition.feature. A pass here
    # cannot fulfil that feature or prove DNS/ingress failover.
    When I query "/" on peer "elohim.host" expecting raw text
    Then the raw response status is 200
    And the raw response body contains "app-root"

  @requires:owned-substrate
  Scenario: A blob rides through its primary's bad hour
    # Nothing here is waiting for an organic stall — I induce one, so the last
    # step is mine to undo, and I assert the undoing rather than assume it: a
    # stopped peer left stopped breaks the substrate for every scenario after
    # this one.
    Given content "manifesto" has a blob that a holder behind doorway "alpha-A" other than its primary answers for
    When doorway "alpha-A" loses its primary storage peer to a stall
    Then the blob still arrives through doorway "alpha-A", from a holder that is not the stalled primary
    And I restore doorway "alpha-A"'s primary storage peer, and it answers again

  Scenario: Serving doorways resolve a declared head and serving siblings agree
    # Failover that changes the answer is worse than an outage. Every doorway
    # currently classified as serving must resolve the declared head for the
    # landing content — and when both serve, their heads must be identical
    # (the ch10 "two doorways, one truth" bar, sampled in this run's state).
    Then every serving doorway among "alpha-A" and "elohim.host" resolves the same declared head for content "elohim-host-landing"

  Scenario: Knowledge reads answer with an explicit freshness receipt
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

  Scenario: Observed authority responses satisfy the green-only policy
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

  Scenario: The doorway publishes its freshness stage and required colours
    # A visitor or operator can inspect the declared policy before requesting
    # content. This checks that publication and its required vocabulary only;
    # it does not correlate the declaration with the other scenarios' responses.
    Then doorway "alpha-A" advertises a freshness stage, with authority green-only and knowledge amber-ok
    And doorway "elohim.host" advertises a freshness stage, with authority green-only and knowledge amber-ok

  Scenario: A completed warmup always has a servable head to show for it
    # A false "completed" is not just an internal bookkeeping error — it is
    # the difference between an operator who KNOWS to restart a stuck
    # doorway and one who doesn't, so the person at the door keeps waiting
    # longer than they had to.
    # MEASURED 2026-08-21 (local mesh): a warmup pass that streamed nothing
    # from storage — pool unreachable, or reachable and empty — used to
    # report `completed: true` anyway, so a doorway that never actually
    # warmed looked identical to one that had, and went on serving an empty
    # projection until someone restarted it. "Completed" now means what it
    # says: it always has a servable head to show for it.
    Then a completed warmup on doorway "alpha-A" has a servable head to show for it
    And a completed warmup on doorway "elohim.host" has a servable head to show for it

  Scenario: Startup reporting agrees with the doorway's observed serving state
    # An operator who trusts a lying startup page cannot fix a shedding
    # doorway they believe is healthy — so the person at the door stays
    # locked out longer than the outage itself required.
    # MEASURED 2026-08-21 (local mesh): a doorway actively shedding 503
    # "catching-up" for every request answered its OWN /health/startup with
    # every upstream circuit "closed" — a second, private breaker map the
    # shed decision never consulted. An operator reading the startup surface
    # saw a healthy doorway that was refusing everything.
    Then doorway "alpha-A" cannot look healthy on its startup page while it is actually shedding
    And doorway "elohim.host" cannot look healthy on its startup page while it is actually shedding
