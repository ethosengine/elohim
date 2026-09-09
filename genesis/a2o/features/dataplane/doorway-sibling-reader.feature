@e2e @dataplane @local @concern:doorway-failover @act:i @requires:owned-substrate
Feature: A reader continues through a sibling when a household doorway stops answering
  A visitor who has already opened the household's site should be able to follow
  its public reading links even when that entrance stops answering. The running
  app learns about the other entrance from the household's own discovery service.
  This story covers anonymous public reads only; session and write safety have
  separate contract tests. After recovery the app must retry the original entrance.

  The prepared household has two doorways serving the same canonical package and
  real landing and manifesto content. A landing atom is the site's landing content
  opened in the app's reader. The shared declared head identifies the same published
  site package at both entrances. This story pauses one owned doorway process,
  so it proves browser content continuity through a real timeout. It does not prove
  a fresh browser can load a dead entrance, shared-name WAN routing, or session migration.
  The process is resumed even if an assertion fails. Application requests and visible
  content are the evidence; a test-side fetch or a cached title cannot pass the story.

  Background:
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"
    And every serving doorway among "alpha-A" and "elohim.host" resolves the same declared head for content "elohim-host-landing"

  Scenario: Following a public reading link survives an entrance outage and recovery
    Given an anonymous reader is viewing the landing atom through doorway "alpha-A" and has discovered doorway "elohim.host"
    When that reader's doorway stops answering
    And the reader follows the landing's manifesto link without leaving the running app
    Then the app receives the manifesto from the discovered sibling and displays its title and body
    And the sibling content request carries no session credentials
    When the original doorway recovers and the app's primary retry interval has elapsed
    And the reader returns to the landing atom within the running app
    Then the app receives the landing atom through the recovered original doorway and displays its title
