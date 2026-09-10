@e2e @requires:doorway @act:i @concern:humans-served
Feature: Humans served — a doorway counts the people it is actually hosting
  As someone deciding whether to trust a doorway with my identity
  I want its front page to tell me how many people it is hosting right now
  So that the number means something I could check, not a figure it inherited

  The threshold landing has always had a card called "Humans Served". Until now it showed a
  dash, because nobody had decided what it should count. This story decides it, in the only
  way this protocol can: the number is the count of live hosted-cell commitments this
  doorway's own pool has made. It is a doorway's statement about itself, derived from
  promises the network notarised, and it is checkable by anyone who can read those promises.

  The four words that give the number its meaning, since the number is all this story is
  about. A POOL is the set of conductors — network runtimes — that a doorway operates on
  other people's behalf; each pool conductor belongs to a household, and that household's
  peer holds a key. A COMMITMENT is a promise the network notarised: it names who promised,
  to whom, what, and until when, and anyone holding the network can read it back. A
  HOSTED-CELL commitment is the particular promise counted here — a household undertaking to
  run one person's cell on its machine until a stated date; how one comes to exist is told in
  `../auth/hosted-human/07-hosted-by-a-household.feature`. "Hosting promise" in this story
  means exactly that and nothing else; the two names are one thing. LIVE means the promise
  is in force right now: its end date has not passed and it has not been withdrawn. Closing
  an account withdraws one, which is the last scenario; expiry is the other way a promise
  stops being live, and this story does not exercise it.

  Two boundaries, so the number cannot drift into a slogan. It is NOT federation-wide: a
  doorway counts its own hosting and nobody else's, and the sibling doorway's figure is its
  own. And it is NOT "people who visited": browsing anonymously is not being hosted, and a
  doorway that hosts nobody says nothing but zero.

  Zero and dash are different answers. A dash means "this doorway cannot tell you" — the
  honest answer before this story existed. Zero means "this doorway is hosting nobody", which
  is a fact. Once the source exists, a doorway that hosts nobody must say zero.

  The THRESHOLD LANDING is where that answer is shown: the public front page a doorway serves
  to any visitor before they sign in — the threshold, because it is what a stranger stands on
  while deciding whether to come in. The card is on it.

  Three scenarios need a doorway that is hosting nobody, and this story may not empty one —
  so it checks instead. A doorway found to be hosting someone fails the scenario; it never
  passes it vacuously, because a zero-count claim proved against a doorway nobody checked is
  the same empty card this story exists to retire.

  Where the people come from. The HOUSEHOLD MESH is the local network of household peers this
  story runs against, and the PROLOGUE is the setup pass it performs before any scenario: the
  mesh CASTS its hosted humans, meaning it registers them at a doorway the way a stranger
  would, through that doorway's own registration form. A household is therefore the owner of
  a pool conductor; the mesh is all of them together. This story never creates or removes
  those people — except in its last scenario, which closes one through the product path and
  then asks the mesh to cast it again, so the Prologue gets its cast back.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And doorway "beta" at "E2E_DOORWAY_B"

  Scenario: The hosting doorway's status names the number of humans it hosts
    Given the household mesh has cast its hosted humans through the doorway's own registration
    When the status of doorway "alpha" is read
    Then it names a humans-served count
    And that count equals the number of live hosted-cell commitments its pool has made

  Scenario: A doorway that hosts nobody says none, not nothing
    Given doorway "beta" is checked to be hosting no humans of its own
    When the status of doorway "beta" is read
    Then it names a humans-served count of 0
    And it does not answer with a dash

  Scenario: A registrant at one doorway is not counted at its sibling
    Given the household mesh has cast its hosted humans through the doorway's own registration
    And doorway "beta" is checked to be hosting no humans of its own
    When the status of doorway "alpha" is read
    And the status of doorway "beta" is read
    Then neither doorway's count includes a human hosted only by the other
    And doorway "alpha" counts every human the mesh cast at doorway "alpha"
    And doorway "beta" counts none of them

  @browser-only
  Scenario: The threshold landing shows the doorway its own count
    Given the household mesh has cast its hosted humans through the doorway's own registration
    And doorway "beta" is checked to be hosting no humans of its own
    When a visitor opens the threshold landing of doorway "alpha"
    Then the humans-served card shows the same number the doorway's status names
    When a visitor opens the threshold landing of doorway "beta"
    Then the humans-served card shows 0 rather than a dash

  Scenario: Closing an account drops the count by one, and casting that human again restores it
    Given the household mesh has cast its hosted humans through the doorway's own registration
    And the humans-served count of doorway "alpha" is recorded
    When one of those humans closes their account through the doorway's own close path
    And the status of doorway "alpha" is read again
    Then the humans-served count is one lower than the recorded count
    When the household mesh casts that human again
    And the status of doorway "alpha" is read again
    Then the humans-served count is what it was recorded as
