# @act:ii on purpose — this is the DEPLOYED fleet's question, and the fleet's
# Dataplane Validation is the Act II lane. Habit: doorway-failover
# (doorway/doorway-service/.epr-meta/doorway-failover.habit.md).
@e2e @dataplane @concern:doorway-failover @act:ii
Feature: The shell a doorway serves can boot
  An EPR is a named, content-addressed app bundle a doorway serves;
  elohim-host-landing is the one projected at the site root "/". Its DECLARED
  BROWSER HEAD is the bundle hash (blobHash) its EPR record declares — the code
  a browser downloads and runs. This is the browser-shell clause of the
  doorway-failover invariant: whichever doorway answers the root, the page it
  hands a person must name only assets from that declared browser head, so the
  app can boot. The sibling concern served-projected-head compares the SSR
  SERVER head instead and never sees these bytes. A 200 at the mount proves the
  door opened; it does not prove anyone got in.

  # Two doorways answer for the same site, so both are asked: a person reaches
  # whichever one their DNS hands them, and neither may serve a dead page.
  Background:
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"

  # 2026-09-04: both doorways answered 200 at "/" with a page from a PREVIOUS
  # bundle era — it named main-EAKNZDUP.js while the declared browser head
  # (blobHash sha256-7725d4…) holds main-7QFGHX5X.js. Fonts, styles and
  # polyfills all resolved; only the entry script 404'd, so nothing booted and
  # every visitor got a blank page.
  @regression @requires:doorway
  Scenario: A visitor asking for the site root is handed a page that can boot
    When a visitor asks peer "alpha-A" for the page at "/"
    And a visitor asks peer "elohim.host" for the page at "/"
    Then every script and stylesheet the page from peer "alpha-A" names is one that peer serves
    And the page from peer "alpha-A" names the same browser entry point as the declared browser head of EPR "elohim-host-landing"
    And every script and stylesheet the page from peer "elohim.host" names is one that peer serves
    And the page from peer "elohim.host" names the same browser entry point as the declared browser head of EPR "elohim-host-landing"

  # THE DYNAMIC CLAUSE (2026-09-08, spec 2026-09-08-epr-app-deliverability-through-doorway
  # D4). The two assertions above are read off the page's TEXT: they prove the
  # page names assets this doorway holds, and names the right era's entry
  # script. Neither of them runs a line of that script. A bundle can pass both
  # and still hand a person a white rectangle — the script 200s, and then
  # throws on its first statement, or reaches for a file the page never named.
  # Only a browser settles it, so this scenario opens one.
  #
  # BOOTS, precisely: the browser reported no uncaught error, every request it
  # made to this same doorway was answered (nothing 4xx, 5xx or aborted), and
  # the app's own root element — <app-root>, the single tag the served page puts
  # in <body> for the framework to fill — has content in it afterwards. An empty
  # <app-root> after load IS the blank page, in the one form a person sees.
  #
  # THE BUILD STAMP is the fourth line. Every browser bundle carries a
  # version.json written by the app job (commit, version, buildTime,
  # environment, service). It records the COMMIT, not the bundle's content
  # hash, so it cannot be compared against a blobHash directly; what it CAN be
  # compared against is the copy of version.json inside the declared browser
  # head's own bundle, reached at /apps/{slug}/version.json — the same
  # projection the entry-point comparison above reads index.html through. Equal
  # stamps mean the file a visitor is served and the file the declared head
  # holds came out of one build. Born red on the fleet 2026-09-06: both apex
  # names render an intact landing whose /version.json 404s, which is the
  # stale-shell shape one notch down — assets 200, stamp absent.
  @browser-only @regression @requires:doorway
  Scenario Outline: A visitor's browser actually starts the app it was handed
    When a visitor opens the page at "/" on peer "<peer>" in a browser
    Then the browser on peer "<peer>" reported no uncaught error
    And every asset the browser asked peer "<peer>" for arrived
    And the app root on the page from peer "<peer>" has content
    And the build stamp peer "<peer>" serves is the one the declared browser head of EPR "elohim-host-landing" carries

    Examples:
      | peer        |
      | alpha-A     |
      | elohim.host |
