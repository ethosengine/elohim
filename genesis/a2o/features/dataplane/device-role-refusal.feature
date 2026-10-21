# Held until slice 5 of the placement-contract design ("the floor controller", §17 of
# genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md),
# which is the first placement code to read whether a device runs on its battery. Nothing reads that
# fact today, so the feature is @wip and none of its steps has a definition.
# @concern:device-capacity-declared names this proof for the habit register; no habit checks it yet.
# No @requires tag: every scenario runs against one phone's own floor, given the records the phone
# would hold, so no household mesh is needed.
@e2e @dataplane @wip @concern:device-capacity-declared
Feature: A phone on its battery is not eligible to carry another household's copies unless its person offers, and she is told why

  Jessica belongs to the Dowell household, with Matthew and James. She carries a phone that is
  one of her household's devices. Households can keep copies of each other's things, so that a
  fire or a flood in one home does not take what it holds. Next door, Adam and Eve's household
  has asked for copies of its photo album "Garden party" to be kept on machines outside their own
  home. Any machine whose person allows it to carry for other households could be one of those
  carriers, and Jessica allows her phone to. A household's request for copies is itself a record:
  it is published to the network so that every device can hold it, and Jessica's phone holds
  Adam and Eve's request.

  Who keeps which copy is decided by the floor, the project's name for the fixed placement rules
  each device runs for itself: the same rules, run on every device, with nobody in charge. Each
  device's floor works out, from the records it holds, which devices are allowed to carry each
  copy (called eligible), and from those same records alone puts the eligible devices in an
  order; the devices that order chooses take the copy on themselves. The rules leave nothing to chance or to anyone's choice, so two devices
  holding the same records reach the same eligible devices in the same order, each on its own,
  without asking a coordinator. What a device offers others is a statement it publishes about
  what it will do for them. It follows what its person allows, and what one person allows changes
  only her own devices.

  This story is about which devices are eligible. It is one of the floor's rules that only
  eligible devices are put in the order at all: a device that is not eligible is left out before
  any order is worked out, so it is never chosen and never takes the copy, whatever the order
  would have been. Whether an eligible device is then chosen depends on the order, which this
  story does not test and no story yet tests; that part is left for a later story on purpose. So
  what this story proves is that the phone on its battery, without the battery offer, is not
  eligible and takes no copy for another household; it does not prove that an eligible phone is
  chosen.

  A phone running on its battery is different from a machine plugged in at home. Carrying another
  household's copies would drain its battery. So a device on its battery is not eligible to carry
  for another household, even when its person allows it to carry, unless its person has also
  offered that it may carry on its battery. This story calls that the battery offer. Holding
  copies for its own household is not refused for being on battery: it waits until the phone
  counts as charging.

  What the phone knows about itself stays on the phone. The phone knows its battery level and
  whether it is charging; nobody else learns either. Other households' machines see only that the
  phone has not offered. Jessica, on her own phone, is shown why.

  A phone plugged in and unplugged through the day would reveal Jessica's routine if every change
  showed at once. So the phone counts as charging only once it has stayed plugged in for 30
  minutes, and counts as on its battery only once it has stayed unplugged for 30 minutes. Both
  what it publishes that it offers and its own household's waiting copies follow that settled
  state, never the moment-to-moment plug. Once the phone counts as charging, a copy it was waiting
  to hold arrives within 10 minutes, the time allowed for the copy to be fetched. Both numbers are
  this story's own, chosen for the test.

  Nothing the floor runs today reads whether a device is on its battery, so this story is marked
  as work in progress and cannot yet be run. It becomes testable when the floor reads a device's
  power source. The project's habit register tracks this story as device-capacity-declared; no
  habit checks it yet. Every scenario runs on Jessica's phone alone, given the records it would
  hold; no other household's machine has to be running.

  Background:
    Given Jessica's phone is one of the Dowell household's devices
    And Jessica allows her phone to carry for other households
    And Jessica's phone is running on its battery
    And Adam and Eve's household has asked for its album "Garden party" to be kept by machines outside its home
    And Jessica's phone holds Adam and Eve's copy request among its records

  Scenario: On its battery, the phone is not eligible to carry for another household, and only Jessica is told why
    Given Jessica has not made the battery offer
    When the phone's floor works out which devices may carry "Garden party"
    Then the phone is not eligible to carry "Garden party" and takes no copy of it
    And Jessica's phone shows her that it carries nothing for other households while it runs on its battery, and that she can make the battery offer
    And Adam and Eve's machines can learn only that Jessica's phone has not offered
    And nothing the phone sends includes its battery level or whether it is charging

  Scenario: Once Jessica makes the battery offer, the phone is eligible to carry for the other household even on its battery
    Given Jessica's phone is still running on its battery
    And Jessica has not yet made the battery offer
    When Jessica makes the battery offer
    And the phone's floor works out which devices may carry "Garden party"
    Then the phone is eligible to carry "Garden party", so the order the floor works out includes it
    And her battery offer changes nothing that Matthew's or James's devices publish about what they offer

  Scenario: Copies for her own household wait for the charger, and are not refused
    Given the Dowell household's floor has chosen Jessica's phone as one of the holders of its album "Family photos"
    And Jessica has not made the battery offer
    When the phone's floor works out what the phone should hold
    Then the phone shows "Family photos" as waiting for the charger, not as refused

  Scenario: A copy waiting for the charger arrives once the phone counts as charging
    Given the Dowell household's floor has chosen Jessica's phone as one of the holders of its album "Family photos"
    And the phone shows "Family photos" as waiting for the charger
    When Jessica plugs her phone in, and it stays plugged in for 30 minutes, so that it counts as charging
    Then within 10 minutes of counting as charging the phone holds "Family photos" and lists it among what it holds

  Scenario: Plugging in and unplugging through the day publishes no routine
    Given Jessica has not made the battery offer
    When for one hour Jessica switches her phone between plugged in and unplugged every 10 minutes
    And after that hour she leaves it plugged in for a further 30 minutes
    Then the phone publishes no change to what it offers other households during the hour of switching
    And at the end of the further 30 minutes plugged in it publishes exactly one change, that it now offers to carry for other households
