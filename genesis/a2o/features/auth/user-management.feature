@e2e @auth @user-management @requires:doorway
Feature: Hosted User Management
  As Matthew, the genesis steward and admin of the alpha doorway
  I want to manage hosted visitors with accounts
  So that I can enforce quotas, suspend bad actors, and maintain my doorway

  Matthew is the only real operator. These scenarios verify that the
  admin user management endpoints work end-to-end, from listing users
  through mutations like suspend, quota updates, and permission changes.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And human "Matthew" is logged in on doorway "alpha"

  Scenario: Matthew lists hosted users
    Given human "Susan" is logged in on doorway "alpha"
    When Matthew queries the admin users list
    Then the users list should contain at least 2 entries
    And the users list should include Matthew's entry
    And the users list should include Susan's entry

  Scenario: Matthew views user details
    Given human "Susan" is logged in on doorway "alpha"
    When Matthew views user details for Susan
    Then the user details should include Susan's identifier
    And the user details should include usage stats
    And the user details should include quota limits

  Scenario: Matthew suspends a user
    When a new human "Troublemaker" registers on doorway "alpha"
    And Matthew suspends user "Troublemaker"
    Then the suspension should succeed
    When Troublemaker checks their identity
    Then the identity check should fail with unauthorized

  Scenario: Matthew updates a user's quota
    Given human "Susan" is logged in on doorway "alpha"
    When Matthew updates Susan's storage quota to 500 MB
    Then the quota update should succeed
    When Matthew views user details for Susan
    Then Susan's storage quota should be 500 MB

  Scenario: Non-admin cannot access user management
    Given human "Susan" is logged in on doorway "alpha"
    When Susan attempts to access the admin users endpoint
    Then the request should be forbidden
