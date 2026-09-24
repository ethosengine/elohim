# Design: genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md §12.8
# (slice 2 — the app bundle as elected content). Sprint lane: native-delivery Lane N.
@e2e @delivery @app-bundle @concern:runtime-upgrade-propagation @requires:household-nodes @act:i
Feature: A new build of an app reaches every peer by election, not by being written onto each one

  Until now, shipping a new build of an app meant the build pipeline wrote
  the new files onto every serving peer itself, one peer at a time, and
  waited for each one to be ready to take the write. When the peers were
  busy restarting, the pipeline sat and waited — for hours on the worst
  days — and delivered nothing.

  This story is the other way round. matthew, who stewards the household's
  channels, publishes a build ONCE: its files go to one doorway, and one
  release names them. Every peer then takes the release up by itself, when
  it is ready, and points its own copy of the app at the new files. Nobody
  writes onto a peer's copy of the app from outside.

  Vocabulary the scenarios lean on:

  An APP here is a web application the household serves. Each one has a
  RECORD — the entry every peer keeps that says which files are the app
  right now. An app has two halves: the BROWSER BUNDLE (the files a
  visitor's browser downloads and runs) and the SERVER BUNDLE (the files a
  doorway uses to render the first page before the browser takes over). A
  bundle's files are addressed by their content, so "the same bundle" means
  byte-for-byte the same files.

  A RELEASE CHANNEL is a named, long-lived feed of releases. Each release is
  one entry on the channel naming the exact browser and server bundles of
  every app it carries. The channel's HEAD is the release every peer should
  run. A head is either STAGING (a candidate that peers acting as canaries
  try first) or EARNED (promoted after a device other than the builder's has
  run it and said it worked — an ATTESTATION). Moving the head is a
  CEREMONY — publish, promote, or revert — and it is the only way the head
  ever moves.

  An app is BOUND to a channel when its own record names that channel.
  Binding is the app's own choice, written in its own record: a release can
  only move the apps that chose its channel, and an app that chose a channel
  is moved by nothing else. A CANARY is a peer that takes up staging
  releases as well as earned ones. In this household every peer is a canary
  for this channel, so a staged release reaches all three.

  A DOORWAY is the household's gateway to the ordinary web. It is where
  matthew's build uploads its files and where a visitor asks for a page; it
  serves whatever its peer's record for the app says.

  Three people share this house. matthew stewards the channel and runs the
  ceremonies. jessica and james run peers of their own; nobody asks either
  of them to install anything.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And peer "matthew" at "E2E_STORAGE_MATTHEW"
    And peer "jessica" at "E2E_STORAGE_JESSICA"
    And peer "james" at "E2E_STORAGE_JAMES"
    And two apps this run owns, each bound in its own record to this run's release channel
    And every household peer follows that channel as a canary
    And the channel's earned head is an earlier release of both apps

  Scenario: Station 1 — matthew publishes a new build of both apps once, as one staging release
    Given this run has built a new browser bundle and a new server bundle for each app
    When matthew publishes them as one release through doorway "alpha"
    Then the release is staged on the channel, beneath the earned head, not earned itself
    And the files reached the household through doorway "alpha" alone, and nothing was written onto any peer's app record

  Scenario: Station 2 — every peer takes the release up by itself
    Given matthew's new release is staged on the channel
    When each household peer's runtime next looks at the channel
    Then on matthew's, jessica's, and james's peers each app's record names the release's browser bundle and its server bundle
    And both apps moved together on each peer, in the same step, so no peer ever showed one app new and the other old

  Scenario: Station 3 — the doorway serves the new build to a visitor
    Given every household peer has taken up matthew's new release
    When a visitor asks doorway "alpha" for each app
    Then within 75 seconds the page each app is served names the new browser bundle's entry script
    And a browser opening the first app on doorway "alpha" starts it without an error

  Scenario: Station 4 — james's peer vouches for the release and matthew promotes it
    Given james's peer has run matthew's new release
    When james's peer attests that the release ran clean on his device
    And matthew promotes the release on that attestation
    Then the release is the earned head of the channel
    And every household peer still names the same bundles for both apps, because promotion moved no files

  Scenario: Station 5 — reverting is the same ceremony pointing backward
    Given matthew's new release is the channel's earned head
    When matthew reverts the channel to the earlier release
    Then on matthew's, jessica's, and james's peers each app's record names the earlier release's bundles again
    And within 75 seconds doorway "alpha" serves each app's earlier page

  Scenario: A build that cannot start is refused by every peer, and nothing moves
    Given every household peer has taken up the channel's earned head
    When matthew publishes a build whose first app's page names an entry script its browser bundle does not contain
    Then matthew's, jessica's, and james's peers each refuse the release as a bundle that cannot boot, naming the missing file
    And no peer's record for either app moved
