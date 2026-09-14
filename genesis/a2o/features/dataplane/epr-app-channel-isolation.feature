@e2e @dataplane @concern:doorway-failover @requires:multi-node @act:i
Feature: Public visitors keep the published app while Matthew stages its successor

  Matthew maintains the household's app for public visitors. He needs a separate preview
  address for unfinished changes, so those visitors keep the known published version until
  he deliberately publishes its successor. This story protects those public visitors; it
  does not test how reviewers approve a release.

  An EPR is the published-app record shared by the household. A release declaration binds
  that record to one exact bundle of bytes. Matthew is this fixture app's root author, so the
  existing root-author election policy authorizes him to submit a declaration and publish it.
  In this story, "earned" means Matthew has made that authorized publication claim. Reviewer
  approval is outside this scenario.

  PUBLIC_NAME and CANDIDATE_NAME are fresh, run-owned Host-header names, not DNS records or
  standing public sites. Matthew registers two hosting contracts for the same app and root path:
  one maps PUBLIC_NAME to the published version, and the other maps CANDIDATE_NAME to the
  staged successor. A hosting contract is the recorded permission for an entrance to serve
  that app at that requested name. For each check, the test sends the chosen Host through
  each doorway entrance: the connection chooses an entrance, while the HTTP Host header
  carries the app name the visitor requested. DNS is bypassed in this household check.
  The doorway accepts or serves a declaration mechanically; it cannot
  confer earned status or choose a winner.

  Matthew submits artifact A's declaration through one doorway, then publishes it by making
  an earned claim. That claim is resolved by the existing shared release-selection policy;
  this scenario observes the resulting public bytes, not the policy's internal algorithm.
  Artifact B's later, unearned declaration remains the staged candidate until Matthew
  publishes B; B then becomes public and the candidate channel becomes empty.

  The fixture labels alpha-A and elohim.host are two doorway entrances backed by Matthew's and
  Jessica's storage peers; they are not the requested Host names. James is part of the shared
  household fixture but has no doorway and is not directly observed here. Artifact bytes
  are preloaded at both entrances. Only alpha-A receives the author's declaration, so B
  appearing through Jessica's entrance checks propagation of declaration state, not transfer
  of artifact bytes or a particular internal network hop.

  "Serves artifact A" or "serves artifact B" means the root HTML, its referenced entry asset,
  and version.json are all fetched with the same selected Host; the HTML names that artifact's
  unique asset and version.json has that artifact's build stamp. UNRELATED_NAME is a fresh
  unbound Host sentinel: its 404s prove a catch-all route did not leak B. The 75-second periods
  are a household reconciliation allowance for this fixture, not a WAN availability promise.

  Background:
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"
    And the household's storage peers are "matthew" behind doorway "alpha-A", "jessica" behind doorway "elohim.host", and "james" behind no doorway

  @requires:owned-substrate
  @deliverability-browser
  Scenario: artifact B stays candidate-only until Matthew publishes it
    Given Matthew has built coherent root app artifact A
    And Matthew has registered PUBLIC_NAME for published releases and CANDIDATE_NAME for staged releases of a fresh empty app
    When each doorway receives artifact A's exact bytes
    And Matthew submits artifact A's byte-binding declaration through doorway "alpha-A"
    And Matthew submits his authorized publication claim for artifact A
    Then within 75 seconds both doorways serve PUBLIC_NAME with artifact A
    When Matthew builds coherent successor artifact B
    And each doorway receives artifact B's exact bytes
    And Matthew submits artifact B's byte-binding staging declaration through doorway "alpha-A"
    Then within 75 seconds both doorways serve CANDIDATE_NAME with artifact B
    And both doorways still serve PUBLIC_NAME with artifact A
    And both doorways answer 404 for UNRELATED_NAME without artifact B's asset or build stamp
    When Matthew submits his authorized publication claim for artifact B
    Then within 75 seconds CANDIDATE_NAME answers 404 with "no-candidate-staged" through both doorways
    And both doorways serve PUBLIC_NAME with artifact B
