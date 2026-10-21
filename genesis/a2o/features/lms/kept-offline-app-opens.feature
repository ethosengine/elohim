# What the tags below mean, for a reader meeting them here first:
# @act:i — Act I, the household chapter of the suite. This story needs only one household member's
# own laptop; no other household machine takes part, so it carries no @requires:household-nodes.
# @concern:kept-offline — a new concern tag: what a person chose to keep on their device opens with
# no network, and what they did not keep is named as not here rather than left to hang. No habit
# names this tag yet.
@e2e @lamad @act:i @concern:kept-offline
Feature: With the network off, every app Matthew kept opens, and a lesson he never opened says it is not here

  Matthew Dowell learns on the household's learning platform from his own laptop. The laptop
  signs as him with a key of its own; no website holds his key for him.

  An app, on this platform, is a bundle of files the platform opens as one thing: a game, a letter,
  a ledger. A course is an app made of lessons, and each lesson is its own set of files inside it,
  so the laptop can hold some lessons of a course and not others. The household's machines are its
  members' own devices, Matthew's laptop among them, and the household hub. The household hub is
  the one household machine at home that stays switched on and holds apps and courses for the whole
  household, so a person can open an app from it without the laptop holding any of that app's files.
  The laptop knows which household machines hold each lesson of a course it follows.

  He uses apps in three ways, and the laptop keeps a different amount of each:
  - Kept: he asked the laptop to keep the app. Every file the app needs is on the laptop and stays
    there until he lets it go.
  - Used as he goes: he follows a course without keeping it. The first time he opens a lesson, it
    arrives from one of the household machines that hold it (the household hub, or another member's
    device that kept the course), and it stays on the laptop afterwards; lessons he has not opened
    are not on the laptop.
  - Opened from the hub: he opens the app from the household hub, and the laptop holds none of it.
    The laptop shows that machine to him by the name "household hub", and the app has always shown
    that it is "not on this device" and is opened from "household hub".
  Each of the first three scenarios below proves one of these ways, and its title begins with the
  way's name.

  The promise is about the moment the network goes away: on a train, in a power cut, at a camp with
  no signal. What he kept still opens. What he read still opens. What was never here is named as
  not here, at once and plainly, and nothing waits on a machine that cannot answer. And when the
  network returns, nothing he must redo stands between him and the rest.

  "The network off" in this story means the laptop truly has no network: no other machine can
  receive anything it sends. Switching the browser to offline is not enough, because the app's
  background worker inside the browser can still reach a machine that is running, and an answer
  from that machine would make a story about having no network pass while the network was there.
  With the network truly off, any attempt to fetch a file from another machine fails, and the
  laptop can see that failure; so "tried to fetch from another machine" is something this story
  checks, not something it guesses at.

  "At once" has a number in this story: with the network off, every answer the laptop gives, whether
  a page, a lesson, or a "not here", comes within 3 seconds. This story sets that budget, not a
  platform guarantee; it is chosen because a laptop waiting for an unreachable machine to answer
  takes far longer than 3 seconds to give up, so an answer that waited cannot pass.

  This story cannot run yet, and every scenario in it is marked @wip: none of its steps has code
  behind it. It waits on two pieces of the platform that are designed but not built. The first makes
  a lesson's files arrive the first time it is opened and stay on the laptop afterwards, which the
  "used as he goes" way needs. The second lets each app's release list every file the app needs to
  start, so that keeping an app means keeping all of those files, which the "kept" way needs. Both
  are described in the design
  genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md,
  sections 12.2 and 12.4, and are slices 3 and 9 of its section 17.

  Background:
    Given Matthew's laptop was on the network while he did each of the following
    And he asked his laptop to keep the apps "evolution-of-trust" and "letter-to-dtpr"
    And he followed the course "Foundations" in the "used as he goes" way, and opened lessons 1 and 2 of its 4 lessons
    And he opened the app "household-ledger" only from the household hub
    And each time he opened "household-ledger", the laptop showed it as not on this device and opened from "household hub"
    And the household hub held every lesson of "Foundations" then, and still does
    And then Matthew's laptop was disconnected from every network, so no other machine can answer it

  @wip
  Scenario: Kept: every app he asked his laptop to keep opens with no network
    When Matthew opens each of the apps he asked his laptop to keep
    Then each kept app shows its first page with visible text within 3 seconds
    And no kept app tried to fetch any of its files from another machine
    And no kept app shows a missing picture, a missing style, or an error for a file it could not load

  @wip
  Scenario: Used as he goes: the lessons he read open, and a lesson he never opened is named as not here instead of hanging
    When Matthew opens the course "Foundations", which he follows in the "used as he goes" way
    Then lessons 1 and 2 are marked as on this laptop, and lessons 3 and 4 as not on this laptop
    When Matthew opens lesson 2
    Then lesson 2 shows its text within 3 seconds
    When Matthew opens lesson 3
    Then within 3 seconds the course says lesson 3 is not on this laptop
    And the course names the household machines it would fetch lesson 3 from, the household hub among them
    And the course shows each machine it names as unreachable now
    And the course stays usable, and Matthew can return to lesson 2

  @wip
  Scenario: Opened from the hub: an app he only ever opened from the hub says it is not on this device, as it always has
    When Matthew opens the app "household-ledger"
    Then within 3 seconds the laptop says "household-ledger" is not on this device and is opened from "household hub"
    And the laptop shows that "household hub" cannot be reached now

  @wip
  Scenario: When the network returns, the lesson he could not open opens with nothing to redo
    Given Matthew has tried to open lesson 3 while offline and been told it is not here
    When Matthew's laptop is connected to the network again
    And Matthew opens lesson 3 once more
    Then lesson 3 shows its text
    And lesson 3 now stays on the laptop like lessons 1 and 2
    And Matthew was asked to do nothing other than open the lesson
