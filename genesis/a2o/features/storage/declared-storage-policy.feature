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

  # Traceability only: the Rule below traces to
  # genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md
  # (§5.1, §6, §7.2, §16; build order §17 slices 9 and 5).
  Rule: How much loss content must survive is a separate promise from how quickly it plays

    Both scenarios under this rule wait on two things that are not built yet: apps
    declaring how much loss their content must survive, and the placement rules
    that every one of the family's devices runs to decide where copies are held.

    Background:
      Given Maria is a parent in the family whose videos and photo albums the family app keeps
      And family videos are kept under the "streaming-media-library" storage policy: resting in deep storage no lower than "shelved", first playback within 2 seconds, and warm for 2 hours after someone watches
      And besides how quickly each kind of content plays, the family app declares how much loss it must survive: the loss of one device, or the loss of the whole home, as in a fire
      And surviving the loss of one device means copies on two of the family's own devices
      And surviving the loss of the whole home means a copy in at least one other household
      And another household holds the family's content only after someone in that household has offered to set aside room on its own devices for the family's copies
      And a household that has not made that offer is never asked to hold the family's content
      And the family app declares that family videos must survive the loss of the whole home

    @wip @concern:blob-durability
    Scenario: How much loss each kind of content must survive decides how many households hold it
      Given the family app declares that photo albums must survive the loss of any one device
      And two other households the family trusts have each offered to hold copies for Maria's family
      When Maria's household saves a new family video and a new photo album
      Then the video is held in Maria's household and in at least one of the two households that offered
      And the album is held on two of Maria's household's own devices and in no other household
      And the video still rests in deep storage, as the "streaming-media-library" storage policy says
      And resting the video in deep storage did not lower the number of copies that surviving the loss of the whole home needs

    @wip @concern:blob-durability
    Scenario: When no other household has offered, the video is kept at home and the shortfall is named
      Given the family knows other households and can reach them, but none of them has offered to hold copies for Maria's family
      When Maria's household saves a new family video
      Then the video is held on Maria's household's own devices and plays as usual
      And the household dashboard shows the video as not yet held outside the home, waiting for another household to offer
      And no other household was asked to hold it or told that the family is short
