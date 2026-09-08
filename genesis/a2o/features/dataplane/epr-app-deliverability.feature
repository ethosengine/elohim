# EPR-app deliverability through the doorway — the Act I (household mesh) half of the
# browser-shell clause of doorway-failover. DOORWAY-FAILOVER is the standing promise that
# whichever of the two doors a visitor reaches, they get the same working site; its
# BROWSER-SHELL CLAUSE is the part of that promise about the HTML and JavaScript a browser
# downloads and runs, as distinct from the copy a doorway renders for it server-side.
# Its Act II sibling,
# features/dataplane/served-shell-boots.feature, asks the SAME question of the deployed
# fleet and may only read it. This file may write, so it can stage the whole deploy —
# build a bundle, hand it to the peers, declare it, and then watch a visitor arrive.
#
# Spec: genesis/docs/superpowers/specs/2026-09-08-epr-app-deliverability-through-doorway.md
# (decisions D1–D4). Habit: doorway/doorway-service/.epr-meta/doorway-failover.habit.md.
#
# HOW TO RUN THIS FILE — it takes TWO lanes, and neither one covers the other:
#   just test mesh         features/dataplane/epr-app-deliverability.feature
#   just test mesh-browser features/dataplane/epr-app-deliverability.feature
# The `mesh` cucumber profile excludes @browser-only by construction, so the first
# command silently omits the one scenario that opens a browser. Run both, or the
# station that proves an app actually STARTS never ran.
#
# Provenance of every station below: measured 2026-09-04 and again 2026-09-08 on the
# deployed fleet. Both apex names served a page from a previous bundle era while storage
# held the new one; the entry script 404'd; every visitor got a blank page for fifteen
# hours; and the build that shipped it went UNSTABLE, which the orchestrator reads as
# success. Nothing in the pipeline objected. A person found it by looking at the page.
@e2e @dataplane @concern:doorway-failover @requires:multi-node @act:i
Feature: An app a doorway serves reaches a visitor able to run

  A steward publishes a new version of a site. Some minutes later a visitor types the
  address, and whichever door they happen to reach hands them the new version — and it
  works. That is the whole promise. Everything in this file exists because each link in
  that sentence has broken on its own at least once, silently, with every health check
  still green.

  VOCABULARY, because every assertion below rests on it.

  A DOORWAY is the gateway a browser talks to; it serves pages on behalf of a peer that
  holds them. This mesh runs two — "alpha-A", whose storage peer is matthew, and
  "elohim.host", whose storage peer is jessica — so that a visitor who reaches either one
  gets the same site. They are two front doors to one household, and the household holds
  a third peer, james, that serves no door.

  An EPR — Elohim Protocol Resource — is a named, content-addressed app bundle: a zip of
  the files a browser needs (an index page, an entry script, a stylesheet, a build stamp),
  reachable at its own address on any doorway that holds it. The BUILD STAMP is one file
  inside that zip, version.json, recording which build produced it. Its DECLARED BROWSER
  HEAD is the one bundle hash its record names as current — the code a browser is supposed
  to download and run; the record calls that field blobHash, and names a second bundle
  under serverBlobHash (Station 2). BYTES and DECLARATION are separate acts: handing a peer
  the zip puts the bytes within reach; declaring the head is what makes those bytes the
  answer. A deploy does the first to every peer and the second exactly once, and every
  failure below lives in the gap.

  A page BOOTS when a browser that loaded it reports no uncaught error, every file it
  asked that doorway for was answered, and the app's root element — the one empty tag the
  page leaves for the framework to fill — has content in it afterwards. An empty root
  element after load is the blank page, in the form a person actually sees. A page that
  merely answers 200 has proved that the door opened, not that anyone got in.

  A bundle is COHERENT when the files its index page names are all present in it. A
  bundle missing its entry script is INCOHERENT: it will answer 200 and boot nothing.

  THE BOUND, used by three of the four stations below: seventy-five seconds. Each doorway
  re-reads the head its peer declares on a fixed tick — BUNDLE_HEADS_TICK_SECS, thirty
  seconds (spec D1) — so two ticks is sixty, and fifteen seconds of slack covers the read
  itself. A convergence that needs longer than that is not slow, it is stuck: nothing in
  this path waits on a human, and the doorway that has not caught up by then is not going
  to without one.

  ONE PIECE OF TEST VOCABULARY, because the tag line uses it: an ACT names the substrate a
  scenario is measured on. Act I runs against a household mesh the test run OWNS and may
  write to; Act II runs against the deployed fleet, which it may only read. A feature file
  carries exactly one act tag.

  THE VEHICLE IS A SITE THIS RUN PUBLISHES, and that is a protocol fact rather than a test
  shortcut. The mesh's own landing page belongs to the household; overwriting its declared
  head with a fixture bundle would deface the page every other scenario in this lane reads.
  So each station publishes its own small site — an index page, one entry script, one
  stylesheet, one build stamp — and a visitor arrives at that site's own address. The
  question being asked is about the deploy path and the doorways, and it does not care
  which site travels it.

  TWO WORDS FOR ADDRESSES, because the steps below use both. In prose, a DOORWAY is a
  gateway and a PEER is a storage node, and the difference is load-bearing in Stations 2
  and 3. In STEP language, `peer "X" at "Y"` is the generic "register an addressable
  endpoint" step every dataplane story shares, and the Background uses it to register the
  two DOORWAYS. So `peer "alpha-A"` and `doorway "alpha-A"` below name the same front
  door; where a step means a storage node it names that node — matthew, jessica, james.

  THIS RUN means the currently executing test, which publishes its own site rather than
  writing to anyone else's. The `@requires:` tags are preconditions the runner evaluates
  before it starts a scenario; they gate whether a scenario runs, never what it asserts.
  `@requires:owned-substrate` is the permission to WRITE: it is satisfied only on a mesh
  this run owns, so on the shared fleet these stations are held rather than run. Every
  station here publishes a site and declares a head, so every station carries it.

  Background:
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"
    And the household's storage peers are "matthew" behind doorway "alpha-A", "jessica" behind doorway "elohim.host", and "james" behind no doorway

  # STATION 1 — the whole promise, end to end, once.
  #
  # THE FAILURE THIS STANDS AGAINST: on 2026-09-04 the bytes reached both doorways and the
  # head was declared, and one doorway went on serving the previous version anyway. Its
  # cached page was filed under the head its own PROJECTION knew — the doorway's local view
  # of which head is current, kept separately from the peer's record — and that projection
  # never moved, so the stale page classified as current forever. The visitor's browser
  # downloaded a page that named a script the doorway no longer held.
  #
  # DECLARED THROUGH ONE DOOR ON PURPOSE. A deploy declares the head exactly once, through
  # whichever doorway can reach a live conductor — the peer-to-peer runtime that actually
  # signs and witnesses the declaration, and which not every doorway can reach. The other
  # doorway is told nothing and has to find out. Declaring through both would hide the
  # entire defect.
  @browser-only @requires:owned-substrate
  Scenario: a steward publishes through one door and a visitor arrives able to run at either
    Given a coherent EPR app bundle this run just built
    And an EPR record this run owns for it
    When each doorway is handed the bundle's bytes
    And only doorway "alpha-A" is told this bundle is the new version
    Then within 75 seconds doorway "alpha-A" serves a page naming that bundle's entry script
    And within 75 seconds doorway "elohim.host" serves a page naming that bundle's entry script
    # The static clause: what the page NAMES, on each door, is held by that same door.
    And every script and stylesheet the page from peer "alpha-A" names is one that peer serves
    And every script and stylesheet the page from peer "elohim.host" names is one that peer serves
    # The dynamic clause: a browser opens each page and the app actually starts.
    When a visitor opens the app this run published on peer "alpha-A" in a browser
    Then the browser on peer "alpha-A" reported no uncaught error
    And every asset the browser asked peer "alpha-A" for arrived
    And the app root on the page from peer "alpha-A" has content
    And the build stamp peer "alpha-A" serves for this app is the one the published bundle carries
    When a visitor opens the app this run published on peer "elohim.host" in a browser
    Then the browser on peer "elohim.host" reported no uncaught error
    And every asset the browser asked peer "elohim.host" for arrived
    And the app root on the page from peer "elohim.host" has content
    And the build stamp peer "elohim.host" serves for this app is the one the published bundle carries

  # STATION 2 — the server-rendered half of the same record, which travelled by a different
  # road and never arrived.
  #
  # A site has two bundles: the BROWSER bundle a person downloads, and the SERVER bundle a
  # doorway runs to render the first page before that download finishes. The record names
  # both — blobHash and serverBlobHash. The browser pointer has propagated peer to peer for
  # months; the server pointer was written straight into the authoring peer's own database
  # and told nobody, so on 2026-09-08 elohim.host's renderer sat in a loop reporting that
  # the head it needed was absent, while the peer next to it had held that head all along.
  #
  # This station asks only for equality, because equality is the whole claim: after ONE
  # declaration on ONE peer, every peer's record answers with the same server pointer. No
  # peer is re-uploaded to, no second declaration is made, and nothing here is a browser
  # question — which is why this scenario runs in the plain mesh lane.
  #
  # EVERY HOUSEHOLD PEER means all three — matthew, jessica AND james — not just the two
  # behind doorways. james is the peer with no front door, and it is the sharpest witness
  # here: a pointer that reached it can only have travelled peer to peer, because no deploy
  # and no doorway ever addressed it.
  @requires:owned-substrate
  Scenario: a server pointer declared on one peer is the pointer every peer answers with
    Given a coherent EPR app bundle this run just built
    And an EPR record this run owns for it
    When only doorway "alpha-A" is told this bundle is the new server-rendered version
    Then within 75 seconds every household peer answers with the same server pointer for this app
    # The negative is observed, not assumed: this run records every declaration it issues
    # and the last line asserts that ledger holds exactly one.
    And no peer other than the one that was told was written to by this run

  # STATION 2b — the server-rendered leg has TWO trips, and each is its own proof.
  #
  # Station 2 proves peer-to-peer: the server pointer a steward declared on one peer is the
  # pointer every peer answers with. That says nothing about the doorway in front of those
  # peers: a doorway renders pages from a server bundle it MATERIALIZED, and it materializes
  # only the slugs it is configured to render, on its own tick, after its peer declared the
  # pointer. The second trip — peer to doorway — is what a visitor actually receives.
  #
  # A doorway attests what it has materialized on its health surface (servedBundleHeads);
  # the Act II story served-projected-head.feature compares that attestation to the declared
  # pointer on the deployed fleet. This run cannot yet make the same comparison on the
  # household mesh for a bundle it owns: the renderer materializes configured slugs only, and
  # a run-owned slug is not one. Until the mesh can mount a run-owned slug as a rendered
  # site, this station stays pending — measured nowhere is said plainly, never shown green.
  @requires:owned-substrate
  Scenario: the doorway in front of the peers renders the server version every peer agreed on
    Given a coherent EPR app bundle this run just built
    And an EPR record this run owns for it
    When only doorway "alpha-A" is told this bundle is the new server-rendered version
    And within 75 seconds every household peer answers with the same server pointer for this app
    Then within 75 seconds both doorways attest they materialized that server pointer for this app

  # STATION 3 — the doorway that comes back before its peer does.
  #
  # A doorway learns the current head two ways: it asks its storage peer at boot, and it is
  # told by that peer when something changes. Both are one-shot. A doorway that boots while
  # its peer is unreachable gets nothing from the first, and if the peer's notification was
  # already sent, nothing from the second either — so it comes up holding no head at all and
  # stays that way, serving whatever it had, indefinitely. That is not a hypothetical: it is
  # how a doorway ends up an hour behind with every health check green.
  #
  # The cure being asserted is that the doorway keeps asking. Nobody clears a cache, nobody
  # restarts anything a second time, nobody notices. It converges on the tick, or it is
  # broken.
  @requires:owned-substrate
  Scenario: a doorway that restarts while its peer is down catches up on its own
    Given doorway "elohim.host" can be restarted while peer "jessica" is held down
    And a coherent EPR app bundle this run just built
    And an EPR record this run owns for it
    When each doorway is handed the bundle's bytes
    And only doorway "alpha-A" is told this bundle is the new version
    And doorway "elohim.host" restarts while peer "jessica" is down
    And peer "jessica" comes back
    Then within 75 seconds doorway "elohim.host" serves a page naming that bundle's entry script
    # Structural, not observed after the fact: this run never calls a cache-clear route and
    # never restarts anything twice, so the last line asserts a property of its own script.
    And nobody cleared a cache or restarted anything to make that happen

  # STATION 4 — the bundle that cannot boot, refused at the door and harmless past it.
  #
  # An incoherent bundle is not a hypothetical either: the 2026-09-04 outage WAS one, from
  # the visitor's side. The published bundle named an entry script that nothing held, and
  # every person who arrived got a white page with no explanation, for fifteen hours, while
  # every machine in the path reported success.
  #
  # Two things must be true, and they are different things. First, the deploy path itself
  # refuses to declare a bundle it can see is incoherent, and says which file is missing —
  # the peer judges the bytes when it first unpacks them, before any head is minted.
  # Second, if such a head is declared anyway — by an older peer, by a hand, by any route
  # that skipped the judgement — the visitor is still never handed a blank page: they get
  # the last version that worked, with the doorway saying on the wire that it is behind and
  # naming the file that is missing. A blank 200 is the one response that is never
  # acceptable, because it is the only one a person cannot act on.
  #
  # THE SUB-CASE THIS STATION DOES NOT PROVE, named so its absence is a decision rather
  # than an oversight: a site whose FIRST published version is incoherent has no last
  # working version to fall back to, and the promise there is a "still converging" answer
  # that tells the visitor when to come back. This station always publishes a working
  # version first, so it never reaches that branch. It is a station of its own and has not
  # been written.
  #
  # AN OPEN SEAM THIS STATION WILL EXPOSE, named here so a red is read correctly. Two more
  # terms, needed only for this paragraph. A PUBLIC MOUNT is a short path a doorway binds to
  # a site so a person can just type it — "/" for the landing page, "/lamad" for the learning
  # app — as opposed to the site's own long address, which every published site has. The WARM
  # SHELL is the copy of a mounted site's index page the doorway keeps in memory so it can
  # answer instantly, and it is the code path that classifies a head as current or behind.
  # A site this run publishes has no public mount, only its own address. The "behind, and
  # here is the missing file" answer is written by the warm-shell path. So if
  # this station reds on the last line while the one above it passes, the finding is not
  # "the doorway lied" — it is that the never-blank promise is currently kept only for
  # sites with a public mount, and a site reached at its own address is outside it. That is
  # a decision about scope, and it belongs to whoever owns the doorway, not to this file.
  @requires:owned-substrate
  Scenario: a bundle that cannot boot is refused, and never reaches a visitor as a blank page
    Given a coherent EPR app bundle this run just built
    And an EPR record this run owns for it
    When each doorway is handed the bundle's bytes
    And only doorway "alpha-A" is told this bundle is the new version
    And this run builds a second bundle with its entry script removed
    And the steward tries to publish the second bundle through doorway "alpha-A"
    Then the deploy path refuses it and names the missing file
    # The head is now forced past that judgement, exactly as a peer predating it would.
    When the incoherent bundle is declared the new version anyway
    Then a visitor asking doorway "alpha-A" for this app is never handed a blank page
    # "On the wire" is the x-elohim-bundle response header, whose value reads
    # `behind;missing-asset:<file>` — machine-readable, and present on every response so a
    # monitor sees it without anyone opening the page.
    And doorway "alpha-A" says on the wire that it is behind and names the missing file
