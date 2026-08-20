# The HTTP egress half of the `reach-enforced-everywhere` habit.
#
# This feature exists because of a defect, and the defect's shape is the reason
# the feature carries @dataplane. On 2026-08-20 the content LIST route decided
# reach filtering from header PRESENCE — `has_auth = AUTHORIZATION.is_some() ||
# X-Agent-Id.is_some()` — so any caller sending `Authorization: Bearer <
# anything>` received every restricted row WITH its content body. Measured live
# on doorway-alpha: 90 rows anonymously, 1000 rows with the literal token
# "bogus", including private and intimate content.
#
# It survived for months because no reach feature in this suite carries the
# @dataplane tag: reach was the one plane the edge Dataplane Validation stage
# never measured. Every other habit here is bound to a probe that runs each
# deploy. This one was not, so the register honestly counted it `unwired`.
#
# The assertions below name NO reach tier. `elohim-storage/CLAUDE.md` forbids
# canonizing a single reach vocabulary while the multi-vocabulary drift is open,
# so the property asserted is a RELATION that holds under every vocabulary:
# a credential the substrate cannot verify grants nothing beyond no credential
# at all. That is drift-proof, and it is precisely what the bypass violated.
@e2e @dataplane @concern:reach-enforced-http
Feature: Reach is enforced at the HTTP egress, not inferred from a header
  "Reach" is the audience an EPR has earned — who may receive it. It is
  independent of which version is canonical and of how many peers hold the
  bytes; only reach decides who may read.

  Authorization is unconditional at every deployment posture (dev, staging,
  production). A posture may cheapen the DEPTH at which a requester's identity
  and relationships are verified — resolving them from cheaper local data rather
  than fully-propagated proof — but it must never decide WHETHER the decision is
  made, and it must never fail toward granting broader access. Presenting an
  unverifiable credential is therefore indistinguishable, in what it returns,
  from presenting none.

  Two properties are compared for each posture: the set of rows returned, and
  how much content each row carried. Same rows with fuller bodies is still a
  leak, so both must match the anonymous answer exactly.

  Background:
    # "alpha-A" is the A-side doorway of the alpha fleet's A/B pair, resolved to
    # a URL by the shared dataplane peer resolver (E2E_DOORWAY_ALPHA). Every
    # scenario probes it as an ordinary web client would.
    Given peer "alpha-A" is reachable for reach probes
    # Control: without this, "returned no more than anonymous" is trivially true
    # on a corpus holding nothing restricted, and all three scenarios would go
    # green while proving nothing. "bdd-smoke-tests" is a private-reach fixture.
    And peer "alpha-A" holds a restricted-reach fixture an anonymous caller is refused, named "bdd-smoke-tests"

  # The measured defect, stated as its own regression test. Before the
  # 2026-08-20 cure this scenario fails with ~910 leaked rows.
  Scenario: An unverifiable bearer token grants nothing beyond anonymous
    When I list content on peer "alpha-A" anonymously
    And I list content on peer "alpha-A" presenting header "Authorization: Bearer not-a-real-token"
    Then both listings answered 200
    And the presented credential returned no rows beyond the anonymous listing
    And the presented credential returned no fuller content than the anonymous listing

  # A distinct defense from the one above, which is why this is its own
  # scenario. Storage trusts `X-Agent-Cid` VERBATIM — `extract_agent_cid`
  # returns the header value without validating it — so a caller reaching
  # storage directly could assert any identity. What stands between that and the
  # open web is the doorway stripping the header before forwarding. This pins
  # the strip: a doorway change that stopped stripping would otherwise be silent
  # until someone probed for it, which is how the bearer bypass survived.
  Scenario: A self-asserted identity header is stripped and grants nothing beyond anonymous
    When I list content on peer "alpha-A" anonymously
    And I list content on peer "alpha-A" presenting header "X-Agent-Cid: matthew"
    Then both listings answered 200
    And the presented credential returned no rows beyond the anonymous listing
    And the presented credential returned no fuller content than the anonymous listing

  # The legacy header name, still honored by some call paths. Same strip
  # invariant, separate wire name — a strip that covered only the current name
  # would leave this one open.
  Scenario: A self-asserted legacy agent header is stripped and grants nothing beyond anonymous
    When I list content on peer "alpha-A" anonymously
    And I list content on peer "alpha-A" presenting header "X-Agent-Id: matthew"
    Then both listings answered 200
    And the presented credential returned no rows beyond the anonymous listing
    And the presented credential returned no fuller content than the anonymous listing
