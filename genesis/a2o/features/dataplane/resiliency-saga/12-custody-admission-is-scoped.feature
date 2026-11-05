@e2e @dataplane @act:i @wip @concern:addresses-are-declared-profiles @requires:household-nodes @requires:owned-substrate
Feature: Chapter 12 — sharing scope determines whether custody can admit a head
  Matthew wants to keep private work private and know when a shared release has
  earned the protection it promises. Jessica and James each run a household
  peer, named "jessica" and "james"; Matthew's peer is named "matthew".

  A head names the current version of an item. A DHT head announces that version
  in the distributed network's shared record. Each item carries a sharing scope.
  The three lowest scopes, Private, SelfScope and Intimate, keep work with its
  author or their closest people; that work creates no DHT heads and needs no
  custody quorum. Trusted and the scopes above it (Familiar, Community, Public,
  Commons) share work beyond that circle, so other peers must be able to rely
  on it being kept.

  A channel is a named stream of releases, and its admitted head is the release
  other peers currently rely on. At Trusted scope and above, an election
  records a proposed release and the outcome of evaluating it. Custody admission
  checks whether that election has the evidence required by its declared
  policy. Permission to share does not itself supply that evidence.

  The policy is carried in adoptionDiscipline, the release's adoption rules.
  A policy might, for example, require that named custodians on independent
  devices have each fetched the release and verified its bytes recently.
  Evidence must satisfy the policy's declared failures, holder independence,
  consent, freshness and retention requirements; household membership alone
  proves none of them.

  Two version numbers travel with a head. Its wire epoch names the format of
  the head itself. Its policy epoch names which version of the adoption rules
  applies. A peer can read a head's format and still not know its policy
  version. The epochs 7 and 8 used below are arbitrary test versions.

  HELD means the receiver retains a candidate and explains what it cannot yet
  interpret. It neither admits the candidate nor declares it rejected.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And peer "matthew" at "matthew"
    And peer "jessica" at "jessica"
    And peer "james" at "james"

  Scenario Outline: Matthew can finish private work without announcing a DHT head
    Given Matthew has a new item "Family notes" at scope "<scope>"
    And no DHT head exists for "Family notes"
    And no other peer has promised custody of "Family notes"
    When Matthew saves a revision of "Family notes" on peer "matthew"
    And peer "matthew" completes the publication decision for that revision
    Then Matthew can reopen the saved revision of "Family notes"
    And the publication record reports zero DHT heads created for "Family notes"
    And a DHT lookup finds zero heads for "Family notes"
    And the save is complete without waiting for a custody quorum

    Examples:
      | scope     |
      | Private   |
      | SelfScope |
      | Intimate  |

  Scenario Outline: A shared election names the custody policy that admitted it
    Given Matthew proposes "Household handbook" as a channel release at scope "<scope>"
    And the candidate head uses wire epoch 0
    And its adoptionDiscipline names custody admission policy epoch 7
    And peer "jessica" understands wire epoch 0 and custody admission policy epoch 7
    And the candidate's custody evidence satisfies every admission requirement of policy epoch 7
    When peer "jessica" evaluates the proposed election
    Then the election's custody admission outcome is "ADMITTED"
    And its admission record names custody admission policy epoch 7
    And the admitted head still carries policy epoch 7 in adoptionDiscipline and wire epoch 0

    Examples:
      | scope     |
      | Trusted   |
      | Familiar  |
      | Community |
      | Public    |
      | Commons   |

  Scenario: Trusted scope alone cannot replace missing custody evidence
    Given Matthew proposes "Household handbook" as a channel release at scope "Trusted"
    And its adoptionDiscipline names custody admission policy epoch 7
    And peer "jessica" understands custody admission policy epoch 7
    And one custodian's verified fetch required by that policy is missing
    And the channel already has an admitted head
    When peer "jessica" evaluates the proposed election
    Then the election remains "pending-election"
    And its recorded reason names the missing custodian's verified fetch
    And the channel's admitted head is unchanged
    And Matthew can still reopen his proposed revision locally

  Scenario: Jessica holds a head whose custody policy she cannot interpret
    Given peer "jessica" has an admitted head for "Household handbook"
    And Matthew supplies a candidate head with wire epoch 0 and a verified signature
    And the candidate's adoptionDiscipline names custody admission policy epoch 8
    And peer "jessica" understands wire epoch 0 but not custody admission policy epoch 8
    When peer "jessica" evaluates the candidate's election
    Then the candidate's custody admission outcome is "HELD"
    And Jessica's admission report names unknown custody admission policy epoch 8
    And the retained candidate can be read back by its original content address
    And the candidate is neither admitted nor recorded as rejected
    And the previously admitted head for "Household handbook" is unchanged
