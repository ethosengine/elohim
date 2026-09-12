# BORN @wip, EVERY SCENARIO. No step definition in this file exists yet, and the
# doorway-to-doorway relay it describes is written but not wired
# (doorway/doorway-service/src/cache/delivery_relay.rs,
# services/federation.rs::fetch_from_remote_doorway). A run of this file reports
# undefined steps, and undefined is not evidence.
#
# Design source: the 2026-09-12 operator ruling, recorded in the read-path
# workstream of
# genesis/docs/superpowers/plans/2026-07-31-doorway-federation-failover-sprint-plan.md.
# The companion half — WHETHER a doorway may answer at all, and what the person
# is shown while it does — is features/dataplane/served-under-standing.feature
# under the same concern tag. This file is the routing half only, and routing
# green certifies no part of eligibility.
@e2e @federation @concern:served-under-standing @act:i @requires:owned-substrate
Feature: A doorway that does not host a name still serves it, through the nearest live holder
  A visitor types one name. Which machine ends up answering is not their problem,
  and it should not become their problem because they happened to reach the
  doorway that does not hold that particular site. Any live doorway can route;
  a doorway that cannot host a name can still find the one that does, fetch from
  it once, and hand the visitor both the page and the direct address so the next
  request skips the detour entirely. What it must never do is answer "not here",
  relay a relay, or pass a visitor to a holder that is currently shedding.

  THE WORDS THIS STORY USES.

  A DOORWAY is a machine that puts substrate records in front of ordinary web
  visitors — a projection point, not an owner of what it shows.

  A NAME is a public root a visitor asks for — the address of a site, not the
  address of a machine. Several doorways may be able to route a name; a smaller
  set actually HOLDS it. STAGING a root, where a scenario below says the household
  does it, means publishing a hosting contract for that root naming that doorway —
  an ordinary record on the shared ledger, which is what makes it visible to every
  doorway's registry fold rather than to the one that was told.

  THE SHARED LEDGER is the record every peer holds a copy of and can read back for
  itself: notarised, so a reader verifies a record rather than trusting whoever
  handed it over.

  THE REGISTRY is where a doorway learns who holds a name, and it is a fold — a
  resolution redone from the records rather than a copy kept — of two kinds of
  record on that ledger, never a file an operator maintains. The doorway
  REGISTRATIONS each doorway publishes about itself say which doorways exist and
  how to reach them. The HOSTING CONTRACTS say which doorway serves which root, and
  for which audience. Routing needs both and collapses them into one question — who
  holds this name and is alive — so the scenarios below say "the registry" and mean
  the pair. The audience half of a contract is not routing's business at all:
  whether a particular requester may receive what the holder serves is the fold
  measured in served-under-standing.feature, and nothing here decides it. What
  routing does owe is that a doorway with a registration but no hosting contract for
  a name is never chosen as its holder, which the last scenario pins.

  A HOLDER of a name is a doorway with a live hosting contract for it. LIVE is the
  operative word: a contract that has LAPSED — expired at its stated end, withdrawn
  by its issuer, or superseded by a later one — was a contract and is no longer one,
  and its doorway is no longer a holder. A lapsed contract can still be the newest
  thing a doorway has read, which is the state the last scenario builds on. The
  ROUTING TABLE for a name is simply the set of its holders, with liveness folded
  on top.

  SHEDDING is a doorway answering, honestly and out loud, that it cannot take
  serving work right now — alive enough to say so, which is exactly why its answer
  must not be mistaken for the site's answer.

  NEAREST, in this story, is health first and then owner order: among holders that
  are serving, the first in the order the contracts themselves declare — the order
  their issuers wrote, not one a doorway picks. Round-trip time and capacity are not
  yet advertised between doorways; when that advertisement lands, "nearest" gains a
  term and this paragraph is what must be rewritten. Saying so here keeps the
  scenarios honest about what they measure — they measure that a LIVE holder is
  chosen and a shedding one is not, never that the fastest was. The order half is
  measured only in its negative form below (a shedding first choice is passed over);
  choosing correctly BETWEEN two healthy holders needs a third doorway to be an
  honest test, so it is named here as the next scenario to write rather than left to
  be mistaken for something these four already prove.

  ONE HOP is the whole budget. A doorway forwards a visitor's request to a holder
  at most once per request, and marks the forwarded request as forwarded. A
  doorway receiving a request already carrying that mark serves it itself or
  refuses it; it never forwards again. That is what stops two doorways that each
  believe the other holds a name from handing a request back and forth until it
  times out.

  THE HOLDER'S ORIGIN is the holder's own address, handed back to the client with
  the response, so the client's next request for that name goes direct. THE SHIPPED
  CLIENT — the one inside the app a visitor's browser is running — already prefers
  an address that answered and sticks to it; this is the doorway telling it which
  address that should be.

  THE HOUSEHOLD runs both doorways in these scenarios, "alpha" and "beta", on
  hardware it holds, which is why it can stage a root on one of them only and
  make a holder shed on purpose. That ownership is what the feature's
  `@requires:owned-substrate` tag asks for: a run that does not own its substrate
  may not induce a fault on anyone's doorway, so these scenarios are HELD on such a
  run rather than failed.

  JESSICA is the visitor throughout — one of the household's own people, asking as
  an ordinary web client would. Nothing here turns on who she is: every scenario
  assumes an audience that admits her, and that assumption is exactly what
  served-under-standing.feature measures instead.

  FAULT SAFETY. The one scenario that makes a doorway shed also restores it, and
  asserts the restoration was observed, so a failed run does not leave a household
  doorway unable to serve.

  WHAT THIS FILE DOES NOT CLAIM. It does not decide whether a requester may be
  served — that is served-under-standing.feature's subject. It does not prove the
  wide-area path: continuing ingress from the open internet is a separate
  prerequisite. And it does not prove which doorways a public name resolves to in
  the first place — shared membership does that, in doorway-apex-transition.feature.

  Background:
    # "E2E_DOORWAY_B" is the household lane's own name for the second doorway —
    # the CI vocabulary, not a typo for a missing "_BETA".
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And doorway "beta" at "E2E_DOORWAY_B"
    And both doorways can read the registry of doorway registrations and hosting contracts
    And Jessica is a visitor asking as an ordinary web client

  # The base promise. Jessica reaches beta — which holds no contract for "garden"
  # and, on any ordinary web server, would be a 404. Beta reads the registry,
  # finds alpha holding it and serving, forwards once, and Jessica gets the page.
  # She never learns that any of this happened. The staging is deliberately
  # one-doorway: if both held it, this scenario would pass by never routing at all.
  @wip
  Scenario: A doorway that does not host a name serves it through the nearest live holder
    Given the household stages the root "garden" as hosted by doorway "alpha" only
    And doorway "beta" holds no hosting contract for "garden"
    And doorway "alpha" is serving
    When Jessica asks doorway "beta" for "garden"
    Then Jessica is served "garden"
    # The transparency promise, made checkable: the detour must not show up in what
    # she got. (The holder's origin DOES come back with the reply — that is the next
    # scenario — but it is a hint for her client, not a mark on the page.)
    And the page Jessica received is the one doorway "alpha" would have served her directly
    # "Not from a configured peer list" means: the forwarding decision cites the
    # hosting-contract record it read, which a static operator-maintained list has
    # none of, so a doorway routing from configuration cannot satisfy this line.
    And doorway "beta" resolved the holder from the registry, not from a configured peer list
    And doorway "beta" forwarded the request exactly once
    And doorway "beta" never answered that it does not host "garden"

  # The detour is paid once, not every time. The reply carries alpha's own address,
  # and the client — the same shipped client that already prefers an address that
  # answered — uses it next. This is what keeps the federation from turning every
  # doorway into a permanent proxy for every other.
  @wip
  Scenario: The reply hands the client the holder's origin so the next request goes direct
    Given the household stages the root "garden" as hosted by doorway "alpha" only
    And doorway "beta" holds no hosting contract for "garden"
    And doorway "alpha" is serving
    When Jessica asks doorway "beta" for "garden"
    Then the reply names doorway "alpha" as the origin for "garden"
    When Jessica's client asks for "garden" again
    Then the request went directly to doorway "alpha"
    And doorway "beta" forwarded nothing for that second request

  # Shedding is a holder being honest that it cannot serve right now, and honesty
  # must not be repaid by handing it a visitor anyway. A forwarding doorway that
  # relays the shed has converted one doorway's bad minute into the name's bad
  # minute — the exact failure the federation exists to absorb. This scenario needs
  # TWO holders, so it stages "garden" on both doorways rather than the one-holder
  # staging the scenarios above use; the assertion that matters is that the shed was
  # never what Jessica received, and that the choice was made on liveness rather
  # than on the order the contracts declare.
  #
  # Read the topology carefully, because it differs from the scenarios above: beta
  # is here BOTH the doorway Jessica reaches AND the second holder. Alpha is first
  # in the order the contracts declare, so on order alone beta would forward to
  # alpha; alpha is shedding, so the next holder is beta itself and no hop is taken.
  # "Tries the next holder" therefore means beta walked past the shedding first
  # choice, and the thing that must not happen is beta forwarding to alpha anyway
  # and handing back what alpha said.
  @wip
  Scenario: When the holder sheds, the forwarding doorway tries the next holder
    Given the household stages the root "garden" as hosted by doorway "alpha" and doorway "beta"
    And doorway "alpha" is the first holder in owner order
    And doorway "beta" is the next holder after it
    When the household makes doorway "alpha" shed
    And Jessica asks doorway "beta" for "garden"
    Then Jessica is served "garden"
    # Named in the steps, not only in the comment above: beta itself is what served,
    # and alpha was never asked. Those two together are what "chosen on liveness,
    # ahead of owner order" means here — alpha was first in order and was passed over.
    And doorway "beta" served "garden" itself, taking no hop
    And doorway "alpha" was never contacted for that request
    And the shedding holder's answer was never handed to Jessica
    # Fault safety, and the assertion that the restore actually happened.
    When the household restores doorway "alpha"
    Then doorway "alpha" is serving again

  # The loop budget, stated as the thing that must not happen. Two doorways that
  # each read the other as the holder — a stale contract, a registry mid-reconcile
  # — would otherwise trade one request until it times out, and the visitor would
  # experience a hang rather than a refusal. A forwarded request is marked; a
  # marked request is answered or refused where it lands.
  #
  # The setup has to CONSTRUCT the loop, not merely arrange for nothing to be
  # found: a doorway that never forwards would satisfy a "did not forward twice"
  # assertion while proving nothing. So beta reads a contract naming alpha that has
  # since lapsed — an ordinary registry-mid-reconcile state — and really does
  # forward. Alpha, holding nothing live either, is exactly the doorway that would
  # bounce it back. That it answers instead is the budget doing its work. This is
  # also where the registry's own discipline shows: both doorways are registered,
  # neither holds a live contract, and being registered is not enough to be chosen.
  @wip
  Scenario: A doorway never forwards a forwarded request
    Given the registry still names doorway "alpha" as a holder of "garden" from a contract that has lapsed
    And doorway "alpha" holds no live hosting contract for "garden"
    And doorway "beta" holds no hosting contract for "garden"
    And both doorways are registered in the registry
    When Jessica asks doorway "beta" for "garden"
    Then doorway "beta" forwarded the request to doorway "alpha" exactly once
    And the forwarded request carried the mark that says it was forwarded
    And doorway "alpha" answered that request itself rather than forwarding it onward
    And Jessica received a refusal naming that no doorway holds a live contract for "garden"
    And being registered was not enough for either doorway to be chosen as holder
    And Jessica received that refusal rather than waiting out a timeout
