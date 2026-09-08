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
# Run every station, including the self-contained headless browser, with:
#   just test mesh features/dataplane/epr-app-deliverability.feature
# This fault-injection lane needs exclusive use of its owned household mesh.
#
# Provenance of every station below: measured 2026-09-04 and again 2026-09-08 on the
# deployed fleet. Both apex names served a page from a previous bundle era while storage
# held the new one; the entry script 404'd; every visitor got a blank page for fifteen
# hours; and the build that shipped it went UNSTABLE, which the orchestrator reads as
# success. Nothing in the pipeline objected. A person found it by looking at the page.
@e2e @dataplane @concern:doorway-failover @requires:multi-node @act:i
Feature: An app a doorway serves reaches a visitor able to run

  This is the web/Angular adapter contract, with SSR only when declared; native and Wasm
  apps do not inherit browser bootstrap markers or server rendering requirements.

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
  page leaves for the framework to fill — has visible content and data-app-ready="true"
  afterwards. Only browser bootstrap writes that marker: server-rendered text alone
  cannot prove the JavaScript ran. An empty root or missing marker fails the visit.
  Each browser visit has its own forty-five-second completion deadline. A page that
  merely answers 200 has proved that the door opened, not that anyone got in.

  A bundle is COHERENT when the files its index page names are all present in it. A
  bundle missing its entry script is INCOHERENT: it will answer 200 and boot nothing.

  THE BOUND, used by the convergence stations below: seventy-five seconds. Each doorway
  re-reads the head its peer declares on a fixed tick — BUNDLE_HEADS_TICK_SECS, thirty
  seconds (spec D1) — so two ticks is sixty, and fifteen seconds of slack covers the read
  itself. Readiness includes the page and its entry script and stylesheet answering
  through the same doorway; the browser starts after those routes converge together.
  Both doorway readiness checks share one clock, starting when browser publication
  confirms its declaration; the second doorway gets no extra seventy-five seconds.
  Recovery instead starts at the first successful health response from the restored
  storage peer. Server-pointer propagation starts when server publication confirms
  its declaration. Initial renderer adoption starts when configuration completes;
  a running renderer's 330-second upgrade clock starts at the next server declaration.
  During an upgrade both heads are published back-to-back. Browser declarations and
  immutable bytes must arrive within the browser's seventy-five-second bound; the
  public rendered page must match the pair after server adoption within 330 seconds.
  A convergence that needs longer than that is not slow, it is stuck: nothing in
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
  @requires:owned-substrate
  @deliverability-browser
  Scenario: a steward publishes through one door and a visitor arrives able to run at either
    Given a coherent EPR app bundle this run just built for the root address
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
    And the browser on peer "alpha-A" completed client bootstrap
    And every asset the browser asked peer "alpha-A" for arrived
    And the app root on the page from peer "alpha-A" has content
    And the build stamp peer "alpha-A" serves for this app is the one the published bundle carries
    When a visitor opens the app this run published on peer "elohim.host" in a browser
    Then the browser on peer "elohim.host" reported no uncaught error
    And the browser on peer "elohim.host" completed client bootstrap
    And every asset the browser asked peer "elohim.host" for arrived
    And the app root on the page from peer "elohim.host" has content
    And the build stamp peer "elohim.host" serves for this app is the one the published bundle carries

  # The first station borrows the unclaimed / mount on both owned doorways, with
  # base href /. Its visits therefore request the real /version.json route,
  # catching service-prefix collisions that a nested mount cannot expose.
  # An existing root projection/commitment refuses setup, never gets replaced.
  # After success or failure, cleanup explicitly cancels its own commitments through
  # the existing canonical API on every owned peer and checks each returned state.
  # This cleanup is not a test of revocation gossip. After cancellation completes,
  # both root projections must disappear within 45 seconds; cleanup failure fails.

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
    When only doorway "alpha-A" is told this bundle is the new version
    And only doorway "alpha-A" is told this bundle is the new server-rendered version
    Then within 75 seconds every household peer answers with the same server pointer for this app
    And every household peer serves the declared server bundle bytes by their content address
    # The negative is observed, not assumed: this run records every declaration it issues
    # and the last line asserts exactly one server declaration, with every head write through the same door.
    And no peer other than the one that was told was written to by this run

  # STATION 2b — the server-rendered leg has TWO trips, and each is its own proof.
  #
  # Station 2 proves peer-to-peer: the server pointer a steward declared on one peer is the
  # pointer every peer answers with. That says nothing about the doorway in front of those
  # peers: a doorway renders pages from a server bundle it MATERIALIZED, and it materializes
  # only the slugs it is configured to render, on its own tick, after its peer declared the
  # pointer. The second trip — peer to doorway — is what a visitor actually receives.
  #
  # A doorway attests its adopted server bundle on /health/startup. This run mounts
  # its own slug at /lamad/concept/<run-slug>, an existing manifest SSR route,
  # and adds it to the renderer configuration, then requires the stamp
  # in raw HTML before any JavaScript runs. After warming both doorways it holds
  # every storage peer down and verifies each refuses health connections: three requests
  # per doorway, all six within ten seconds, must return cache HIT and that
  # rendered version. Rendering and caching belong to the doorway; visitor volume
  # must not become per-request rendering work on peers. Peers hold and converge
  # the declared identity and bundle bytes. Public anonymous output is the scope
  # here; request-dependent/private SSR needs its own cache-isolation contract.
  @requires:owned-substrate
  Scenario: the doorway in front of the peers renders the server version every peer agreed on
    Given a coherent EPR app bundle this run just built
    And an EPR record this run owns for it
    When each doorway is handed the bundle's bytes
    And only doorway "alpha-A" is told this bundle is the new version
    And only doorway "alpha-A" is told this bundle is the new server-rendered version
    And within 75 seconds every household peer answers with the same server pointer for this app
    And every household peer serves the declared server bundle bytes by their content address
    And both doorways are configured to render this run-owned site
    Then within 75 seconds both doorways attest they materialized that server pointer for this app
    And both doorways return that server-rendered build before any browser script runs
    # A running renderer must replace N with N+1, without another restart.
    # Its normal adoption tick is 300 seconds; thirty seconds covers the fetch.
    When this run builds a next coherent browser and server version
    And each doorway is handed the bundle's bytes
    And only doorway "alpha-A" is told this bundle is the new version
    And only doorway "alpha-A" is told this bundle is the new server-rendered version
    # Browser bytes converge independently; public SSR waits for the paired server.
    And within 75 seconds both doorways serve the new browser bundle by content address
    Then within 75 seconds every household peer answers with the same server pointer for this app
    And every household peer serves the declared server bundle bytes by their content address
    And within 330 seconds both doorways attest they materialized that server pointer for this app
    And both doorways return that server-rendered build before any browser script runs
    And neither doorway restarted while adopting the next rendered version
    When a visitor opens the app this run published on peer "alpha-A" in a browser
    Then the browser on peer "alpha-A" reported no uncaught error
    And the browser on peer "alpha-A" completed client bootstrap
    And every asset the browser asked peer "alpha-A" for arrived
    And the app root on the page from peer "alpha-A" has content
    And the build stamp peer "alpha-A" serves for this app is the one the published bundle carries
    When a visitor opens the app this run published on peer "elohim.host" in a browser
    Then the browser on peer "elohim.host" reported no uncaught error
    And the browser on peer "elohim.host" completed client bootstrap
    And every asset the browser asked peer "elohim.host" for arrived
    And the app root on the page from peer "elohim.host" has content
    And the build stamp peer "elohim.host" serves for this app is the one the published bundle carries
    And both warm doorways keep serving that rendered build while all storage peers are down

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
  @deliverability-restart
  Scenario: a doorway that restarts while its peer is down catches up on its own
    Given doorway "elohim.host" can be restarted while peer "jessica" is held down
    And a coherent EPR app bundle this run just built
    And an EPR record this run owns for it
    When each doorway is handed the bundle's bytes
    And only doorway "alpha-A" is told this bundle is the new version
    # First Jessica must carry the new head within the publication bound.
    # Before restoring it, witness the restarted doorway logging its failed
    # initial lookup for this exact app while Jessica remains unreachable.
    And doorway "elohim.host" restarts while peer "jessica" is down
    And peer "jessica" comes back
    Then within 75 seconds doorway "elohim.host" serves a page naming that bundle's entry script
    And every script and stylesheet the page from peer "elohim.host" names is one that peer serves
    When a visitor opens the app this run published on peer "elohim.host" in a browser
    Then the browser on peer "elohim.host" reported no uncaught error
    And the browser on peer "elohim.host" completed client bootstrap
    And every asset the browser asked peer "elohim.host" for arrived
    And the app root on the page from peer "elohim.host" has content
    And the build stamp peer "elohim.host" serves for this app is the one the published bundle carries
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
  # the SDK packager checks the files locally before uploading any bytes or minting a head.
  # Storage also judges extracted bundles, so an older publisher cannot avoid the peer's check.
  # Second, if such a head is declared anyway — by an older peer, by a hand, by any route
  # that skipped the judgement — the visitor is still never handed a blank page: either
  # the previous version actually boots, or a 503 with Retry-After asks them to return.
  # Having published a good version does not guarantee a usable local fallback remains.
  # In both cases the doorway says it is behind and names the missing file. A blank 200 is never
  # acceptable, because it is the only one a person cannot act on.
  #
  # THE SUB-CASE THIS STATION DOES NOT PROVE, named so its absence is a decision rather
  # than an oversight: a site whose FIRST published version is incoherent has no last
  # working version to fall back to. This station always publishes a working version
  # first, even though it permits an honest 503 if that fallback cannot be served.
  # First-publication failure needs a separate station and has not been written.
  #
  # The run-owned site has a public mount, so this exercises the warm-shell
  # decision used by deployed sites, including its diagnostic response header.
  @requires:owned-substrate
  @deliverability-refusal
  Scenario: a bundle that cannot boot is refused, and never reaches a visitor as a blank page
    Given a coherent EPR app bundle this run just built
    And an EPR record this run owns for it
    When each doorway is handed the bundle's bytes
    And only doorway "alpha-A" is told this bundle is the new version
    And within 75 seconds doorway "alpha-A" serves a page naming that bundle's entry script
    And this run builds a second bundle with its entry script removed
    And the steward tries to publish the second bundle through doorway "alpha-A"
    Then the deploy path refuses it and names the missing file
    # The negative test deliberately uploads an unchecked archive and declares it,
    # exactly as an older publisher could. Normal developer packaging cannot do this.
    When the incoherent bundle is declared the new version anyway
    # Within 75 seconds of forced declaration, this missing-file header proves
    # the doorway judged the broken head; storage must still declare it.
    Then doorway "alpha-A" says on the wire that it is behind and names the missing file
    # Accept previous-build browser bootstrap within 45 seconds, or 503 + Retry-After.
    And a visitor asking doorway "alpha-A" for this app is never handed a blank page
    # "On the wire" is the x-elohim-bundle response header, whose value reads
    # `behind;missing-asset:<file>` — machine-readable, and present on every response so a
    # monitor sees it without anyone opening the page.
