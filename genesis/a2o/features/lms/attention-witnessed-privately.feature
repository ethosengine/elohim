# @browser: the story is a person reading a page in a real browser. Reading time and
# scroll depth are measured by the page itself, so Playwright is the driving mechanism
# and the browser lane runs it; in an HTTP-only lane the page steps hold as pending.
# @requires:household-nodes: the last check reads the household storage peer's own
# /metrics, which only a household mesh this run owns exposes.
@act:i @e2e @browser @lamad @requires:household-nodes @concern:attention-witnessed-privately
Feature: What a person reads stays with them, and the rules that arrange it are printed
  Reading leaves a trace: which page someone opened, how long they stayed, how far
  down they read. In most software that trace is sent to an analytics service the
  reader never sees and cannot question, and it is used to rank and target them.

  In the Elohim Protocol the trace belongs to the reader. Each household member's
  account is kept by a storage peer — a computer the household runs. When a person
  moves on from a page to another page inside the app, the app sends one small note
  to that peer: this page, this long, this far, and the peer replies that it has kept
  it. (This story covers moving on inside the app; closing the tab is not covered.) The
  note has a declared kind, "lamad:content-viewed" — a page viewed in the app's
  learning area, which is called lamad. The peer keeps the note privately for that
  person alone. Peers normally tell each other when they hold something new, so the
  others can fetch it; for a private note the peer deliberately tells no one. The peer
  keeps two running counts per kind of note on its own metrics page, which the people
  running the household can read: how many notes it withheld from the other peers, and
  how many it told them about. In this story the test reads those two counts itself,
  standing in for the people running the household; it is a check on the peer, not
  something Jessica or James does.

  The person can read their own notes on a page called "My stream". The stream is a
  view: a recipe decides which notes appear and in what order. A recipe is a small
  file of rules shipped with the storage peer — whose notes may be shown (only the
  asker's own), over what time window, and how to sort them. The page prints the
  recipe's name and its content address (a fingerprint of the recipe's
  exact text), so the person, or anyone they ask, can look up the precise rules that
  shaped what they see and challenge them. The story checks the printed address two
  ways: as the page shows it, and as the peer states it when asked directly, without
  the page — two channels, one answer, so the page cannot print a label of its own.

  The people: Jessica and James live in the same household. Both sign in through the
  household's doorway "alpha", the web entrance to their storage peer, so one peer
  keeps both of their accounts. Keeping them in one place must never mean mixing them.
  Jessica reads in a web browser — that is what "with device" means when she signs in.
  James signs in without a browser: he opens no pages here; his sign-in is used only
  to ask the peer for his own stream, to show that her reading is not in it.

  The page: the "manifesto" is a long public essay installed on every household's
  peer when the household is set up, long enough that reading to the end means
  scrolling. How far someone read is the deepest
  point of the page they brought into view. The story asks for at least half, not all:
  a long page can still grow as its last parts load, so the bottom she reached may
  sit a little short of the page's final end.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And human "Jessica" is logged in on doorway "alpha" with device
    And human "James" is logged in on doorway "alpha"

  # The one scenario carries the whole promise, so each Then checks a different
  # part of it: Jessica sees her own reading; the printed recipe is the real one;
  # James cannot see her reading; and her peer never told the other peers about it.
  Scenario: Jessica's reading appears in her own stream with its recipe printed, not in her housemate's stream, and is told to no other peer
    Given before Jessica reads, the storage peer behind doorway "alpha" is asked how many "lamad:content-viewed" notes it has withheld from other peers so far
    When Jessica opens the "manifesto" page
    And Jessica stays on the page for at least 3 seconds
    And Jessica scrolls to the bottom of the page
    And Jessica moves on from the page to the home page without leaving the app
    And Jessica waits until her storage peer replies to her app that it has kept her note on the "manifesto" page
    And Jessica opens her stream
    Then Jessica's stream lists the "manifesto" page with at least 3000 ms of reading and at least 50 percent read
    And the recipe address printed on Jessica's stream page matches the one her storage peer returns when she asks it for her stream through its web API, independently of the page
    And James's stream, fetched with his own sign-in, has no entry for the "manifesto" page
    And the storage peer behind doorway "alpha" has withheld at least 1 more "lamad:content-viewed" note from other peers and told other peers about none
