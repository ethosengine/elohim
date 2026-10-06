# Design: genesis/docs/content/elohim-protocol/architecture/2026-10-05-storage-physics-benchmark.md §7
# (the retention floor for code) and §8 (declared defaults).
@e2e @delivery @app-bundle @concern:release-retention @requires:household-nodes @act:i
Feature: A peer keeps the files of a channel's latest releases and lets the files of older ones go

  Every new build of an app adds files to the peers that serve it. Until
  now nothing ever removed them: a peer that had served nineteen builds
  still held the files of all nineteen, and its disk only grew.

  This story is the bound on that. A peer keeps the files of the latest
  few builds, so it can step back to a recent one, and lets the files of
  older builds go. How many it keeps is the peer's own setting. A peer
  that holds builds on behalf of others keeps ten; this story sets one
  peer to two so the bound can be seen with three builds instead of
  eleven.

  Vocabulary the scenario leans on:

  An APP here is a web application the household serves. A BUILD of an app
  is two bundles of files: the browser bundle a visitor's browser runs and
  the server bundle a doorway uses to render the first page. A RELEASE is
  one published entry naming the exact bundles of every app it carries, and
  a RELEASE CHANNEL is the named feed releases are published on.

  A newly published release is a STAGING release: it has been published
  but nobody has yet vouched that it works. A peer that follows a channel
  AS A CANARY runs the newest staging release; trying new releases first
  is what a canary is for. In this household every peer is a canary. A
  later ceremony can promote a release so that peers which are not
  canaries run it too. This story does not need that step and does not
  take it.

  A peer's RECORD of an app is the entry that peer keeps saying which
  bundles are the app right now. An app is BOUND to a channel when its
  record names that channel: only releases on that channel can move the
  record, and an app is bound to at most one channel. Each peer's runtime watches the channels
  it follows and TAKES UP the release it should run by itself: it
  downloads the bundles, checks them, and points its own record of each
  app at them. A release a peer has
  taken up is one the peer has SEEN. A DOORWAY is the household's gateway
  to the ordinary web, and it serves whatever its peer's record for an
  app points at. So a doorway whose peer is a canary serves a staging
  release as soon as the peer has taken it up. Doorway
  "alpha", the one this story asks, is served by matthew's peer: the same
  peer that lets the first release go. That is why the last check matters.
  It shows matthew's peer still has every file it needs to serve the app.

  A peer KEEPS a release while it holds the release's bundle files. It
  keeps the latest releases it has seen on a channel, up to its setting,
  counted in the order it first saw them. A release older than that is
  LET GO: the peer deletes its copies of that release's bundles. Two things
  are never let go, however old the release that named them: files the
  peer's own record of an app still points at, and files a release it is
  keeping also names. Each peer gives its OWN ACCOUNT of this on request:
  for every channel it follows, the releases it is keeping, newest first.
  The scenario reads that account and then looks at the peer's disk as
  well, so the account is checked against the files themselves.

  The check that lets old releases go runs every five minutes. A peer may
  also have made a STANDING PROMISE: a pledge to the household to keep
  holding the bundle an app's record points at. When the record moves to
  a newer bundle, a separate five-minute check moves the promise to the
  newer bundle and withdraws it from the older one. Until then the older
  bundle is not let go. So at worst a bundle waits one check for its
  promise to be withdrawn and another to be let go. The scenario allows
  fifteen minutes to cover both with room. It does not test the promise;
  the promise is here only to explain the fifteen minutes.

  Three people share this house, and the peers they run are one
  HOUSEHOLD. matthew stewards the channel and publishes the releases.
  jessica and james run peers of their own. James's peer, like jessica's,
  keeps the default of ten; the scenario checks jessica's.

  The scenario counts three releases, the FIRST, SECOND and THIRD,
  published one after another on a channel that starts empty. Every peer
  takes each one up before the next is published, so every peer sees all
  three in the same order. With three seen and a setting of two, the
  first is the one let go.

  The second scenario is about bundles no release names. Before release
  channels existed, an app's record named its bundle directly. A new
  build was published by handing the bundle to a peer and pointing the
  app's record at it. To HAND a peer a bundle is to upload the bundle to
  that peer. Nothing kept count of the builds that came before, so
  nothing ever let them go. This story calls such a bundle an EARLIER
  BUILD: a bundle a peer still holds that no app's record points at and no
  release names.

  A peer knows how each file it holds ARRIVED, because whatever stored
  the file wrote that down. There are three ways. The peer was handed the
  file. The peer fetched the file from another peer. Or another peer
  placed the file there. A file the peer was handed or fetched is its
  OWN: it can always fetch it again, so once nothing names the file the
  peer may let it go. A file another peer placed is that peer's to take
  back, and is never let go this way. A file with no note of how it
  arrived is UNRECORDED: the peer cannot tell whose it is, so it reports
  the file and keeps it.

  A peer lets an own file go slowly, on purpose. The file must have gone
  unnamed on two checks in a row, and it must have arrived at least a day
  ago. Both are the peer's own settings. The second scenario sets
  matthew's and jessica's peers to check every fifteen seconds and to
  wait one minute instead of a day, so the rule can be seen in minutes.
  A peer's own account also says how much it holds for each reason, lists
  each own file that nothing names, and lists the files it has lately let
  go.

  The second scenario uses one more app, which this run also owns. Its
  record names its bundle directly and it is bound to no channel. Only
  matthew's peer is handed its builds. Jessica's peer comes to hold the
  first build another way: it is asked for the app's page while it holds
  no copy of the bundle, so it fetches the bundle from another peer and
  then serves the page. That makes the first build an own file on both
  peers, handed to one and fetched by the other. A standing promise can
  cover this app's bundles too, which is why the second scenario also
  allows fifteen minutes. Each peer runs its checks on its own clock, and
  jessica's peer hears that a promise was withdrawn only after matthew's
  peer has withdrawn it. So her peer may let the first build go a little
  after his does, and the scenario gives it five minutes more.

  The story runs as a test on the household's own mesh. "This run" means
  one execution of it, which creates its own two apps and its own channel
  so it never touches the household's real pages. There are two apps
  because a release usually carries more than one, and letting a release
  go must remove every app's bundles, not one app's.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And peer "matthew" at "E2E_STORAGE_MATTHEW"
    And peer "jessica" at "E2E_STORAGE_JESSICA"
    And peer "james" at "E2E_STORAGE_JAMES"
    And two apps this run owns, each bound in its own record to this run's release channel
    And every household peer follows that channel as a canary

  Scenario: A peer set to keep two releases lets the first of three go, and a peer set to keep ten does not
    Given matthew's peer is set to keep the two latest releases of each channel
    And jessica's peer keeps the default of ten
    When matthew publishes three releases of both apps, one after another, and every peer takes each up before the next is published
    Then within 15 minutes matthew's peer's own account of the channel lists the second and third releases as kept and no longer lists the first
    And the first release's bundle files, for both apps, are gone from matthew's peer
    And the second and third releases' bundle files, for both apps, are still on matthew's peer
    And the first release's bundle files, for both apps, are still on jessica's peer
    And doorway "alpha" still serves the third release's build of each app

  Scenario: A peer lets go of an earlier build that nothing names, and keeps a file it has no record of
    Given one more app this run owns, whose record names its bundle directly and which is bound to no channel
    And matthew's peer and jessica's peer are set to check every 15 seconds and to let an own file go once it has been unnamed for one minute
    And matthew's peer holds a file with no note of how it arrived
    When matthew hands his peer a first build of that app and points the app's record at it
    And jessica's peer serves that app once, fetching the first build to do so
    And matthew hands his peer a second build of that app and points the app's record at that instead
    Then within 15 minutes matthew's peer's own account has listed the first build as an own file that nothing names, and then as let go
    And the first build's bundle file is gone from matthew's peer
    And within 5 minutes more the first build's bundle file is gone from jessica's peer
    And the second build's bundle file is still on matthew's peer
    And the file with no note of how it arrived is still on matthew's peer, and his peer's own account counts it as unrecorded
    And doorway "alpha" serves the second build of that app
