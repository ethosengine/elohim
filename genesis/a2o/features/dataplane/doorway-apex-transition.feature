@e2e @dataplane @concern:doorway-failover @act:i
Feature: A visitor's public name continues to reach the site through a doorway's bad hour
  A visitor uses one public name for a site. Two doorways hold the same declared
  version, and the serving path can select either doorway. The visitor should
  not need to discover or type a different address when one doorway sheds work.

  The household is the group of people who run these doorways themselves, on
  hardware they hold, and who therefore own the routing apparatus that decides
  which doorway a visitor reaches — and own the controls that make one of them
  fail on purpose. Where a scenario says the household does something, it is
  that group acting on its own infrastructure, never a test harness acting on
  someone else's.

  The declared head is the notarized identity of the current site version.
  Shedding means a doorway is alive but reports that it cannot currently accept
  serving work. Its diagnostic address stays available. Shared membership is
  the set of doorway addresses currently advertised as eligible to serve.
  Each doorway owns the address records it contributes to that set: its owner
  records. Its exclusive diagnostic name is a separate address that reaches
  only that doorway, so an operator can inspect it even while it sheds.
  The apex name is the public name at the top of the site's own address — the
  one a visitor types with nothing in front of it — so it is the name with no
  fallback to fall back to. A build stamp is the version file inside the
  declared app bundle. Matching the stamp checks that the browser booted the
  same version the head names; completing browser bootstrap means the served
  bundle came up far enough to report that stamp.
  Membership and selection are separate: two advertised addresses can still
  lead to an intermediary that always selects the same unavailable doorway.
  Selection here is the one a shipped client makes over several advertised
  addresses: try them in order and stick to the first that answers. A visitor
  reaching the public name in these scenarios makes exactly that choice, over
  exactly the set membership currently advertises.

  Membership changes only on repeated evidence, and it is deliberately harder
  to leave than to return: a doorway must look non-serving three times running
  before its records leave, and serving twice running before they come back.
  The asymmetry is what stops one slow answer from flapping a healthy doorway
  in and out of the set. The declared recovery bound is the household's own
  convergence window for this mesh, declared in its fixture manifest alongside
  the withdraw and rejoin bounds the apparatus itself expects.

  The household must own the public-name routing apparatus and its fault
  controls before exercising these scenarios. Kubernetes and DNS are possible
  test-bench projections of this contract, not its authority. A test-only proxy
  that does not execute the actual routing configuration cannot certify the
  deployed path. Browser retries after an already loaded app do not prove that
  a new visitor can obtain the first page. A steady-state GET of the site root
  is never fulfilment of this feature: nothing is proved without an induced
  fault. A household that has staged no routing apparatus fails these scenarios
  by naming that absence, rather than by reading a set nothing maintains. And
  the address the public name resolves to must actually be reachable from
  outside: continuing ingress over the wide-area network is a separate
  prerequisite, which a working membership set does not supply.

  The first scenario proves the mechanism — that membership tracks which
  doorway can serve, per doorway, without collateral damage to its sibling.
  The second proves the promise that mechanism exists for: that a visitor who
  arrives during the outage, knowing only the one name, still gets the page.

  Both scenarios are tagged with the capability they depend on: a substrate
  this run owns. On a run that owns none — a deployed fleet nobody here may
  fault on purpose — they are HELD (skipped), never failed, because a
  scenario that cannot be exercised has proved nothing either way.

  # How the household owns it here: `just mesh start` stages one
  # relay-addr-beacon leg per owned doorway (`--sink file`), each maintaining
  # only its own entry in one membership document, decided by the same serving
  # probe and join/leave hysteresis the fleet's DNS projection runs. The
  # scenarios read that document and resolve the public name through it.
  @requires:owned-substrate
  Scenario: Shared membership removes only the doorway that sheds and readmits it after recovery
    Given both owned doorways advertise eligibility for the same public name
    When the household makes one doorway report non-serving for three consecutive probes
    Then only that doorway's owner records leave shared membership
    And its exclusive diagnostic name and its sibling's membership remain unchanged
    When that doorway reports serving for two consecutive probes
    Then its owner records rejoin shared membership without duplicating the sibling

  @requires:owned-substrate
  Scenario: The apex name survives its doorway's shed
    Given a new visitor reaches the declared landing page through one owned public name
    And the household observes which doorway answered that visit
    When the household makes that doorway shed while its sibling keeps serving the same declared head
    Then another new visitor using the same public name receives the landing page from the sibling
    And the raw response status is 200
    And the raw response body contains "app-root"
    And that visitor completes browser bootstrap with the same declared build stamp
    When the household restores the shedding doorway
    Then it rejoins the serving set within the declared recovery bound
    And the same public name still serves the declared landing page
