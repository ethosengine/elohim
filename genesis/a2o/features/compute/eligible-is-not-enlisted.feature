# @act:i: this story belongs to Act I, the household chapter of the test suite.
# @requires:household-nodes: the household's three peers must be running.
# @requires:owned-substrate: the scenarios record and withdraw a real offer, so they run only on a
# mesh this run owns outright.
@e2e @act:i @compute @wip @concern:placement-floor @requires:household-nodes @requires:owned-substrate
Feature: An always-on machine is told it could host the household's live sessions, and hosts only once its person offers it

  This story is held, and the whole feature is tagged @wip, until two pieces are built. The
  first is slice 5 of the placement-contract design, the part that works out what a machine is
  eligible for; that design is
  genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md,
  and slice 5 is in its section 17. The second is the host for live sessions, which no machine
  runs yet. Until both exist, none of these steps can be run. The tag @concern:placement-floor is
  the name the habit register will use for this proof; no habit checks it yet.

  Matthew, Jessica and James share a household. Each of them keeps a peer: an always-on machine
  at home that holds their data and does work for them. Each peer is named after its person:
  matthew, jessica and james. In the steps a capitalized name is the person and a lowercase name
  is that person's peer: Matthew offers, matthew hosts. Matthew's peer sits in the hallway
  cupboard and is never switched off.

  A live session is the time two or more people spend in the same shared document at once, seeing
  who else is there and which part each of them is pointing at. That passing information is
  called presence. It is not saved, and nothing is decided from it. A host is a machine that
  carries presence between the people in a session. When the household has no host on offer, the
  people's own machines pass presence directly to one another, which is how this household works
  today.

  Hosting is a service, not a way of watching. A host passes presence along while a session is
  open and keeps no record of the session. Everything the hosting role on a machine writes, its own
  log and any data it stores, is called that machine's hosting records; the machine's other logs
  are not part of them. The hosting records may say things about the host's own running, such as
  when it began hosting, but once a session ends they name nobody who was there and hold no time at
  which that session opened or closed. This matters here because an offer lends a machine to
  carry other people's sessions, and that must not turn the machine into a log of their comings
  and goings, which its person could read later.

  Every machine counts the presence messages it sends and receives, as a total that names nobody
  and records no time. Whether presence passed through a machine, or whether a host sent anything
  while idle, is read from that machine's count.

  A machine's facts can make it eligible to host: being always on, with room and bandwidth to
  spare. The machine reads these facts from itself: how long it has stayed switched on, how much
  free disk space and memory it has, and how much of its network connection goes unused. It
  compares them with what hosting needs, and the facts themselves never leave the machine.
  Eligible is not enlisted. A machine takes on a role for others only when its person has
  made an offer for it: a statement of what that machine will do for others, kept among the
  household's machines. Each person in the household has their own cryptographic key, which
  identifies them and which only they can sign with. An offer is signed with its person's key, so
  any machine can check for itself who made the offer and that nobody has altered it, without
  asking any central authority. That signature is what separates an offer from a setting a
  machine could switch on for itself.

  The household's list of hosts is made only of machines under such an offer; a machine never
  puts itself on it. Every machine holds the list, so when a session opens each person's
  machine picks its host from the list without asking any other. In this story just one host is
  ever on offer, so each connects through that one. How machines choose among several hosts is
  not this story's subject; its subject is that only an offered machine is on the list at all.

  Each machine works out from its own facts what it is eligible for and shows that to its own
  person, with the reasons. Nothing changes until its person decides.

  Matthew stays out of "Holiday plans" throughout. Every session here is between Jessica and
  James, and the only question is what his machine does for them.

  When Matthew offers or withdraws the machine, the household's list of hosts must show the change
  within 60 seconds: short enough that he can stay and watch his decision take effect, and that a
  session opened a minute after he decides already follows it. Sixty seconds is this story's target,
  not a reading taken today, and a slower change fails the scenario.

  Background:
    Given the household peers matthew, jessica and james are running
    And matthew has stayed switched on for the last 30 days, with free disk space, free memory and unused network capacity above what hosting needs
    And Jessica and James both have access to the household's shared document "Holiday plans"

  Scenario: Matthew is told the hallway machine is eligible to host, and that it hosts only if he offers it
    Given Matthew has not offered matthew as a host for live sessions
    When matthew compares its own facts with what hosting needs
    And Matthew looks at what matthew is eligible for
    Then he is shown that matthew is eligible to host the household's live sessions, with the facts that made it so: switched on for 30 days, and free disk, memory and network above what hosting needs
    And he is shown that matthew will host only if he offers it
    And matthew is not in the household's list of hosts

  Scenario: An eligible machine that has not been offered carries none of a session
    Given Matthew has not offered matthew as a host for live sessions
    And matthew is eligible to host the household's live sessions
    When Jessica and James open "Holiday plans" together
    Then each sees that the other is there
    And presence passes directly between jessica and james, and matthew's count of presence messages does not rise

  Scenario: Once Matthew offers it, the hallway machine joins the list of hosts and waits idle
    Given Matthew has not offered matthew as a host for live sessions
    When Matthew offers matthew as a host for the household's live sessions
    Then within 60 seconds the copy of the household's list of hosts on each of matthew, jessica and james names matthew, under an offer signed with Matthew's key
    And for the next 60 seconds, while no session is open, matthew's count of presence messages sent and received does not rise

  Scenario: An offered machine hosts the next session opened
    Given Matthew has offered matthew as a host for the household's live sessions
    And matthew is in the household's list of hosts
    And no session is open anywhere in the household, and matthew's count of presence messages is noted
    When Jessica and James open "Holiday plans" together
    And Jessica moves to the second page of "Holiday plans", and James sees her there
    Then each of their machines chooses matthew from the household's list of hosts and connects through it
    And each sees that the other is there
    And matthew's count of presence messages has risen from the noted value, and with no other session open in the household that rise is their presence passing through it

  Scenario: A host keeps no record of who was in a session it carried, or when
    Given Matthew has offered matthew as a host for the household's live sessions
    And Jessica and James have opened "Holiday plans" together, connected through matthew
    When Jessica and James both leave "Holiday plans"
    Then matthew's hosting records, its hosting log and the data its hosting role stores, name neither Jessica nor James, by name, key or any other identifier
    And matthew's hosting records hold no time at which that session opened or closed
    And matthew's hosting records hold nothing from the session itself: not the document's name, not an identifier for the session, and not anything that passed through it

  Scenario: Withdrawing the offer stops the hosting from the next session on
    Given Matthew has offered matthew as a host for the household's live sessions
    And the copy of the household's list of hosts on each of matthew, jessica and james names matthew
    And Jessica and James have already held one session in "Holiday plans" connected through matthew, and both have left it
    When Matthew withdraws that offer
    Then within 60 seconds the copy of the household's list of hosts on each of matthew, jessica and james no longer names matthew
    When Jessica and James next open "Holiday plans" together
    Then presence passes directly between jessica and james, and matthew's count of presence messages does not rise
