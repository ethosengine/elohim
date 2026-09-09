@e2e @dataplane @concern:doorway-failover @act:i
Feature: A visitor's public name continues to reach the site through a doorway's bad hour
  A visitor uses one public name for a site. Two doorways hold the same declared
  version, and the serving path can select either doorway. The visitor should
  not need to discover or type a different address when one doorway sheds work.

  The declared head is the notarized identity of the current site version.
  Shedding means a doorway is alive but reports that it cannot currently accept
  serving work. Its diagnostic address stays available. Shared membership is
  the set of doorway addresses currently advertised as eligible to serve.
  Each doorway owns the address records it contributes to that set: its owner
  records. Its exclusive diagnostic name is a separate address that reaches
  only that doorway, so an operator can inspect it even while it sheds.
  A build stamp is the version file inside the declared app bundle. Matching
  the stamp checks that the browser booted the same version the head names.
  Membership and selection are separate: two advertised addresses can still
  lead to an intermediary that always selects the same unavailable doorway.

  The household must own the public-name routing apparatus and its fault
  controls before exercising these scenarios. Kubernetes and DNS are possible
  test-bench projections of this contract, not its authority. A test-only proxy
  that does not execute the actual routing configuration cannot certify the
  deployed path. Browser retries after an already loaded app do not prove that
  a new visitor can obtain the first page.

  # Missing apparatus and implementation remain visible through @wip.
  # Never ingest a steady-state root GET as fulfilment of this feature.
  @wip @requires:owned-substrate
  Scenario: Shared membership removes only the doorway that sheds and readmits it after recovery
    Given both owned doorways advertise eligibility for the same public name
    When the household makes one doorway report non-serving for three consecutive probes
    Then only that doorway's owner records leave shared membership
    And its exclusive diagnostic name and its sibling's membership remain unchanged
    When that doorway reports serving for two consecutive probes
    Then its owner records rejoin shared membership without duplicating the sibling

  @wip @requires:owned-substrate
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
