@e2e @delivery @transport @perf @requires:bench-suite
Feature: Transport perf parity — what the dual-stack promise feels like

  The protocol promises that *whoever you are, whatever device you're on,
  whatever your network blocks*, you can still participate. That's the
  diversity guarantee. The protocol keeps that promise by carrying the
  same wire schemas over two different transports — iroh (fast, modern,
  needs UDP) and libp2p (universal, browser-capable, works through almost
  any network). The substrate picks whichever your peer can speak.

  These scenarios pin down what that promise actually delivers in
  measurable terms. They aren't about benchmarks for their own sake. They
  exist because every one of them, if it ever stops being true, breaks a
  specific human's specific experience — and we need to know before they
  do.

  Re-runnable via `just bench-all` (loopback, release, 5 warmup + 30
  measured per cell). Numbers cited as baselines were established
  2026-05-09. Drift past the thresholds means an architectural decision
  needs revisiting, not just a tuning regression.

  Background:
    Given the iroh + libp2p parallel stacks are both compiled in
    And `just bench-all` is run on loopback in release mode

  # =========================================================================
  # The collaborative-editing story
  # =========================================================================

  @wip @regression
  Scenario: Two students co-editing a study guide don't feel the keystrokes lag
    Lior and her study partner Maya are working on a shared notes
    document for tomorrow's exam. Lior is on her dad's old ThinkPad in
    her bedroom; Maya is across town on her phone. Every time either of
    them types, their elohim agents have to sync the change so the other
    sees it within human-reaction time (~100ms feels instant, ~250ms
    feels sluggish, ~500ms feels broken).

    Behind every keystroke: a CRDT delta gets shipped over the sync
    plane. The transport carrying that delta has to add as little
    overhead as possible — ideally sub-millisecond, so the keystroke
    feels native rather than networked. On a healthy iroh-capable path
    (Lior's home wifi), this is exactly what we measure.

    If iroh's small-frame win ever stops holding, real-time
    collaboration starts feeling laggy and we have to re-evaluate
    whether iroh is still the right canonical transport for chatty
    flows like this one.

    Given the chatty planes (sync, EPR, EPR-atom, view-fed, identity-handshake, trust)
    When the REUSE scenario runs for each plane
    Then iroh delivers a perf-bump assertion (iroh p50 < libp2p p50) on at least one size class per plane
    # Operational baseline: 45×–541× p50 win across planes as of 2026-05-09.
    # Constraint: iroh's small-frame stream-multiplex advantage is the
    # load-bearing claim of the iroh-canonical-for-hub-to-hub verdict.

  # =========================================================================
  # The "I'm at school and UDP is blocked" story
  # =========================================================================

  @wip
  Scenario: A learner on a UDP-blocked school network still gets responsive learning
    Eli is doing a Sophia quiz from a school-issued Chromebook during
    his free period. The school's IT policy blocks UDP at the firewall —
    iroh's QUIC transport can't get out. His browser-based elohim falls
    back to libp2p, which uses TCP and works through the school's
    proxy.

    Eli still feels the protocol working — but his interactions carry a
    ~125ms transport tax per round-trip. Drag a slider on a Sophia
    widget? ~125ms. Submit an answer? ~125ms. The UX needs to absorb
    this with optimistic updates and skeleton loading states; if it
    doesn't, every interaction feels like dial-up.

    This scenario isn't a perf assertion to celebrate — it's a boundary
    we need to remember exists. UI budgets, retry policies, and timeout
    values for libp2p-only paths must accommodate it.

    Given any chatty plane (sync, EPR, EPR-atom, view-fed, identity-handshake, trust)
    And payload size at or below 64 KiB
    When the REUSE scenario runs
    Then libp2p p50 latency is in the range 100–135 ms regardless of payload size
    # Operational baseline: 123–127ms across 6 chatty planes as of 2026-05-09.
    # Constraint: this is a hard-to-explain libp2p+request_response+yamux property
    # — likely Nagle, yamux flow control, or ack timing — that bounds UX
    # latency budgets for any libp2p-only delivery path.
    # Open question: tuning yamux window / TCP_NODELAY / request_response config
    # might lower this floor. If we ever close this gap, it tightens UX budgets
    # across every browser-served learner.

  # =========================================================================
  # The "shared video lesson" story
  # =========================================================================

  @wip @regression
  Scenario: A 50-MB lesson video downloads at near-parity regardless of stack
    Mr. Okafor records a 50-MB explainer video about photosynthesis
    and shares it into his class's collective. Some students watch
    from their household NAS at home (iroh works fine — fast). Some
    watch from a tablet at after-school care (corporate network — only
    libp2p gets through). The diversity guarantee says: both should
    finish downloading the lesson in roughly the same time. Not
    identical — but the slow path can't be 10× slower than the fast
    one, or the experience is meaningfully unequal.

    For chatty interactive traffic, iroh wins by hundreds-of-X (small
    messages amplify the per-frame overhead difference). For bulk
    transfer like this video, the per-frame overhead becomes a small
    fraction of the total — both stacks spend their time pushing bytes,
    and the playing field levels out.

    Given the shard plane carrying payloads of 1 MiB and larger
    When the REUSE scenario runs
    Then iroh p50 latency is within 2× of libp2p p50 latency
    # Operational baseline: 1.18×–1.36× as of 2026-05-09.
    # Constraint: the dual-stack-permanent verdict assumes neither stack
    # is dramatically disadvantaged at bulk transfer. If the gap widens
    # past 2×, transport-selection needs to start biasing by payload
    # size — currently it doesn't because the gap is small enough not to.

  # =========================================================================
  # The "future-engineer-doesn't-shoot-themselves-in-the-foot" story
  # =========================================================================

  @wip @regression
  Scenario: A connection pool that reuses iroh QUIC connections doesn't silently hang
    Phase 11 will optimize chatty traffic by pooling iroh connections
    instead of opening a fresh one for every request. A future engineer
    sets up the pool, runs the test suite, sees green, ships. But if
    they ever revert the loop-on-accept_bi pattern in the ALPN
    handlers, the second request from any pooled peer will hang
    silently for 30 seconds (or, with a previous bug, forever) before
    timing out.

    This scenario doesn't address a current user — it addresses a
    future engineer who would otherwise spend a frustrating afternoon
    bisecting why their pool doesn't work. The bug was discovered when
    a perf bench tried to amortize handshake cost; it took 86 silent
    minutes to notice. The fix is simple; the regression anchor is
    cheap; the value is preventing the same trap.

    Given a single QUIC connection between two iroh peers on any custom ALPN (sync, EPR, EPR-atom, shard, view-fed, identity-handshake, trust)
    When the fetcher opens N bidi streams in sequence on that connection (N ≥ 2)
    Then every stream completes its request-response within 30 seconds
    # Anchor: if this regression ever fails, someone has reverted the
    # loop-on-accept_bi pattern (commit 7db8def2a). Production callers
    # currently always open fresh connections — but the moment Phase 11
    # adds a pool, the loop pattern becomes load-bearing.

  # =========================================================================
  # The "post-quantum-onboarding 2030" story
  # =========================================================================

  @wip @regression
  Scenario: When my signature outgrows the protocol cap, identity onboarding fails clearly
    It's 2030. Quantum-resistant signature schemes have replaced
    Ed25519 across the protocol. A new learner — call her Naomi —
    tries to onboard with her PQ-signed identity binding. Her
    signature is 8 KiB instead of the old 256 bytes. The protocol's
    identity-handshake request has a 4-KiB cap (set in 2026 when
    256-byte signatures were tiny). Naomi's request doesn't fit.

    What we DON'T want: Naomi's onboarding times out silently after
    30 seconds with no explanation, and her elohim retries forever in
    the background while she gives up.

    What we DO want: the cap surfaces as a clear, debuggable error
    immediately, AND someone reading the regression scenario in the
    test suite knows why the cap exists and what it would take to
    raise it (frame fragmentation, or a protocol-version bump).

    Given an IdentityHandshakeRequest whose serialized wire size exceeds 4 KiB
    When sent over the iroh /elohim/identity-handshake/2.0.0 ALPN
    Then the handler closes the connection rather than processing the request
    And the client surfaces a clear error within the read timeout (30s)
    # Operational parameter: IDENTITY_HANDSHAKE_REQ_MAX = 4 * 1024.
    # Effective signature payload ceiling is ~3 KiB raw bytes (after
    # base64 expansion + HandshakeBindingPayload overhead).
    # Informs: any future post-quantum signature scheme integration must
    #         either raise the cap OR fragment across multiple frames
    #         OR bump to /elohim/identity-handshake/3.0.0 with a new cap.
