@e2e @deployment @substrate-floor @compute-commitment @wip
Feature: Compute commitments are bounded and breach without contagion
  As a contributor whose compute capacity is finite
  I want compute commitments to be first-class economic objects with explicit
  bounds, executed by the substrate floor with elohim discernment as a ceiling
  So that catastrophic compute loss does not silence my authored work, my
  recognition, or my standing — only the forward-looking compute I can no
  longer provide

  Compute has limits. Every node, no matter how rich, commits bounded scopes
  to bounded counterparties for bounded windows. Breach is normal, not
  exceptional. The protocol records it gracefully, and attribution-class
  valueflows continue regardless. Stewardship through catastrophic loss is in
  the same family as stewardship through disability or stewardship handoff —
  the protocol treats them with equal dignity.

  See: docs/superpowers/specs/2026-05-04-compute-commitment-substrate-floor-design.md

  Background:
    Given the alpha cluster has a compute-capacity ledger declaring per-node bounds
    And the ledger records 46 cores, 134 GiB RAM allocatable across 7 ready nodes
    And matthew, jessica, timothy are deployed with active compute commitments
    And adam, pete, frank were deployed on shem with active compute commitments
    And shem has been catastrophically lost
    # Reference: shem-decommissioned anomaly in compute-capacity.json

  # --- Decoupling: compute breach does not contaminate attribution ---

  Scenario: Catastrophic compute loss does not silence authored work
    Given adam's source chain on shem is unrecoverable
    And matthew's storage holds adam's previously-replicated authored content
    When a peer queries content authored by adam
    Then matthew's storage returns the work attributed to adam
    And adam's authorship is preserved in citation graphs
    And the love-map between adam and eve remains queryable
    And no flow that recognizes adam's prior contribution is gated on adam's
      compute being online

  Scenario: Catastrophic compute loss is recorded as compute-class breach only
    When the operator marks adam, pete, frank as suspended in deployments.json
    Then their compute-allocation commitments enter breach state
    And each breach event records cause "catastrophic-loss"
    And each breach event records witness "counterparty" or "collective"
      because the breached agent cannot self-attest
    But their attribution-class flows continue unaffected:
      | flow                                                | status     |
      | matthew citing adam's contribution                  | continues  |
      | jessica's love-map references adam                  | continues  |
      | pete's pastoral teachings remain attributed         | continues  |
      | frank's farming-knowledge contributions             | continues  |
      | adam's stewardship standing                         | frozen     |
      | pete's recognition by his faith community           | continues  |

  Scenario: Standing agreements with the lost executor breach with clear cause
    Given adam had a standing agreement to validate alpha-collective trust commits hourly
    When the next scheduled execution time arrives after shem's loss
    Then a compute-breach event records cause "standing-execution-missed"
    And the breach references the original standing agreement
    And the breach recovery_path is "rebootstrap-required"
    And other contributors' standing agreements are unaffected

  # --- Substrate floor: deterministic, elohim-free ---

  Scenario: Substrate negotiates a compute request without elohim
    Given matthew's elohim is not running
    And matthew's household has 800 cpu_m headroom and 2 GiB memory headroom
    When a peer requests 500 cpu_m and 1 GiB memory for a one-shot inference task
    Then the substrate scheduler evaluates the request against the capacity ledger
    And returns Granted deterministically
    And records a Commitment with signal_kind "compute-allocation"
    And the commitment's negotiated_by.kind is "substrate-floor"
    And no elohim discernment is required for the verdict to be valid

  Scenario: Substrate denies a compute request that exceeds bounds
    Given matthew's household has 200 cpu_m headroom
    When a peer requests 500 cpu_m
    Then the substrate scheduler returns Denied
    And the denial reason is "capacity-exhausted"
    And a Commitment with signal_kind "compute-allocation" is NOT created
    And the peer receives a structured response a human can act on
    # Operator parameter: bounds derived from compute-capacity.json headroom

  Scenario: Standing agreement fires deterministically without elohim
    Given matthew authored a standing agreement to gossip-relay imagodei-trust signals
    And the standing agreement is recorded with trigger_kind "standing"
    When a matching gossip message arrives
    Then the substrate executes the relay deterministically
    And no elohim wake-up is required for execution
    And fulfillment is recorded as a fulfillment event against the standing agreement

  # --- Elohim ceiling: enriches, never gates ---

  Scenario: Elohim adds discernment without invalidating substrate verdict
    Given the substrate has Granted a peer's compute request
    And matthew's elohim is now lit and has discernment context
    When the elohim observes the granted request
    Then the elohim may record a FeedbackSignal expressing discernment
      | discernment kind         | example                                        |
      | reciprocity-pattern      | "this counterparty has honored prior commits"  |
      | values-alignment         | "this purpose aligns with household priorities" |
      | mint-recommendation      | "credit reciprocity at premium rate"           |
    But the substrate's Granted verdict remains the floor truth
    And no elohim signal can retroactively un-grant a substrate-granted commitment
    # Elohim CAN refuse future requests by authoring policy; cannot reverse history

  Scenario: Elohim accepts what substrate would have denied via exception
    Given the substrate would deny a compute request for capacity reasons
    When matthew's elohim authors an exception commitment with explicit rationale
    Then a Commitment with signal_kind "compute-allocation" is created
    And the commitment's negotiated_by.kind is "elohim"
    And the commitment carries the elohim_signature
    And the substrate's would-have-denied verdict is preserved as the rationale field
    # The substrate's denial is not erased — it's the public reason the elohim
    # had to make the exception explicit

  # --- Three trigger kinds ---

  Scenario Outline: Substrate handles all three trigger kinds without elohim
    Given a compute-allocation commitment exists with trigger_kind <kind>
    When the firing condition for <kind> is met
    Then the substrate fulfills the commitment deterministically
    And fulfillment is recorded with no elohim involvement required

    Examples:
      | kind            | firing condition                                   |
      | request-driven  | incoming peer request matching the commitment     |
      | standing        | scheduled time / threshold / gossip-rule triggers  |
      | subscription    | counterparty arrives within the agreed window     |

  # --- Recovery and re-entry ---

  Scenario: Replacement hardware allows breached commitments to either resume or formally retire
    Given adam's compute commitments are in breach state with cause "catastrophic-loss"
    When replacement remote-labeled hardware comes online
    And a re-bootstrap of adam's agent identity completes
    Then each prior compute-allocation commitment is either:
      | option            | result                                                |
      | resumed           | breach event closed; commitment state returns active  |
      | formally retired  | retirement event recorded; commitment state final     |
    And no commitment is silently abandoned

  Scenario: Stewardship through catastrophic loss preserves dignity
    Given adam's compute is in breach state
    When the protocol surfaces adam's status to other peers
    Then adam is presented as "compute unavailable" not "agent failed"
    And adam's authored work, recognition, and standing remain visible
    And the path to re-entry is explicit:
      | required               | source                                       |
      | replacement hardware   | community-arranged or self-funded            |
      | re-bootstrap procedure | documented in operator runbook               |
      | community attestation  | counterparties witness identity continuity   |
    # Memory: project_stewardship_philosophy.md — graduated capability, accountable
    # authority, visible shape; spans child/custody/probation/disability/elder.
    # Catastrophic compute loss is in the same family.
