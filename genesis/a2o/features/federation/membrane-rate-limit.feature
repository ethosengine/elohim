@wip @e2e @delivery @federation @requires:doorway @act:i
# @wip until the ~11 step-defs are implemented (see the doc-comment below + `npx tsx scripts/generate-step-skeletons.ts`).
# Graduate (drop @wip) AFTER the membrane deploys to alpha — the flood/header/Prometheus step-defs need a LIVE
# membrane to author + verify reliably. Until then, verification is manual: `scripts/membrane-probe.sh` +
# the doorway Grafana dashboard + the MembraneBanStorm alert. This is the correct e2e sequencing (can't write a
# reliable e2e for a behavior that isn't deployed yet), not a gap left open.
Feature: Doorway membrane rate-limits the front door without shaping real visitors
  As a household member visiting elohim.host
  I want the doorway to absorb a flood from one disposable source — slowing,
  challenging, then banning it — while my own page load passes through clean
  So that one abusive client never degrades the front door for everyone, and
  the membrane is provably present in the deployment, not just in unit tests.

  # The membrane is the doorway's per-source admission stage (doorway :8080):
  # every non-exempt, non-static request is source-keyed (ip:/agent:), assessed,
  # and given a verdict — Allow / Shape(+250ms) / Challenge(429) / Deny(403).
  # Exempt: /health, /metrics, WS upgrades, static assets (/assets/, /fonts/,
  # Angular bundle basenames). Source:
  #   doorway/doorway-service/src/server/membrane.rs
  #   doorway/doorway-service/src/metrics.rs   (verdict counter + bans gauge)
  #   doorway/doorway-service/src/server/http.rs::apply_membrane (verdict→response)
  #
  # Edge GuardConfig (calibrated 2026-06-20): window 60s, shape 300,
  # challenge 600, ban 1200, ban_secs 900, shape_delay 250ms. A real alpha
  # landing makes ~3-5 non-static counted requests — a 60-100x margin below the
  # shape floor, so a normal visitor is NEVER membrane-shaped.
  #
  # Only the `x-membrane` response header indicates a membrane action. Content
  # 404/403 (e.g. the manifesto reach gate) are NOT membrane verdicts — a body
  # error with no `x-membrane` header is the substrate answering, not the door.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"

  # --- A normal visitor is never shaped (the safety property) ----------------

  Scenario: A normal page load carries no membrane header and is counted as allow
    When I GET "/" from the doorway
    Then the doorway response status is 200
    And the doorway response has no "x-membrane" header
    When I scrape the doorway metrics
    Then the metric "doorway_membrane_verdict_total" with label verdict "allow" is present and greater than zero
    And the membrane "allow" verdict count exceeds the "challenge" verdict count

  # --- A disposable flood is throttled, challenged, then banned --------------

  Scenario: A disposable source exceeding the rate threshold is challenged then denied
    Given a disposable membrane source identified by a unique X-Forwarded-For client
    When the disposable source sends 650 rapid requests to "/" through the doorway
    Then at least one response status is 429
    And the 429 response carries the header "x-membrane" with value "challenge"
    And the 429 response carries a "Retry-After" header
    When the disposable source sends 600 more rapid requests to "/" through the doorway
    Then at least one response status is 403
    And the 403 response carries the header "x-membrane" with value "deny"

  # --- The membrane is observable at the deployment edge (the verifiability) --

  Scenario: Membrane metrics render at boot and move under a challenge probe
    When I scrape the doorway metrics
    Then the gauge "doorway_membrane_bans_active" is present
    And I record the membrane "challenge" verdict count
    Given a disposable membrane source identified by a unique X-Forwarded-For client
    When the disposable source sends 650 rapid requests to "/" through the doorway
    And I scrape the doorway metrics
    Then the membrane "challenge" verdict count has increased since it was recorded
