@e2e @storage @act:i @requires:household-nodes @requires:owned-substrate @wip @concern:artifact-custody
Feature: Shared artifacts keep their reach, safety, and consent promises
  Matthew needs private work to stay private and shared releases to remain
  recoverable without conscripting another person's device. Jessica and James
  need every admission, repair, safety action, and loss of authority to be
  visible, so a missing peer is never mistaken for missing evidence or content.

  Words used below:
  - The PUBLICATION DECISION is the step that evaluates an artifact's reach before any DHT action.
  - REACH is one of Private, SelfScope, Intimate, Trusted, or wider; below Trusted creates zero DHT heads.
  - A channel's HEAD is the release peers currently serve; it moves only through an admitted election.
  - A CUSTODY ADMISSION POLICY EPOCH is a versioned set of requirements a release must satisfy before shared custody; the release names it in its adoption discipline.
  - A FAILURE DOMAIN is a declared dimension with provenance, household first; an undeclared domain never counts as independent.
  - STANDING means authority to act during an interval.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And peer "matthew" at "matthew"
    And peer "jessica" at "jessica"
    And peer "james" at "james"
    And Matthew publishes artifacts through channel "Household builds"
    And James holds the household safety-steward role for channel "Household builds"

  Scenario Outline: Private, self, and intimate artifacts create no DHT heads
    Given Matthew has artifact "Family build" at reach <reach>
    And Matthew has saved its bytes on peer "matthew"
    And Matthew has a second device holding a copy of those bytes
    When peer "matthew" completes the publication decision
    Then the publication record reports zero DHT heads created
    And a DHT lookup for "Family build" returns zero heads
    And the save completes without requesting a custody quorum
    And the second device's copy is reported only as that device's copy

    Examples:
      | reach       |
      | "Private"   |
      | "SelfScope" |
      | "Intimate"  |

  Scenario: Trusted reach admits an election only under its versioned custody policy
    Given Matthew proposes "Trusted build" as a release at reach "Trusted"
    And its adoptionDiscipline names custody admission policy epoch 7
    And policy epoch 7 names the release closure, coding parameters, tolerated failure, evidence ages, and retention window
    And Jessica declares her household as a failure domain
    And James declares his household as a different failure domain
    And Jessica and James each have an authorized custody commitment
    And each has a recent authenticated availability proof
    And each has a recent fetch whose bytes verified and that was served by that same peer
    And no device of publisher Matthew is counted as external custody
    When peer "jessica" evaluates the election under policy epoch 7
    Then the election outcome is "ADMITTED"
    And the admission record names policy epoch 7 and the evidence that satisfied each requirement
    And the channel head moves only through that admitted election

  Scenario: An unknown custody policy epoch is held as a pending election with its reason
    Given "Trusted build" is proposed under custody admission policy epoch 8
    And peer "jessica" can verify the proposal's signature and wire format
    And peer "jessica" does not understand custody admission policy epoch 8
    When peer "jessica" evaluates the proposal
    Then the candidate is retained with election state "pending-election"
    And the recorded reason is "unknown custody admission policy epoch 8"
    And the candidate is neither admitted nor rejected
    And the current channel head is unchanged

  Scenario: A custody shortfall is detected and named
    Given "Trusted build" is admitted under a policy that must survive one whole-copy loss across two household failure domains
    And Jessica's and James's verified copies currently satisfy that policy
    And Jessica's counted availability evidence expires
    When peer "matthew" evaluates the release's current custody
    Then the release is reported "under-custodied" with Jessica's expired evidence as the reason
    And the channel head does not move because of the shortfall alone

  Scenario: Repair asks only an eligible peer and completes within the measured objective
    Given "Trusted build" is reported "under-custodied" because Jessica's availability evidence expired
    And peer "james" holds an authorized custody commitment for "Trusted build", consents to serve it, and is not one of publisher Matthew's devices
    When the repair window opens
    Then repair requests a copy only from an eligible peer, one that holds an authorized custody commitment, consents, and is not one of the publisher's devices
    And within the measured 900 second objective that peer verifies the full release and records fresh serving evidence
    And the repair record reports elapsed seconds and returns the release to its required custody level
    And a missed deadline is reported with its cause

  Scenario Outline: Quarantine is an expiring hold while refusal is terminal
    Given James can fetch release "Unsafe build" from peer "jessica"
    When the household's safety steward applies <action> to that release on peer "jessica"
    Then adoption, serving, and redistribution of "Unsafe build" are blocked
    And the decision is recorded as <decision>
    And the local payload is <payload state>
    And the decision reports <duration>
    And the signed decision proof remains available separately from the payload

    Examples:
      | action                                 | decision      | payload state                       | duration                         |
      | "quarantine until 2026-11-08T00:00Z"  | "QUARANTINED" | "retained but unavailable to serve" | "expiry 2026-11-08T00:00Z"      |
      | "refuse for verified harmful content" | "REFUSED"     | "destroyed on peer jessica"          | "terminal with no expiry"       |

  Scenario: Consent withdrawal retires replication and serving without deleting proof
    Given Matthew's consent authorizes peer "jessica" to replicate and serve "Trusted build"
    And Jessica has recorded the content address of that consent and its custody proof
    When Matthew withdraws that consent
    And peer "jessica" verifies the withdrawal
    Then replication and serving authority for "Trusted build" on peer "jessica" are retired with reason "consent-withdrawn"
    And peer "jessica" supplies no payload bytes for a later request
    And the former custody proof remains retrievable and verifiable at its recorded content address
    And that proof is reported as historical evidence, not current authority

  Scenario: Supersede, withdraw, and standing revocation remain separate lifecycle acts
    Given channel "Household builds" has releases "build-1", "build-2", and "build-3"
    And each release has separate eligibility, serving authority, retention, and authority evidence
    When an authorized election supersedes "build-1" with "build-2"
    And Matthew withdraws "build-2" from new adoption without electing a replacement
    And the issuer's standing to serve "build-3" is revoked for a named interval
    Then "build-1" records "superseded" and can return only through current qualification and a new election
    And "build-2" records "withdrawn", prevents new adoption, and names no replacement head
    And "build-3" records "standing-revoked" with the affected acts and interval
    And the issuer's standing at publication time for releases outside that interval remains assessable
    And no one of the three records is inferred from either of the others

  Scenario: No eligible head is explicit and an unreachable candidate is not absent
    Given the current release of "Household builds" no longer satisfies its qualification rule
    And a known candidate release has signed evidence but peer "james" is unreachable
    When peer "jessica" runs the channel election
    Then the channel reports "no-eligible-head" with the failed qualification rule
    And the known candidate is reported "unreachable" with its last evidence time
    And no report calls the unreachable candidate absent or deletes its evidence
    And the channel does not silently fall back to an earlier release

  Scenario: An older reader cannot admit an election under a policy epoch it cannot interpret
    Given peer "james" understands custody admission policy epoch 8 and has admitted "Trusted build" under it
    And peer "jessica" is an older reader that understands policy epoch 7 but not epoch 8
    When peer "jessica" receives the epoch 8 election and its verified evidence
    Then peer "jessica" records "unsupported-policy-epoch:8"
    And its local election state remains "pending-election"
    And peer "jessica" does not admit the election or move its channel head
    And the epoch 8 candidate remains retained by its content address for a reader that can interpret it
