@e2e @resilience @local @concern:blob-durability @dataplane @act:i
Feature: A household artifact remains readable after its storing peer stops
  As a household member entrusting a large artifact to consenting peers
  I want a surviving peer to recover the same bytes when the original peer stops
  So that losing one storage process does not require an operator to restore my artifact.

  The owned test mesh has three storage peers. Matthew is the ingest peer and receives the original
  artifact; Jessica and James are the designated surviving peers. This drill isolates recovery between storage peers:
  requests go directly to storage, without a doorway gateway. The test harness owns these processes and their
  test data, so it may stop and restart them. Test-integrity controls make the
  observation trustworthy: the exclusive drill lock prevents
  another fault test from changing the measured topology. Each eligible peer
  has declared its consent to hold public artifacts through an active commons
  custody commitment. The reconstruction behavior remains an open acceptance
  obligation awaiting evidence. This proves process loss on one host, not loss
  of a physical household or network site.

  The fixture uses Reed–Solomon RS-4-7 encoding: a 68 MiB artifact produces seven distinct erasure-coded shards, each 17 MiB.
  Any four shards can reconstruct the original artifact. The fixture creates
  fresh bytes and records their hash; all subsequent assertions refer to those
  same bytes. A content record is a description naming those stored bytes. Publishing
  that description to Matthew initiates shard assignment to consenting peers;
  it does not upload a second artifact. Placement is complete for this drill
  only after Matthew records their acknowledgements and local presence checks
  prove the required survivor shard distribution.

  Presence checks inspect local holdings without fetching from another peer,
  so counting shards cannot preload the recovery. Before stopping Matthew,
  these checks must show that Jessica and James together hold at least four
  distinct shards, but neither has four or a complete artifact. Matthew must
  also record Jessica and James acknowledging their assigned shards.
  The client represents the household member. Direct peer selection is test
  instrumentation; application routing is outside this storage-recovery proof. Its final read is one direct
  request to Jessica, with Matthew still down. Together, local absence of a
  complete copy, insufficient local shards, successful remote receipt and
  a four-shard local inventory after the read and matching returned bytes
  are the observable recovery proof.
  Jessica's structured transfer log must record receiving a hash-verified
  missing shard from James during this read, naming the shard and transport
  to prove that recovery crossed the peer byteplane.
  Harness lifecycle invariant: an After-scenario hook always restores Matthew
  if this drill stopped him, including when any assertion fails. That hook
  verifies storage health and releases the lock; cleanup failures fail the
  scenario. The final steps additionally exercise successful-path restoration.

  # WIP is an unfulfilled acceptance obligation. Explicit owned-mesh runs can
  # measure it with A2O_RUN_WIP=1.
  @wip @requires:owned-substrate @rs-source-loss
  Scenario: One surviving peer reconstructs fresh bytes from the surviving shard holders
    Given the harness holds the exclusive drill lock on its owned household mesh
    And Jessica and James have active commons consent commitments and are eligible for public-artifact placement
    And an artifact of 68 MiB, above the erasure-coding threshold
    And the artifact is unique to this run and its original content hash is recorded
    When the harness sends the artifact to Matthew by PUT under its recorded content hash
    Then Matthew returns HTTP 200 or 201 for the artifact
    When the harness publishes a content record to Matthew naming this artifact to request peer custody
    Then Matthew's log records Jessica and James acknowledging their assigned shards of this artifact
    And local-only non-fetching presence checks show Jessica and James together hold at least four distinct shards of its seven 17 MiB shards, fewer than four each, and no complete copy
    When the harness stops Matthew after placement is proven
    Then Matthew has no running storage process and does not answer direct storage requests
    When the client requests the artifact directly from Jessica while Matthew remains stopped
    Then the returned bytes match the original artifact hash
    And Matthew has no running storage process and does not answer direct storage requests
    And local-only checks show Jessica now holds at least four distinct shards of this artifact
    And Jessica's transfer log records successful receipt from James of a verified missing shard of this artifact, identifying the shard hash and peer transport during this read
    When the harness restores Matthew after the recovery drill
    Then all three storage peers respond successfully to their storage health checks
