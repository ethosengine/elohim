# App delivery refuses fast — the Act I (household mesh) proof that a deploy meeting a
# doorway that cannot yet take a write is told so in seconds, waits no longer than it was
# told, and goes through on its own once the doorway is ready again.
#
# Plan: genesis/docs/superpowers/plans/2026-09-24-native-delivery-sprint-plan.md (Lane C).
# Habit: genesis/orchestrator/.epr-meta/push-delivers-within-budget.habit.md.
#
# Run every scenario with:
#   just test mesh features/dataplane/app-delivery-refuses-fast.feature
# This lane sheds a doorway and restarts every conductor, so it needs exclusive use of the
# household mesh it runs on.
#
# Provenance: app builds #1719 to #1725 spent about twelve pipeline-hours and delivered
# nothing. Each one's delivery step met doorways that were still recovering from a
# restart, and waited inside the pipeline for up to two hours for them to recover. Nobody
# was told why until the pipeline gave up.
@e2e @dataplane @act:i @concern:push-delivers-within-budget
Feature: A deploy that meets a doorway not ready for it is answered in seconds, not hours

  A steward pushes a new version of an app. Somewhere between the push and a visitor, a
  deploy has to write that version into the household's peers. If the peers are not
  ready to take the write, two things matter to the person who pushed: that they are
  told so at once, with the reason, and that the deploy waits only as long as it was
  told it may. After that, it has either gone through on its own or stopped and said why.
  A deploy that waits in silence for hours holds the pipeline, delivers nothing, and
  teaches the person nothing. That is the failure this file stands against.

  VOCABULARY, because every step below rests on it.

  A DOORWAY is the gateway that browsers and deploys talk to; it serves on behalf of the
  storage peer behind it. This household runs two doorways, "alpha-A" and "elohim.host".
  Behind every storage peer runs a CONDUCTOR: the peer-to-peer runtime that signs and
  witnesses every write. The part of a conductor that does this for one app is called a
  CELL. A write that reaches a doorway whose conductor has no running cell for it cannot
  be taken, however healthy the doorway looks.

  An app is shipped as a BUNDLE (a zip of the files a browser needs), and its record
  names one bundle, by content hash, as current: its DECLARED HEAD. A DEPLOY does two
  things. First it hands the bundle's bytes to every doorway. Then it declares the new
  head exactly once, through one doorway. Only the second act needs a running cell, so
  it is the act a not-ready doorway refuses.

  A doorway is NOT READY while it answers requests but cannot take a write. The period
  after a restart during which that is true is the NOT-READY WINDOW. Every not-ready
  answer names one of three causes, called FACES:
    cell-not-running          the conductor behind the doorway has no running cell
                              for this write yet;
    catching-up               the doorway reports that it is behind and is turning
                              work away;
    storage-forward-timeout   passing the bytes on to the storage peer timed out.
  SHEDDING is how the catching-up face looks on the wire. The doorway answers HTTP 503
  with a RETRY-AFTER header ("come back in N seconds") instead of doing the work. Its
  health and administration routes keep answering while it sheds. The household can make
  one of its own doorways shed on purpose, for a stated number of seconds, through a
  control that answers only on the household's own machine. A doorway shedding on purpose
  gives the same answer a genuinely overloaded doorway gives. The control supplies the
  cause, never the answer.

  The FLEET WRITE-READINESS PROBE is the question a deploy asks before it writes: "can
  these doorways take a write right now?" It is given the doorways' addresses and asks
  each one once. It answers READY (exit status 0), or NOT-READY (exit status 3) with one
  line per doorway that cannot, naming the doorway, the face and the retry-after. Exit
  status 2 means it was called wrongly. It never waits for a window to close. Its whole
  job is to answer in seconds, so that a pipeline can decide instead of block.

  A deploy is told two budgets. Its READINESS BUDGET is how long it may keep re-offering
  the same declaration while the doorway is not ready. The clock starts at the first
  not-ready answer. Its TRANSPORT BUDGET is how long it may keep retrying ordinary
  failures such as a dropped connection. Re-offering sends the same bundle hash again,
  which is safe to repeat. A budget decides when a new attempt may START. An attempt
  already on the wire when the deadline passes is allowed to finish, and thirty seconds
  covers that attempt. The deploy prints a TIMING LINE every time it waits, in the form
  "N seconds waited, M left of B". Reading those lines back is how this story checks that
  the deploy obeyed what it was told.

  THE BOUNDS used below. Thirty seconds for any one answer from the readiness probe; a
  probe that takes longer is waiting, which is the thing it must not do. Six hundred
  seconds for the household to catch up after every conductor restarts. On the deployed
  fleet the same restart window has been measured at nearly an hour on one peer and six
  hours on another. The household holds far less, so a household still behind after ten
  minutes is stuck, and the run has measured that. Seventy-five seconds for a doorway to
  serve a newly declared version: each doorway re-reads the declared head every thirty
  seconds, so two re-reads plus fifteen seconds of slack.

  The last two scenarios publish a small site of their own: an index page, one entry
  script, one stylesheet and a BUILD STAMP (a version file naming which build made the
  bundle). They publish it at an address of its own, so no page anyone else reads is
  ever touched. Each publishes one working version first. This is the PREVIOUS
  VERSION. Then each builds a NEXT VERSION and tries to deploy it while the doorway
  sheds.

  An ACT names the substrate a scenario is measured on. Act I runs against a household
  mesh the test run owns and may write to. Act II runs against the deployed fleet, which
  it may only read. A feature file carries exactly one act tag. `@requires:owned-substrate`
  is the permission to break things on purpose: it is satisfied only on a mesh this run
  owns. Every scenario here sheds a doorway or restarts conductors, so every scenario
  carries it. On the shared fleet they are held, never run.

  Background:
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"

  # STATION 1 — the question a deploy asks first, answered in seconds while a doorway sheds.
  #
  # The probe is asked about both doorways and only one is shedding, so the answer must
  # name the right one. Before the shed it must answer ready, or a not-ready answer below
  # could be one the household already had.
  @requires:owned-substrate
  Scenario: a doorway that is shedding is named as not ready within seconds, and ready again once it stops
    Given the fleet write-readiness probe is part of this checkout
    And the fleet write-readiness probe answers ready within 30 seconds
    When the household makes doorway "alpha-A" shed every write for 120 seconds
    Then the fleet write-readiness probe answers not-ready within 30 seconds
    And the probe names doorway "alpha-A" as not ready, with the face "catching-up" and a retry-after no longer than the shed
    And the probe does not name doorway "elohim.host"
    When the household lets doorway "alpha-A" serve again
    Then the fleet write-readiness probe answers ready within 30 seconds

  # STATION 2 — a real not-ready window, caused by restarting every conductor.
  #
  # This is the window the pipeline used to wait out blindly. Here the probe is asked
  # every few seconds for the whole window, and each answer is recorded with how long it
  # took. The window counts as a window rather than a failure only if the probe reported
  # it by name while it was open, never failed or hung while answering, and it closed with
  # nobody intervening.
  @requires:owned-substrate
  Scenario: restarting every conductor opens a window the probe names, and the window closes on its own
    Given the fleet write-readiness probe is part of this checkout
    And the fleet write-readiness probe answers ready within 30 seconds
    When the household restarts every conductor while the probe keeps asking every 3 seconds
    Then within 600 seconds both doorways report they have caught up
    And the fleet write-readiness probe answers ready within 30 seconds
    And while the window was open the probe answered not-ready at least once, naming the face "cell-not-running" or "catching-up"
    And every answer the probe gave was ready or not-ready, and none took longer than 30 seconds

  # STATION 3 — a window shorter than the deploy's readiness budget: the same offer goes
  # through by itself.
  #
  # The shed lasts forty seconds and the deploy may wait sixty. The next version's bytes
  # are handed over before the shed begins; turning work away is about declarations, not
  # bytes. Only the declaration meets the window. One run of the deploy must meet the
  # window, wait it out, and go through when it closes. Nobody runs the deploy a second
  # time.
  @requires:owned-substrate
  Scenario: a deploy that meets a shorter window waits it out and goes through without being sent again
    Given a coherent EPR app bundle this run just built
    And an EPR record this run owns for it
    And each doorway is handed the bundle's bytes
    And only doorway "alpha-A" is told this bundle is the new version
    And within 75 seconds doorway "alpha-A" serves a page naming that bundle's entry script
    And this run's deploy is told it may wait at most 60 seconds for a doorway that is not ready
    When this run builds a next version of its site and hands its bytes to each doorway
    And the household makes doorway "alpha-A" shed every write for 40 seconds
    And this run tells doorway "alpha-A", while it sheds, that the next version is current
    Then the deploy met the window, waited it out, and went through without being sent again
    And doorway "alpha-A" answers with the next version as this app's declared head
    And within 75 seconds both doorways serve a page naming the next version's entry script
    And nothing in the deploy waited longer than it was told

  # STATION 4 — a window longer than the deploy's readiness budget: the deploy stops at its
  # deadline and says why.
  #
  # This is the household form of the cost bound. The shed lasts two minutes and the deploy
  # may wait twenty seconds. It must stop within thirty seconds of its deadline, name the
  # face it kept meeting, and leave the previous version as the declared head. It must not
  # half-declare the next version. The earlier pipeline stopped after hours.
  @requires:owned-substrate
  Scenario: a deploy that meets a longer window stops at its deadline, names the face, and leaves the previous version in place
    Given a coherent EPR app bundle this run just built
    And an EPR record this run owns for it
    And each doorway is handed the bundle's bytes
    And only doorway "alpha-A" is told this bundle is the new version
    And within 75 seconds doorway "alpha-A" serves a page naming that bundle's entry script
    And this run's deploy is told it may wait at most 20 seconds for a doorway that is not ready
    When this run builds a next version of its site and hands its bytes to each doorway
    And the household makes doorway "alpha-A" shed every write for 120 seconds
    And this run tells doorway "alpha-A", while it sheds, that the next version is current
    Then the deploy stopped at its deadline, saying the doorway was still not ready with the face "catching-up"
    And the deploy stopped no later than 30 seconds after its deadline
    And nothing in the deploy waited longer than it was told
    When the household lets doorway "alpha-A" serve again
    Then doorway "alpha-A" still answers with the previous version as this app's declared head
