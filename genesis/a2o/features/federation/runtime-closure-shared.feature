# Design: genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md
# (§5.1 code residency, §6 the `closure` and `runtimes` needs, §13 "how many copies", §15 "two apps →
# one runtime copy", §16; held on §17 slices 9 and 10).
# @concern:code-residency-shared-closure is a new tag: no habit in the project's habit register
# (genesis/manifests/habits.yaml) lists this story as its automated check yet.
@e2e @federation @act:i @wip @concern:code-residency-shared-closure @requires:household-nodes
Feature: A second app that runs on the same runtime adds no second copy of that runtime

  A household here is a small group of people and the machines they run together. Each person
  keeps a peer: a program on their own machine that holds their data and does work for them.
  matthew, jessica and james each run one, and the three peers are one household.

  Every app a peer runs needs code. Some of that code belongs to the app alone. Some of it is a
  runtime: a larger body of shared code that many apps run on, published once, under its own
  name, in versions of its own, which are numbered. When last measured, in October 2026, the code
  on this household's peers took about ten times the storage space of the data it serves. A
  machine people actually own, a laptop or a small box at home, has only so much space, and every
  extra copy of a runtime is space taken from the people's own things.

  Files here are named by a fingerprint of their exact bytes, called their address, so two copies
  of the same file are the same file, and a peer stores a file once however many things need it.
  Each app carries a declaration: a file that ships inside every release of the app. It lists, by
  address, the files the app needs in order to start, and it names the runtime the app runs on.
  Each version of a runtime is likewise a fixed list of file addresses. A runtime is published on
  release channels: a channel is a named feed whose latest entry says which version of the
  runtime is in use. The household chooses one channel of each runtime to follow, and every peer
  in the household follows that same channel, so all three peers agree on which version is in
  use. An app that names the runtime without naming a version runs on the version the
  household's channel gives; an app that names a version runs on that version. Two apps that both
  name the runtime without a version therefore run on the same version, and so name the same
  files.

  Each peer keeps a holds report: a list the peer gives only to its own person, of everything it
  stores and the reason it keeps each piece. For code, the holds report shows the total size, in
  bytes, of all the code files the peer stores, and names, for each app and each runtime version,
  the reasons it is kept, which are the apps that need it. That reported total is what this story
  measures. When no reason remains, the peer lets the piece go and the total falls by that piece's
  size.

  This story describes behaviour that is not built yet. Today a peer already stores an identical
  file once, and its holds report already names a reason for each piece of stored data. But no app
  can yet declare the runtime it runs on, so nothing can tell that two apps share one, and the
  holds report cannot yet name an app as the reason a piece of code is kept. The story is held
  until stages 9 and 10 of the design's build order land: stage 9, "the manifest needs", which
  lets an app declare its starting files and its runtimes, and stage 10, "the runtime redesign",
  which changes how runtimes are installed so that apps share them.

  Background:
    Given a household of three peers, run by matthew, jessica and james
    And each household peer stores every file once under its address, so a file that several apps or runtimes need is one copy on that peer
    And a runtime called the kitchen runtime, the shared code the household's food apps run on, published on a release channel that the household follows
    And that channel's latest entry names version 2 of the kitchen runtime
    And every household peer follows that channel
    And so, on every household peer, an app that names the kitchen runtime without naming a version runs on version 2
    And two apps, the recipe book and the meal rota, each declaring that it runs on the kitchen runtime without naming a version
    And a third app, the garden planner, declaring that it runs on version 3 of the kitchen runtime
    And version 3 of the kitchen runtime shares most of its files with version 2, and has only a few files that version 2 does not have

  Scenario: A second app that names the same runtime adds only its own files
    Given each household peer holds the recipe book and the kitchen runtime it runs on
    When matthew, jessica and james each install the meal rota on their own peer
    Then on matthew's, jessica's and james's peers the total code size the holds report shows grows by exactly the size of the meal rota's own files
    And each of those peers holds one copy of version 2 of the kitchen runtime
    And each peer's own holds report names both apps as the reasons it keeps version 2 of the kitchen runtime

  Scenario: Uninstalling one of two apps that share a runtime keeps the runtime
    Given james's peer holds the recipe book, the meal rota and one copy of version 2 of the kitchen runtime
    And james's peer holds no other app, and no other version of the kitchen runtime
    When james uninstalls the recipe book
    Then james's peer lets go of the recipe book's own files, and the total code size its holds report shows falls by exactly the size of those files
    And james's peer keeps version 2 of the kitchen runtime, and its holds report names the meal rota as the one reason left to keep it

  Scenario: Uninstalling the last app that needs a runtime lets the runtime go
    Given james's peer holds the meal rota and one copy of version 2 of the kitchen runtime
    And james's peer holds no other app, and no other version of the kitchen runtime
    And james's peer's holds report names the meal rota as the only reason to keep version 2 of the kitchen runtime
    When james uninstalls the meal rota
    Then james's peer's holds report names no reason left to keep version 2 of the kitchen runtime
    And james's peer lets go of the meal rota's own files and of every file of version 2 of the kitchen runtime
    And the total code size james's peer's holds report shows falls by exactly the size of those files together, each file counted once

  Scenario: An app on a newer version of the runtime costs only the files that version does not share with the older one
    Given jessica's peer holds the recipe book and version 2 of the kitchen runtime
    When jessica installs the garden planner
    Then jessica's peer holds both version 2 and version 3 of the kitchen runtime
    And the total code size its holds report shows grows by exactly the size of the garden planner's own files plus the size of the files version 3 has that version 2 does not
    And the files version 2 and version 3 share are held once
