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
  #     The "arrival preflight" (`just status device`) reads a growing list of conditions and prints one line
  #     per condition — `ok <condition>: …` or `degraded|REFUSED|unknown <condition>: <why> — <cure>`. The
  #     conditions these scenarios name:
  #        epr binary    the native tool `epr` is reachable
  #        brit binary   followed tool or explicit local candidate
  #        repository trusted  git accepts this checkout natively or by device declaration
  #        device key    this device's signing key exists, and whether a native session
  #                      (below) could be kept beside it, on which basis
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
  # "A locally usable workspace…" checks that no printed line refuses, whatever the list has
  # grown to, on a bound fixture with local tool candidates.
  #
  # Where a device keeps its things (the "device layout", `epr device layout show`): the
  # config home (key, roster pins, sessions), the bin-store (the executables `epr` lands and
  # runs), the cargo pool (shared build output, when the device has one) and the checkout. A
  # workspace container keeps them under its `/projects` tree; a laptop has no such tree and
  # keeps them under its own home. Each place is chosen in one order: a variable the person
  # set, else a place the device remembered (`epr device layout save`), else the host's default.
  # The checkout the person stands in always beats a remembered one.

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

  Scenario: Every place this device keeps its things is named with the reason it was chosen
    Given this workspace names its own place for executables
    When this device's layout is shown
    Then the executables are kept in the place this workspace named, chosen by "BIN_STORE"
    And the checkout named is the repository this workspace stands in, found where it stands
    And the config place is under this workspace's own config home

  Scenario: A remembered place gives way to one the person names, and the difference is said, not refused
    Given this device remembered a place for its executables
    And this workspace names a different place for executables
    When this device's layout is shown
    Then the executables are kept in the place this workspace named, chosen by "BIN_STORE"
    And the remembered place is reported beside it as a difference
    And showing the layout succeeds

  # A device's home is durable or ephemeral, and the native tool is told which, never guesses.
  # A "native session" (`epr session open`) is the device's record of one agent run: who is
  # acting, for which person, from which checkout. Its records (the "envelope") live beside the
  # device key and must outlive a restart, so opening one is where persistence is decided. The
  # arrival preflight asks the same question without opening anything: its "device key" line
  # names the basis a session would open on, or says that none can open and how to cure it.
  # The session names the "basis" it was accepted on: "explicit-mount" (the key lives in a
  # persistent directory outside $HOME) or "declared-durable-home" (below). Its refusals carry
  # stable codes beginning `envelope-`.
  # A workspace container's home ($HOME) is wiped when the workspace restarts, so by default a
  # key kept under $HOME cannot hold a native session: the session would vanish with it. A
  # laptop's home survives restarts. Its person says so once with
  # `epr actor device persistence --declare durable-home`, which writes `persistence.json`
  # beside the key: a note the key itself signs, naming that key and the directory the key
  # lives in. A note that names another key or another directory, or that has been edited, no
  # longer matches its signature and is refused. A container platform that announces itself
  # (an Eclipse Che workspace sets DEVWORKSPACE_ID) is refused a home key even when such a note
  # is present; its key belongs in the persistent directory its platform mounts outside $HOME.

  # A device can stand for its person and still be unable to keep a session: nothing has yet
  # said its home survives a restart. It can act, so the preflight degrades rather than refuses.
  Scenario: A bound device whose home is not yet declared durable is told how, and declaring it clears the line
    Given a device whose key lives under a home that survives restarts
    And the device stands for "matthew" on the roster
    And no container platform marker is set
    When the arrival preflight runs on that device
    Then the "device key" line is degraded and names "epr actor device persistence --declare durable-home"
    And the "roster bound" line reads ok
    When the person declares the home durable on that device
    And the arrival preflight runs on that device again
    Then the "device key" line reads ok and names the persistence basis "declared-durable-home"

  Scenario: A device whose home survives restarts declares it once and opens a native session
    Given a device whose key lives under a home that survives restarts
    And the device stands for "matthew" on the roster
    And no container platform marker is set
    When the person declares the home durable on that device
    Then a persistence declaration signed by the device key sits beside the key
    And declaring a second time reports the existing declaration and rewrites nothing
    And a native session opens on the device with the basis "declared-durable-home"

  Scenario: A container workspace whose home is wiped on restart is refused even with a declaration
    Given a workspace whose key lives under $HOME and whose platform sets DEVWORKSPACE_ID
    And the workspace stands for "matthew" on the roster
    And a persistence declaration signed by that key sits beside the key
    When a native session is opened there
    Then it is refused with "envelope-home-ephemeral-platform"
    And the refusal names setting XDG_CONFIG_HOME to a persistent directory outside $HOME
    And no session record was written
    And declaring the home durable there is refused the same way

  Scenario: A declaration copied from another device or another directory is refused
    Given a device whose key lives under a home that survives restarts
    And the device stands for "matthew" on the roster
    And the persistence declaration beside its key was copied from another device
    When a native session is opened there
    Then it is refused with "envelope-persistence-declaration-invalid" naming the device mismatch
    When the key and its declaration are instead moved together into another directory
    And a native session is opened from that directory
    Then it is refused with "envelope-persistence-declaration-invalid" naming the directory mismatch
    And no session record was written in either place
