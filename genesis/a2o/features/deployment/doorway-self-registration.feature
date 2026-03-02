@e2e @deployment @node-registration @requires:doorway
Feature: Doorway Self-Registration
  As Matthew, the genesis steward operating the alpha doorway
  I want my doorway to register itself as a node in the orchestrator
  So that my operator dashboard shows my node's status and capacity

  Matthew's doorway IS his node. When the orchestrator starts, the doorway
  should register itself so Matthew sees at least his own machine in the
  admin dashboard. Without this, the dashboard is empty and gives no
  feedback about the network he is stewarding.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And human "Matthew" is logged in on doorway "alpha"

  Scenario: Matthew's doorway status includes orchestrator section
    When Matthew checks doorway status
    Then the status should include an orchestrator section

  Scenario: Matthew sees his own node in the admin dashboard
    When Matthew queries the admin nodes endpoint
    Then the response should include at least 1 node
    And at least one node should have status "online"

  Scenario: Matthew's node reports real hardware capacity
    When Matthew queries the admin nodes endpoint
    Then at least one node should report cpu cores greater than 0
    And at least one node should report memory greater than 0
