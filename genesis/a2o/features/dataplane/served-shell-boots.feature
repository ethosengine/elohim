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
