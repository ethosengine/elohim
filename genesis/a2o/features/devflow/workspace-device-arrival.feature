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
  #     the new device ENROLLS (mints its key and prints an enrollment request — its public key and
  #     a one-time number — for the person to hand to another device of theirs); the other
  #     device AUTHORIZES that request (prints an approval it signed); the new device BINDS the
  #     approval (countersigns and appends the binding row). The person carries the two notes
  #     by hand; only public material is in them, and the joining scenario ("A second device
  #     joins…") proves it.
  #   - "Arrival" is the state in which a workspace has everything it needs to act as a device.
  #     The "arrival preflight" (`just status device`) reads eight conditions and prints one line
  #     per condition — `ok <condition>: …` or `degraded|REFUSED|unknown <condition>: <why> — <cure>`:
  #        epr binary    the native tool `epr` is reachable
  #        brit binary   followed tool or explicit local candidate
  #        repository trusted  git accepts this checkout natively or by device declaration
  #        device key    this device's signing key exists on its persistent home
  #        roster bound  the person's roster lists this device
  #        embed model   the model that turns text into searchable vectors is in one of its
  #                      declared places (the paths its manifest lists, checked in order) — a
  #                      missing semantic search is degraded; lexical recall still works
  #        fold attested the device has run that model over the repository's text into its own
  #                      local search index (the "fold"), and the index covers all but a
  #                      declared small number of recently changed files
  #        berth moored  the workspace has registered itself with the machine's berth — the
  #                      scheduler that shares heavy work (builds, the mesh) between the
  #                      sessions on one machine — so it can claim capacity instead of competing
  # Contract: status never fixes anything; it exits 1 only for REFUSED. Degraded means a missing
  # enhancement; unknown means an unreachable cross-device fact. Session arrival fails open. The
  # cure it names IS the complete ceremony: enroll here, authorize on the other device, bind
  # here. A pending request identifies authorization as the next step; running
  # the ceremony is a person's act. A missing key or binding is refused by name below.
  # Optional model, fold and berth gaps are degraded: core work can still proceed.
  # The last scenario checks all eight lines on a bound fixture with local tool candidates.

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

  # The cure names the complete sequence; once a request exists, authorization is next.
  Scenario: After enrolling, the refusal names the next step and the device to run it on
    Given this workspace holds no device key
    When this workspace enrolls for "matthew" and receives an enrollment request to hand to the other device
    And the arrival preflight runs here
    Then the "device key" line reads ok
    And the "roster bound" line is REFUSED and names "epr actor device authorize"
    And that cure is addressed to a device already on the roster
    And the "roster bound" line also names "epr actor device bind"

  Scenario: A second device joins through its person's other device, and no key crosses
    Given this workspace holds no device key
    When this workspace enrolls for "matthew" and receives an enrollment request to hand to the other device
    And the other device authorizes that request
    And this workspace binds the authorization it was handed back
    Then matthew's roster carries one binding for this workspace approved by the other device
    And the two devices hold different keys and neither key file moved
    And neither the request nor the authorization carries either device's secret key
    And the "roster bound" line of the arrival preflight reads ok here

  Scenario: A model missing from its declared places degrades recall and names its provisioner
    Given no declared place holds the embedding model
    When the arrival preflight runs here
    Then the "embed model" line is degraded and names "genesis/agentic/bin/embed-model-provision"

  Scenario: A locally usable workspace permits degraded enhancements and exits zero
    Given the core arrival conditions are met on this workspace
    Then every line reads ok or degraded and the preflight exits zero
