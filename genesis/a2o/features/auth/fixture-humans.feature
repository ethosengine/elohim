@e2e @auth @fixture-sweep @hosted-human @requires:doorway @requires:seeded-humans
Feature: Who may sign in at a doorway
  A doorway is the sign-in gateway between a person's browser and the
  network. Everyone on the network has a cell: their own running instance
  of the application, which costs the computer running it real memory.
  The people in these scenarios are fixture humans: named personas
  seeded for testing. Each one declares a stage, a household and whether
  they have a runtime of their own, and every scenario opens with a table
  of exactly that, checked against the fixture data before anyone signs in.

  A stage is one of: "device" or "node" (they run their own cell),
  "doorway" (they run their own cell and also operate the doorway, which
  changes nothing about how they sign in), "hosted"
  (the doorway's operator runs their cell), or "none". Stage alone
  decides who may sign in. "Own runtime" says what is true of them today:
  "running", "switched off" (they have one, but it is not running), or
  "none" (they have never had one). Household makes no difference either:
  the first and third scenarios give the same outcome to people with a
  running runtime of their own, whatever their household. The cast has one
  doorway operator (Matthew) and one node human (Adam). The hosted newcomers here have not joined a
  household yet; joining one would not change how they sign in.

  A doorway lets two kinds of fixture human sign in, and nobody else.

  A human with their own runtime (a device or node of their own, running
  their cell) signs in through the doorway, but the doorway runs nothing for
  them. It keeps an account and hands them to their own runtime. While
  their runtime is switched off there is nothing to hand them to,
  so they are refused; once it is running again they sign in as before.

  A human at the "hosted" stage has no runtime yet. The doorway's operator
  runs their cell for them, on the operator's own computer. Doorway
  "alpha" is operated by Matthew, the one human at the "doorway" stage,
  who also signs in himself.

  Anyone whose fixture declares no stage has no account and is refused.
  Each refusal step makes a real sign-in attempt and expects it to fail. A
  doorway never hosts someone by default: default hosting is how one
  household computer once ended up running cells for twenty people who
  were not its own.

  Each sign-in gets its own token, so no two people ever share a session.

  The act tags name where a scenario can run. Act:i runs on the Dowell
  household alone (Matthew, Jessica and James's own machines). Act:iii
  needs the wider network of other households' devices, which
  runs on the remote cluster named shem. The Background names the doorway
  and the environment variable, set by the test harness, that holds its
  address.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  @act:i
  Scenario: The household signs in and the doorway runs no cell for them
    Given these fixture humans declare:
      | human   | stage   | household        | own runtime |
      | Matthew | doorway | household-dowell | running     |
      | James   | device  | household-dowell | running     |
      | Jessica | device  | household-dowell | running     |
    When human "Matthew" is logged in on doorway "alpha"
    And human "James" is logged in on doorway "alpha"
    And human "Jessica" is logged in on doorway "alpha"
    Then all 3 humans should have distinct tokens
    And doorway "alpha" runs no cell for "Matthew"
    And doorway "alpha" runs no cell for "James"
    And doorway "alpha" runs no cell for "Jessica"

  @act:i
  Scenario: Newcomers at the hosted stage sign in and the doorway runs their cells
    Given these fixture humans declare:
      | human  | stage  | household | own runtime |
      | Maria  | hosted | none      | none        |
      | Ronald | hosted | none      | none        |
      | Ezra   | hosted | none      | none        |
    When human "Maria" is logged in on doorway "alpha"
    And human "Ronald" is logged in on doorway "alpha"
    And human "Ezra" is logged in on doorway "alpha"
    Then all 3 humans should have distinct tokens
    And doorway "alpha" runs a cell for "Maria"
    And doorway "alpha" runs a cell for "Ronald"
    And doorway "alpha" runs a cell for "Ezra"

  @requires:shem @act:iii
  Scenario: People from other households sign in and the doorway runs no cell for them
    Given these fixture humans declare:
      | human    | stage  | household          | own runtime |
      | Susan    | device | household-susan    | running     |
      | Gertrude | device | household-gertrude | running     |
      | Eve      | device | household-eden     | running     |
      | Adam     | node   | household-eden     | running     |
    When human "Susan" is logged in on doorway "alpha"
    And human "Gertrude" is logged in on doorway "alpha"
    And human "Eve" is logged in on doorway "alpha"
    And human "Adam" is logged in on doorway "alpha"
    Then all 4 humans should have distinct tokens
    And doorway "alpha" runs no cell for "Susan"
    And doorway "alpha" runs no cell for "Gertrude"
    And doorway "alpha" runs no cell for "Eve"
    And doorway "alpha" runs no cell for "Adam"

  @act:i @regression
  Scenario: Personas that declare no stage are refused, a sample across the cast
    Given these fixture humans declare:
      | human        | stage | household | own runtime |
      | Charlie      | none  | none      | none        |
      | Sam          | none  | none      | none        |
      | Dr. Dolittle | none  | none      | none        |
      | Pam          | none  | none      | none        |
      | Tommy        | none  | none      | none        |
      | Meriadoc     | none  | none      | none        |
      | Georgina     | none  | none      | none        |
      | Manny        | none  | none      | none        |
      | Bub          | none  | none      | none        |
    Then human "Charlie" is refused sign-in on doorway "alpha"
    And human "Sam" is refused sign-in on doorway "alpha"
    And human "Dr. Dolittle" is refused sign-in on doorway "alpha"
    And human "Pam" is refused sign-in on doorway "alpha"
    And human "Tommy" is refused sign-in on doorway "alpha"
    And human "Meriadoc" is refused sign-in on doorway "alpha"
    And human "Georgina" is refused sign-in on doorway "alpha"
    And human "Manny" is refused sign-in on doorway "alpha"
    And human "Bub" is refused sign-in on doorway "alpha"

  @act:i @regression
  Scenario: People whose own runtime is switched off are refused, not hosted instead
    Given these fixture humans declare:
      | human    | stage  | household | own runtime  |
      | Terrance | device | none      | switched off |
      | Nancy    | device | none      | switched off |
      | Pete     | device | none      | switched off |
      | Frank    | device | none      | switched off |
    Then human "Terrance" is refused sign-in on doorway "alpha"
    And human "Nancy" is refused sign-in on doorway "alpha"
    And human "Pete" is refused sign-in on doorway "alpha"
    And human "Frank" is refused sign-in on doorway "alpha"
