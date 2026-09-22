# VISION (2026-09-22): a north-star story, not a habit check. Every step is undefined and the
# feature stays @wip until the substrate can stage it. It is written down so the substrate and the
# doorway architecture are judged against a real use as they mature. Two scenarios touch the live
# serving-edge campaign — genesis/docs/superpowers/plans/2026-09-19-serving-edge-failover-balance-stream-campaign-plan.md:
# "keeps editing together" leans on its story 4.2 (doorway↔peer push) and story 4.3 (eager delivery
# on every transport, see dataplane/transport-comparison-matrix.feature); the others are ahead of any plan.
@e2e @federation @local-first @act:iii @wip @vision
Feature: A small group edits one spreadsheet together, from a laptop at home and a browser at work

  Rachel keeps her church small group's meal-rota spreadsheet in a spreadsheet app on her own
  laptop. The sheet is hers: it lives on her laptop, it keeps working with no network, and her
  household hub — an always-on box at home she never thinks about — keeps a copy so the sheet
  stays reachable when the laptop is shut. She has shared it with her small group, and with
  nobody else.

  Her friend Dana is in that small group. During the working day Dana only has a web browser on
  a company laptop. Dana reaches the sheet through a doorway: a public web entry point that
  relays Dana's view and edits to Rachel's hub and holds nothing of its own.

  The promise has four parts. Both see each other's edits and cursors as they happen. The sheet
  keeps working when Rachel's laptop sleeps. Nobody on the path — the doorway, the company
  network, the internet in between — can read the sheet or pass off a forged edit. And nothing
  about this depends on the doorway being trusted: a different doorway serving Dana works the
  same way.

  Transport, in plain terms. Rachel's laptop and her hub talk directly, each proving itself by
  its own key; no web certificate is involved. The doorway reaches the hub the same way, by key,
  across Rachel's home router — never over a plain connection. Dana's browser reaches the doorway
  over ordinary web HTTPS, which a company network may be able to open and read. That is why the
  sheet is encrypted to the small group itself, so every hop between the two of them carries only
  sealed changes, and every change is signed by the person who made it.

  Background:
    Given Rachel's laptop runs the spreadsheet app on her own peer
    And Rachel's household hub holds a replica of her sheets behind her home router
    And Rachel's sheet "Meal rota" is shared with reach "small group" and with no one else
    And Dana belongs to that small group and is signed in through a public doorway in a browser
    And the browser's network opens and reads every HTTPS connection it makes

  Scenario: They see each other's edits and cursors as they happen
    When Rachel types "Soup — Tuesday" into cell B4 on her laptop
    Then Dana's browser shows "Soup — Tuesday" in cell B4 within 1 second
    And Dana's browser shows Rachel's cursor on cell B4 within 1 second
    When Dana moves her cursor to cell C4 and types "Bread"
    Then Rachel's laptop shows "Bread" in cell C4 and Dana's cursor on it within 1 second
    And neither cursor position is stored anywhere once both of them leave the sheet

  Scenario: Dana keeps editing together with Rachel after Rachel's laptop is shut
    Given Rachel has closed her laptop lid
    When Dana types "Salad — Thursday" into cell B6
    Then Rachel's household hub holds Dana's edit within 1 second
    When Rachel opens her laptop again
    Then Rachel's laptop shows "Salad — Thursday" in cell B6 without Rachel doing anything
    And the edit is shown as made by Dana

  Scenario: Rachel edits with no network at all, and nothing is lost when she reconnects
    Given Rachel's laptop has no network connection
    When Rachel types "Pie — Friday" into cell B7
    And Dana types "Pasta — Friday" into cell B7 from her browser at the same time
    And Rachel's laptop reconnects
    Then both of them see the same value in cell B7 within 5 seconds
    And the sheet's history keeps both edits, each shown as made by its author

  Scenario: Nobody on the path can read the sheet
    When Dana and Rachel edit the sheet together for one minute
    Then every change the doorway relays is sealed to the small group and unreadable by the doorway
    And every change the company network sees is unreadable by the company network
    And the household hub stores the sheet sealed, so a thief of the hub's disk cannot read it

  Scenario: Nobody on the path can pass off a forged edit
    Given a doorway that alters the next change it relays from Dana to Rachel
    When Dana types "Soup — Wednesday" into cell B4
    Then Rachel's laptop refuses the altered change and keeps "Soup — Tuesday" in cell B4
    And Rachel's laptop reports that a change failed its author's signature

  Scenario: Another doorway serves Dana exactly the same way
    Given the doorway Dana uses is withdrawn
    When Dana's browser reopens the sheet through a different doorway that also serves the small group
    Then Dana sees the sheet at the same version Rachel's laptop holds
    And editing and cursors resume with no step asked of Rachel
