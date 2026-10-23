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

  A STEWARD is the person who publishes and keeps an EPR; where this file says the content's
  owner, it means its steward. A steward publishes a new version of a site. Some minutes later a visitor types the
  address, and whichever door they happen to reach hands them the new version — and it
  works. That is the whole promise. Everything in this file exists because each link in
  that sentence has broken on its own at least once, silently, with every health check
  still green. One clause rides inside "it works": a page that runs but does not say what
  it is, or who holds it, has left the visitor unable to judge what they are looking at,
  and Station 5 holds the doorway to that.

  VOCABULARY, because every assertion below rests on it.

  A DOORWAY is the gateway a browser talks to; it serves pages on behalf of a peer that
  holds them. This mesh runs two — "alpha-A", whose storage peer is matthew, and
  "elohim.host", whose storage peer is jessica — so that a visitor who reaches either one
  gets the same site. They are two front doors to one household, and the household holds
  a third peer, james, that serves no door. A HOUSEHOLD is the protocol's unit of holding:
  a home's set of peers. This mesh is one household; the protocol's network is many.

  An EPR — Elohim Protocol Resource — is a named, content-addressed app bundle: a zip of
  the files a browser needs (an index page, an entry script, a stylesheet, a build stamp),
  reachable at its own address on any doorway that holds it. The BUILD STAMP is one file
  inside that zip, version.json, recording which build produced it. Its DECLARED BROWSER
  HEAD is the one bundle hash its record names as current — the code a browser is supposed
  to download and run; the record calls that field blobHash, and names a second bundle
  under serverBlobHash (Station 2). BYTES and DECLARATION are separate acts: handing a peer
  the zip puts the bytes within reach; declaring the head is what makes those bytes the
  answer. A deploy does the first to every peer and the second exactly once, and every
  failure below lives in the gap. In the steps, "each doorway is handed the bundle's bytes"
  is the first act, and "only doorway X is told this bundle is the new version" is the
  second; telling a doorway also hands that one doorway's peer the bytes, if it does not hold
  them yet, so a station that skips the separate hand-off still has bytes on that one peer
  and nowhere else.

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
  Both doorway readiness checks share one clock, starting when the step that tells a
  doorway "this bundle is the new version" has had that declaration confirmed; the second
  doorway gets no extra seventy-five seconds.
  Recovery instead starts at the first successful health response from the restored
  storage peer. Server-pointer propagation starts when the step that tells a doorway "this
  bundle is the new server-rendered version" has had that declaration confirmed. Initial
  renderer adoption starts when configuration completes and uses the same seventy-five-second
  bound.
  THE UPGRADE BOUND is separate: a renderer that is already running has 330 seconds to adopt a
  newer server version (its adoption tick is 300 seconds, plus thirty for the fetch), counted
  from the next server declaration.
  During an upgrade both heads are published back-to-back. Browser declarations and
  immutable bytes must arrive within the browser's seventy-five-second bound; the
  public rendered page must match the declared browser and server bundles after server
  adoption within 330 seconds.
  A convergence that needs longer than that is not slow, it is stuck: nothing in
  this path waits on a human, and the doorway that has not caught up by then is not going
  to without one.

  ONE PIECE OF TEST VOCABULARY, because the tag line uses it: an ACT names the substrate a
  scenario is measured on. Act I runs against a household mesh the test run OWNS and may
  write to; Act II runs against the deployed fleet, which it may only read. A feature file
  carries exactly one act tag. Three scenario tags group the stations for focused runs. The
  tag deliverability-browser marks a station that drives a real browser, and also attaches
  the cleanup Station 1 describes, which fails the scenario when cleanup fails. The tags
  deliverability-restart and deliverability-refusal are labels only.

  THE VEHICLE IS A SITE THIS RUN PUBLISHES, and that is a protocol fact rather than a test
  shortcut. The mesh's own landing page belongs to the household; overwriting its declared
  head with a fixture bundle would deface the page every other scenario in this lane reads.
  So each station publishes its own small site — an index page, one entry script, one
  stylesheet, one build stamp — and a visitor arrives at that site's own address. The
  question being asked is about the deploy path and the doorways, and it does not care
  which site travels it.

  THREE KINDS OF PEER, and the steps keep them apart; the difference is load-bearing in
  Stations 2 and 3. A FEDERATED PEER is a doorway: a front door that federates delivery
  for a public name. The Background registers the two with `federated peer "X" at "Y"`,
  where the value after "at" is the key under which the harness finds that doorway's
  address, and `federated peer "alpha-A"` and `doorway "alpha-A"` below name the same front
  door; the steps use both names, so read either one as that door. A PEER is a device of the protocol's own peer-to-peer network, where records are held
  with authority: the household's storage peers matthew, jessica and james. A HUB is an
  always-on peer that serves live aggregates and steadier live channels and needs no
  doorway; no station here uses one.
  The ROOT ADDRESS is a doorway's / mount, the address with no path after the name; only
  Station 1 publishes there, and its comment says why. A SLUG is the path segment that names
  one site on a doorway, as in /apps/<slug>/; each run publishes under a slug of its own.
  A doorway's PROJECTION is its local view of which head is current for a mount, kept
  separately from the peer's authoritative record; Station 1 stands against a projection
  that never moved. The APPS WORKER is a service worker the browser keeps after visiting any
  page of the protocol's app on a doorway, standing between the page and the doorway for
  /apps/ addresses; Station 5 describes it in full, including NAVIGATION PRELOAD. A browser
  says on every request what the request is for (the Sec-Fetch-Dest and Sec-Fetch-Mode
  headers): "the way a browser opening a page requests it" means a request that declares a
  page being opened, and "the way a frame requests it" means one that declares filling a
  frame. A doorway answers the two differently.

  THIS RUN means the currently executing test, which publishes its own site rather than
  writing to anyone else's. The `@requires:` tags are preconditions the runner evaluates
  before it starts a scenario; they gate whether a scenario runs, never what it asserts.
  `@requires:owned-substrate` is the permission to WRITE: it is satisfied only on a mesh
  this run owns, so on the shared fleet these stations are held rather than run. Every
  station here publishes a site and declares a head, so every station carries it.

  TWO KINDS OF APP, used only by Station 5. A record declares its app's FORMAT, a field
  its steward sets from a fixed list of content formats; two of them matter here. A
  CONTENT APP (format html5-app) is a piece of content packaged as a site, and the sites
  this run publishes are declared that way unless a step says otherwise. A CHROMED APP
  (format spa-bundle) is an app that draws the protocol's own CHROME itself (the quiet
  chip that tells a person what they are looking at and who holds it). A
  CONTENT SHELL is what a doorway wraps around a content app when a person opens it as a
  page: the chrome, with the app beneath it in a frame; Station 5 describes it in full. The
  chrome's page info has a HOLDERS card that says how many households hold the app, read
  from the doorway's HOUSEHOLD RESILIENCE READING for it, and ends with a link to its EPR
  page. An app's EPR PAGE is the page every doorway serves
  at /epr/<record id>: the full account of the record, its reach, its holders, its steward
  and its governance.

  Background:
    Given federated peer "alpha-A" at "alpha-A"
    And federated peer "elohim.host" at "elohim.host"
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
  #
  # AT THE ROOT ADDRESS. This station borrows the unclaimed / mount on both owned doorways, with
  # base href /. Its visits therefore request the real /version.json route,
  # catching service-prefix collisions that a nested mount cannot expose.
  # An existing root projection/commitment refuses setup, never gets replaced.
  # After success or failure, cleanup explicitly cancels its own commitments through
  # the existing canonical API on every owned peer and checks each returned state.
  # This cleanup is not a test of revocation gossip. After cancellation completes,
  # both root projections must disappear within 45 seconds; cleanup failure fails.
  # The cleanup is not a step: it runs in the After hook every @deliverability-browser
  # scenario carries, and that hook fails the scenario when restoration fails.
  @requires:owned-substrate
  @deliverability-browser
  Scenario: a steward publishes through one door and a visitor arrives able to run at either
    Afterwards, whether the scenario passed or failed, its cleanup cancels this run's root
    commitments on every owned peer, and both root projections must disappear within 45
    seconds. A cleanup that fails fails the scenario.
    Given a coherent EPR app bundle this run just built for the root address
    And an EPR record this run owns for it
    When each doorway is handed the bundle's bytes
    And only doorway "alpha-A" is told this bundle is the new version
    Then within 75 seconds doorway "alpha-A" serves a page naming that bundle's entry script
    And within 75 seconds doorway "elohim.host" serves a page naming that bundle's entry script
    # The static clause: what the page NAMES, on each door, is held by that same door.
    And every script and stylesheet the page from federated peer "alpha-A" names is one that peer serves
    And every script and stylesheet the page from federated peer "elohim.host" names is one that peer serves
    # The dynamic clause: a browser opens each page and the app actually starts.
    When a visitor opens the app this run published on federated peer "alpha-A" in a browser
    Then the browser on federated peer "alpha-A" reported no uncaught error
    And the browser on federated peer "alpha-A" completed client bootstrap
    And every asset the browser asked federated peer "alpha-A" for arrived
    And the app root on the page from federated peer "alpha-A" has content
    And the build stamp federated peer "alpha-A" serves for this app is the one the published bundle carries
    When a visitor opens the app this run published on federated peer "elohim.host" in a browser
    Then the browser on federated peer "elohim.host" reported no uncaught error
    And the browser on federated peer "elohim.host" completed client bootstrap
    And every asset the browser asked federated peer "elohim.host" for arrived
    And the app root on the page from federated peer "elohim.host" has content
    And the build stamp federated peer "elohim.host" serves for this app is the one the published bundle carries


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
    # Telling a door about a version, browser or server-rendered, hands that door's peer the
    # bundle's bytes and declares them in one act. That is the only hand-off here: the other
    # peers can get the bytes only from each other.
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
  Scenario: the doorway in front of the peers renders the server version every peer agreed on, adopts the next one without a restart, and keeps serving it while every peer is down
    Given a coherent EPR app bundle this run just built
    And an EPR record this run owns for it
    When each doorway is handed the bundle's bytes
    And only doorway "alpha-A" is told this bundle is the new version
    And only doorway "alpha-A" is told this bundle is the new server-rendered version
    # The two lines below are Station 2's proof, repeated here as preconditions: the doorway
    # leg starts only once every peer agrees on the pointer and holds its bytes.
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
    # A precondition, not the proof: the new browser bundle reaches both doors on its own,
    # and the server-rendered page waits for the server version declared with it.
    And within 75 seconds both doorways serve the new browser bundle by content address
    Then within 75 seconds every household peer answers with the same server pointer for this app
    And every household peer serves the declared server bundle bytes by their content address
    And within 330 seconds both doorways attest they materialized that server pointer for this app
    And both doorways return that server-rendered build before any browser script runs
    And neither doorway restarted while adopting the next rendered version
    When a visitor opens the app this run published on federated peer "alpha-A" in a browser
    Then the browser on federated peer "alpha-A" reported no uncaught error
    And the browser on federated peer "alpha-A" completed client bootstrap
    And every asset the browser asked federated peer "alpha-A" for arrived
    And the app root on the page from federated peer "alpha-A" has content
    And the build stamp federated peer "alpha-A" serves for this app is the one the published bundle carries
    When a visitor opens the app this run published on federated peer "elohim.host" in a browser
    Then the browser on federated peer "elohim.host" reported no uncaught error
    And the browser on federated peer "elohim.host" completed client bootstrap
    And every asset the browser asked federated peer "elohim.host" for arrived
    And the app root on the page from federated peer "elohim.host" has content
    And the build stamp federated peer "elohim.host" serves for this app is the one the published bundle carries
    # This step takes every storage peer down itself, checks that each refuses health
    # connections, and only then reads both doorways.
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
    # The restart step first waits until jessica carries the new head, within the publication
    # bound, and fails if she never does; only then does it take her down. Before restoring
    # her, it witnesses the restarted doorway logging its failed initial lookup for this
    # exact app while jessica is unreachable.
    And doorway "elohim.host" restarts while peer "jessica" is down
    And peer "jessica" comes back
    # The 75 seconds run from jessica's first healthy response, not from the doorway's restart.
    Then within 75 seconds doorway "elohim.host" serves a page naming that bundle's entry script
    And every script and stylesheet the page from federated peer "elohim.host" names is one that peer serves
    When a visitor opens the app this run published on federated peer "elohim.host" in a browser
    Then the browser on federated peer "elohim.host" reported no uncaught error
    And the browser on federated peer "elohim.host" completed client bootstrap
    And every asset the browser asked federated peer "elohim.host" for arrived
    And the app root on the page from federated peer "elohim.host" has content
    And the build stamp federated peer "elohim.host" serves for this app is the one the published bundle carries
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
  # the SDK PACKAGER (the tool a steward runs to build, check and upload a bundle) checks the
  # files locally before uploading any bytes or minting a head.
  # Storage also judges extracted bundles, so a publisher running an older packager, one that
  # predates the local check, cannot avoid the peer's check.
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
  # The run-owned site has a public mount, so this exercises the decision deployed sites use
  # when their newest head is broken: serve the last working version, or answer 503 with
  # Retry-After, and name the missing file in a response header either way.
  @requires:owned-substrate
  @deliverability-refusal
  Scenario: a bundle that cannot boot is refused, and never reaches a visitor as a blank page
    The scenario has two halves. First the normal deploy path is asked to publish the broken
    bundle and refuses. Then the broken head is declared anyway, through a route that skips
    the packager's check, to prove what a visitor gets: the previous version booting in their
    browser within 45 seconds, or a 503 with Retry-After. Nothing else passes.
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
    # Two outcomes keep that promise, and only these two: the previous version boots in the
    # visitor's browser within 45 seconds, or the doorway answers 503 with Retry-After.
    And a visitor asking doorway "alpha-A" for this app is never handed a blank page
    # "On the wire" is the x-elohim-bundle response header, whose value reads
    # `behind;missing-asset:<file>` — machine-readable, and present on every response so a
    # monitor sees it without anyone opening the page.

  # STATION 5 — a person who opens a content app's own address meets it qualified.
  #
  # Every station above asks whether a visitor gets a page that runs. This one asks whether
  # they can tell what they are looking at. A CONTENT APP is an EPR whose record declares
  # its format as html5-app: a self-contained piece of content (a simulation, a letter, an
  # explorable essay) whose files live in a zip. Its ENTRY ADDRESS is the address where the
  # doorway serves the first file of that zip, /apps/<slug>/<entry file>; for the site this
  # run publishes the entry file is its index page, so the address is
  # /apps/<this run's slug>/index.html. People share that address.
  # Until this station, a person who followed such a link got the app's page bare — nothing
  # on screen said what it was, who holds it, or where its full record lives, and a bare
  # page could have come from anywhere. On this protocol, nothing a doorway shows a person
  # should arrive without the frame that qualifies it.
  #
  # So a doorway answers a person opening an entry address with a CONTENT SHELL: the
  # protocol's CHROME, which loads first, and the app itself filling the window in a frame
  # beneath it. The chrome is a quiet chip that opens PAGE INFO, a short list of readings
  # about what is on screen. Its HOLDERS card says how many households hold the app, and like
  # every card it ends at a link to the app's EPR PAGE, /epr/<record id>: the full page about
  # the record, its reach, its holders, its steward, its governance. The person stays on the app; the qualification is
  # one glance and the full account is one click away.
  #
  # THE HOLDERS FIGURES come from that doorway's HOUSEHOLD RESILIENCE READING for this app.
  # N is how many distinct households appear among the content's RECORDED HOLDERS. A peer
  # that has the bytes counts only once its holding is recorded under the household it
  # belongs to, so N is not how many peers happen to have the bytes right now. M is a
  # target: how many households the content should live in, set by its RESILIENCE TIER, a
  # value class the content's owner may declare (a keepsake wants more homes than an
  # ephemeral note). A record that declares no tier gets the standard tier, whose target is
  # three. A record this run has just published usually has no recorded household holder
  # yet, so zero is an honest reading. This station asserts faithfulness, not a number: the
  # Holders card carries exactly the counts the reading carries. It states them in the
  # reading's own words when the reading gives a headline, and otherwise in one of three
  # wordings, literally:
  #   "Held by N of M households" when at least one household holds the app;
  #   "Not yet held by any household" when N is zero;
  #   "Held by households · see where it lives" when the doorway could not get a reading.
  #
  # HOW THE DOORWAY TELLS A PERSON FROM A FRAME. A browser declares, on every request, what
  # the request is for: opening a page, filling a frame, fetching a script. A person typing
  # or following the address is opening a page. The protocol's own surfaces that show
  # content, such as the EPR page, show the app in a frame at that same entry address, and
  # they already draw their own qualifying account around it. A second chrome inside their
  # frame would be noise. Worse, if the shell ever answered a
  # frame, the shell's own frame would load another shell, endlessly. So a frame always
  # gets the zip's bytes exactly as built, and the bytes a person runs inside the shell are
  # still exactly the bytes the record's content address names. The shell decides what is
  # around the app; it never touches the app.
  #
  # A PERSON WHO HAS BEEN HERE BEFORE. A browser that has opened any page of the protocol's own
  # app on a doorway (its landing page, an EPR page) holds the protocol's APPS WORKER: a
  # service worker, a small program the browser keeps and runs for the site, which stands
  # between the page and the doorway for every /apps/ address and
  # answers from app files it has already verified, so an app still opens offline. If the
  # worker opens a page by sending the request again itself, the browser labels that copy as
  # the worker's own request (Sec-Fetch-Dest: empty) instead of a page being opened
  # (Sec-Fetch-Dest: document), and the doorway answers it with the bare file, exactly as it
  # answers a frame. That is what happened on 2026-10-15: the shell reached alpha and nobody
  # who had visited before saw it. So the worker uses NAVIGATION PRELOAD, a browser feature in
  # which the browser itself sends the page-opening request, with its own labels, while the
  # worker starts; the worker hands the person that answer and falls back to its stored files
  # only when the doorway cannot be reached or cannot serve. The worker still stands between
  # the person and the doorway; the doorway still sees a page being opened.
  #
  # The person in the first scenario below has been here before. Their browser holds the
  # worker THIS SOURCE BUILDS: the scenario compiles app/elohim-app/src/apps-sw.ts from the
  # checkout under test, the same way the app's own build does, and when the browser asks
  # doorway "alpha-A" for /apps-sw.js the harness hands it those compiled bytes instead of
  # whatever worker the doorway's staged site carries. The browser registers that worker for
  # /apps/ and waits until it is active. So the scenario proves the worker that will ship.
  # "The apps worker stood between the visitor and the doorway" then means the browser
  # reports that the answer to the page-opening request came through that worker; a visit it
  # did not stand between would prove nothing about returning visitors.
  #
  # CACHES MUST NOT MIX THE TWO ANSWERS. One address now has two answers. If any cache on
  # the path (the doorway's own, a CDN's, the browser's) handed a frame the shell, every
  # embedding surface would show a chrome inside a chrome; if it handed a person the bare file,
  # the qualification would silently vanish. The shell is therefore never stored, and the
  # bare file declares that it depends on how the request was made, so no cache hands it to
  # a person opening a page. On the wire that is the shell's `X-Elohim-Chrome: content-shell`
  # and `Cache-Control: no-store`, and the bare file's `Vary: Sec-Fetch-Dest, Sec-Fetch-Mode`
  # (the two request headers in which a browser declares what a request is for). The doorway
  # sends the same Vary on the shell too, but a cache that never stores the shell cannot mix
  # it up, so the scenario checks no-store on the shell and Vary on the bare file, which is
  # where each one does the work.
  #
  # THE DOORWAY MUST KNOW THE RECORD FIRST. The doorway reads a slug's record id, format and
  # entry file from its own copy of the content records, and when it has no copy it serves
  # the bare file, exactly as before the shell existed. So each scenario first waits until
  # the doorway knows this app's record and its declared format. Without that wait, the
  # second scenario's "served as it is" would pass on a doorway that simply had not heard of
  # the app yet, and prove nothing.
  #
  # ONE DOOR IS ENOUGH HERE. Whether both doors converge on a new version is Station 1's
  # question. The shell is a decision each doorway makes alone from what it already holds,
  # so this station asks it of doorway "alpha-A" only.
  #
  # WHERE THE BOOT CHECKS LOOK. The app inside the shell runs in the frame's own document,
  # not in the shell's page. The two frame steps below look inside that document for the
  # app's root element and its data-app-ready marker, exactly as Station 1 looks at a bare
  # page; the shell's own page has no app root and is never what they inspect.
  #
  # NOT PROVEN HERE, named so each gap is a decision: a request that declares nothing about
  # its purpose (an older client, a command-line tool) is served the bare file as before;
  # this station asserts where the Holders card leads, not that the EPR page there renders,
  # which is that page's own story; and the card's third wording, for a doorway that
  # could not get a reading, is checked by the doorway's own unit tests rather than here,
  # because a household run cannot make the reading fail on purpose without breaking the
  # peer it reads from.
  @requires:owned-substrate
  @deliverability-browser
  Scenario: a person whose browser holds the apps worker opens a content app's own address and still sees it inside the protocol's chrome, whose Holders card says how many households hold it, while a frame gets the app's bytes untouched
    Given a coherent EPR app bundle this run just built
    And an EPR record this run owns for it
    When each doorway is handed the bundle's bytes
    And only doorway "alpha-A" is told this bundle is the new version
    And within 75 seconds doorway "alpha-A" serves a page naming that bundle's entry script
    And within 75 seconds doorway "alpha-A" knows this app's record and that its declared format is "html5-app"
    # The person has been here before: their browser already holds the apps worker.
    # "This source" is the application source in the checkout this run tests; see above.
    And the visitor's browser holds the apps worker this source builds, installed from doorway "alpha-A"
    # The person: open the entry address as a page, in a real browser.
    When a visitor opens this app's entry address on federated peer "alpha-A" as a page in a browser
    # The next two steps are one proof: the worker answered the page-opening request, and
    # the doorway still saw a page being opened, which only navigation preload allows.
    Then the apps worker stood between the visitor and doorway "alpha-A" when the page opened
    And doorway "alpha-A" answered the visitor with a content shell that no cache may keep
    And the chrome's Holders card from doorway "alpha-A" states the holder count from that doorway's own resilience reading for this app
    And the chrome's Holders card from doorway "alpha-A" ends at this app's EPR page
    And the shell from doorway "alpha-A" shows the app in a frame whose address is this app's entry address
    # The app inside the frame starts exactly as it would bare: the frame's requests are
    # the browser's own, so they count toward the checks below.
    And the browser on federated peer "alpha-A" reported no uncaught error
    And every asset the browser asked federated peer "alpha-A" for arrived
    And the app inside the shell's frame from doorway "alpha-A" completed client bootstrap in the frame's own document
    And the app root in the frame's own document from doorway "alpha-A" has content
    # The frame: the same address, asked for the way a frame asks for it.
    # "Depends on how it was requested" is the bare file's Vary on the two headers in which a
    # browser says what a request is for; see CACHES MUST NOT MIX THE TWO ANSWERS above.
    When this app's entry address is requested from doorway "alpha-A" the way a frame requests it
    Then doorway "alpha-A" answers with the index page's bytes exactly as the bundle this run built holds them
    And that answer carries no content-shell marker and declares that it depends on how it was requested

  # The other half of the rule. A CHROMED APP is an EPR whose record declares its format as
  # spa-bundle: an app that draws the protocol's chrome itself (the learning app is
  # built this way). It is already a qualified surface, so a
  # doorway serves its entry address as built, even to a person opening it as a page.
  #
  # The doorway decides from the record's DECLARED format, never by inspecting the bytes.
  # So this scenario publishes the same small site as above under a record declared
  # spa-bundle; the claim is that the doorway adds no chrome, not that the site draws one.
  # Nothing here needs a browser: the claim is about what the doorway adds, not about what
  # the app does, so the answer to a page-opening request is read directly off the wire.
  @requires:owned-substrate
  Scenario: an app that draws its own chrome is served as built when its entry address is opened as a page
    Given a coherent EPR app bundle this run just built
    And an EPR record this run owns for it, declared as an app that draws its own chrome
    When each doorway is handed the bundle's bytes
    And only doorway "alpha-A" is told this bundle is the new version
    And within 75 seconds doorway "alpha-A" serves a page naming that bundle's entry script
    And within 75 seconds doorway "alpha-A" knows this app's record and that its declared format is "spa-bundle"
    When this app's entry address is requested from doorway "alpha-A" the way a browser opening a page requests it
    Then doorway "alpha-A" answers with the index page's bytes exactly as the bundle this run built holds them
    And that answer carries no content-shell marker and declares that it depends on how it was requested
