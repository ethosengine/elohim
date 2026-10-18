@e2e @devflow @concern:idle-is-free @requires:epr-cli @act:host
Feature: Read authored work and native acceptance on my device
  As an engineer choosing work from a declared local collection
  I want independent authored and evidence readings that I can deliberately keep offline
  So that a captured reading never pretends to be a new acceptance act or fresh evidence.

  Scenario Outline: Deliberate retention survives failed replacements and a cold browser restart
    Given the production Avodah build and the native local-view node
    When I exercise the private <collection> collection through this offline journey:
      """
      Open the launcher link, capture once on opening, and remove its capability fragment.
      Search, open the fixed-cohort chart, use the keyboard to open a card and read its captured source.
      Refresh for a second successful capture, select the first observation, and keep the history offline until saving is confirmed.
      Interrupt a replacement transaction, exhaust cache storage, corrupt replacement bytes, and fail a refresh.
      Close the entire browser, stop every serving process, disable networking, and reopen the same profile.
      Read the retained board, chart and exact source, verify its recorded observation time, then forget the offline copy.
      """
    Then the board read before forgetting had <subjects> subjects and the chart retains two observations from exactly two captures
    And before forgetting the retained source bytes and observation time are unchanged and the reading is visibly offline
    And interrupted, exhausted and corrupt replacements preserve the previous usable copy
    And viewing and reopening leave captured source files unchanged and create no workflow acts
    And forgetting removes the explicitly retained copy

    Examples:
      | collection   | subjects |
      | orchard      | 1        |
      | idle-is-free | 8        |
