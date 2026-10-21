# HELD (2026-10-20). Every scenario is @wip: no step below has a definition yet, and the thing it
# checks is not built. Nothing yet tells a person who holds their key when an app opens, and
# nothing records that they were told. The placement-contract design lists that record among its
# missing nodes ("hub opens an app, the person is told: a disclosure record", §15 of
# genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md).
# §17 of the same design places the hub that runs a cell for someone in slice 10, the runtime
# redesign. This story is held until the disclosure record lands, with slice 10 or before it.
# Glue belongs in steps/ui/hosted-human.steps.ts, reusing that file's phrases first.
@e2e @auth @requires:doorway @requires:household-nodes @hosted-human @act:i @concern:hosted-compute-contracted @wip
Feature: Opening an app, a hosted person is told which machine holds their key and can sign as them
  As a newcomer whose doorway arranged for a household's machine to run my cell
  I want every app I open to tell me, before it does anything in my name, whose machine holds my key
  So that I know who can sign as me, and I am never told my key is somewhere it is not

  This is a station in the hosted-human series: one story per stage of a person's life at a
  doorway, told in order and indexed by `README.md` in this directory. It follows the station
  told in `07-hosted-by-a-household.feature`, which proves that a household promised to run the
  person's cell. That station says who lends the machine. This one says what the person is told
  about it at the moment it matters: when they open an app that will act in their name.

  The words this story turns on. A DOORWAY is the gateway service that makes the network
  reachable from an ordinary browser; its PORTAL is the sign-in and account pages it shows a
  person. A CELL is one person's own record chain, kept inside a CONDUCTOR, the network runtime
  that runs cells. Every record a cell writes is signed with the person's KEY, and whoever holds
  that key can sign as the person. A HOSTED person is one whose doorway keeps their identity for
  them: the doorway arranged for a conductor it uses, one of its POOL, to run their cell and to
  keep their key. A POOL CONDUCTOR is one of the conductors in the doorway's pool. Each pool
  conductor runs on a machine that belongs to one HOUSEHOLD, the people who own that machine and
  answer for what runs on it; that household is the conductor's STEWARD. Here that machine is
  the household's HUB: a machine the household leaves on all the time, which can run cells for
  people who have no machine of their own.

  So a hosted person's key is held on a household's hub, by arrangement of the doorway, and not
  on any device the person holds. Every scenario below tests that arrangement.

  Every machine that takes part in the network is a PEER, and a household may run several. A
  doorway's pool is only some of those peers: the ones it asks to run cells. A peer outside the
  pool is one the doorway does not use and does not control. The NOTARY is the network's shared
  witness: a record it keeps is copied across peers, not held by the doorway alone, so a peer
  outside the doorway's pool can read it back. Reading a record from such a peer shows that it
  is on the network and not only in the doorway's own storage. A COMMITMENT is a promise the
  notary recorded, naming who promised, to whom, what, and until when. The one this
  story reads is a "hosted-cell" commitment: a household's promise to run a person's cell on its
  machine. The household that made the promise is the commitment's PROVIDER, and the person it
  was made to is its recipient. A commitment is LIVE while the notary holds it as current and
  not withdrawn.

  An APP is any program served over the network that acts with the person's cell: the learning
  app, or one someone else writes. An app WRITES IN A PERSON'S NAME when it signs any record with
  their key onto their cell. TELLING is a short statement the app shows when it opens, in
  the person's own words: which household's hub holds their key, which doorway arranged it, that
  this hub can sign as them, and that this lasts while they are hosted and ends when they move
  onto a key of their own. That move is GRADUATION, told in its own series of stories, where a
  hosted person carries their key onto a device of their own. The DISCLOSURE RECORD is the
  network's note that the telling was shown: which person, which app, which holder it named, and
  when. The notary keeps it, so the person can read it back later from a peer outside the doorway's
  pool, without asking the doorway.

  What is claimed. A hosted person who opens an app is told who holds their key before the app
  writes anything in their name. The holder they are told is the household named on their
  hosting promise (station 7). The telling leaves a disclosure record. Where the holder cannot be
  read back, as when the household's other machines are switched off or cut off from the network,
  the app says so plainly; it does not call the key safe.

  What is not claimed. The telling is not consent to be hosted. At this doorway, creating an
  account through its registration form is choosing to be hosted (station 1), so every newcomer
  here is a hosted person from the moment their account exists; and moving off hosting is
  graduation, not this station. When the telling stops appearing, at graduation or after it, is
  told in the graduation series, not here.
  Nor does the telling claim that the hub will not sign as them. A hub that holds a key can sign
  with it. The point is that the person knows.

  The person is created by the story and removed by it, as everywhere in this series. Each
  scenario makes its own newcomer through the doorway's registration form, so it never relies on
  the standing cast: the named people the household mesh seeds before any run starts. The doorway is named by "alpha",
  and "E2E_DOORWAY_ALPHA" is the environment variable that holds its address.

  The background sets nothing up by hand. The newcomer's account is made through the real
  registration form, and that registration is what places their cell on one pool conductor and
  leads the notary to record the promise to host it. The background then reads that promise back
  and checks that its provider is that conductor's steward, using the same checks station 7
  proves. Nothing is injected, so if the background fails, the registration of station 7 is
  broken and no scenario below has anything to test. The household on that promise is the one
  every scenario below expects to be named.

  Only the first scenario is browser-only. It is the one that looks at what the person sees on
  the screen, and in what order: the telling, shown before the app acts. The other scenarios
  check what the telling names and what the network records, which can be read without a
  browser. Where one of them has the newcomer open an application, or tell it to go on, the
  newcomer does so by sending the same requests the application's screen would send, and the
  outcome is read from the network, not from a screen.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And a newcomer who has created an account at this doorway
    And the doorway hosts a cell for them on one of its pool conductors
    And the notary records a live "hosted-cell" commitment for them
    And that commitment names the steward of that pool conductor as provider
    And the household runs at least one peer that is not in the doorway's pool

  @browser-only
  Scenario: The first app a hosted person opens tells them whose hub holds their key before it acts
    When the newcomer opens an application served by doorway "alpha"
    Then before the application writes anything in their name, it tells them which household's hub holds their key
    And the telling names doorway "alpha" as the one that arranged it
    And the telling says that this hub can sign as them
    And the telling says how the arrangement ends, by moving onto a key of their own
    And the telling never says their key is held on a device of theirs

  Scenario: The household the app names is the household that promised to host them
    When the newcomer opens an application served by doorway "alpha"
    Then the household the telling names is the household that promised to run their cell, the provider on their live "hosted-cell" commitment
    And the telling does not name the doorway itself as the holder of their key

  Scenario: The telling is recorded, and a peer that is not the doorway reads it back
    When the newcomer opens an application served by doorway "alpha"
    Then a household peer that is not in the doorway's pool can read back the newcomer's disclosure record
    And that disclosure record names the newcomer's own agent key
    And it names the application that showed the telling
    And it names the same household as the holder of the key
    And it carries the time the telling was shown

  Scenario: A second app tells them too, and records its own telling
    Given the newcomer has opened one application served by doorway "alpha"
    When the newcomer opens a second, different application served by doorway "alpha"
    Then the second application also tells them which household's hub holds their key
    And the network holds one disclosure record for each of the two applications

  Scenario: When the holder of their key cannot be read back, the app says so rather than calling the key safe
    Given every household peer outside the doorway's pool is switched off or cut off from the network
    And for that reason the newcomer's hosting commitment cannot be read back from any peer outside the doorway's pool
    When the newcomer opens an application served by doorway "alpha"
    Then the telling says that who holds their key could not be confirmed
    And the telling does not say that their key is safe
    And no record signed with their key is written onto their cell until the newcomer tells the application to go on
