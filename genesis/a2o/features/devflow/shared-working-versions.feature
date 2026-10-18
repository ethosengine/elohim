@e2e @devflow @wip @concern:dev-system-equilibrium @requires:household-nodes
Feature: People share unfinished work without silently publishing or executing it
  Matthew works from two enrolled devices with Jessica in the Ethosengine collective.
  A working version holds unfinished changes; a candidate fixes one version for verification.
  A branch agreement says who may follow, edit, or submit work and to which audience.
  Avodah is their work view over native records, showing evidence and available choices.
  A commons charter governs shared storage, bandwidth and maintenance labor;
  shared compute requires a separate agreement. Source and evidence are artifacts.
  A person's elohim is their assistant, never an independent collective decision maker.
  These are pending acceptance stories, not evidence of implemented behavior.

  # Design: genesis/docs/superpowers/specs/2026-10-13-shared-working-versions-social-coding-design.md

  Scenario Outline: Saved changes survive reopening offline
    Given Matthew holds a working version and no other device is reachable
    When he changes it through "<surface>" and saves
    And closes and reopens the workspace without a network
    Then he can read and continue editing his saved change
    And the change remains unpublished
    And other copies have unknown freshness

    Examples:
      | surface             |
      | the Avodah editor   |
      | a filesystem editor |

  Scenario: Private work overrides the ordinary own-device agreement
    Given Matthew's standing agreement admits his two enrolled devices to ordinary working versions
    And he saves a new ordinary change while his other device is offline and holds the old version
    And he saves another working version marked private to this device
    When his other enrolled device becomes reachable
    Then the other device receives the new ordinary change
    And the private version is neither listed nor transferred

  Scenario: Collective membership alone does not disclose a personal branch
    Given Jessica belongs to Ethosengine but is not admitted to Matthew's personal branch
    When she requests its listing and contents
    Then neither request discloses the branch or its contents

  Scenario Outline: The accepted branch mode bounds participation
    Given Matthew and Jessica have accepted "<agreement>" for this branch
    When Jessica receives unfinished work and attempts to edit its shared copy
    Then "<result>"
    And publication still requires the agreement's separate submission authority

    Examples:
      | agreement          | result                                                               |
      | a follow agreement | her shared edit is refused and she may prepare a separate proposal    |
      | a coedit agreement | her permitted edit enters the shared copy with attribution to Jessica |

  Scenario Outline: Agent shared-edit authority comes from a human agreement
    Given Matthew authorizes an agent under "<agreement>"
    When the agent prepares a change
    Then the change "<result>"
    And changes outside the authorized scope remain proposals for human acceptance

    Examples:
      | agreement                                | result                                                  |
      | the default proposal agreement           | waits for acceptance before entering the shared copy    |
      | an explicit bounded live coedit agreement | enters the shared copy only within its authorized scope |

  Scenario: Following unfinished source does not execute it
    Given Jessica follows a branch and runs a previously verified application
    And her application has an active test watcher in a separate execution worktree
    When unfinished source arrives in her collaboration worktree
    Then her editor shows it as unbuilt
    And its arrival starts no hook, build, install, test, preview, or restart
    And the running application's version remains unchanged and separately identified

  Scenario: Push evidence belongs to an immutable candidate
    Given Matthew requests push of candidate A with its source, target, and gate declarations fixed
    And reviewers are selected only after those declarations are fixed
    When native gates run in A's isolated checkout
    And Jessica adds B to the live branch during that run
    Then Avodah shows the request, running gates, and outcomes for A
    And B remains unverified and cannot inherit A's permission to push or merge
    And readiness, review, consent, and completed merge remain separate states

  Scenario: Failed verification is not a sanction
    Given Matthew requests push of an immutable candidate
    When a required native gate fails
    Then Avodah shows the refused operation and evidence
    And Matthew can keep editing
    And the failure alone changes no membership, standing, or contribution right

  Scenario: Other holders add availability without replacing local work
    Given Matthew's publication audience admits the hub and two serving doorways
    And the doorways hold projection agreements for that publication
    When he publishes through his own node
    And each receiver verifies the declared head and acquires its bytes
    Then both doorways serve that declared version with separate freshness evidence
    When the hub and doorways become unreachable
    Then Matthew can read his held version and prepare another change locally

  Scenario: Different development spaces do not silently share authority
    Given a record belongs to an isolated development DNA space
    And a DNA space identifies a distinct conductor validation and network context
    When the record reaches a collective node in another DNA space
    Then it is neither adopted nor re-authored there
    And the refusal names the source-space mismatch

  Scenario Outline: Commons rules require declarations and probes
    Given members adopted a fixture charter with "<declaration>" and its concrete rule values
    When a member runs its "<probe>"
    Then evidence establishes "<outcome>"
    And if the probe cannot run its result is unmeasured rather than satisfied

    Examples:
      | declaration                         | probe                                  | outcome                                               |
      | Q1 purpose and resource rules       | fetch the charter-named rules          | fetched bytes match their declared content address and state the purpose and draw rules |
      | Q2 proportional draw rule           | request a draw above the agreed limit  | the granted allocation is capped at the member's remaining allowance in the declared window           |
      | Q3 facility condition measure       | read as an ordinary member             | condition is readable without a steward                |
      | Q5 draw-record visibility           | list permitted recent draws            | records stay within the member's admitted audience      |
      | Q6 joining and leaving              | withdraw and request another draw      | current membership refuses the new draw                |
      | Q7 holdings and boundaries          | draw outside holdings                  | refusal names the resource boundary                    |
      | Q8 member-proposed amendments       | propose and adopt a charter amendment  | two peers derive the same revised charter head         |
      | Q9 dispute and appeal               | open and settle a challenge            | the human outcome is witnessed and appeal is available |
      | Q10 parent recognition              | resolve the parent's recognition       | it names this charter and reserved powers              |

  Scenario: Q4 repeated violation awaits the declared human decision
    Given members adopted a ladder of notice, temporary draw restriction, and exclusion with appeal
    And their charter names Jessica and Matthew as the human decision body for this fixture
    And a member has received the first notice for a witnessed resource-rule violation
    When the same violation is witnessed again
    Then the evidence awaits that body's decision without automatically restricting the member
    When Jessica and Matthew decide to apply the declared second rung
    Then the temporary draw restriction is recorded with their decision and its return path
    And the member can appeal through the charter's declared challenge process

  Scenario Outline: Q11 decision size is bounded directly or through approved nesting
    Given the fixture charter limits each deciding body to five people
    And "<structure>"
    When a member inspects the decision structure and its membership counts
    Then the evidence shows "<outcome>"

    Examples:
      | structure                                                                | outcome                                                        |
      | this body has four members                                               | four is within the five-person ceiling                         |
      | eight members belong to two approved four-person bodies under a two-person parent body    | both child bodies and the parent meet the ceiling and the parent recognition resolves |
      | this body has eight members and no approved nesting                       | the size rule fails and merely naming a parent cannot satisfy it |

  Scenario: Management and exclusion rights cannot be sold away
    Given members adopted a coding commons charter with declared artifact rights
    When they inspect those rights
    Then access, contribution, extraction, removal, management, and exclusion name their holders
    And nobody may sell or lease management and exclusion
    And leaving ends future access without promising erasure of already received copies

  Scenario: Several devices and agents do not multiply a human choice
    Given Matthew has two devices and agents while members decide an amendment
    When he votes from one device and repeats the act from another
    Then the decision contains one human choice from Matthew
    And agents may carry evidence but cannot cast or multiply his vote
    And his serving doorway has no additional seat

  Scenario: Repeated agents cannot supply missing plurality
    Given members have approved an amendment to their resource allocation rule
    And the charter also requires evidence from distinct assistant model lineages before that amendment takes effect
    And the fixture evidence falls short of that declared diversity rule
    When the amendment is evaluated before and after adding agents of an already counted lineage
    Then neither evaluation permits the amendment to take effect and both show the unmet rule
    And the previous resource allocation rule remains in force

  Scenario: Sensitive participation stays outside public pool carriers
    Given Jessica holds a private team working version
    And the available pool carrier exposes membership and compute draws publicly
    When she requests sharing through it
    Then transfer refuses before disclosing membership or the working version
    And she can continue editing locally

  Scenario: Assistance watches the facility and leaves choices with people
    Given a member's own elohim observes a facility bound approaching its limit
    When it advises under the standing agreement
    Then it offers evidence and one choice without requiring a monitoring dashboard
    And the warning alone imposes no sanction
    And no pool agent gains authority to monitor people or decide for members
