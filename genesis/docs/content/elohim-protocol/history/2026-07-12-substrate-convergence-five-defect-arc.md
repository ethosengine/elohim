---
title: The Five-Defect Convergence Arc — a museum record of stacked invisible failures
id: substrate-convergence-five-defect-arc
date: 2026-07-12
status: history
author: dht-unity arc close (Fable session, 2026-07-11/12)
cites:
  - substrate-trust-contract-runbook | The Substrate Trust Contract | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
  - genesis/data/timeline/backlog/genesis-pair-cross-conductor-fetch-blocks-canonical-convergence.md
---

# The Five-Defect Convergence Arc (museum record)

One outcome measure — "every federation peer resolves the SAME canonical
head" — stayed red for days while FIVE independent defects hid behind it,
each invisible from the outcome alone, several masking each other. This
record preserves the anti-pattern *shapes* (they will try to recur in other
clothes) with the primary evidence that finally named each one.

## The five classes, in discovery order

### 1. The silently-dropped config key (`ice_servers` → `iceServers`)
The conductor config carried `webrtc_config.ice_servers` from the day it was
written. Holochain passes that object VERBATIM into tx5's serde-camelCase
`WebRtcConfig` — unknown field, silently ignored — so **every conductor ran
with zero ICE servers since fleet inception**. WebRTC held together on host
candidates while the genesis pair was co-located, and died the day adam
moved to shem (2026-05-27) — which is why "gossip used to work" (2,081
divergent anchors accumulated in the co-located era) and then didn't.
**Shape:** a config surface that tolerates unknown keys turns a spelling
into a total, silent capability loss; the system "works" until topology
makes the missing capability load-bearing. **Guard left behind:**
`validate-conductor-config.sh` render gate (rejects the dead key by name).
**Evidence:** Holochain's own conductor-config doc example (`iceServers`,
camelCase); tx5-connection `config.rs` `#[serde(rename_all = "camelCase")]`.

### 2. The resurrection heal (boot re-projection restamps superseded state)
Two minutes after the first-ever cross-conductor adoption (20:40:35Z), the
restarted node's projection-reconcile heal stamped the OLD head back
(20:42:40Z): a cold conductor's `resolve_content_head` falls through to the
root-author election while the canonical link is not yet retrievable, and
the heal wrote that fallback over the adopted canonical row — for 2,838
divergence-queued rows, serially, after every restart.
**Shape:** a self-healing loop with a weaker information source than the
state it overwrites converts every restart into a rollback. Heal must FILL
absence, never MOVE authority. **Guard:** `StampMode::{Declare,GapFill}` +
unit-pinned transitions.
**Evidence (Loki, adam-alpha):** `WARN projection-reconcile[content]:
HEALED content anchor from own conductor — content_id: "elohim-host-landing"`
at `2026-07-11T20:42:40.185613Z`, matching the row's
`updatedAt` to the second.

### 3. The over-broad guard (GapFill also blocked forward adoption)
The fix for #2, applied blindly, froze convergence: the heal was ALSO the
only working path by which a peer adopted a canonical that gossiped in
between deploys. **Shape:** a guard keyed on "who is writing" instead of
"what authority the write carries" blocks the legitimate case with the
illegitimate one. The cure is making the SOURCE name its authority: the zome
now returns `canonical: bool` (coordinator-only change, hot-swapped), and
the heal stamps Declare for canonical answers, GapFill for fallbacks.

### 4. Racing the publish window (declare-propagation always failed fresh)
The per-deploy propagation declares a head authored SECONDS earlier to a
remote conductor — whose DHT `get` legitimately cannot retrieve it yet
(publish/gossip takes minutes). Every deploy-coupled attempt failed with
`not retrievable`; the head then adopted ~10 minutes later via gossip+heal,
making the probe look permanently broken while the mechanism worked.
**Shape:** a probe that fires inside its subject's warm-up window measures
the window, not the subject (see also: restart peer-store churn, ~20 min,
measured 5/7 stale URLs → 0/7 after expiry). **Guard:** the propagation
retries only the `not retrievable` refusal (×4, 90s apart).

### 5. The gate that couldn't run in its own container (PyYAML)
The render-time validator (guard for #1) imported PyYAML — absent in the
deploy container — so it failed all seven human deploys BEFORE kubectl
apply (edge #1183), silently pinning the fleet on pre-fix builds and making
defects #2/#3's fixes look inert. Negative-tested where it was written,
never where it runs. **Shape:** CI tooling verified outside its runtime is
itself an unverified claim — the same class it polices. **Guard:** the
script is bash+coreutils only, with the requirement written into its
header; a bonus class instance: notary sweettest retries self-poisoned via
fixed content ids on the process-global mem-bootstrap store (dna #1357) —
per-invocation `unique_id()` fixed it and un-broke the retry mechanism for
the whole test family.

## The doctrine the arc leaves behind

Every trust claim gets a probe; every probe failure names itself; every fix
leaves its guard behind. The operational half lives in the trust contract
(cited above) — invariants, probes, per-red runbook. The night's meta-lesson
for future debuggers: **when an outcome measure stays red through multiple
correct fixes, suspect a STACK, not a miss** — enumerate the layers, give
each its own discriminating probe, and burn them down cheapest-first.
