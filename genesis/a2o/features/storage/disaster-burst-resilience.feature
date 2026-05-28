@e2e @storage @wip @wip-collective-steward
Feature: When disaster strikes, the wider community absorbs the load
  As a family in a region hit by a storm
  I want our church community's shared server to hold more of our memories for a while
  So that even when half our own devices are gone, the photos and records survive

  Most of the time a family's content is spread across the people they trust. But
  when disaster takes out many of those copies at once, the wider community a
  family belongs to — like their church — steps in and temporarily carries more
  of the weight, until the family is back on its feet.

  Background:
    Given the Smith family belongs to the Saint Mary's church community
    And Saint Mary's shared server has offered to help back up the families in it
    And the Smith family's memories are kept as 11 recoverable pieces spread across people they trust

  Scenario: A storm wipes out half a region and the church community steps in
    Given 8 of the Smith family's 11 pieces have gone offline
    When the network notices the family's memories are now at risk
    Then Saint Mary's shared server fetches the 8 missing pieces
    And within hours the Smith family's memories are marked "protected" again
    And the Smith family's home screen shows "your church community is helping right now"
