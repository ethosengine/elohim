@e2e @storage @wip @wip-tier-controller @act:i
Feature: Content keeps the promises its app declared — cheap to keep, ready when wanted
  As a parent with years of family videos
  I want our videos to rest cheaply in deep storage most of the time, but play within moments when someone presses play
  So that keeping our memories doesn't crowd out the family's everyday space, and movie night never stalls

  An app declares, once, how each kind of content should be kept: family videos
  rest in the cellar and warm up on demand; build caches evaporate in minutes;
  photo albums stay close at hand. The household's own devices reconcile toward
  those declarations — and a declaration is a wish, not a command: no app can
  push content below the floor a steward has pledged to hold, and no app can
  demand more warmth than the pledge can carry.

  Background:
    Given the family app declares a "streaming-media-library" storage policy for family videos
    And that policy rests videos in deep storage with a floor of "shelved"
    And promises first playback within 2 seconds
    And keeps a video warm for 2 hours after someone watches it

  Scenario: A video rests cheaply when nobody is watching
    Given the reunion video has not been watched for over a week
    Then the video rests in deep storage on the household's cellar destinations
    And the household dashboard shows it costs almost nothing to keep

  Scenario: Movie night starts within moments
    Given the reunion video is resting in deep storage
    When Maria presses play
    Then the first moments of the video play within 2 seconds
    And the rest of the video streams in while she watches

  Scenario: A watched video stays warm for the evening
    Given Maria finished watching the reunion video
    When her cousin presses play within the next 2 hours
    Then playback starts instantly from the warm copy

  Scenario: The video cools back down after the gathering
    Given nobody has watched the reunion video for a week
    Then the video returns to deep storage on its own
    And no one in the family had to manage any of it

  Scenario: A declared wish never breaks a steward's promise
    Given a steward has pledged to keep the family's videos at least "shelved"
    When the app's policy is misconfigured to demand less than that
    Then the steward's pledge holds and the videos stay safe
    And the mismatch is surfaced to the app's steward as a gap, not hidden

  Scenario: An app cannot demand more warmth than the household pledged
    Given the family's pledge can carry 2 hours of warm video at a time
    When the app asks to keep 10 hours of video warm
    Then the household grants only what the pledge can carry
    And the app is told how much was granted
