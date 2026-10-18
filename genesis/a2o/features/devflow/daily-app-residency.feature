@devflow @wip @concern:release-retention
Feature: Jessica makes room on her laptop without losing her orchard work
  Jessica tries the orchard app through her community's hub, a computer that serves it online.
  She keeps the app and selected records on her laptop so she can work in the disconnected field.
  Later she removes the local installation to free space, keeping her records and connected access.
  These stories specify the intended experience; real UI and runtime steps are not implemented yet.

  Scenario: Hosting pins the app in the background while Jessica keeps working
    Given Jessica can open her authorized orchard app through her community's hub
    And her laptop supports local execution of that app
    And the app is not installed on her laptop
    And Matthew is authorized to edit their shared harvest sheet
    When she starts a harvest entry "Morning picking" in the connected app
    Then her entry autosaves without a save action
    When she chooses "Host on this device" while editing
    Then a pin progress indicator shows background preparation of the app and current working records
    And she can continue editing and autosaving without waiting for preparation
    And offline readiness is withheld until both the app and current working records are verified locally
    When Matthew joins and edits the shared harvest sheet while pinning is incomplete
    Then they see each other's authorized changes and live presence
    When pinning completes
    Then the pin shows offline readiness independently of the live connection indicator
    And Jessica remains in the same document with her selection and edits preserved
    And her local edits no longer wait for a round trip to the hub
    When she disconnects the laptop from every network
    Then the live indicator shows she is disconnected while the completed pin remains
    And the same app continues running locally
    When she adds the private note "Check the north orchard" without a save action
    And she closes the app and restarts the laptop
    Then she can reopen the app and its retained orchard records while disconnected
    And the entry "Morning picking" and private note "Check the north orchard" are present
    And the app tells her the note is saved only on this laptop
    When she changes a shared harvest entry offline and reconnects
    Then the shared change reaches Matthew automatically
    And her private note remains outside the shared harvest sheet

  Scenario: Removing the local installation preserves local work and hub access
    Given Jessica has an inactive orchard app installed on her laptop
    And her community's hub has an authorized copy of this season's shared orchard records
    And her private note "Check the north orchard" exists only on her laptop
    And another retained app can read that note without the orchard installation
    And a second installed app still needs some executable files shared with the orchard app
    When Jessica chooses to remove the orchard app from this device
    Then only its executable files with no remaining hold are released
    And she sees the actual space freed and why shared files remain
    And her identity and private note remain on her laptop
    And the orchard app is no longer advertised as ready offline
    When she opens the orchard app through the available authorized hub without reinstalling it
    Then she can read the shared orchard records held by that hub
    And the hub does not claim to hold her local-only private note
    When the hub becomes unreachable
    Then the connected orchard app is shown as unavailable
    And Jessica can still read "Check the north orchard" in the retained app on her laptop
