---
id: "backlog-portal-health-dev-fixture-reachable-on-public-alpha"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "PUT /admin/dev/portal-health is gated on dev_mode alone, and DEV_MODE=true is set on every deployed doorway including public alpha"
slug: "portal-health-dev-fixture-reachable-on-public-alpha"
written: "2026-09-20"
author: "serving-edge failover-balance-stream campaign, 2026-09-20 review"
status: "resolved"
priority: "high"
jobs: [elohim-edge]
cluster: "arch-authority-in-integrity-backlog"
relatedNodeIds:
  - "habit:doorway-failover"
tags: [doorway, security, auth-posture, dev-mode]
---

**The fact.** `fixture_only_gate` (`doorway/doorway-service/src/routes/admin_dev.rs:70-80`) is the sole
guard on `PUT /admin/dev/portal-health` (`handle_set_portal_health`, `:89-133`): when `state.args.dev_mode`
is true the handler runs unauthenticated, no credential of any kind. The file's own module doc names the
crate's stated intent — a fixture surface reachable only in dev — but `doorway-service/CLAUDE.md`'s "Auth
posture" section states the general rule this route does not follow: *"`DEV_MODE: "true"` is set on EVERY
deployed manifest, so a gate keyed on it is in practice ungated."* `genesis/orchestrator/manifests/doorway/alpha.yaml:159-160`
sets `DEV_MODE` to `"true"` on the public alpha doorway deployment, the same manifest that serves
`alpha.elohim.host` and `elohim.host`.

**Evidence.** `doorway/doorway-service/src/routes/admin_dev.rs:70-80` (`fixture_only_gate`), `:89-133`
(`handle_set_portal_health`); `genesis/orchestrator/manifests/doorway/alpha.yaml:159-160`
(`DEV_MODE` / `"true"`); `doorway/doorway-service/CLAUDE.md` §"Auth posture" (the general rule, and the
two prior incidents it names — the seed/`/admin/cache/*` bypass closed by `62b658784`, and the anonymous
conductor admin socket closed 2026-08-27 — that a `dev_mode`-gated route on the fleet has produced twice
before).

**Why it matters.** Any caller who can reach the public doorway can flip a portal host's reported health
(`healthy: bool` on an arbitrary `hostUrl`), which the portal-host probe consults whenever dev mode is on
— on the live fleet, always. This is the same class of defect the CLAUDE.md section was written to
prevent, on a route that predates that section's most recent incident closures and was not swept with them.

**Smallest next step.** Follow the pattern `story-3.1-design.md` §9.5 already proposes for a different
dev-gated fixture (`PUT /admin/dev/shed`): replace the `dev_mode`-alone gate with the network-stage
predicate (`AppState::network_stage`, fail-closed to `Bootstrap`) plus a loopback/declared-network-stage
check, so the route is reachable from a household launcher but refuses on any deployed stage. Confirm no
a2o fixture depends on the current unauthenticated path before narrowing it (`grep -rn "portal-health"
genesis/a2o/`).

**Resolved 2026-09-20, in the commit that files this.** Both fixture surfaces (`PUT /admin/dev/portal-health`
and the new `PUT /admin/dev/shed`) sit behind one predicate, `admin_dev::fixture_surface_gate`: the doorway's
DECLARED network stage equals `Simulacra` — equality, because `Bootstrap` also precedes `Coordinated` and
alpha, declaring nothing, fail-closes to `Bootstrap` — AND the accepted TCP peer is loopback (canonicalised;
no forwarded header is read anywhere in the crate). `dev_mode` decides nothing on either side: the portal
probe honours an override only at `Simulacra`. Red-teamed before commit: no deployed manifest declares
`ELOHIM_NETWORK_STAKES`; nothing mutates the stage after boot outside `#[cfg(test)]`; no in-pod process
forwards outside traffic over loopback. The two portal-handoff features that call the fixture from their
`Background` are tagged `@requires:owned-substrate` — they were already exercised only on the household
browser lane. Pinned by `dev_mode_true_does_not_open_portal_health_when_stage_is_not_simulacra`. What
remains open is the rest of the 2026-06-06 `DEV_MODE=true` scope exception on alpha, which this does not touch.

**Links.** Habit: `doorway/doorway-service/.epr-meta/doorway-failover.habit.md`. Auth posture rule:
`doorway/doorway-service/CLAUDE.md` §"Auth posture — read BEFORE touching any gate". Sibling fixture
design (same pattern, done right): `genesis/a2o/reports/recovery/serving-edge-20260919/story-3.1-design.md` §9.5.
