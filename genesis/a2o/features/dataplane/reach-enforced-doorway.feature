# A doorway projects and caches storage answers for web clients. Those copies
# must never widen the audience storage allowed. Each scenario compares a
# present scoped identifier with a same-shaped unknown identifier, and the
# control proves the source bytes exist on the storage peer behind this doorway.
@e2e @dataplane @concern:reach-enforced-doorway @act:i @wip
Feature: Doorway projections and caches preserve a scoped EPR's audience boundary
  Matthew authored and stewards the fixture for an audience containing only his
  verified author identity; an anonymous client, a self-asserted identity,
  and an invalid bearer are outside that audience. The doorway preserves that
  human choice while making storage convenient to web clients.

  The doorway may project signals, stock responses in its pantry, and proxy EPR
  atoms. None of those conveniences turns a self-asserted identity into verified
  standing or makes a scoped response anonymous.

  An EPR is an Elohim Protocol Resource, and its reach declares its audience. A
  scoped reach restricts that audience to named verified identities; this
  fixture names only Matthew's verified author identity. Its signal projection
  is a shared, anonymous cache of small update documents, so it must never serve
  scoped signals. The pantry is a response cache that can accidentally stock a
  personalized storage answer under a shared lookup key. The EPR atom API reads
  the signed resource record itself. These are three separate exits for the same
  audience boundary.

  A content address is the content-derived identifier of the fixture bytes and
  is itself sensitive when the fixture is scoped. Each unknown control uses the
  same identifier format and route request as the scoped fixture but names no
  resource in storage, projection, pantry, or lookup state.

  A verified doorway session may arrive through a session cookie rather than an
  Authorization header. Pantry admission therefore uses the resolved request
  identity, not the presence of that header.

  Two answers are indistinguishable when their HTTP status, response headers
  that describe content or addresses, and body bytes match. Timing and transport
  length are not measured by these scenarios.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And doorway "alpha" uses household storage peer "james" for reach witness probes
    And household storage peer "james" holds scoped reach fixture "reach-witness-scoped"
    And Matthew retrieves the scoped fixture's expected bytes from peer "james"
    And peer "james" derives the scoped fixture's content address
    And every unknown control uses the scoped fixture's identifier format and route request but is absent from doorway "alpha", peer "james", and their projection, pantry, and lookup state
    And doorway comparisons include status, content or address headers, and body bytes

  @regression
  Scenario: The projection cache does not serve a scoped signal document anonymously
    Given doorway "alpha" has observed the scoped fixture's signal projection
    And doorway "alpha" serves an open signal control through the same projection cache
    When an anonymous client and a client claiming "Agent matthew" ask the projection cache for the scoped fixture and an unknown EPR
    Then each client's scoped answer is indistinguishable from that client's unknown answer
    And no answer contains the scoped signal document or its address
    But the projection cache returns the open signal control's expected bytes

  @regression
  Scenario: The pantry does not stock an identity-scoped response for anonymous reuse
    Given doorway "alpha" resolved Matthew's verified cookie session into request context without relying on an Authorization header
    And doorway "alpha" serves an open control from the warmed pantry
    When Matthew's session requests the scoped fixture through doorway "alpha"
    Then Matthew receives the scoped fixture's expected bytes
    When the doorway pantry attempts to stock that response under the scoped fixture's lookup key
    And an anonymous client asks the warmed pantry for the scoped fixture and an unknown EPR
    Then the pantry answers are indistinguishable
    And neither answer contains scoped fixture bytes
    But the pantry returns the open control's expected bytes

  @regression
  Scenario: A bogus bearer grants no EPR atom through the doorway API
    When a client requests the scoped fixture's atom and an unknown atom from doorway "alpha" with bearer "not-a-real-token"
    Then both doorway EPR API responses are indistinguishable and use the same not-found status
    And within the measured status, headers, and body, neither response reveals the scoped atom or its existence
    But the fixture's verified author can retrieve the scoped atom through the same doorway API
