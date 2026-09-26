# Design: genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md §12.8
# (slice 2 — the app bundle as elected content). Sprint lane: the native-delivery lane, whose name is "Lane N" (stations N1–N6).
@e2e @delivery @app-bundle @concern:app-bundle-elected-delivery @requires:household-nodes @act:i
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
  every app it carries. The channel's HEAD is the release every peer
  following it should run; once a release has been promoted, the head is
  EARNED. A release becomes earned only after a device other than the
  builder's has run it and said it worked — an ATTESTATION. A newly
  published release does not move the head: it waits as a STAGING
  candidate beneath the earned head, and only peers acting as canaries run
  it. A candidate stands only while it is newer than the head; promoting it
  makes it the head, and reverting makes an earlier release the head again,
  so after either one no candidate is left and every peer, canary or not,
  runs the head. Publish, promote and revert are CEREMONIES, and a ceremony
  is the only way anything on the channel ever moves.

  Each peer's RUNTIME (the storage service it runs) looks at the channels
  its apps are bound to once a minute and applies, itself, the release it
  should run: the earned head, or the staging candidate when the peer is a
  canary. Taking one up can span a look, a download of the bundles and the
  apply, so the take-up checks below allow five minutes. It applies a
  release to every app it carries in one database transaction, so its
  record never names one app's new build beside another app's old one; and
  if any bundle in a release cannot boot, it refuses the whole release and
  no record moves.

  An app is BOUND to a channel when its own record names that channel.
  Binding is the app's own choice, written in its own record: a release can
  only move the apps that chose its channel, and an app that chose a channel
  is moved by nothing else. A CANARY is a peer that takes up staging
  releases as well as earned ones. In this household every peer is a canary
  for this channel, so a staged release reaches all three.

  A DOORWAY is the household's gateway to the ordinary web. It is where
  matthew's build uploads its files and where a visitor asks for a page; it
  serves whatever its peer's record for the app says. A doorway re-reads
  those records every 30 seconds, which is why the serving checks below
  allow 75: two re-reads plus slack.

  The story runs as a test on the household's own mesh. "This run" means
  one execution of it, which creates its own two apps and its own channel
  so it never touches the household's real pages. The two apps are
  interchangeable; where a step says "the first app" it simply picks one.

  An app's ENTRY SCRIPT is the JavaScript file its page names to start the
  app; a bundle whose page names an entry script the bundle does not contain
  cannot BOOT — start in a browser at all. An app starts "without an error"
  when its page loads and throws no uncaught script error. The first five scenarios are
  numbered as STATIONS: stages of one journey, each taking up where the one
  before left off. The last scenario stands alone, as a guard against a
  build that cannot boot.

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
    And throughout the take-up, no peer was ever seen naming one app's new build beside the other app's old one

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
    And each household peer's runtime next looks at the channel and takes the earlier release up itself
    Then on matthew's, jessica's, and james's peers each app's record names the earlier release's bundles again
    And within 75 seconds doorway "alpha" serves each app's earlier page

  Scenario: A build that cannot start is refused by every peer, and nothing moves
    Given every household peer has taken up the channel's earned head
    When matthew publishes a build whose first app's page names an entry script its browser bundle does not contain
    Then matthew's, jessica's, and james's peers each refuse the release as a bundle that cannot boot, naming the missing file
    And no peer's record for either app moved
