# Steward-space records cross from one device into another device's local view.
# The receiving device must be able to prove where each crossed record came
# from before the record can coordinate work or stand in for evidence.
#
# A VIA SLOT is a `via:<device>` field on a crossed record. It is only a label.
# CROSSING PROVENANCE is the proof behind that label: an admitted device signed
# a head, that head committed a tree, and that tree contained the original
# record. ENFORCE posture lets only proven crossed records affect coordination.
# Unproven records remain visible for diagnosis and can only deny reuse when
# they report a red gate result.
#
# These stations preserve two deliberate fail-closed asymmetries. A red gate
# result may deny reuse without trusted provenance because denial grants
# nothing. A record claiming to cross from this same device is unproven even
# when signed with this device's key, because a local writer must not manufacture
# a sibling witness.
#
# A brit attestation signature proves possession of the workspace's brit key;
# it does not prove which local process used that key. The brit scenario below
# rejects notes not signed by that key and does not claim to close the
# same-user key-possession residual.
#
# BRIT is the local build-attestation store behind gate reuse. A crossed green
# gate verdict names a brit attestation, so both halves must verify before the
# green can spare Matthew a repeated gate. A red result is deliberately
# different: even an unproven crossed red or a refused local brit note vetoes
# reuse because forcing a local gate grants no authority and is the safe side.
@e2e @devflow @concern:devices-share-one-berth @act:host @wip
Feature: Crossed steward-space records earn authority from their signed carrier
  Matthew works from more than one admitted device. Records copied between
  those devices can coordinate his leases and avoid repeating a gate only when
  the receiving device can trace them through an admitted device's signed head.
  A writable label, a self-authored substitute, or bytes served outside the
  signed tree must never acquire that authority.

  The steward space is the private coordination channel shared by Matthew's
  devices. Its leases prevent two sessions from landing, building, or deploying
  over each other. A gate checks one exact input closure before that work lands;
  reusing a verified green result avoids repeating the same expensive check.
  A closure is the content-derived identity of that exact gate input set, so an
  attestation for another closure covers different work even when its signer and
  displayed result match.
  A `via:<device>` slot is only a claimed crossing peer. Enforce posture requires the
  signed-head and tree-membership proof behind that claim before it grants a
  lease, crossing-peer identity, or reusable green result. A self-originating proof is
  never an independent sibling witness.

  In this story, the proven origin is the crossing peer whose signed head carries
  the record. It proves carriage into the steward space, not who originally
  authored the record. The proof always has three legs: verify the peer's head
  signature, verify the root that head commits, and verify record membership
  under that root.

  Brit is the local build-attestation store. A crossed green result is reusable
  only when its named brit attestation also verifies with the origin workspace's
  separate brit key and Matthew's roster binds that signer to the admitted
  device. An untrusted red still forces a local gate because that denial grants
  no authority and is the safe side. A red remains applicable when no later
  verified green causally declares that it rechecked and replaced that red.
  Brit verification proves possession of the bound brit key, not which local
  process used it; same-user process authorship remains outside this station.

  When a device reports that it read the other device's negative acknowledgement,
  it can normally close a synchronization attempt. An unproven report cannot do
  that: the missing record remains outstanding until a later signed tree contains
  it. A moor record announces that a device is present in the steward space; its
  nack-read claim says it read the other device's missing-record notice. A
  quarantined record is one the receiver has not admitted, so it stays
  outstanding rather than joining the accepted local view.

  After upgrading from records without crossing receipts, a backfill
  reconciliation can match a held gate verdict's original content address to a
  retained signed tree and reconstruct its missing proof. The held set is the
  receiver's set of records already incorporated into its local view. A planned
  fold is the steward-space daemon's ordinary fold with that held set supplied;
  backfill must inspect those held records even though it does not fetch them
  again.

  Shared coordination between real workspaces is admitted only after each
  workspace records its running binary release and enforcement capability. The
  compatibility check joins those receipts, and the berth refuses shared work
  when the running pair cannot enforce the same provenance posture.

  Folding means verifying records under the other device's signed head and then
  incorporating the accepted records into this device's local view.

  Device A and device B have serialized identifiers "device-A" and "device-B".
  Crossed records can be proven, unproven, or revoked. Proven means the signed
  carrier is distinct from the receiving device, and its head signature,
  committed root, record membership, and present roster admission all verify.
  Unproven means that proof is missing or invalid. Revoked means the proof
  identifies a carrier the roster does not presently admit. Only proven records
  grant positive authority; any red gate result still forces the safer local run.

  Background:
    Given two devices of "matthew", "A" and "B", each with a signing key and steward-space peer
    And fixture labels "A" and "B" have serialized device identifiers "device-A" and "device-B"
    And matthew's roster admits devices A and B
    And matthew's roster binds each admitted device to its workspace brit signer
    And device A enforces crossing provenance for every crossed record

  Scenario: A valid crossing can spare Matthew a repeated gate
    Given device B signs a head whose committed tree contains a green gate verdict for closure "closure-1"
    And that verdict names a brit attestation for the same green result and exact closure "closure-1"
    And the attestation is signed by device B's workspace brit key
    And no red verdict for "closure-1" remains that a later verified green has not declared rechecked and replaced
    When device A verifies device B's head signature, committed root, and verdict membership
    And device A folds the verdict from device B
    And device A verifies the named brit attestation with device B's workspace brit key
    Then the verdict resolves as proven from admitted "device-B"
    And device A may reuse the verdict for "closure-1" instead of repeating the gate

  Scenario: Valid crossed lease and inbox records coordinate Matthew's devices
    Given device B has an active steward lease naming resource "dev-integration"
    And device B signs a head whose committed tree contains that lease and an inbox note
    When device A verifies device B's head signature, committed root, and both records' membership
    And device A folds those records from admitted "device-B"
    Then device B's crossed lease holds "dev-integration" on device A
    And device A's inbox identifies "device-B" as the note's proven crossing peer

  @regression
  Scenario: Proven crossing provenance cannot rescue an invalid brit attestation
    Given device B signs a head whose committed tree contains a green gate verdict for closure "closure-1"
    But that verdict names a brit attestation with an invalid signature
    When device A verifies device B's head signature, committed root, and verdict membership
    And device A folds the verdict from device B
    And device A evaluates the named brit attestation for reuse
    Then the verdict's crossing provenance resolves as proven
    But device A refuses the named brit attestation
    And device A repeats the gate for "closure-1"

  @regression
  Scenario: A valid brit signature for another closure cannot authorize reuse
    Given device B signs a head whose committed tree contains a green gate verdict for closure "closure-1"
    But that verdict names a brit attestation signed by device B's bound workspace brit key
    And the attestation matches the signer and green result but names different input closure "closure-2"
    When device A verifies device B's head signature, committed root, and verdict membership
    And device A folds the verdict from device B
    And device A evaluates the named brit attestation for reuse
    Then device A refuses the mismatched brit attestation
    And device A repeats the gate for "closure-1"

  @regression
  Scenario: A device outside Matthew's roster cannot supply a reusable sibling verdict
    Given device C has a known signing key and a green gate verdict for closure "closure-1" under a tree committed by its verified signed head
    But device C is not admitted by matthew's present roster
    When device A verifies device C's signed tree and asks for a reusable verdict for "closure-1"
    Then device C's verdict is marked revoked on device A
    And device A receives no reusable sibling verdict for "closure-1"

  @regression
  Scenario: A forged via slot on a lease claim has no lease effect
    Given device A's local flow store contains a lease claim forged with "via:device-B"
    And no proven crossing from device B contains that claim
    When device A rebuilds its steward lease view
    Then the forged claim remains visible as unproven
    And the forged claim holds no steward-scoped resource

  @regression
  Scenario: A forged via slot on a gate verdict cannot become reusable evidence
    Given device A's local flow store contains a green gate verdict forged with "via:device-B"
    And no proven crossing from device B contains that verdict
    When device A lists peer gate verdicts for the forged verdict's closure
    Then the forged verdict remains visible as unproven
    And the peer verdict list contains no reusable green verdict for that closure

  @regression
  Scenario: A forged via slot on an inbox note grants no crossing-peer authority
    Given device A's local flow store contains an inbox note forged with "via:device-B"
    And no proven crossing from device B contains that note
    When device A reads its steward-space inbox
    Then the note remains visible as unproven
    And the inbox does not identify device B as the note's proven crossing peer

  @regression
  Scenario: A forged moor nack-read cannot make a quarantined fold complete
    Given device A is still missing a quarantined record from device B
    And a local record forged with "via:device-B" claims device B read A's missing-record notice
    And no proven crossing from device B contains that acknowledgement
    When device A folds the next signed head from device B
    Then the forged acknowledgement remains visible as unproven
    And the missing record remains outstanding despite the forged acknowledgement
    And the synchronization attempt remains incomplete
    When device B publishes the quarantined record under a later signed head
    And device A folds that later head
    Then device A fetches the quarantined record
    And device A verifies its membership under the later head's committed root
    And the record resolves as proven from device B
    And the record is no longer outstanding
    And the synchronization attempt completes

  @regression
  Scenario: A peer serving a tree outside its signed root cannot advance the recorded head
    Given device A has recorded signed head "prior-head" with committed root "prior-root" for device B
    And device B signs later head "candidate-head" with committed root "signed-root"
    But device B serves a self-consistent tree rooted at "other-root"
    When device A folds the signed head from device B
    Then the fold fails crossing-integrity verification
    And device A keeps "prior-head" as device B's folded head and "prior-root" as its committed root
    And no record from "other-root" gains proven crossing provenance

  @regression
  Scenario: A brit gate note not signed by the workspace key cannot be reused
    Given the local brit note index contains a green gate note for closure "closure-1"
    But that note is not signed by device A's workspace brit key
    When device A asks brit for reusable local evidence for "closure-1"
    Then brit refuses that note because its signature is not from device A's bound workspace brit key
    And device A runs the gate instead of reusing that note

  @regression
  Scenario: An unproven crossed red verdict keeps its veto
    Given device A has a proven green sibling verdict for closure "closure-1"
    And device A has an unproven crossed red verdict for the same closure
    And no later verified green declares that it rechecked and replaced that red verdict
    When device A asks for reusable gate evidence for "closure-1"
    Then the red verdict vetoes the proven green verdict
    And device A runs the gate locally

  Scenario: A local brit red note refused as positive evidence keeps its veto
    Given device A has a proven green sibling verdict for closure "closure-1"
    And device A's brit index has a red note for the same closure with an invalid signature
    And no later verified green declares that it rechecked and replaced that red note
    When device A asks for reusable gate evidence for "closure-1"
    Then brit does not accept the red note as positive reusable evidence
    But the red note still vetoes the proven green verdict because forcing a local gate grants nothing
    And device A runs the gate locally

  Scenario: A verified recheck can replace an earlier red verdict
    Given device A has an applicable red verdict with content address "red-cid" for closure "closure-1"
    When device B publishes a later candidate green for the same closure that names "red-cid" as the verdict it rechecked and replaces
    And device A verifies its signed-head provenance and matching brit attestation
    Then the replacement green verdict is proven
    And the earlier red no longer vetoes closure "closure-1"
    And device A may reuse the replacement green verdict

  @regression
  Scenario: A gate verdict folded before crossing receipts existed becomes proven after backfill
    Given device A holds an unproven gate verdict from device B
    And device B's retained events tree under its signed head contains the original verdict
    And device A's held set already contains that record
    When device A's daemon processes device B's retained signed head through the ordinary planned fold with the populated held set
    And the planned fold verifies the retained head signature and its committed events-tree root
    And the planned fold verifies the held verdict's membership at the same original content address
    Then the planned fold reconstructs a crossing receipt from the retained head and tree membership
    And the record resolves as proven under device B's carrying signed head
    And device A may evaluate that verdict for gate reuse

  @regression
  Scenario Outline: A via-self record signed with the local device key is not a sibling witness
    Given device A's local flow store contains a forged <record_kind> with "via:device-A"
    And device A's own key validly signs the head anchor
    And the anchor's committed tree contains the record and matches its receipt
    When device A resolves the record's crossing provenance
    Then the record resolves as unproven because its claimed origin is the local device
    And the record grants no <authority>

    Examples:
      | record_kind        | authority                  |
      | lease claim        | crossed lease effect       |
      | inbox note         | crossing-peer identity     |
      | green gate verdict | reusable green gate result |

  @requires:shem @regression
  Scenario: Joined real-workspace receipts refuse a posture-skewed compatibility station
    Given Matthew's roster admits real workspaces "ethosengine" and "shem"
    And both device homes declare enforce posture
    But workspace "shem" runs an older peer binary that treats unproven crossed records as authoritative
    And the pair requests admission for shared lease coordination on "dev-integration"
    When each workspace probes its running binary and emits a receipt with the observed release and crossing-enforcement capability
    And the compatibility check compares the two receipts before the berth admits shared coordination
    Then the compatibility station exits nonzero for workspaces "ethosengine" and "shem"
    And the station reports that workspace "shem" cannot enforce crossing provenance
    And the berth grants no cross-workspace lease on "dev-integration" until both peers enforce the declared posture
