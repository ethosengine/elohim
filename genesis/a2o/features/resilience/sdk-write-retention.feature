@resilience @sdk @wip @concern:blob-durability
Feature: Retain an application's writes until its configured endpoint accepts them
  As an application developer using the Rust content SDK
  I want failed writes to remain available for retry
  So that a temporary endpoint failure does not silently discard my work

  # Executable local proof: crates/elohim-sdk/tests/flush_retention.rs,
  # run by just gate elohim-sdk. Cucumber bindings are not implemented (@wip).
  # A group contains queued writes sharing one content type, sent in one request.
  # An ambiguous result retains the whole group, including individually successful
  # items whose identities the bulk acknowledgement does not disclose.
  # This is in-process buffering only: it promises neither restart persistence,
  # notarization nor remote custody. An uncertain response can require replay.

  Scenario: An unavailable endpoint does not consume a pending write
    Given my application has queued a content write
    When its configured peer or doorway rejects the flush or disconnects
    Then the flush reports an error and retains the write
    And a later successful flush sends the retained content

  Scenario: A successful HTTP response can still report failed content
    Given my application has queued a content write
    When its endpoint returns HTTP 200 with bulk operation errors
    Then the flush reports an error and retains the ambiguous group

  Scenario: Confirmed groups stay acknowledged across a partial flush
    Given my application has queued writes for several content types
    When one group succeeds and the next group fails
    Then retry sends only the failed and unattempted groups

  Scenario: Cancelling a flush preserves writes without a confirmed response
    Given a queued write has an unanswered request in flight
    When my application cancels the flush task
    Then that write remains available for retry

  Scenario: An old acknowledgement cannot remove a newer write
    Given an older version of an item is in flight
    When my application queues a newer version with a different priority
    And requests another flush before the first finishes
    Then the flushes send in sequence
    And the newer version remains pending until its own acknowledgement

  Scenario: Replacing queued content preserves bounded capacity
    Given the buffer is full
    When my application replaces an existing item at a different priority
    Then the replacement is accepted without duplicating the item
    And a different new item is refused with backpressure

  Scenario: An existing different payload is not an acknowledgement of my write
    Given my application queues content whose ID already exists with different bytes
    When the create-only endpoint reports that item as skipped
    Then the flush reports an error and preserves my pending content
    And when my application reads that ID it can observe the existing different content
    And the SDK still retains my pending content without automatically resolving the conflict
