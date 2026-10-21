# @concern:served-under-standing names the habit this feature proves. The habit register lists it
# in the habit's "checks" field, and CI reports results under the same name, so this one tag
# joins the feature, its CI results and the register.
# @act:i: this story belongs to Act I, the household chapter of the test suite.
# @requires:household-nodes: the feature needs the running household mesh with its three doorways.
# @requires:owned-substrate on one scenario: it pauses a doorway's process on purpose, so it runs
# only on a mesh this run owns outright.
# Held (@wip): no step definitions exist yet, and two readings wait on work the placement
# contract names (spec genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md):
# a doorway reporting which publications it keeps a copy of, and the projection's lifecycle the
# `projectable` need carries (§15 "Projection → retention floor"; §17 slice 9).
@e2e @doorway @concern:served-under-standing @act:i @requires:household-nodes @wip
Feature: When the household withdraws a publication, no doorway keeps serving it or keeps a copy of it

  The Dowell household is a small group of people, Matthew, Jessica and James, who run their own
  machines together. Each of them keeps a peer: an always-on machine that holds their data. The
  household has published a notice, "spring-planting-notice", to the web at commons reach. Reach
  is how widely something may be read, and the protocol defines several levels of it; commons is
  the widest, the open, shared level that anyone may read without saying who they are, and it is
  the only level this story uses.

  A doorway is a public web entrance: an address anyone with a browser can use to reach what a
  household has published. Doorways are plural, and the household runs three of them here,
  alpha, beta and gamma. None of them owns what it serves. The household's publication record,
  called its projection contract, says that the notice may be shown on the web, at which reach,
  and which doorway holds it for serving: here, alpha. Beta and gamma do not hold it; when a
  visitor asks one of them for the notice, it forwards the request once to alpha and returns the
  answer, which is called relaying. Any doorway that has served the notice, including one that
  only relayed it from alpha, may keep a copy of its bytes ready, so the next visitor gets it
  quickly. So once a visitor has been served the notice through all three doorways, beta and
  gamma can each hold a copy even though neither holds the notice for serving. That copy is a convenience, never a
  permission: a doorway may serve the notice only while the projection contract says it may.
  That is what this story calls serving under standing: the projection contract gives the
  notice its standing on the web, every doorway serves it only under that standing, and when
  the household withdraws the standing, every doorway stops serving it. Withdrawing that
  standing, by recording in the projection contract that the notice may no longer be shown on
  the web, is what this story calls a projection withdrawal.

  The projection contract lives with the household, on its members' peers, not on any doorway.
  A doorway learns what it says by reading it from those peers: when the doorway starts, and
  again on a repeating cycle while it runs. Nothing is pushed to a doorway, so a doorway that was
  stopped when the contract changed learns of the change only by reading it again, and it must
  do so before it serves anything.

  Why this matters to the household: people take things down. A notice was posted by mistake, an
  address in it should no longer be public, the event was cancelled. When the household
  withdraws a publication, they need it gone from the web through every entrance, not only the
  one where they recorded the decision. A copy still served by a doorway nobody thought to check
  is a publication the household no longer controls.

  Withdrawing ends the publication and nothing else. It does not delete the notice: the
  household's own peers still hold it. It cannot reach copies outside the doorways either: a
  visitor who saved the page before the withdrawal still has it, and this story promises nothing
  about that copy.

  Any member of the household may record a withdrawal on the household's behalf, through a
  doorway; how that authority is governed is the subject of a different story, and these
  scenarios do not check who may record it. Each doorway answers, on its own status page,
  whether it keeps a copy of a given publication's bytes, and this story reads that answer.
  The notice's content address is the fingerprint computed from its exact bytes, which anyone
  can use to ask for those bytes directly. The fixture manifest is the test harness's own record
  of how it set up this household: when the harness starts the household's machines and creates
  its members and their content (called casting the mesh), it writes down the settings the
  household runs with. One of them is the reconcile window: the longest a doorway may take to
  learn of a change to a projection contract, which is the length of the doorway's repeating
  reading cycle plus the time one reading takes. The window is one value for the whole
  household: all three doorways run the same reading cycle, so the same window applies to each
  of them. Its length is a setting the household is cast with, not a constant, which is why this
  story reads it from the fixture manifest rather than naming a number. Every deadline below is
  that window plus 15 seconds, the allowance for observing the change.

  A collective is a group that decides together and is recorded as one party, under an
  identifier of its own. The Dowell household is recorded as a collective, and its projection
  contract names the household by that collective identifier. Wherever a step below says an
  answer or a record names the Dowell household, it is that collective identifier that is
  checked, not the household's display name. After a withdrawal a doorway answers a request for
  the notice by its publication name with HTTP 410, which says the publication existed and was
  withdrawn, and the answer's body names the Dowell household as the party that withdrew it. That answer is neither "never existed" nor a reach refusal nor a service error.
  A request by content address is answered less specifically, on purpose: a content address
  names bytes, not a publication, so the doorway answering it cannot always say which
  publication those bytes belonged to or who withdrew it. For that request this story promises
  only that the answer is a refusal, an HTTP status from 400 to 499, and carries none of the
  notice's bytes.

  This story is held and does not run yet: its steps are not implemented, and two of the things
  it reads do not exist yet. Those are a doorway's report of whether it keeps a copy of a
  publication's bytes, and the projection contract's withdrawn state. Both are work named by the
  placement contract design (genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md).

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And doorway "beta" at "E2E_DOORWAY_BETA"
    And doorway "gamma" at "E2E_DOORWAY_GAMMA"
    And the household's declared reconcile window is read from its fixture manifest
    And the Dowell household has published "spring-planting-notice" at commons reach, and its projection contract holds it for serving on doorway "alpha" only
    And an anonymous visitor has been served "spring-planting-notice" through each of doorways "alpha", "beta" and "gamma", with "beta" and "gamma" relaying it from "alpha"
    And having served the notice, each of doorways "alpha", "beta" and "gamma" reports that it keeps a copy of the notice's bytes, "beta" and "gamma" each having cached the copy it relayed
    And the content address of the notice's bytes is recorded

  Scenario: Withdrawing the notice clears it from all three doorways
    When the Dowell household's decision to withdraw the publication of "spring-planting-notice" is recorded through doorway "alpha"
    Then within the reconcile window plus 15 seconds each of doorways "alpha", "beta" and "gamma" reports that it keeps no copy of the notice's bytes
    And an anonymous visitor asking each of the three doorways for "spring-planting-notice" receives HTTP 410 naming the Dowell household as the collective that withdrew it
    And an anonymous request at each of the three doorways for the recorded content address is refused with an HTTP status from 400 to 499 and carries none of the notice's bytes

  Scenario: Withdrawal ends the publication, and the household still holds the notice
    When the Dowell household's decision to withdraw the publication of "spring-planting-notice" is recorded through doorway "alpha"
    Then within the reconcile window plus 15 seconds an anonymous visitor asking doorway "beta" for "spring-planting-notice" receives HTTP 410 naming the Dowell household as the collective that withdrew it
    And Matthew reads "spring-planting-notice" from his own peer, through no doorway, with bytes matching the recorded content address
    And Matthew's peer shows the projection contract as withdrawn, naming the Dowell household and the time of the withdrawal

  # Harness note: the harness registers gamma's resumption before it pauses gamma, so gamma is
  # resumed even when an assertion in this scenario fails.
  @requires:owned-substrate
  Scenario: A doorway that was paused during the withdrawal drops its copy before it serves the notice again
    Pausing a doorway means stopping its process so that it can neither answer nor hear
    anything, then confirming that it is stopped. The household resumes it afterwards.
    Before resuming gamma, the household waits until the two doorways that kept running, alpha
    and beta, have dropped their copies. That wait is not what this scenario proves (the first
    scenario proves it); it sets up the world gamma wakes into, one where the withdrawal has
    already reached every running doorway and gamma alone still holds a copy from before it.

    Given the household pauses doorway "gamma"
    When the Dowell household's decision to withdraw the publication of "spring-planting-notice" is recorded through doorway "alpha"
    And the household waits until doorways "alpha" and "beta" each report that they keep no copy of the notice's bytes, which must happen within the reconcile window plus 15 seconds
    And once that wait is over and the reconcile window plus 15 seconds has passed since the withdrawal, the household resumes doorway "gamma"
    Then the first answer doorway "gamma" gives to an anonymous visitor asking for "spring-planting-notice" is HTTP 410 naming the Dowell household as the collective that withdrew it
    And an anonymous request at doorway "gamma" for the recorded content address is refused with an HTTP status from 400 to 499 and carries none of the notice's bytes
    And doorway "gamma" reports that it keeps no copy of the notice's bytes
