# HELD until circle admission lands: the missing node of the placement-contract design
# (genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md,
# section 15, "Circle admission") by which a signed record admits a person to a group and the peers
# that validate records check it. Section 17 lists it among the planes that build beside the numbered
# slices. Until it lands, no space promises its members confidentiality: today a person joins a
# group by writing a membership record about themselves, the admitting member's signature travels
# only in an invitation kept off the shared network and checked by the joiner's own peer, and the
# peers that validate records check only that the record's fields are present. Every step is
# undefined, so the feature is @wip.
# @concern:peer-admission: see the description's last paragraph.
# @act:i: this story belongs to Act I, the household chapter of the test suite.
# @requires:household-nodes: the household's three peers must be running.
# @requires:owned-substrate: the scenarios add a stranger's peer to the network and admit a person,
# so they run only on a mesh this run owns outright.
@e2e @qahal @act:i @wip @concern:peer-admission @requires:household-nodes @requires:owned-substrate
Feature: A stranger who claims to be a member of the household is refused on every named path to what the household shares only with its members

  Matthew, Jessica and James Dowell share a household. Each of them keeps a peer: an always-on
  machine at home that holds their data and does work for them. Each peer is named after its
  person: matthew, jessica and james.

  Some of what the household keeps is shared with its three members and with nobody else: its
  folder "Household papers" (scans of its bills, insurance and tax papers) and the household
  thread, where the three of them leave each other messages. A thread is a running list of messages
  that its members' peers keep in step with each other; it is not a file in a folder. Together these
  are the household's member-only space. The space also covers who is in it: which members have the household thread
  open at a given moment is told only to members, like the files and messages themselves.

  A stranger, in this story, is a person the household has not admitted. The stranger runs a peer
  of their own, named "stranger". The network the household's peers use is open: any peer can join
  it and send requests to the household's peers. Being on that network gives no standing in the
  household.

  The stranger has also learned, from outside the household, the content address of one file in
  "Household papers". A content address is a code computed from a file's exact bytes. It tells
  nothing about what the file says or what it is called, but anyone who holds it can ask a peer for
  exactly that file. An address can leak in ordinary ways: in a link a member pasted into an email
  or chat outside the household, in a log, or in a file's details copied somewhere else. The story
  assumes the worst case, that the stranger holds one, and does not depend on how it was learned.

  What this story specifies, once admission is built:

  Apart from the person who founds the household, a person becomes a member only by an admission:
  a record that names the person admitted and is signed by a member whom the household has agreed
  may admit. Which members may admit is the household's own decision, and the household keeps a
  record of that decision that each of its peers holds. Every peer that is asked for the
  member-only space makes one check for itself before it answers, and so does every peer that
  validates records before keeping them; each of the household's three peers does both. The check
  is that the admission carries a signature, and that the signature belongs to a member the
  household's record says may admit. A signature by anyone else, including a member who may not
  admit, admits nobody. So does a record a person writes about themselves, saying "I am a member",
  whichever member it names as the one who admitted them. Both halves of that check, and the
  household's record of who may admit, belong to the same admission design.

  In this story the household has agreed that Matthew alone may admit new members. Jessica and
  James are members, but they cannot admit.

  The promise is that whichever of the named paths the stranger asks by, every household peer
  refuses, and no refusal names anything in the space: no file, folder, message or member. The named
  paths are the ones the household's peers and doorway offer today: asking a peer for a file by its
  content address, a request any peer on the network can send to another; asking a peer for the
  list of a folder, for a thread's messages by sync, by search, or for who is present; and asking a
  doorway for a file. A path the system adds later is not covered until it is added to this story.

  Today, however, none of that is built, and nothing here claims the promise holds. A person joins
  a group by writing a membership record about themselves; the admitting member's signature travels
  only in an invitation that is kept off the shared network and checked by the joiner's own peer; no
  record of who may admit exists for a peer to read; and the peers that validate records check only
  that the record's fields are present. Until every peer checks admission for itself, no member-only
  space promises its members that a stranger cannot read it. So this story is held: it waits on the
  "Circle admission" design, section 15 of
  genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md.

  The project keeps a register of the promises it keeps, each tied to a story that proves it. The
  tag @concern:peer-admission is how that register will find this story as the proof of this
  promise. Until the promise holds, nothing in the register checks it.

  Background:
    Given the household's peers "matthew", "jessica" and "james" are running
    And Matthew founded the household and is its first member
    And the household has agreed that Matthew alone may admit new members, and each household peer holds the household's record of that agreement
    And Jessica and James are members, each by an admission: a record that names them and that Matthew signed
    And the household's member-only space holds the folder "Household papers", the household thread, and who has the household thread open at any moment
    And peer "stranger", the peer the stranger keeps and that acts for the stranger, has joined the network the household's peers use
    And no member has admitted the stranger
    And the stranger has learned the content address of one file in "Household papers"

  Scenario: A membership record the stranger writes for themselves admits nobody
    When peer "stranger" publishes a membership record it wrote about the stranger, naming Matthew as the member who admitted the stranger
    Then every household peer refuses the record as an admission
    And each peer refuses it because the only signature on the record is the stranger's own
    And each peer refuses it because the household's record names Matthew alone as a member who may admit, and the stranger is not Matthew
    And no household peer keeps that record
    And the household's members are still Matthew, Jessica and James and no one else

  Scenario: Every named path to the household's peers refuses a stranger whose peer holds only a membership record it wrote about the stranger
    Given peer "stranger" holds the membership record it wrote about the stranger
    When peer "stranger" asks each household peer for the member-only space in each of these ways:
      | way          | what it asks for                                                  |
      | by address   | the file in "Household papers" whose content address it learned   |
      | by listing   | the list of what is in "Household papers"                         |
      | by sync      | the messages written in the household thread during the past week |
      | by search    | anything in the member-only space that mentions "insurance"       |
      | by presence  | to be told who has the household thread open right now            |
    Then every household peer refuses every one of those requests
    And no refusal names a file, a folder, a message or a member of the space
    And peer "stranger" has received no byte of the member-only space from any household peer

  Scenario: A doorway serves the stranger nothing from the member-only space and keeps none of it
    A doorway is a public web entry point: someone with only a browser signs in on a doorway, and
    the doorway relays their requests to the peers. A doorway may also keep copies of what it relays
    in a cache, so that it can answer the next request for the same thing without asking a peer
    again. The promise covers that cache too: serving a stranger must never put member-only content
    into it, where a later request could find it. Doorway "alpha" is the one Matthew runs for the
    household.

    Given a visitor who is not signed in on doorway "alpha" has asked it for the file in "Household papers" whose content address the stranger learned
    And doorway "alpha" served that visitor no byte of the file and gave them a refusal
    And the stranger is signed in on doorway "alpha" as themselves and presents the membership record their peer wrote about them
    When the stranger asks doorway "alpha" for the file in "Household papers" whose content address they learned
    Then doorway "alpha" serves no byte of that file
    And doorway "alpha" gives the stranger the same refusal, with the same status and the same body, that it gave the visitor who was not signed in
    And that refusal names no file, folder, message or member of the space
    And doorway "alpha" holds no byte of "Household papers" in its cache that it did not hold before the stranger asked

  Scenario: Once Matthew admits the stranger, every named path to the peers and the doorway answers, each after the peer checks Matthew's signature
    When Matthew signs an admission of the stranger as a member of the household
    And Matthew's admission of the stranger reaches every household peer
    And peer "stranger" asks each household peer for the member-only space in each of these ways:
      | way          | what it asks for                                                  |
      | by address   | the file in "Household papers" whose content address it learned   |
      | by listing   | the list of what is in "Household papers"                         |
      | by sync      | the messages written in the household thread during the past week |
      | by search    | anything in the member-only space that mentions "insurance"       |
      | by presence  | to be told who has the household thread open right now            |
    And the stranger, signed in on doorway "alpha", asks it for the file whose content address they learned
    Then every household peer answers every one of those requests
    And doorway "alpha" serves that file to the stranger
    And each household peer checked Matthew's signature on the admission before it answered, including when doorway "alpha" relayed the request
