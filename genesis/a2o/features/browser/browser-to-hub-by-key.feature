# @concern:browser-to-hub-by-key joins this story to the habit that will hold it; no habit names it yet.
# @act:i: this story belongs to Act I, the household chapter of the test suite.
# @requires:household-nodes: the household's three nodes are running. @requires:doorway: the
# household's doorways are serving. @requires:owned-substrate: the test stops the hub and stands an
# impostor at its address, so it runs only on a mesh this run owns. @browser-only: a real browser runs it.
# HELD: every step is undefined. The placement-contract design
# (genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md)
# names a browser reaching a hub by key as a missing node whose feasibility is unverified (§15), on
# the remote tier that §17 slice 2 builds, and states the fetch order this story uses in §12.3. The
# feature stays @wip until a browser is shown able to reach a hub by key and slice 2 has landed.
@e2e @browser @browser-only @wip @concern:browser-to-hub-by-key @requires:household-nodes @requires:doorway @requires:owned-substrate @act:i
Feature: A browser fetches an app's files from the household hub by its key, beside the doorways it already uses

  This story describes behaviour that is not yet built, and whose way of being built is not yet
  established: no browser has yet been shown able to reach a hub by its key, and none of the steps
  below is wired to a test. The part of the design it depends on, the browser's way of asking a
  node other than a doorway for a file, has not yet been built either. Until all three are done the
  story is held, and the test suite skips it.
  When it runs, the test switches the household's hub off and stands an impostor machine at the
  hub's address, so it runs only on a household that the test run owns, never on one others share.

  Matthew, Jessica and James share a household. Its hub is node "matthew", the always-on machine in
  the hallway cupboard that keeps a copy of what the household shares, including the household's
  apps. Jessica's work laptop runs only a web browser, no node of her own, and she uses it on the
  home network.

  A browser reaches the network through doorways: public web entry points that many people use and
  that serve an app's files to anyone with standing to see them. The household can reach more than
  one, and any of them will do. That stays as it is. This story adds a second way in, for when the
  browser can reach the household's own hub.

  An app is a set of files: its code, its styles, its pictures. Each file is named by a fingerprint
  of its exact bytes, so the browser can check any file it is given against its name, whoever gave
  it. A file that does not match is refused. Which files make up an app is itself written in a
  file, the app's list of files, which names each file by its fingerprint. The household's record,
  described next, names each household app's list by the list's own fingerprint, so the browser
  checks the list as it checks every other file.

  The browser reaches the hub by key. When Jessica signs in through a doorway, the doorway's answer
  to her sign-in carries the household's record, and her browser keeps it. The record is one
  document that carries everything listed below together, and the browser receives it whole. The
  record names the hub by its public key: a long number published by the hub, whose matching secret key only the hub
  holds. The record also carries the network address on the home network where the hub was last
  reached, so the browser knows where to try first. It is the only node address the record carries:
  the browser has no address for the household's other nodes, "jessica" and "james", and reaches
  them only through a doorway. The record also names the doorways that serve the household,
  "alpha" and "beta", so the browser knows it may use either of them, not only the one Jessica
  signed in through. The hub's address says where to look; only the hub's key says whom to trust.
  When the browser connects to that address, it sends the machine that answers a fresh challenge, a
  random number it has just chosen, and the machine must return that challenge signed with the
  hub's secret key. The browser checks the signature against the hub's public key
  before it takes anything from that machine. No web link, web certificate or domain name is
  involved in trusting the hub: its home-network address is only where to try.

  The hub checks the browser in turn. When Jessica signs in, her browser makes a fresh key pair of
  its own for that sign-in, and its secret key never leaves her browser. The doorway's answer to
  Jessica's sign-in also carries a membership pass: a statement, signed by one of the household's
  nodes, that the key her browser holds for this sign-in belongs to a member of the household. The
  doorway only passes the pass on; any of the household's nodes may sign one, and the hub knows
  each of their public keys. Only after the browser has checked
  the hub's signature does it show the hub that pass, and it signs each request it sends the hub
  with the key the pass names. The hub checks the pass's signature and the request's signature
  before it serves anything.

  The hub serves the household's apps only to a browser that shows a valid membership pass of the
  household. Knowing where the hub is and what to ask for is not enough: the hub's home-network
  address can be learned by any machine on the home network, and a file's fingerprint by anyone
  who has seen the file or its app's list of files, without signing in. To a browser with no valid
  pass the hub serves nothing, and its refusal says only that the browser has no standing in the
  household, not whether the hub keeps the file that was asked for.

  The browser decides itself where to ask, in this order: first what it already keeps from an
  earlier visit, then the household's hub, then the doorways. A file it already keeps is not
  fetched again from anywhere, and a file it lacks is asked of the hub before any doorway. When it
  goes to the doorways it may use either one or both, as it chooses; which one does not matter,
  because every file is checked against its fingerprint. The app's list of files is fetched in the
  same order, and checked against the fingerprint the household's record names for it.

  When the hub serves a file that does not match its fingerprint, the browser sends an integrity
  report through a doorway to the household's other nodes, "jessica" and "james", because a doorway
  is the browser's only way to reach them. The doorway passes the report on to both nodes; it does
  not hold it for them to collect. Each keeps the report, so the household's members can see
  that their hub served altered bytes and look into why.

  Background:
    Given the household's nodes "matthew", "jessica" and "james" are running
    And node "matthew" is the household's hub and keeps the household app "Meal planner"
    And the household reaches the network through two doorways, "alpha" and "beta"
    And Jessica has signed in, in the browser on her work laptop, through doorway "alpha"
    And doorway "alpha"'s answer to her sign-in gave her browser the household's record, naming the hub by its public key, the home-network address where node "matthew" was last reached, the doorways "alpha" and "beta" that serve the household, and the fingerprint of the list of files of "Meal planner"
    And that answer gave her browser a membership pass naming the key her browser holds for this sign-in

  Scenario: The browser fetches the app's files from the hub by key, checking each one
    Given her browser keeps no file of "Meal planner"
    When Jessica opens "Meal planner" in her browser
    Then her browser connects to node "matthew" at the address in the household's record
    And her browser sends node "matthew" a fresh challenge
    And node "matthew" returns the challenge signed with the hub's secret key
    And her browser finds that signature valid against the hub's public key in the household's record
    And her browser then shows node "matthew" her membership pass and signs each request to it with the key the pass names
    And node "matthew" finds the pass and each request's signature valid before it serves any file
    And her browser fetches every file of "Meal planner" from node "matthew"
    And her browser checks each file against its fingerprint before using it
    And "Meal planner" opens in her browser
    And no doorway was asked for any file of "Meal planner"

  Scenario: Files the browser already keeps are not fetched again, and the rest come from the hub
    Given Jessica has opened "Meal planner" in her browser once before
    And her browser has since deleted the pictures of "Meal planner" it kept, and kept its other files
    When Jessica opens "Meal planner" in her browser again
    Then her browser fetches the pictures of "Meal planner" from node "matthew"
    And her browser fetches no other file of "Meal planner" from node "matthew" or from any doorway
    And no doorway was asked for any file of "Meal planner"
    And "Meal planner" opens in her browser

  Scenario: With the hub off, the browser fetches the same files through a doorway, and asks Jessica nothing
    Given her browser keeps no file of "Meal planner"
    And node "matthew" is switched off
    When Jessica opens "Meal planner" in her browser
    Then her browser fetches every file of "Meal planner" through one or both of doorways "alpha" and "beta"
    And her browser checks each file against its fingerprint in the app's list of files before using it
    And "Meal planner" opens in her browser
    And Jessica was not asked to choose a doorway, to sign in again or to retry

  Scenario: A machine that cannot prove it holds the hub's secret key is given nothing to serve
    Given her browser keeps no file of "Meal planner"
    And a machine that does not hold the hub's secret key answers at the home-network address where node "matthew" was last reached
    When Jessica opens "Meal planner" in her browser
    Then her browser sends that machine a fresh challenge
    And that machine returns no signature on the challenge that is valid against the hub's public key
    And her browser refuses that machine
    And her browser takes no file from that machine
    And her browser does not show that machine her membership pass
    And her browser fetches every file of "Meal planner" through one or both of doorways "alpha" and "beta"
    And "Meal planner" opens in her browser

  Scenario: A file that does not match its fingerprint is refused and fetched from the next place
    Given her browser keeps no file of "Meal planner"
    And node "matthew" will serve one file of "Meal planner" with its bytes altered
    When Jessica opens "Meal planner" in her browser
    Then her browser refuses the altered file, because it does not match its fingerprint
    And her browser fetches that one file through one or both of doorways "alpha" and "beta"
    And "Meal planner" opens in her browser
    And her browser sends an integrity report through one of doorways "alpha" and "beta", which passes it on to nodes "jessica" and "james", naming node "matthew", "Meal planner" and the fingerprint the altered file did not match
    And nodes "jessica" and "james" each keep that integrity report

  Scenario: The hub serves nothing to a browser that shows no valid membership pass of the household
    Given a browser on the home network holds no membership pass of the household
    And that browser has learned node "matthew"'s home-network address without signing in
    And that browser has learned the fingerprint of a file of "Meal planner" without signing in
    When that browser asks node "matthew" at its home-network address for a file of "Meal planner" by its fingerprint
    Then node "matthew" finds no valid membership pass of the household with the request
    And node "matthew" refuses that browser and serves it no file
    And the refusal says that browser has no standing in the household
    And the refusal does not say whether node "matthew" keeps that file
