# Design: genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md
# (§3 the spaces, §5.3 head and version, §6 the `channels` need, §15 "head → space", §16; held on
# §17 slice 9). The scope limit stated in the description follows from §19 question 11 (may a space
# hold a head back from the commons channel?), which is open.
# @concern:space-scoped-channel is a new tag; no habit's checks: entry names it yet.
@e2e @delivery @app-bundle @act:i @wip @concern:space-scoped-channel @requires:household-nodes
Feature: A co-op's channel and the commons channel carry different releases of one app at the same time

  A household here is a small group of people and the machines they run together. Each person
  keeps a peer: a program on their own machine that holds their data and serves the apps they
  use. matthew, jessica and james each run one, and the three peers are one household. A doorway
  is a public web entry point that serves apps to anyone on the ordinary web; doorway "alpha"
  serves what matthew's peer holds.

  An app here is a program people use together; this one is a rota app, for sharing out jobs. A
  release of it names its exact files. A release channel is a named feed of releases, and its
  head is the release everyone following it should run. A head moves only by a ceremony, such as
  promoting a newer release, performed by the people who steward that channel. Each peer takes
  up a new head by itself, by fetching the release's files on its own request; nobody writes the
  new files onto anyone's peer from outside. Each peer keeps a record of every release it took
  up, saying who asked for the files.

  A space is a group of people using something together. Three matter here. The commons is
  everyone, strangers included: nobody joins it and nobody leaves it, so a stranger who is in no
  other space is still in the commons. The household is a space too: matthew, jessica and james
  keep their household rota in the rota app. The co-op is a food co-op that matthew and jessica
  belong to and james does not. One person can be in several spaces at once, so matthew and
  jessica are in the co-op, the household and the commons. The co-op keeps its own rota in the
  rota app; only its members can open it. Opening a space's rota opens the rota app in that
  space; a stranger who opens the rota app through a doorway opens it in the commons.

  The rota app has three releases, made one after another: its first, its second and its third.
  "First", "second" and "third" below always mean these same three, in this order, whichever
  channel carries them.

  An app's declaration ships with every release and names the channels the app follows by name
  only. No app's declaration names a space: which space's channel of that name the app runs is
  settled each time the app is opened, from the space it is opened in. The rota app follows a
  channel called "stable". Each space may supply its own "stable" channel. The commons supplies
  one, stewarded by the people who publish the app. The co-op supplies its own, which matthew
  stewards for the co-op and uses to run a newer release than the commons runs. The household
  supplies none.

  This is how every app settles its channel, not a setting of the rota app: opened in a space
  that supplies the named channel, an app runs that channel's head; opened in a space that
  supplies none, it runs the commons channel's head. So inside the co-op the rota app runs the
  co-op channel's head. In the household it runs the commons channel's head, because the
  household supplies none, and so it does for a stranger, who is opening it in the commons.
  Neither head is out of date: each is the release its own space chose.

  Each time the rota app is opened, it shows its user two things on screen: the space it was
  opened in, and the channel whose head it is running, named by the space that supplies that
  channel. The two can differ: opened in the household, it shows the household as its space and
  the commons "stable" channel as its channel. That is how anyone can see why they have the
  release they have, and every scenario checks it at each opening.

  This story covers only a space whose channel runs ahead of the commons. Whether a space may
  hold a release back, keeping an older head than the commons has, is still an open question in
  the design, so no scenario here does that.

  The Background describes a settled household: before any scenario starts, each peer has
  already taken up every head the rota app runs in the spaces its own person is in. matthew's
  and jessica's peers have taken up both heads, the commons head and the co-op head; james's
  peer has taken up only the commons head, because he is not in the co-op and no space he is in
  runs the co-op's channel. So a check made straight after the Background needs no wait.
  Taking up a head that moves during a scenario, or a change of membership, can span a look at
  the channel, a download and an apply, so checks that follow such a change allow five minutes.
  What happens to a copy of the app that is already open at the moment of such a change is not
  part of this story; each check opens the app afresh.

  This story describes behaviour that is not built yet: today a channel is not tied to any space.
  It is held until the stage of the design's build order that lets an app declare the channels it
  follows, and lets a space supply them, has landed. It runs only where the household's three
  peers are running together; that need is what its tag "requires household-nodes" names, and
  the run skips the story anywhere else. Nothing yet watches whether it holds: no
  entry in the project's record of what the system reliably does names this story as its check.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And a household of three peers, run by matthew, jessica and james
    And matthew, jessica and james keep a household rota in the rota app, and the household supplies no channel of its own
    And matthew and jessica are members of the co-op, and james is not
    And matthew is the steward of the co-op's own "stable" channel
    And the rota app has three releases, made in this order: its first, its second and its third
    And the rota app's declaration names one channel it follows, "stable", by name only, as every app's declaration does
    And the commons "stable" channel for the rota app has the first release as its head
    And the co-op's own "stable" channel for the rota app has the second release as its head, promoted there by matthew
    And matthew's and jessica's peers have each already taken up both heads, and james's peer has already taken up only the commons head

  Scenario: Inside the co-op the rota app runs the co-op's release, and everywhere else the commons release
    When jessica opens the co-op's rota on her peer
    Then the rota app runs its second release
    And it shows the co-op as its space and the co-op's "stable" channel as its channel
    When james opens his household's rota on his peer
    Then the rota app runs its first release
    And it shows the household as its space and the commons "stable" channel as its channel
    When a stranger, who is in no space but the commons, opens the rota app through doorway "alpha"
    Then the rota app runs its first release
    And it shows the commons as its space and the commons "stable" channel as its channel

  Scenario: matthew, in both spaces, runs each space's release where he uses it, and changes nothing to do so
    When matthew opens the co-op's rota on his peer
    Then the rota app runs its second release
    And it shows the co-op as its space and the co-op's "stable" channel as its channel
    When matthew opens his household's rota on the same peer
    Then the rota app runs its first release
    And it shows the household as its space and the commons "stable" channel as its channel
    And between the two openings matthew changed no setting, of the app or of his peer, and no prompt or choice was put to him

  Scenario: Moving one space's head leaves the other space's head where it was
    When matthew, as steward of the co-op's channel, promotes the third release on it
    And matthew and jessica each open the co-op's rota on their own peer, once their peers have had up to five minutes after the promotion to take up the new head
    Then on each of the two peers the rota app runs its third release
    And on each it shows the co-op as its space and the co-op's "stable" channel as its channel
    When james opens his household's rota on his peer
    Then the rota app runs its first release
    And it shows the household as its space and the commons "stable" channel as its channel
    When a stranger opens the rota app through doorway "alpha"
    Then the rota app runs its first release
    And it shows the commons as its space and the commons "stable" channel as its channel
    And the record on matthew's and on jessica's peer shows that peer fetched the third release on its own request, and no peer's record shows release files arriving that it did not ask for

  Scenario: A member who leaves the co-op is back on the commons release, with nothing to do
    Given jessica has opened the co-op's rota on her peer, and it ran the second release
    When jessica leaves the co-op
    Then within five minutes the co-op's rota is no longer among the rotas jessica's peer lets her open
    When jessica opens her household's rota on her peer
    Then the rota app runs its first release
    And it shows the household as its space and the commons "stable" channel as its channel
    And no rota jessica can open on her peer runs the second release
    And between leaving and opening jessica changed no setting, of the app or of her peer, and no prompt or choice was put to her
