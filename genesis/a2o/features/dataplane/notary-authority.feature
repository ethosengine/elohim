# RED-FIRST: the notary-overlay (Plan C1-C5) is unwired. These scenarios ARE the
# schedulable red for spine node `notary-authority`. Two invariants, honestly failing
# today against real HTTP surfaces on the live alpha federation:
#   1. Re-staged content reaches trust=notarized on EVERY federation peer — today the
#      DHT-notary HEAD anchor stamps on the author peer (alpha-A) but never propagates
#      to the alpha-b federation peer (elohim.host), which serves the same bytes at the
#      lower "published" tier.
#   2. A non-author cannot move HEAD — today there is no notary-arbitrated HEAD authority
#      surface at all (declare_content_head, Plan C1/C3, returns 404), so the served HEAD
#      advances by last-writer recency (LWW), not by the notary. Nothing refuses a non-author.
#
# WHY THIS MATTERS (the whole point of the notary): the CRDT plane converges the *value*
# like HTTP moves bytes; the DHT notary adds the *authority* like HTTPS adds a certificate.
# Without the notarized HEAD, "whoever wrote last" wins — grandma's photo caption, an
# EPR's blob, a household's shared page can be silently rewritten by any peer on the wire
# and every reader would serve the overwrite as truth. The notarized green tier is the
# padlock: it says *this* version is the one the author declared and the community witnessed,
# and no non-author can move it. Converged-but-unconfirmed content is still served (an app
# beats a 404), but it is served as "unconfirmed" and is NEVER consumed for authority.
#
# Design:  genesis/docs/superpowers/specs/2026-07-01-crdt-authoritative-content-state-dht-notary-decouple-design.md
# Plan:    genesis/docs/superpowers/plans/2026-07-01-crdt-content-dataplane-full1c-implementation-plan.md (C1-C5)
# Spine:   genesis/manifests/spine.yaml — node `notary-authority`
#
# Live state observed 2026-07-03:
#   alpha-A      /db/content/elohim-host-landing.trust        = "notarized"  dhtAnchorHash = uhCkk0oj2X…  (green — author peer, conductor up)
#   elohim.host  /db/content/elohim-host-landing.trust        = "published"  dhtAnchorHash = null          (the gap — notarization never propagated)
#   alpha-A      /db/content/elohim-host-landing/head         → HTTP 404      (declare_content_head unwired — no HEAD authority gate)
@e2e @dataplane @concern:notary-authority
Feature: Notary authority — notarized HEAD reaches every peer, and only the author can move it
  Converged content state (the value) lives on the CRDT plane; the DHT notary witnesses
  authority, provenance, and HEAD-of-DAG selection over it. This feature asserts the two
  things that make the notary the notary: that the green (notarized) trust tier is reachable
  for content on every federation peer — not just the peer whose conductor happened to run
  the deploy — and that moving the declared HEAD is an author-only act the notary arbitrates,
  never a race won by whoever wrote last.

  Both notarization invariants are RED today because the notary overlay (Plan Phase C:
  declare_content_head, HEAD-election-replaces-recency, and the green-over-amber serving
  path) is not yet wired. The failures below are the deliverable: they name exactly what
  "notarized" must come to mean, and they pass unchanged the moment it does.

  Background:
    Given peer "alpha-A" at "alpha-A"
    And peer "elohim.host" at "elohim.host"

  @requires:multi-node
  Scenario: The author peer notarizes the re-staged landing EPR (baseline — green)
    # Green baseline. alpha-A is the deploy-time author peer; its conductor is reachable,
    # so the re-staged landing EPR was re-authored through update_via_conductor and the
    # ContentCommitted signal stamped dht_anchor_hash onto the serving row. The content
    # view therefore reports trust="notarized" — the padlock is lit on the author peer.
    # This confirms the notary path works where the conductor is present; the gap is
    # purely that it does not reach the federation peer (next scenario).
    Then EPR "elohim-host-landing" trust is "notarized" on peer "alpha-A"

  @requires:multi-node
  Scenario: The federation peer reaches trust=notarized for the re-staged landing EPR (RED — the propagation gap)
    # RED-FIRST. elohim.host is the alpha-b federation peer. It holds the SAME converged
    # bytes for elohim-host-landing (blobHash matches alpha-A) and has enough peer-attested
    # custody to serve them, so it reports trust="published" (blue) — served, but not
    # notarized. The DHT-notary HEAD anchor (dht_anchor_hash) that alpha-A carries never
    # propagated here: there is no cross-peer re-notarization overlay, so the padlock stays
    # dark on the federation peer even though the content is byte-identical.
    #
    # This scenario FAILS today with:
    #   EPR "elohim-host-landing" on elohim.host: trust is "published", expected "notarized"
    #   — the DHT-notary HEAD anchor has not reached this peer.
    #
    # When it passes, the notarized HEAD has propagated to every federation peer (Plan C1/C3/C4),
    # so a reader on elohim.host gets the same green authority answer as one on alpha-A —
    # authority is uniform across the federation, not a privilege of the peer that ran the deploy.
    Then EPR "elohim-host-landing" trust is "notarized" on peer "elohim.host"

  Scenario: A non-author cannot move the declared HEAD (RED — no notary authority gate exists)
    # RED-FIRST. The declared HEAD (declare_content_head, Plan C1/C3) is the notary's
    # authority answer: which version of a content node is the one the author declared and
    # the community witnessed. Moving it must be an author-only act. Today there is no HEAD
    # authority surface at all — the notarized read-and-declare endpoint for the landing EPR
    # returns HTTP 404 — so the served HEAD advances by last-writer recency (VERDICT L2 #3),
    # and any peer on the wire could overwrite it (REQ-N5 poisoning vector). With no gate,
    # a non-author is not refused; they simply win the race.
    #
    # The check is read-only-and-safe today: it probes the HEAD-authority surface for a real
    # notarized EPR and, only if that surface exists, attempts an unauthenticated (non-author)
    # move against a dedicated test-persona id — never production content. Today it stops at
    # the 404 and never issues a write.
    #
    # This scenario FAILS today with:
    #   HEAD-authority surface /db/content/elohim-host-landing/head on alpha-A returned 404 —
    #   the notary HEAD-declaration surface (declare_content_head, Plan C1/C3) is not wired.
    #
    # When it passes, the notary arbitrates HEAD selection and refuses a non-author's move
    # with 401/403 (Plan C1/C3/C5 — reach-cohort edit-membership + author signature): the
    # author declares, the notary witnesses, and no one else can move the HEAD.
    Then the HEAD authority surface refuses a non-author move of "elohim-host-landing" on peer "alpha-A"
