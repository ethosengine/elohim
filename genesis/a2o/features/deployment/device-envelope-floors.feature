@e2e @deployment @substrate-floor @device-envelope @wip
Feature: Device-envelope CPU floors keep small peers reaching for the mesh, loudly
  As an operator stewarding a fleet of household-grade devices
  I want each device archetype's CPU floor to be an evidenced value — shown
  to let a peer reach for the mesh, sitting above a value shown to fail
  So that starvation is visible before a household's backups silently stop —
  never again a peer passing every liveness gate while outside the network

  Vocabulary, once: an archetype's *floor* is the smallest CPU cap a node of
  that archetype may be deployed with. Caps are cgroup limits (ceilings), so
  raising the floor GRANTS more CPU, not less. A below-floor cap is still
  sanctioned when a deployment record carries an explicit resourceOverride
  with justification (records live in
  genesis/orchestrator/data/deployments.json); the contract scenario binds
  only the no-override path. 2000m allocates two of the device's four cores
  to the elohim-node container — the conductor+storage stack's share,
  leaving the rest for the household's own use of the device.

  Susan is a persona fixture: sister of Matthew (the fleet's operator
  persona). Her household node runs two services: a conductor — the
  Holochain runtime that holds the household's data and gossips it through
  the DHT mesh, the peer-to-peer network — and elohim-storage, whose health
  endpoint is what liveness probes actually read. Her household is one leg
  of a three-household reciprocal backup chain: Matthew's in Texas, Susan's
  in the Pacific Northwest, Gertrude's (an elder neighbor's household). In
  the alpha fleet her node is a container on shem, a shared remote host,
  declared as the "device-recycled-laptop" archetype: a 4-core recycled
  laptop, the typical-steward floor device. The container cap stands in for
  the scheduling starvation a genuinely loaded physical device suffers: the
  floor is the provisioning contract for simulated nodes and the sizing
  guidance for real ones. While her node was silently outside the mesh (the
  discovery below), her household's content was not replicating to or from
  her backup partners, and nothing anyone could see said so.

  What this feature proves, and what it does not: these scenarios prove the
  UPSTREAM stations of participation — gossip initiation attempts and peer
  discovery — plus the contract and the operator surface around them. They
  deliberately stop short of a completed gossip round (owned by the
  relay-registration seam's story) and of content actually replicating
  between backup partners (owned by the resilience/dataplane stories). A
  green run here means Susan's node is trying and can see peers — not yet
  that her backups are flowing. The floor method is bracketing, not
  minimization: 1000m is shown to fail and 2000m to work; the true minimum
  may sit between, and moving the floor down again requires new evidence
  (the budget file's own law). The fix specified here has two parts: raise
  the floor (recorded in the budget file), and make starvation visible — a
  per-node gossip-initiation attempt metric and a fleet mesh-participation
  report over it, both specified into being by the report scenario (fix-
  shape claimed by the edge-deploy-ready-gate-liveness-only backlog item).
  Liveness gates are deliberately NOT being taught to detect starvation;
  the report is the surface that must. Scenarios run against fixture
  deployments in the alpha test fleet — every household here is a persona —
  and any below-floor override a scenario installs is reverted after its
  window.

  The discovery this preserves (2026-08, live alpha): susan's node ran at the
  archetype's then-canonical 1000m CPU limit (one core, in millicores), hit
  sustained CFS throttle (the kernel's Completely Fair Scheduler pausing the
  container at nearly every enforcement period), and her conductor's gossip
  layer (kitsune2) never populated a peer store — so gossip initiation was
  silently skipped, logged at debug level only, for weeks, while her storage
  health endpoint stayed green and edge deploys reported "7/7 peers Ready".
  Zero gossip ATTEMPTS is a distinct, upstream failure mode from
  attempt-and-fail relay errors: a starved peer never discovers anyone to
  dial. The behaviour was observed live by log archaeology; the scenarios
  here are its later transcription and are not yet wired to run. The
  2026-08-09 class realignment raised the archetype floor to 2000m on this
  evidence. The 1000m/2000m literals below are the FROZEN 2026-08 EVIDENCE
  BRACKET — they intentionally do not track the budget file
  (genesis/data/devices/archetype-resource-budgets.json, the single source
  of the *currently declared* floors, which the contract scenario reads
  live).

  Background:
    Given Susan's household node runs on shem, declared as the "device-recycled-laptop" archetype
    And at least one other household node is running and reachable through the fleet's discovery path (the bootstrap service peers use to find each other)
    And gossip-initiation logs, at debug level, and cgroup CPU-throttle counters (nr_periods / nr_throttled) are sampled as deltas over each scenario's observation window

  @regression @requires:shem
  Scenario: A CFS-throttled peer goes silently dark on gossip while liveness surfaces stay green by design
    Given Susan's deployment record carries an explicit "1000m" CPU override, below the 2026-08 evidence bracket's "2000m" working value
    When her node is restarted at that limit and a "5 minute" observation window elapses from conductor startup
    Then her CPU-throttle counters for the window show throttled periods at or above "95%" of enforcement periods
    And her gossip log for the window shows zero kitsune2 initiation attempts
    But her storage health endpoint still reports healthy
    And the deploy-time readiness gate, which reads only that health endpoint, still counts her node Ready
    And neither of those two surfaces distinguishes her from a gossip-initiating node
    # The last three steps pin the gap the report scenario must close: two
    # surfaces stay green by design, and the absence of a discriminating
    # surface is asserted, not tolerated. Throttle basis: at the live
    # incident the ratio was 100%; 95% grants counter jitter while staying
    # far above a busy-but-scheduled node. The Background's discoverable
    # peer makes the zero a real negative (a counterpart existed to find).
    # If zero-attempts stops reproducing at 1000m, the constraint may have
    # MOVED (discovery got cheaper, or the runtime learned to degrade
    # loudly) — revisit the floor before deleting this anchor.

  @requires:shem
  Scenario: At the evidence bracket's 2000m working value, Susan's node attempts gossip and discovers a peer
    Given Susan's deployment record carries an explicit "2000m" CPU override — the evidence bracket's working value, pinned independently of the currently declared floor
    When her node is restarted at that limit and a "5 minute" observation window elapses from conductor startup
    Then her gossip log for the window shows at least "30" kitsune2 initiation attempts
    And her peer store contains at least one peer from another node
    # Threshold basis: kitsune2 retries initiation every 1-5s, so a healthy
    # node produces ≥60 attempts in 5 minutes; 30 grants startup jitter a
    # 50% margin while staying orders above the starved case (zero). Which
    # peers (her named backup partners) and whether content flows are the
    # downstream stations named in the preamble — not claimed here.

  @requires:shem
  Scenario: A deployed archetype inherits its floor from the budget contract
    Given Susan's deployment record declares the "device-recycled-laptop" archetype with no resource override
    And the fleet renders per-node manifests from the shared edgenode template, which stamps each archetype's declared resource limits
    When her node's manifest is rendered and deployed
    Then her container's CPU limit equals the floor the archetype currently declares in archetype-resource-budgets.json (on the no-override path the declared floor is also the stamped cap)
    And that declared floor is at or above the evidence bracket's "2000m" working value
    # The contract half — exercises the render path, not the observation
    # window. The outage was produced by a mis-set canonical, so proving
    # the floor value right is only half the guard: the fleet must be shown
    # to HONOR the contract end-to-end (budget file → template render →
    # running container) on the no-override path this Given fixes.
    # validate:deployments (the pre-push conformance check) gates the
    # declared side; this proves the deployed side. The fleet-wide aim — no
    # household silently below its archetype's promise — is delivered one
    # node at a time by this contract plus the report below.

  @requires:shem
  Scenario: An operator can distinguish Ready from gossip-initiating
    Given each node exports its gossip-initiation attempt count as a runtime metric, computed over a "5 minute" window since its last restart
    And Susan's node, deployed at "1000m", has exported an attempt count of "0" for its window
    And Gertrude's node — one of Susan's two backup-partner households, also shem-hosted, at her own archetype's floor — attempts gossip normally
    And both nodes report healthy on their liveness probes
    When the operator runs the fleet mesh-participation report
    Then the report lists each node's exported gossip-initiation attempt count for the window
    And Susan's node's row shows attempt count "0" and gossip-initiating "false"
    And Gertrude's node's row shows a nonzero attempt count and gossip-initiating "true"
    # The report REVEALS Susan's zero — the count arrives through the
    # exported metric from her recorded window, not injected by the test.
    # The column is named for exactly what it measures — gossip-initiating,
    # derived as attempt count > 0 — NOT "participating": attempts are the
    # station this feature proves, and a bolder label would ship a new
    # reassuring-green that outruns its evidence (the original sin, in a
    # new column). Gertrude's different archetype (device-home-nuc) is
    # incidental: the pair discriminates the SURFACE, not the cause. That
    # the truthful witness is one of Susan's own backup partners is the
    # chain's point: partners can only cover for a household they can
    # actually reach.

  # Maintainer trailer — context for preset work, not under test here:
  # memoryLimit 3Gi and the 4-core physical envelope bound this archetype
  # but are not exercised by these scenarios. Informs: archetype presets,
  # the planned operational envelope derivation
  # (genesis/plans/2026-04-13-device-archetypes-design.md), operator sizing
  # docs. Review after: conductor major bumps, gossip-stack changes,
  # container-runtime CPU accounting changes. Backlog homes:
  # genesis/data/timeline/backlog/susan-kitsune2-gossip-never-attempts.md,
  # genesis/data/timeline/backlog/edge-deploy-ready-gate-liveness-only.md.
