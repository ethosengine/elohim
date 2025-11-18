@epic:public_observer
@user_type:community_organizer
@governance_layer:community
@related_users:citizen,parent,activist,board_member
@related_layers:municipality,neighborhood
@elohim_agents:public_observer,community_elohim,coalition_elohim

Feature: Community-Level Coalition Building for Community Organizer
  As a community organizer in the public_observer system
  Operating at the community governance layer
  I want to build collective power through hyperlocal organizing and leadership development
  So that communities discover their own agency and create sustained civic infrastructure

  Background:
    Given the Elohim Protocol is operational
    And the community_organizer user is registered in the system
    And the community governance context is active

  Scenario: Organizer detects organizing opportunity from community complaint patterns
    Given 47 residents across three blocks have reported the same landlord issues
    And most residents don't know others are affected
    When the Observer detects the shared grievance pattern
    Then the organizer sees the organizing opportunity alert
    And receives contact information for residents who opted in
    And views specific issues by building and timeline
    And gets recommended organizing approach for tenant association
    And isolated frustration becomes basis for collective action

  Scenario: Organizer creates accessible education materials for community meeting
    Given the organizer is holding meeting about proposed zoning changes
    And community members have limited policy literacy and speak multiple languages
    When the organizer requests education materials from Observer
    Then they receive one-page summary in plain language
    And get versions in Spanish, Mandarin, and Vietnamese
    And see visual infographics showing community impact
    And access deeper detail for residents who want more information
    And 47-page staff report becomes accessible to whole community

  Scenario: Organizer develops community leader for public testimony
    Given a tenant has never spoken at public meetings
    And they want to testify about rent burden at city council
    When the organizer uses Observer to support leadership development
    Then the tenant receives accessible data on local rent patterns
    And gets sample testimony framework personalized to their experience
    And sees common questions and effective response strategies
    And practices using Observer's preparation tools
    And emerges as powerful community voice not dependent on organizer

  Scenario: Organizer coordinates block club coalition on neighborhood safety
    Given five block clubs care about pedestrian safety improvements
    And members work different shifts and speak different languages
    When the organizer uses Observer for coalition coordination
    Then all members see shared information base in their languages
    And can engage asynchronously on their schedules
    And track who's committed to testify at traffic commission
    And coordinate meeting attendance across 35 residents
    And coalition functions at scale without exhausting organizer

  Scenario: Organizer tracks accountability for community park improvements
    Given the organizer led campaign that won park renovation commitment
    And city promised completion in six months
    When the Observer tracks implementation progress
    Then the organizer sees weekly updates on construction status
    And receives alerts when timeline slips
    And documents the victory for community to see their power
    And has evidence to pressure officials if implementation stalls
    And communities learn their organizing creates real change
