@e2e @devflow @concern:idle-is-free @requires:epr-cli @act:host
Feature: Read authored work and native acceptance on my device
  As an engineer choosing work from a declared local collection
  I want independent authored and evidence readings that I can deliberately keep offline
  So that a captured reading never pretends to be a new acceptance act or fresh evidence.

  Vocabulary. Avodah is the work pillar of the Elohim client. The native local-view node is the
  process started on the engineer's own machine by `just dev view <declaration>`; it reads the
  declared files with the `epr view` reader and serves the production Avodah build to the browser
  through a launcher link that the launcher issues for one use and expires after five minutes (its
  own tests prove reuse is refused; this story does not). That link carries its capability (the permission to open
  the private view) in the URL fragment, the part after the "#"; removing the fragment from the
  address bar keeps the capability out of browser history and shared screens.
  A capture reads the declared files once and records one observation (the captured projection
  plus the time it was taken); the chart plots observations, so two captures yield two
  observations. The fixed-cohort chart is the burndown: its cohort is the set of subjects present
  in the first selected observation, and later filters never change it. A replacement transaction
  is the browser replacing its retained offline copy of a view with the newly captured one; it
  must be all-or-nothing. A workflow act is a recorded acceptance or status change; viewing
  never creates one, and the story's "acceptance act" means the same thing. The collections
  differ in size on purpose: orchard is a one-subject sample, idle-is-free is the eight-subject
  backlog of the idle-is-free habit. "Saving" means the viewer's explicit confirmation that the
  offline copy was stored.
  Concern: tagged idle-is-free because its eight-subject collection is that habit's backlog; this
  feature proves reading and retention, not idle cost, and the habit does not name it as a check.

  Scenario Outline: Deliberate retention survives failed replacements and a cold browser restart
    Given the production Avodah build and the native local-view node
    When I exercise the private <collection> collection through this offline journey:
      """
      Phase 1, open: open the launcher link, capture once on opening, and remove its capability fragment.
      Phase 2, read: search, open the fixed-cohort chart, use the keyboard to open a card and read its captured source.
      Phase 3, retain: refresh for a second successful capture, select the first observation, and keep the history offline once the viewer confirms the offline copy was saved.
      Phase 4, attack: interrupt a replacement transaction, exhaust cache storage, corrupt replacement bytes, and fail a refresh.
      Phase 5, cold restart: close the entire browser, stop every serving process, disable networking, and reopen the same profile.
      Phase 6, offline read and forget: read the retained board, chart and exact source, verify its recorded observation time, then forget the offline copy.
      """
    # Proves phases 1-3 and 5-6: the board, two captures and two observations.
    Then the board read before forgetting had <subjects> subjects and the chart retains two observations from exactly two captures
    # Proves phases 5-6: cold restart, offline, exact source and observation time.
    And before forgetting the retained source bytes and observation time are unchanged and the reading is visibly offline
    # Proves phase 4.
    And interrupted, exhausted, corrupt and failed replacements preserve the previous usable copy
    # Proves phases 2 and 6: reading creates no acceptance act.
    And viewing and reopening leave captured source files unchanged and create no workflow acts
    # Proves the end of phase 6.
    And forgetting removes the explicitly retained copy

    Examples:
      | collection   | subjects |
      | orchard      | 1        |
      | idle-is-free | 8        |
