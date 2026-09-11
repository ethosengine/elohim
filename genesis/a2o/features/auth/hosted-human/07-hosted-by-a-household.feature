@e2e @auth @requires:doorway @hosted-human @act:i @concern:hosted-compute-contracted
Feature: Hosted by a household — the compute a newcomer is lent is promised out loud
  As a newcomer with no computer of my own
  I want the household that runs my cell to have promised it where anyone can check
  So that being hosted is something someone undertook, not a favour that can quietly stop

  This is a station in the hosted-human series — one story per stage of a person's life at a
  doorway, told in order, indexed by `README.md` in this directory. A station born for the
  series is numbered and lives here; a station whose story was written earlier stays where it
  was born and the index names it by path. The station before this one is of the second kind:
  being hosted, told in `../conductor-pool-recovery.feature`. It proves the doorway keeps a
  CELL — the doorway-side home of one person's own record chain, the thing that makes their
  writes theirs — and gets it back to them when its own machines change. It says nothing
  about whose machines those are. A hosted human's cell runs on somebody else's hardware.
  This station says who is actually lending it, and says it in the one place a promise
  survives the doorway that made it — the notary.

  The words this story turns on. A DOORWAY is the gateway service that makes the network
  reachable from an ordinary browser and can run cells on people's behalf; a HOUSEHOLD is
  the people who own one machine on the network and answer for what runs on it. The
  NOTARY is the network's own shared witness: what it
  records, any peer holding the network can read back, without asking the doorway and
  without the doorway being able to take it back. The POOL is the set of conductors —
  network runtimes — a doorway operates on other people's behalf; one conductor holds many
  cells, each cell its own person's, and each pool conductor belongs to one household. That
  household's STEWARD is the peer holding the key that speaks for it on the notary. A
  steward is not the doorway: the doorway arranges the hosting, the steward's machine
  performs it, and it is the steward's key the promise must carry — the doorway's own
  PORTAL, the account pages it shows a person in a browser, is a face on the arranger, not
  on the machine. An AGENT KEY is a cell's cryptographic identity, the key that makes one
  person's writes theirs; it is what a promise names when it says who is being hosted, and
  it is not a username. A COMMITMENT is a promise the notary recorded: it names who
  promised, to whom, what, and until when, and the notary knows it by an opaque identifier —
  a machine's handle for it, never a name to show a person. DELEGATES-COMPUTE is the
  promise this story is about — "I will run your
  cell on my machine, within these bounds". Its SCOPE says what the lent compute is for;
  here it is "hosted-cell", and every commitment carrying that scope is a delegates-compute
  promise, so the scenarios below name them by scope alone once the kind is established.
  A commitment is LIVE while the notary holds it as current and unretracted; WITHDRAWAL
  ends that, and the record stays. The doorway's own SERVICE IDENTITY is the key it uses
  when it acts for itself rather than for anyone it hosts; a promise carrying that key
  would be the doorway promising on its own behalf, which is not what hosting means.

  What is being claimed, and what is not. The claim is that a newcomer who registers at a
  doorway ends up with a cell of their own AND a notarised promise naming the household that
  runs it, with an end date; and that closing the account withdraws that promise. The claim
  is NOT that the network stops holding what the human wrote — a notary keeps what it
  witnessed — nor that the household is obliged to renew. Withdrawal is honest, not amnesia.

  Judged from two sides, as this series requires. The portal side: the account page names the
  household hosting the person, in the words a person uses. The notary side: the doorway
  names the commitment, a peer that is not the doorway reads it back, and the provider on it
  is the steward's key, not the doorway's own. A portal can paint a household's name; only
  the notary can be asked.

  Scenario one registers its newcomer in the open, so the whole chain is visible once. The
  later scenarios say "a newcomer who has created an account" and let the step make one for
  them under a name of its own choosing, because what those scenarios weigh is the promise,
  not the name on it — the one scenario where a name is load-bearing is the account page,
  and what it must show there is the household's name, not the person's.

  The person here is created by the story and removed by it, so this runs against the
  household mesh — the local network of household peers a developer starts on their own
  machine — or against a deployed doorway, and leaves either as it was found. The Prologue
  is the setup pass that mesh performs before any scenario, registering a standing cast of
  hosted humans through the doorway's own registration form. This story never touches that
  cast: it makes its own newcomers. Counting the Prologue's is a different story, told in
  `../../dataplane/doorway-humans-served.feature`.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  Scenario: A newcomer with no computer of their own is lent a cell, and the lending is promised
    Given a newcomer who has never registered at this doorway
    When they create an account at this doorway with the display name "Ada of no fixed machine"
    Then the doorway hosts a cell for them on one of its pool conductors
    And no other account at this doorway shares that cell's agent key
    And the doorway holds a live "hosted-cell" delegates-compute commitment naming that agent key as recipient
    And that commitment names the steward of that pool conductor as provider
    And that commitment carries an end date in the future

  @browser-only
  Scenario: The account page says which household is hosting them
    Given a newcomer who has created an account at this doorway
    When they open their doorway account page
    Then the page names the household hosting them
    And the page names the date their hosting is promised until
    And the page never shows the raw commitment identifier as the household's name

  Scenario: Asked directly, a peer that is not the doorway returns the promise, not the doorway's claim about itself
    Given a newcomer who has created an account at this doorway
    When the doorway names that human's hosting commitment
    And that commitment is read back, by its name, from a household peer that is not the doorway's pool
    Then the read-back commitment's scope is "hosted-cell"
    And its provider is the agent key that the pool conductor's own peer names as its steward
    And its provider is not the doorway's own service identity
    And its recipient is that human's own agent key

  Scenario: Two newcomers are lent two cells under two promises
    Given a newcomer who has created an account at this doorway
    And a second newcomer who has created an account at this doorway
    When both of their hosting commitments are read
    Then the two humans hold different cells
    And each holds their own "hosted-cell" commitment
    And neither commitment names the other human as recipient

  @browser-only
  Scenario: Closing the account withdraws the promise
    Given a newcomer who has created an account at this doorway
    And the agent key of the cell the doorway runs for them
    And the doorway holds a live "hosted-cell" commitment for them
    When they close their account through the portal
    Then no pool conductor holds a cell for that agent key
    And the doorway holds no live "hosted-cell" commitment for that agent key
    And a household peer that is not the doorway's pool still reads that commitment back, carrying the date it was made and the date it ended
