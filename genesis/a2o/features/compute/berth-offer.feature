@e2e @act:i @compute @wip @concern:berth-offer-matched @requires:household-nodes @requires:owned-substrate
Feature: A new blade slotted into the household's hub offers its room, and an album that was short of a holder is kept on it

  This story describes a capability that is planned and not built yet: today no machine
  publishes a berth offer and no machine runs a floor, so none of these steps can run, and the
  story is marked @wip. What it waits on is the floor controller, a planned piece of software in the
  placement-contract design. It is not a separate decider: it is the part of each machine that
  computes the floor described below, and it will count a berth offer among the records the floor
  ranks with. Once that piece exists, these steps can run.
  The tag berth-offer-matched is how a future check will find this story; nothing checks it yet.
  The story belongs to Act I, the household chapter of the test suite. It requires the household's
  three peers running, and it runs only on a household this test run owns outright, because it
  adds a fourth machine and moves real copies onto it.

  Matthew, Jessica and James share a household. Each of them keeps a peer: an always-on machine
  at home that holds their data and does work for them. Each peer is named after its person:
  matthew, jessica and james. Matthew's peer is the household's hub, a box in the hallway cupboard
  built to take blades. A blade is one whole computer on a board that slides into the hub. The
  peer called matthew runs on the hub's first blade. Sliding a blade in adds a machine to the
  household; pulling one out takes a machine away. A new blade takes part only after its person
  has enrolled it as one of their own devices, which signs it into the household under that
  person's name. Enrolment leaves two things behind: the blade's own signing key, which it signs
  its offer and commitments with, and an entry in its person's list of devices, which every
  machine in the household holds. That entry gives the blade the name the household knows it by,
  and it is this name the floor ranks it by. This story takes that enrolment as already done and
  does not test it; it needs only that the key and the entry are in place.

  The household keeps copies of what it shares on more than one machine, so no copy belongs to
  any single blade. When a machine goes, what it kept is short one holder, not lost. A need is a
  request to have something kept on a number of machines. Which machine takes a copy is decided
  by the floor, described below. To take a copy, a machine fetches it from a machine that already
  holds it and stores it on its own disk. It then signs a commitment, a record that it holds this
  copy for this need, and adds the copy to its list of holdings: the list each machine keeps of
  what it holds. From the moment the copy is on that list, the machine is an honoured holder of
  it. Before then, while it is still fetching or storing the copy, it is not a holder of it at all,
  even if some of the copy is already on its disk. A need with fewer honoured holders than it
  asked for is short of a holder.

  In the scenarios below, "held by" and "holder" always mean honoured in this sense: a machine
  holds a copy, and counts as one of its holders, only when it is honoured.

  A berth is what one blade offers the household and what it is keeping right now. When a blade
  starts, it measures its own disk and memory and publishes a berth offer: a signed statement of
  how much room and work it will give the household. The room a machine offers is never more than
  the limit its person set for it. A person may set a limit for each machine they enrol, and a
  limit belongs to that one machine. In this story only Matthew has set one, on the blade that
  runs matthew, and he set it to none: that blade has room on its disk but offers none of it.
  jessica, james and any new blade offer whatever room they have.

  To publish is to sign the offer, keep it among the blade's own records, and hand it to the
  household machines the blade is connected to. Each machine that receives a record it does not
  already hold keeps it and passes it on to the household machines it is connected to, so the
  record reaches every machine in the household, and in time a copy of it comes back to the blade
  that published it, passed on by one of the others. The offer asks nobody's permission and
  assigns no work. It is information that every machine in the household receives and weighs.

  Needs, commitments and lists of holdings spread through the household the same way offers do.
  Who keeps which copy is decided by the floor: a fixed ranking that each machine computes for
  itself from the records it holds, with no vote, no messages about the choice and nobody in
  charge.

  Two machines holding the same records always compute the same answer. It is called the floor
  because it is the base layer
  of placement: it works with no person, no assistant and nothing outside the household, and
  anything added above it may only reorder the machines the floor allows, never allow one it
  refused. From the needs, offers, commitments and lists of holdings a machine has received, its
  floor first lists the machines allowed to keep each copy, called eligible: a machine is eligible
  only if its offer has room enough for the copy and it does not hold that copy already. The floor
  then puts the eligible machines in a fixed order computed from each machine's name and the
  copy's, so the order is the same on every machine and does not depend on who answers fastest. A
  machine is a candidate when it is eligible and its own offer has come back to it, passed on by
  another household machine: every candidate is eligible, but an eligible machine whose offer has
  not yet come back is not a candidate. It reads its offer from the same passed-on records every
  other machine reads, not from its memory of publishing it, so it acts only on an offer that has
  already spread beyond itself. A candidate that finds itself among the first in the order, as
  many as the need is short, takes the copy on itself.

  Inside a household, offers and commitments are signed records kept among the household's own
  machines. They never appear on the shared network: the records that households beyond this one
  also keep and check, such as who each person is and the content a person publishes for anyone
  to read. An offer describes one machine's disk and memory, so it stays inside the household.

  James's peer keeps his recorded music lessons, and its disk has filled up with them.

  Jessica's album "Family photos" used to be kept on jessica and on a blade in the hub. Last week
  that blade's disk began to fail and Matthew pulled it out, so the album lost one of its two
  holders.

  The times in the scenarios are design targets for the floor controller, not readings from a
  running system, since none exists yet.

  Background:
    Given the household peers matthew, jessica and james are running
    And Jessica set a need for her album "Family photos": to be kept on two of the household's machines
    And jessica is the only honoured holder of "Family photos"
    And james's disk is full of James's music lessons, so james offers no room for copies
    And matthew's blade has room, but Matthew set the limit on what that blade gives the household to none, so matthew offers no room for copies
    And so each of the three peers shows "Family photos" as short of a holder, with no eligible machine to take it

  Scenario: The new blade's offer reaches the household, and the album is kept on it
    Given Matthew has slid a new blade, "blade-2", into the hub and enrolled it as one of his own devices
    When blade-2 starts and publishes its berth offer, with room enough for "Family photos"
    Then within 60 seconds each of the household's machines has received blade-2's offer
    And no record on the shared network names blade-2's offer
    And blade-2's floor finds blade-2 the only eligible machine for "Family photos"
    And blade-2's floor therefore puts blade-2 first in the order for "Family photos"
    And blade-2 takes a copy of "Family photos" itself, fetching it from jessica, the only machine that holds it
    And within 5 minutes "Family photos" has two honoured holders, jessica and blade-2, and each of the four machines names those two holders
    And among the records the four machines hold, none tells blade-2 to take the album and none is a person's approval of the move
    And blade-2's commitment names only the need for "Family photos" it serves

  Scenario: The new blade waits until its offer has come back to it before taking anything
    Given Matthew has slid a new blade, "blade-2", into the hub and enrolled it as one of his own devices
    And the other machines are not yet passing blade-2's offer back to it
    When blade-2 starts and publishes its berth offer, with room enough for "Family photos"
    Then blade-2 does not count itself as a candidate and takes no copy of "Family photos"
    When the other machines pass blade-2's offer back to it
    Then within 5 minutes "Family photos" has two honoured holders, jessica and blade-2

  Scenario: Two new blades both have room, and the floor's order decides which one keeps the album
    Given Matthew has slid two new blades, "blade-2" and "blade-3", into the hub and enrolled both as his own devices
    When both blades start and publish berth offers, each with room enough for "Family photos"
    And each blade's offer has come back to it
    Then each of the five machines finds blade-2 and blade-3 both eligible for "Family photos", and computes the same order for them
    And the album is short by one, so only the blade first in that order takes a copy, and the blade second in it takes none
    And within 5 minutes "Family photos" has exactly two honoured holders: jessica and the blade first in the order
    And the blade second in the order still offers its room, and holds no commitment for "Family photos"

  Scenario: A blade without enough room is not chosen, and Jessica is shown why her album is still short
    Given Matthew has slid a new blade, "blade-2", into the hub and enrolled it as one of his own devices
    When blade-2 starts and publishes a berth offer with less room than a copy of "Family photos" takes
    Then blade-2 takes no copy of "Family photos"
    And Jessica's peer shows her that "Family photos" is still short of a holder, because no machine with room enough has offered
    And Jessica's peer works this out from the offers and commitments it already holds, and no machine sends her a message about it
