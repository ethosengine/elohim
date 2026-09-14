@e2e @dataplane @concern:served-under-standing @act:i @requires:owned-substrate
Feature: Doorway visitors can trace access decisions, contributions and response obligations
  These acceptance specifications cover successful serves and reach-based HTTP 403
  refusals through the household's owned mesh. Here, standing is the current identity
  and relationship evidence considered under the resource's declared reach.

  The Dowell household is the governing collective in this story. It decides who may
  read the shared garden resource and owes an answer when James objects. The fixture
  records that collective decision through alpha; these scenarios do not verify the
  recording actor's authority or a formal ruling ceremony.

  EPR means Elohim Protocol Record: here it identifies the published garden resource.
  The project-epr contract is the same record later called the projection contract or
  declaration. It gives alpha permission to serve and names alpha's provider. Its reach
  field states the audience. Narrowing updates
  that field in place, under the same contract identifier; it does not create a new
  contract. The full declaration means the complete returned contract record, including
  its metadata; comparing it requires every returned field to remain unchanged.
  The hosting-agreement records computing and storage contribution,
  operate-doorway records operation of beta, and a challenge record applies the
  contract's prior response promise to James's objection.

  Matthew, Susan, and James make named requests with hosted bearer credentials tied to
  their canonical fixture humans: the named people's existing Genesis identities.
  The bearer credential and the separately read peer membership record must identify
  the same person. This proves only the hosted-identity path used here; direct device-to-doorway
  credential handoff remains unproved.

  Alpha holds the scenario's uniquely marked bytes. Beta is a separate entrance that
  can relay them but cannot invent reach or claim alpha's holding work. Anonymous
  requests are clean probes of public permission: after the contract's reach changes
  from commons to Dowell members, their HTTP 403 shows that public permission was
  withdrawn. The member-control scenarios separately show that the scenario-created
  bytes still exist for an admitted member.

  Private reach is for the person the resource belongs to, established from that person's
  own record; household membership alone does not make someone that beneficiary. The current
  doorway has no beneficiary verifier, so it cannot establish that access for anyone, even
  if James might hold it outside this test. His hosted identity and Dowell membership therefore
  remain insufficient for this current serving path. His objection asks the household to
  reconsider choosing private reach regardless of whether his own record makes him the beneficiary;
  it does not ask the doorway to invent or confer that status.

  A reach refusal names the failed reach, the Dowell household, the same contract
  identifier whose reach field changed, and the challenge address. The tests do not
  extend this traceability promise to 404, 5xx, or standing-unavailable 503 responses.

  The attribution-only receipt scenario remains at commons reach. Its introduction says that
  someone kept the resource ready and someone put it in front of Matthew; separate plain-language
  credit sentences name who kept it and who operated the entrance. Their records support that
  audit but do not prove payment. A protocol-form URL returns the supporting records. The HTTP
  response frame is called chrome here; no browser presentation or rendered role label is asserted.

  The fixture declares a 60-second convergence window. The holder-side anonymous-refusal
  observation has an explicit deadline of that window plus one request/poll interval, at most
  15 seconds; it does not claim to prove the unobserved instant when enforcement began. Beta's
  unprompted poll has its own 75-second local observation bound. These are local topology limits,
  not WAN guarantees. James's response is due
  72 hours after submission; the test allows five minutes on either side for request
  observation and client/server clock differences, not extra response time.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And doorway "beta" at "E2E_DOORWAY_B"
    And the collective "the Dowell household" stewards the EPR "community-garden-club" at commons reach
    And doorway "alpha" holds a live projection contract for "community-garden-club"
    And alpha's original contract identifier and scenario-created byte marker are observed before any reach change
    And doorway "beta" holds no contract for "community-garden-club"
    And the household's declared reconcile window is read from its fixture manifest

  Scenario: Beta refuses anonymous access after the steward narrows the audience through alpha
    Given an anonymous probe at doorway "beta" observes public permission for "community-garden-club" before withdrawal
    When the Dowell household decision recorded through alpha says "the Dowell household" has narrowed "community-garden-club" to members of "the Dowell household"
    And within 75 seconds beta is polled without a refresh instruction until it enforces and references the changed reach on the same contract
    Then an anonymous visitor at doorway "beta" is refused "community-garden-club"
    And the HTTP 403 refusal names reach as the failed term, not absence or a service error
    And the refusal names "the Dowell household" as the collective whose recorded decision narrowed it
    And the refusal gives the collective's challenge address for that decision
    And following the refusal's declaration reference at doorway "beta" returns that same contract identifier with the changed reach and deciding collective

  Scenario: A member and a nonmember receive different answers under one narrowed declaration
    Given Matthew's peer membership record states his membership in "the Dowell household"
    And Susan's peer membership record states no membership in "the Dowell household"
    When the Dowell household decision recorded through alpha says "community-garden-club" now admits only members of "the Dowell household"
    And within 75 seconds beta is polled without a refresh instruction until it enforces and references the changed reach on the same contract
    And canonical Matthew presents his hosted bearer when asking doorway "beta" for "community-garden-club"
    Then Matthew is served the originally observed byte marker for "community-garden-club"
    And doorway "beta" names alpha as the live holder that supplied the bytes
    And the chrome names the reach that admitted him and the holder that served the bytes
    When canonical Susan presents her hosted bearer at doorway "beta" between two further Matthew requests for "community-garden-club"
    Then Susan is refused "community-garden-club"
    And the refusal names "the Dowell household" as the collective whose recorded decision narrowed it
    And the refusal gives the collective's challenge address for that decision
    And the surrounding Matthew controls return the original marker while before-and-after reads keep the same contract identifier, reach and full declaration
    And the HTTP 403 refusal names reach as the failed term, not absence or a service error
    And following the refusal's declaration reference at doorway "beta" returns that same contract identifier with the changed reach and deciding collective

  Scenario: Narrowing refuses an anonymous visitor while the holder can still serve a member
    Given Matthew's peer membership record states his membership in "the Dowell household"
    And doorway "alpha" has already served "community-garden-club" to an anonymous visitor
    When the Dowell household decision recorded through alpha says "the Dowell household" has narrowed "community-garden-club" to members of "the Dowell household"
    Then within the fixture window plus 15 seconds of observation allowance an anonymous visitor at doorway "alpha" is refused "community-garden-club"
    And the HTTP 403 refusal names reach as the failed term, not absence or a service error
    And the refusal names "the Dowell household" as the collective whose recorded decision narrowed it
    And the refusal gives the collective's challenge address for that decision
    And following the refusal's declaration reference at doorway "alpha" returns that same contract identifier with the changed reach and deciding collective
    And doorway "alpha" serves Matthew the scenario-created resource bytes because the narrowed reach admits his current membership

  Scenario: Matthew sees alpha credited for holding while beta is credited only for operating the entrance
    Given Matthew's peer membership record states his membership in "the Dowell household"
    And "community-garden-club" remains at its original commons reach
    And doorway "alpha" is the holder whose hosting agreement records computing time and storage
    When canonical Matthew presents his hosted bearer and is served "community-garden-club" through doorway "beta"
    Then doorway "beta" names alpha as the live holder that supplied the bytes
    And the chrome carries a contribution receipt for this serve
    And the receipt names computing time and storage as the hosting contribution
    And the receipt's held credit resolves to alpha's exact project-EPR record and provider
    And the receipt's projected credit resolves to beta's exact operate-doorway record, separately from the holder
    And the receipt explains in plain sentences who kept the resource ready and who operated the entrance
    And each receipt record link returns the exact credited record identifier
    And the receipt begins with its plain introduction and offers one protocol-form link

  Scenario: James challenges a private reach decision and sees a recorded obligation and deadline
    Given James's peer membership record states his membership in "the Dowell household"
    And the Dowell household decision recorded through alpha sets private reach for "community-garden-club"
    And the projection contract already names the collective and its 72-hour response promise
    And within 75 seconds beta is polled without a refresh instruction until it enforces and references the changed reach on the same contract
    When canonical James presents his hosted bearer and membership evidence to doorway "beta" under private reach for "community-garden-club"
    Then the HTTP 403 refusal shows the current doorway cannot serve private reach without beneficiary verification
    And the private refusal points to "the Dowell household" and the contract that recorded its decision
    And following the refusal's declaration reference at doorway "beta" returns that same contract identifier with the changed reach and deciding collective
    And the refusal gives the collective's challenge address for that decision
    When James submits "I ask the Dowell household to reconsider keeping this resource at private reach, whether or not my own record makes me its beneficiary. Please look at the audience decision again." through the refusal's challenge address
    Then James receives a challenge record identifier carrying the collective's existing response promise
    And the challenge record names "the Dowell household" as the party that owes the response
    And the challenge deadline is between submission plus 72 hours minus five minutes and submission plus 72 hours plus five minutes
    And James retrieves the challenge record with his written objection, the collective that owes a response and its due date through beta using that identifier
    And doorway "beta" did not answer the challenge on the collective's behalf
