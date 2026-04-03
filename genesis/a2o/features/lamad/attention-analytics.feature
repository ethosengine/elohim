@lamad @attention @analytics
Feature: Protocol-Native Attention Analytics
  As a learner on the Elohim Protocol
  I want my content interactions recorded as economic events
  So that attention flows to contributors through the protocol, not to Google

  Background:
    Given a learner "Maya" is authenticated
    And content node "concept-trust" exists with steward "Genesis Collective"

  # --- Attention Event Recording ---

  Scenario: Content view generates an economic event after dwell threshold
    When Maya navigates to content "concept-trust"
    And Maya remains on the content for 3 seconds
    Then an economic event of type "content-view" is recorded
    And the event provider is Maya's agent ID
    And the event receiver is "concept-trust"
    And the event action is "use" with resource type "attention"

  Scenario: Bounce view does not generate an economic event
    When Maya navigates to content "concept-trust"
    And Maya navigates away within 2 seconds
    Then no "content-view" economic event is recorded for "concept-trust"

  Scenario: Duplicate views within session are deduplicated
    When Maya views content "concept-trust" for 5 seconds
    And Maya navigates away
    And Maya returns to content "concept-trust"
    Then only one "content-view" economic event exists for this session

  # --- Session Lifecycle ---

  Scenario: Session start event on app initialization
    When the application initializes for Maya
    Then a "session-start" economic event is recorded
    And the event action is "use" with resource type "attention"

  Scenario: Session end event on tab close
    Given Maya has an active session
    When Maya closes the browser tab
    Then a "session-end" economic event is recorded
    And the event includes session duration in minutes

  # --- Learner Attention Dashboard ---

  Scenario: Learner sees their attention flow
    Given Maya has viewed 5 content nodes this week
    When Maya navigates to "/lamad/attention"
    Then Maya sees a list of content she engaged with
    And each entry shows the content title and time spent
    And the total session time is displayed

  # --- Steward Analytics ---

  Scenario: Steward sees content engagement metrics
    Given content "concept-trust" has 42 views and 8 completions
    When Maya views the Network tab for "concept-trust"
    Then Maya sees "42 views" and "8 completions"
    And Maya sees the completion rate as "19%"

  # --- GA Removal ---

  Scenario: No external analytics scripts loaded
    When the application loads in production
    Then no Google Analytics script is present in the DOM
    And no requests are made to googletagmanager.com
