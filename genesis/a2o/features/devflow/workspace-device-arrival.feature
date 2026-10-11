@e2e @devflow @concern:workspace-device-arrival @act:host @requires:epr-cli
Feature: A workspace arrives as one of its person's devices, and a refusal names its cure
  As a person who opens a fresh development workspace on a second machine
  I want the workspace to become one of my devices through a short ceremony my other device approves
  So that what I remember and write there is mine, without my key ever leaving the machine it was made on

  # Spec (design rationale, supplementary): elohim/lvi/docs/specs/2026-10-10-workspace-arrival-as-declared-device.md
  #   (lvi is the devspace peer-runtime this workspace model is heading toward)
  # Vocabulary a cold reader needs:
  #   - "matthew" is the fixture persona (the person whose devices these are).
  #   - A "device" is a machine, or a workspace container, that holds its own signing key. The
  #     key is made on the device and never copied. A device "stands for" a person once the
  #     person's roster lists it: it may then act in their name. Before that it stands for
  #     nobody, and the native tool says so rather than guessing.
  #   - The "roster" is the per-person list of devices that stand for them, kept as a tracked
  #     file (`.eprfs/status/participants/<handle>.jsonl`, one JSON row per line) so it travels
  #     with the repository. Its first row is written on the person's first device; every later
  #     row is a "binding": a row that pairs a new device's public key with the approval
  #     signature of a device already on the roster, signed by both.
  #   - The ceremony that produces a binding is three commands of the native tool, `epr`:
  #     the new device ENROLLS (mints its key and prints a signed request — its public key and
  #     a one-time number — for the person to hand to another device of theirs); the other
  #     device AUTHORIZES that request (prints an approval it signed); the new device BINDS the
  #     approval (countersigns and appends the binding row). The person carries the two notes
  #     by hand; only public material is in them, and the joining scenario ("A second device
  #     joins…") proves it.
  #   - "Arrival" is the state in which a workspace has everything it needs to act as a device.
  #     The "arrival preflight" (`just status device`) reads six conditions and prints one line
  #     per condition — `ok <condition>: …` or `REFUSED <condition>: <why> — <cure>`:
  #        epr binary    the native tool `epr` is reachable
  #        device key    this device's signing key exists on its persistent home
  #        roster bound  the person's roster lists this device
  #        embed model   the model that turns text into searchable vectors is in one of its
  #                      declared places (the paths its manifest lists, checked in order) — a
  #                      device that cannot search what its person wrote is not yet theirs
  #        fold attested the device has run that model over the repository's text into its own
  #                      local search index (the "fold"), and the index covers all but a
  #                      declared small number of recently changed files
  #        berth moored  the workspace has registered itself with the machine's berth — the
  #                      scheduler that shares heavy work (builds, the mesh) between the
  #                      sessions on one machine — so it can claim capacity instead of competing
  # Contract: the preflight never fixes anything, and a refusal never blocks other work. The
  # cure it names IS the ceremony, and it names the next step from wherever the person is (before
  # enrolling: enroll; after enrolling: authorize on the other device, then bind here); running
  # the ceremony is a person's act. Three conditions are refused by name below; the other three
  # (epr binary, fold attested, berth moored) are read on a real workspace by `just status
  # device` and are owed a fixture (last scenario).

  Background:
    Given a committed repository whose roster for "matthew" was begun on another device

  Scenario: A workspace with no key is refused by name, and nothing else is blocked
    Given this workspace holds no device key
    When the arrival preflight runs here
    Then the "device key" line is REFUSED and names "epr actor device enroll --handle matthew"
    And the "roster bound" line is REFUSED and names "epr actor device authorize"
    And that cure is addressed to a device already on the roster
    And the preflight exits non-zero
    But epr still answers, and says this device stands for nobody yet

  # A cure names only what is actionable from the current state: before enrolling, enroll;
  # once a request exists, authorize on the other device and then bind here.
  Scenario: After enrolling, the refusal names the next step and the device to run it on
    Given this workspace holds no device key
    When this workspace enrolls for "matthew" and receives a signed request to hand to the other device
    And the arrival preflight runs here
    Then the "device key" line reads ok
    And the "roster bound" line is REFUSED and names "epr actor device authorize"
    And that cure is addressed to a device already on the roster
    And the "roster bound" line also names "epr actor device bind"

  Scenario: A second device joins through its person's other device, and no key crosses
    Given this workspace holds no device key
    When this workspace enrolls for "matthew" and receives a signed request to hand to the other device
    And the other device authorizes that request
    And this workspace binds the authorization it was handed back
    Then matthew's roster carries one binding for this workspace approved by the other device
    And the two devices hold different keys and neither key file moved
    And neither the request nor the authorization carries either device's secret key
    And the "roster bound" line of the arrival preflight reads ok here

  Scenario: The model is found by its declared places, and a missing model names its provisioner
    Given no declared place holds the embedding model
    When the arrival preflight runs here
    Then the "embed model" line is REFUSED and names "genesis/agentic/bin/embed-model-provision"

  @wip
  Scenario: A workspace that has fully arrived prints six ok lines and exits zero
    # Owed: a fixture whose local search index is complete within its freshness limit, so the
    # three lines not exercised above (epr binary, fold attested, berth moored) are proven here
    # too. Today the six-ok case is read on a real workspace (`just status device` on shem,
    # 2026-10-10) rather than proven in an isolated fixture, because the fold needs the pinned
    # model bytes.
    Given every arrival condition is met on this workspace
    When the arrival preflight runs here
    Then every line reads ok and the preflight exits zero
