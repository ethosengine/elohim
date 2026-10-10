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
  right now. A record is filed under the app's SLUG, the short name a
  visitor's address uses for it; a "bound slug" is the slug of an app bound
  to a channel (see BOUND below). An app has two halves: the BROWSER BUNDLE (the files a
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
  is moved by nothing else. Binding and following are two different
  declarations: the app's record binds the app, and a peer FOLLOWS a channel
  when that peer's own runtime is told to look at it. An app bound to a
  channel on a peer that does not follow it is still held by the channel —
  nothing else may move its record — but nobody takes releases up for it, so
  that peer keeps serving whatever bundles the record named last. A release's
  ADOPTION MEASURE asks every peer that follows the channel whether its
  records name the release, and reads "adopted" once all of them do; a peer
  that follows no channel is never asked. A CANARY is a peer that takes up staging
  releases as well as earned ones. In this household every peer is a canary
  for this channel, so a staged release reaches all three.

  Two older mechanisms still touch an app's record, and both must defer to its
  channel. Before channels existed, the build PIPELINE stamped a HEAD straight
  onto each app's record, and some records still carry that stamp beside their
  channel binding. A head is a pointer to one past version of the app's record;
  "the stamped head's own record" is the version it points to, which names the
  bundle that was current then. The channel head and a stamped head are
  different pointers: one belongs to the channel, the other to the app's past. A peer's POINTER-AUDIT SWEEP visits records on a timer and,
  when a record's bundle disagrees with the head it carries, would normally
  HEAL it by writing that head's bundle back. On a record bound to a channel
  the sweep must instead count the visit as HELD by the channel and write
  nothing: the record names its elector once, and nothing else moves it.

  A DOORWAY is the household's gateway to the ordinary web. It is where
  matthew's build uploads its files and where a visitor asks for a page.
  Each doorway reads ONE peer's records and serves whatever that peer's
  record for the app says; a household may stand several doorways, each in
  front of a different peer, and doorway "alpha" in this story reads
  matthew's. A doorway re-reads those records every 30 seconds, which is
  why the serving checks below allow 75: two re-reads plus slack.

  The story runs as a test on the household's own mesh. "This run" means
  one execution of it, which creates its own two apps and its own channel
  so it never touches the household's real pages. The two apps are
  interchangeable; where a step says "the first app" it simply picks one.

  An app's ENTRY SCRIPT is the JavaScript file its page names to start the
  app; a bundle whose page names an entry script the bundle does not contain
  cannot BOOT — start in a browser at all. An app starts "without an error"
  when its page loads and throws no uncaught script error. The first five scenarios are
  numbered as STATIONS: stages of one journey, each taking up where the one
  before left off. The sixth guards a regression: a stamped head pulling a
  released build back. The seventh guards a second regression and overrides
  the Background's healthy household to do it: a doorway serving a stale
  build because the peer it reads never joined the channel, and the peer
  joining by its own act. The eighth names a stage that is not built yet: a
  doorway's answer saying on whose authority it serves the app and where the
  bytes came from, so that a visitor (or a test) can tell a stale peer from a
  stale doorway without reading either one's insides. The last stands alone,
  as a guard against a build that cannot boot.

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

  @wip @regression
  Scenario: An app's stamped head never pulls a released build back
    # Provenance: alpha, 2026-10-07 21:07:51Z the release moved lamad-spa; 21:08:41Z the
    # sweep put the September bundle back and the adoption ledger kept reading "applied".
    Given every household peer has taken up matthew's new release
    And the first app's record still carries a head the old pipeline once stamped on it
    And that stamped head's own record names the earlier browser bundle
    When each peer's pointer-audit sweep next visits the first app's record
    Then on matthew's, jessica's, and james's peers the first app's record still names the release's browser bundle
    And each peer counts the visit as held by the app's release channel, not as a heal
    And within 75 seconds doorway "alpha" still serves the first app's page naming the new browser bundle's entry script

  @wip @regression
  Scenario: A doorway can only serve what the peer it reads has taken up, so a peer that follows no channel joins it by its own act
    # Provenance: alpha, 2026-10-08. After the stamped-head cure above, the channel became the only
    # writer of an app's record. One of the household's doorways read a peer that followed no
    # app-bundle channel, so nothing was left that could move that peer's record: that doorway served
    # a build from two weeks earlier while the other doorway served the current release.
    # (On alpha the peer was adam and the doorway elohim.host.)
    # First arc: the problem. Second arc: the cure.
    Given the household from the Background, except that jessica's peer has left the channel's followers and follows no app-bundle channel
    And a second doorway in this household reads jessica's peer for every app it serves
    When matthew publishes a new build of both apps as one release through doorway "alpha"
    And matthew's and james's peers have taken the release up
    Then jessica's record for each app still names the earlier bundles
    And jessica's pointer-audit sweep counts its visit to each app's record as held by the channel, not as a heal
    # Her apps are bound to the channel, so the sweep holds their records; her peer follows no channel, so nobody takes a release up for them.
    And the second doorway still serves each app's earlier page
    And the release's adoption measure, which asks only the peers that follow the channel, reads "adopted" with jessica's peer never asked
    When jessica's peer joins the channel as a canary by its own act, through its own admin endpoint, with no steward writing onto it
    Then on jessica's peer each app's record names the release's browser bundle and its server bundle
    And within 75 seconds the second doorway serves each app's page naming the new browser bundle's entry script

  @wip
  Scenario: A doorway names the authority it serves a bound slug under, and the provenance of the bytes
    # Not built. The scenario above could only find its stale doorway by asking every peer
    # behind it; the doorway's own answer said nothing about why it served what it served.
    # This is the missing stage between "a peer took the release up" and "a doorway serves it":
    # the doorway states the channel and release it is serving under, and which peer's record
    # and which bundle the bytes came from. The doorway only reports; the peer's record stays the
    # sole authority. The cure is deferred: the peer side that would hand the doorway those
    # facts is being changed elsewhere at the same time.
    Given every household peer has taken up matthew's new release
    When a visitor asks doorway "alpha" for the first app
    Then the answer names this run's release channel and the release the first app's record is bound to on matthew's peer
    And the answer names matthew's peer as the one whose record it read, and the release's browser bundle as the bytes it served
    And asking a doorway that reads jessica's peer instead names jessica's peer and the same channel, release and bundle

  Scenario: A build that cannot start is refused by every peer, and nothing moves
    Given every household peer has taken up the channel's earned head
    When matthew publishes a build whose first app's page names an entry script its browser bundle does not contain
    Then matthew's, jessica's, and james's peers each refuse the release as a bundle that cannot boot, naming the missing file
    And no peer's record for either app moved
