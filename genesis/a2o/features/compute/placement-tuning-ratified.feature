# Held until the elohim-ceiling slice of the placement-contract design, the unnumbered slice that
# follows slice 5 ("the floor controller") in §17 of
# genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md.
# It rests on two pieces not built yet: the floor itself, and a way for a household to ratify a
# change and keep a record of it. Until both land, every floor reads only the settings it ships
# with, so the feature is @wip and none of its steps has a definition.
# @concern:placement-ceiling names this proof for the habit register; no habit checks it yet.
# @act:i: this story belongs to Act I, the household chapter of the test suite.
# @requires:household-nodes: the household's three peers must be running.
# @requires:owned-substrate: the scenarios move real copies between peers, so they run only on a
# mesh this run owns outright.
@e2e @act:i @compute @wip @concern:placement-ceiling @requires:household-nodes @requires:owned-substrate
Feature: A change to how the household ranks the peers that keep its copies takes effect only when the household agrees to it

  Matthew, Jessica and James share a household. Each of them keeps a peer: an always-on machine
  at home that holds their data and does work for them. Each peer is named after its person:
  matthew, jessica and james. The household keeps copies of what it shares on more than one peer,
  so that losing one machine loses nothing.

  Deciding which peers keep which copies is called placement. Placement is decided by the floor:
  the same fixed rules, running on every peer, with nobody in charge. For each thing to be kept, a
  peer's floor lists the peers allowed to keep it, puts them in order by a calculation, and the
  peers at the top of that order take the copies on themselves. A peer is allowed to keep copies
  only once its person has offered room on it: declared how much of that machine's disk the
  household may fill with copies. Free disk space that has not been offered does not count. Each
  peer keeps a list of the copies it holds; the peer's own storage writes that list itself as it
  takes copies up and lets them go, and no other peer or service keeps it. That list is dated: it
  records when each copy was taken up and when each copy was let go. A peer that lets a copy go
  also records which other peer it saw listing that copy at that moment. So whether a copy was let
  go only after another peer had taken it up can be read afterwards from the list of the peer that
  let it go, without comparing the clocks of different peers.

  The calculation has settings: how much weight each consideration carries when two allowed peers
  are put in order. The floor ships with settings of its own. The household may adopt different
  settings, but only by agreeing to them. In this household the rule is that new settings are
  in force once at least two of its three members have voted for them, each vote signed by the
  member who cast it. The household keeps one record of which settings it has adopted, in order.
  Every set of settings is named by its fingerprint: a short code computed from its exact contents,
  which changes if anything in them changes. So two peers that name the same fingerprint mean
  exactly the same settings.

  Settings only reorder the peers that are already allowed to keep a copy. They cannot let in a
  peer that has not offered room, and they cannot lower how many peers keep a copy of each thing.
  The last two scenarios check both.

  Each person may have an elohim: an AI helper that works for its own person. Jessica's elohim
  can notice that the placement is not serving her and write a draft: proposed settings, with the
  evidence that led to them, naming which elohim wrote it. An elohim is named by three things: its
  base model (the general-purpose AI it is built on), its tuning (any further training given to
  that model), and its standing instructions (the rules its person has given it). Two elohims that
  differ in any of the three are different elohims. A draft is only a proposal. It does not move any
  copy, and no peer's floor reads it, until the household has voted it in. The elohims' drafts are
  called the ceiling of placement: they sit above the floor and may change how the floor weighs
  allowed peers, never which peers it allows or how many peers keep a copy of each thing.

  When new settings come into force, a copy that should change peers moves make-before-break: the
  new peer takes its copy up and lists it before the old peer lets its copy go, so nothing is ever
  kept on fewer peers than the household asked for. New settings may also limit how many copies
  are moving at any one time. Peers do not all receive new settings at the same moment, so for a
  short while they may rank differently; a copy stays where it is until its peers have the new
  settings.

  In these scenarios, folders are the things to be kept: the household keeps five folders for
  Jessica, each with a copy on two peers, and those copies are what the floor places. Five is more
  than the two copies the new settings let move at once, so the folders have to move in rounds.
  The floor's own settings put allowed peers in order by a score mixed from the folder's
  fingerprint and each peer's public key: a hash that looks random but comes out the same on every
  peer. They give no weight to free room. The test chose five folders for which that score puts
  jessica and james first, so under the floor's own settings both copies of every folder sit on
  jessica and james, and james is running out of room. If james fills up, Jessica's folders could be left with a copy on only one
  peer, and losing that one machine could lose her data.

  None of this runs yet. Two pieces are still to be built: the floor itself, and the household's
  way of voting settings in together with its record of the settings it has adopted. Until both
  exist, every peer uses only the floor's own settings. This story describes what the household
  will see once both exist. Both are planned in the elohim-ceiling part of the placement-contract
  design. Until both are built the story is marked work in progress and its steps are not yet
  wired to anything; once they are, it runs as part of Act I, the household chapter of the test
  suite, and it is the proof named placement-ceiling that the project's register of habits (its
  list of things the system must keep doing, each tied to a test that can be run) can check.

  The times in the scenarios are this story's targets, not readings taken today.

  Background:
    Given the household peers matthew, jessica and james are running, and each has offered room for copies
    And the floor's own settings give no weight to how much free room a peer has
    And under those settings, each of Jessica's five folders has a copy on jessica and a copy on james, the two peers the floor's own score puts first for each of those folders
    And james has little free room left, and matthew has plenty

  Scenario: A draft the household votes in is recorded with who wrote it and who voted for it, and every peer checks those votes itself
    Given Jessica's elohim has drafted settings that give weight to free room, limiting moves to two copies at a time
    And Jessica has seen the draft, with the evidence that led to it
    When Jessica, Matthew and James each read the draft, and Jessica and Matthew vote for it
    Then the draft each of them read names the elohim that wrote it by its base model, its tuning and its standing instructions
    And the draft each of them read carries its evidence: how much free room james has left, and how much matthew has
    And the household's record of adopted settings names the draft's fingerprint and the two members who voted for it
    And each peer's floor checks those two signed votes itself before it uses the new settings

  Scenario: Settings the household votes in move Jessica's folders, make-before-break
    Given Jessica and Matthew have voted in settings that give weight to free room, limiting moves to two copies at a time
    When the new settings reach each peer
    Then within 10 minutes each of Jessica's five folders has a copy on jessica and a copy on matthew, and none on james
    And for every folder, matthew's list records taking up its copy, and james's list records that when it let its copy go it saw matthew's list naming that copy
    And at no moment did any folder have copies on fewer than two peers
    And at no moment were more than two copies moving
    And each of the three peers, asked which peers hold a copy of each folder, names jessica and matthew

  Scenario: Voting the earlier settings back in moves the folders back the same way
    Given Jessica and Matthew have voted in settings that give weight to free room, limiting moves to two copies at a time
    And each of Jessica's five folders has moved make-before-break to a copy on jessica and a copy on matthew under those settings
    When Jessica and James vote to return to the floor's own settings
    Then the household's record of adopted settings gains a new entry naming the fingerprint of the floor's own settings
    And the earlier entries are still in the record, in the order they were adopted
    And within 10 minutes each of Jessica's five folders has a copy on jessica and a copy on james again, and none on matthew
    And for every folder, james's list records taking up its copy again, and matthew's list records that when it let its copy go it saw james's list naming that copy
    And at no moment did any folder have copies on fewer than two peers

  Scenario: A draft with too few votes moves nothing on any peer
    Given Jessica's elohim has drafted settings that give weight to free room
    When the draft is shown to Jessica, Matthew and James, and only Jessica votes for it
    Then the household's record of adopted settings is unchanged
    And no peer's floor uses the draft, and each peer, asked why, says the draft has too few votes
    And after 10 minutes each of Jessica's five folders still has a copy on jessica and a copy on james
    And no copy of any folder was taken up or let go during those 10 minutes

  Scenario: A draft whose votes are not signed by the members it names is refused, and every peer says why
    Given Jessica's elohim has drafted settings that give weight to free room
    When the draft arrives with a record claiming that Matthew and James voted for it, without their signatures
    Then no peer's floor uses the draft, and each peer, asked why, says the votes it claims are not signed by those members
    And after 10 minutes each of Jessica's five folders still has a copy on jessica and a copy on james

  Scenario: Settings that would lower how many peers keep each folder are refused, even with enough votes
    Given Jessica's elohim has drafted settings that give weight to free room and keep each folder on one peer instead of two
    When Jessica and Matthew vote for it
    Then no peer's floor uses those settings, and each peer, asked why, says settings may not change how many peers keep a copy
    And after 10 minutes each of Jessica's five folders still has a copy on jessica and a copy on james

  Scenario: Settings the household votes in place no copy on a peer whose room is not offered
    Given Matthew has withdrawn the room he offered on matthew
    And Jessica and Matthew have voted in settings that give weight to free room, limiting moves to two copies at a time
    When the new settings reach each peer
    Then after 10 minutes each of Jessica's five folders still has a copy on jessica and a copy on james, and none on matthew
    And each peer, asked why matthew keeps no copy, says matthew has not offered room
