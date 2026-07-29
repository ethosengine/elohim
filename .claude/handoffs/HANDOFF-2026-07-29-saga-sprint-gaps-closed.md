# Handoff 2026-07-29 — Saga gap-closure sprint: 4/10 → 6/10 local, machinery unwedged, two operator ceilings named

**Objective (operator's words):** "verify our status, plan and execute a sprint
to try and close as many gaps as you can in our saga." Parent handoff:
`HANDOFF-2026-07-28-resilience-cards-converge.md`.

## Scoreboard

Entered at 4/10 green (stale local report). Leaves at **6/10 green**
(`saga-status.py`, report jenkins-elohim-edge-dev-1253): ch08 flipped on
report-sync alone; ch04 flipped on the regression re-commitment fix. ch02 /
ch06 / ch10 remain red — every in-repo blocker for them was fixed this
sprint; what remains is on the two operator ceilings below plus one batched
push.

## Landed (all local on dev, commit-only — NOT pushed, one batched push intended)

- `8a1c531dc` epr-cli **regression re-commitment** — green-after-Dismiss now
  re-Produces (was a named v1 exclusion; "regressed" was a sticky terminal
  state). ch04 ✅. 41 tests, idempotency verified live.
- `4602a9bfe` **james agencyPhase** — humans.json roster silently dropped him
  (no agencyPhase field); this is why genesis #1382's formation could not
  bind james ("no conductor bound — cannot affirm", 1/3 affirmed).
- `8a740add8` + `b498b42ac` **DoD-1 verdict captures** — see ceilings below;
  plus the minted declared-vs-declared conflict station in ch06.
- `47667c564` **ch06 station counters de-@wip'd** —
  `elohim_content_witness_reauthor_failed_total{chain_head_moved|already_exists}`
  + `elohim_content_witness_sweep_abandoned_total`, pre-touched at
  registration. 2226 lib tests green.
- `eebd46755` **identity_fill 120s timeout** — the diagnosed permanent hang
  (unbounded await chain; jessica-only because only-non-empty union) is the
  thing wedging the household bootstrap circle shut. Loop now survives a hung
  tick (paused-time test proves it). jessica backlog resolved with evidence.
- Codex (relayed): `e26c1cc05` formation operator entrypoint
  (`just seed-household-formation`); `6a3507ae0` qahal
  `list_memberships_for_collective` batched DHT get (coordinator-only,
  hot-swap); che-devworkspaces `a782dd8` pre-baked pinned pnpm in ci-builder
  (kills the storybook #245 corepack-flake class; submodule gitlink
  deliberately unstaged).

## The two operator ceilings (everything else is behind them)

1. **All four shem conductors DHT-silent since the 21:15Z 07-28 post-DNS-flip
   boot** ("No local agents available"; bootstrap store holds only the
   on-prem trio; B cannot retrieve its own declared head). Prime suspect:
   `signal.elohim.host` now resolves to shem's own WAN IP → GFiber hairpin
   trap (the A-side hairpin fix created a B-side one). One-command probe from
   any shem pod: `curl -sv https://signal.elohim.host/` — hang = confirmed →
   split-horizon (repo manifests) + restart the four. Full sequence:
   `genesis/data/timeline/backlog/shem-conductors-signal-hairpin-suspect-dht-silent.md`.
2. **Head-direction decision for elohim-host-landing**: BOTH doorways hold
   declared, notarized heads — A 08:56Z vs B 10:30Z (newer), different SPA
   blobs. "B adopts A" would roll B backward and fills-never-moves correctly
   refuses it. Decide the intended blob, then the carried-record declare is
   the one-lever cure (station minted in ch06, `b498b42ac`).

## First moves next session

1. **One batched push** (or operator fires it) — triggers orchestrator:
   genesis re-runs formation with james bindable (2/3 affirm possible even
   with matthew's UUID conflict), edge re-verdicts the saga, sweettest gate
   exercises `6a3507ae0`, deploy ships the identity_fill timeout + counters.
   After deploy: jessica's timeout WARN firing is the confirmation signal her
   loop was the hang; if her fill completes, watch the collectives arm for
   the first non-zero `collectives_ids_discovered`.
2. Re-check the ceilings above; if the operator restarted shem conductors,
   `GET https://elohim.host/db/p2p/conductor-diagnostics` agentCount > 0 is
   the pass, then drive the declare.
3. ch02 chain to green (no matthew needed — scenario needs discovered>=1 +
   created>=1 on alpha-A): formation (jessica founder + james affirm) →
   MembershipCommitted stamps on james's storage → collectives arm carries →
   alpha-A identity_fill discovers + creates. matthew's captured-UUID-chain
   migration stays the known operator-scope deferred item.
4. ch10 arbiter unchanged: both doorways, same non-zero
   stewardingCollectives (`resiliency-saga.steps.ts:517`).

## Watch-outs carried forward

- storybook #245 was infra (corepack ETIMEDOUT), fixed at the image layer;
  needs a che-devworkspaces main push + image rebuild to take effect.
- susan's storage failed the steward-peers manifest fetch (`:8090/manifest`)
  during the refresh — uninvestigated.
- doorway-A conductor-diagnostics transportStats errors
  `missing field 'is_direct'` whenever connections exist — investigated
  (Codex): the deserializer lives in locked kitsune2_api 0.4 while the
  checked-in Kitsune source is incompatible 0.3, so the safe fix is a
  0.4-compatible vendored/forked API patch, not doorway route code. BLOCKED
  pending that decision.
- Review pass over the sprint window landed two fixes: `0038aaa78`
  (formation entrypoint was CWD-dependent) and an epr-cli ordering-rule
  alignment (fulfill's "latest event" was append-order while saga-status is
  occurredAt-order — replay/backfill could permanently re-wedge "regressed";
  fix + out-of-order regression tests in flight at handoff time). Also
  landed: `b0150a4cc` gherkin pre-push lint (parses all 159 features before
  E2E; manifest-routed with husky fallback).
- `projection_reconcile.rs` is over the 3000-line soft LoC ceiling —
  modularization backlog when the tail drains.
