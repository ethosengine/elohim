# Handoff 2026-07-28 (night) — Next: converge the EPR resilience cards

**Objective (operator's words):** "pick up on our saga and converge the EPR
resilience cards." Finish line inherited from the 07-27 sprint (parent
handoff: `HANDOFF-2026-07-27-heads-converge-truthful-resilience.md`): **both
doorways tell the same, non-zero truth about any EPR's resilience — and keep
telling it across a deploy restart.** The card is the human-visible face of
that truth; today's session cleared the transport layer out of its way.

---

## What today (2026-07-28) closed — the transport/DNS chapter

All landed on dev, pushed, deployed (edge #1252 + #1253), and verified live:

- **Two-premises DNS model is real.** Apex `elohim.host` A = shem WAN
  (adam's doorway-B), `alpha.elohim.host` A = operations WAN (matthew),
  both beacon-owned; adam-side CNAMEs follow the apex, matthew-side names
  re-homed to alpha. Matthew's signal = `wss://signal.alpha.elohim.host`
  (cert SAN verified reissued); adam's stays `wss://signal.elohim.host`.
  ICE is role-named: `turn:alpha.elohim.host:3478` / `turn:elohim.host:3478`.
  The §5.3 signalling hairpin (adam's rendezvous entering at matthew's
  premises) is GONE. Commits: 0b44ecddc (rename set), 707bbc7c5 (pool +
  cleanup), fbcd14bac (docs+a2o).
- **Operations relay pool: 41 → 848 ports** (49152–49999, router forward
  confirmed by operator first). Pre-fix evidence: 85% ALLOCATE error-508 on
  the ops leg correlated with adam's 85% gossip-round timeouts. Shem's leg
  stays 49160–49200 forever — GFiber cannot edit forward ranges (harvested
  as `genesis/a2o/features/dataplane/relay-capacity.feature`, born-red).
- **The apex write-war is found and dead.** A ddclient on the ethosengine
  HOST (outside k8s — invisible to Loki; Cloudflare audit-log forensics
  named it) reverted the beacon's apex flip 25s after publish. Operator
  removed elohim.host from it (it still maintains ethosengine.com). The
  beacon now self-heals this class: commit 345844b06 gives the owned lane
  per-cycle freshness verification (re-assert on external clobber/delete,
  WARN `exclusive record DRIFTED`) + ownership stamps on every write.
  Verified cycling `freshness verify ok` on both legs, zero drift warnings.
- **`ice_servers` → `iceServers`** fixed in all five remaining configs
  (61ae7e95e + che-devworkspaces d3c324e) — the serde-silent-drop class.
- **First-ever movement on adam:** consecutive `caught_up: true` heal cycles
  (21:25, 21:31) with nonzero `content_healed` — flat-zero all day before.
  This is the leading indicator that the transport cures worked.

Memory entry `project-two-premises-dns-beacon-owned` carries the non-repo
facts (ddclient, GFiber constraint, DNS ownership map).

## What today did NOT move — and why that's the next sprint

Edge #1253: 35 passed (floor held, zero new failure names, deploy stages
clean), same 9 reds as #1252, byte-identical. They partition into exactly
the card-convergence work:

1. **DoD-1 family — doorway-B's conductor has no chain.** Seam-smoke
   sharpened it: `peer_store elohim.host: FAIL (0 agents)`. The re-keyed
   conductor never authored `elohim-host-landing`; overnight's ghost sweep
   authors it but its zome calls hit the same timeouts that were blamed on
   adam's exhaustion. **KEY RE-CHECK: that blocker may now be GONE** — the
   overnight diagnosis was write-pool starvation + zome-call timeouts on
   adam, and adam is now caught-up with a healthy relay. Re-run the
   declare/adopt path before assuming the overnight blocker still stands.
   Stations: canonical-head divergence (A `uhCkk78Z…` vs B `uhCkkl4C9…`),
   DHT-anchor divergence, "B adopts A's declared head after restart",
   `elohim.host` caughtUp=false, "canonical head propagated" probe absent.
2. **DoD-2 family — the card is data-starved** (memory:
   `project_resilience_card_data_plumbing`): snapshot joins need
   substrate-owned humans, but 7 household humans carry `household_id` with
   NULL `agent_pub_key`; `elohim_identity_fill_discovered_cids` = 0 on
   alpha-A (jessica's identity-fill loop runs silent — backlog entry from
   389813d16); fleet DHT has ZERO household memberships (ch02
   household-forms is the saga frontier, pending-env). The 0-vs-1
   `stewardingCollectives` split is a seeded pre-coherence row on A,
   truthfully absent on B.
3. **`divergentAnchor <= 100` gate** (638 observed) — the flapping windowed
   re-spec ask is still open with the operator; also genuinely elevated by
   restart churn. Re-measure after the quartet settles before touching it.

Overnight machinery already landed and verified working (07-27/28 sprint):
adopt-before-author (b91ee0f95, `head_adopted_total` jessica=4/james=3),
GapFill self-election guard, TOCTOU unique index, reach-gated head routes,
collectives reconcile arm. The leg-2 tripwire never fired
(`refused_stale=0`) — the ordering-proof gate is exonerated. Read the RCA
first: `genesis/data/timeline/backlog/content-divergence-unhealable-without-canonical-heads.md`.

## First moves next session (ordered)

1. **Re-measure the shem quartet** (shard-aware — compare same
   `content_ids_discovered` denominators only):
   Loki `{namespace="elohim-alpha", pod=~"elohim-(adam|eve|gertrude|susan)-alpha-0"} |~ "heal complete"`.
   Convergence verdict decides whether transport is truly done or the
   quartet needs another look. Also pull coturn 508 counts per leg (expect
   ~0 on ops now) and adam's gossip timeout ratio (was 85%).
2. **Probe doorway-B's conductor directly** — `GET
   https://elohim.host/db/p2p/conductor-diagnostics`, peer store count, and
   whether `declare_canonical_head` still errors `no content found for id`.
   If adam's recovery cleared the timeouts, drive the overnight's declare
   race to completion and the whole DoD-1 station family should green
   together. The declare-route admission class question (sheds harder than
   plain PATCH) is backlog'd with evidence if zome calls still shed.
3. **Household formation on alpha (ch02)** — code-ready per overnight; the
   ceiling item is forming a real household membership on the fleet DHT.
   Design question backlog'd (collectives-arm bootstrap gap, in 389813d16).
   This + identity-fill unblocks the NULL agent_pub_key joins → the card's
   data supply.
4. **Then the card itself:** both doorways serve the same non-zero
   resilience truth for `elohim-host-landing` and `household-dowell` — the
   ch10 felt-safety scenario (`resiliency-saga.steps.ts:517`) is the
   arbiter, plus the held 6peer scenario when substrate allows.

## Rails and gotchas (verified today)

- No kubectl — Loki via observability MCP + doorway HTTP + anonymous
  Jenkins REST (`https://jenkins.ethosengine.com/job/<job>/job/dev/...`).
  Loki empty/502 = "no data", never zero.
- Doorway reads are per-doorway single-target: A→`doorway-alpha.elohim.host`
  (matthew), B→`elohim.host` (adam) — and post-flip those names now really
  terminate at different premises. `environment.alpha.ts` doorwayFallbacks
  is now a genuine cross-WAN failover.
- Edge deploys restart conductors (~20min churn); don't measure convergence
  inside the churn window. Coturn manifests roll only on a
  `elohim.host/conf-revision` annotation bump (unchanged-apply = silent
  non-deploy).
- Any future DNS weirdness: check coturn beacon logs FIRST (`DRIFTED` =
  competing writer; the beacon self-heals in ≤30s and stamps provenance).
- Operator ceiling still open: delete `turn.elohim.host` +
  `turn-shem.elohim.host` in Cloudflare (safe — nothing references them);
  DNS-as-code (SHEM action #8 → now only in the feat(dns) commit message /
  coturn manifest headers); CI gate manifest-args-vs-binary (backlog OPEN);
  ethosengine-zone federated doorway idea (versioned EPR access — operator
  floated it, needs `--cf-zone` + token re-scope if beacon-managed).

**Saga scoreboard going in:** 4/10 green · frontier ch02 household-forms ·
the transport preconditions for ch06 heads-converge are now fully met for
the first time.
