---
name: feedback_upgrade_propagation_north_star_wall_clock
title: "North star: p2p upgrade propagation before builders"
description: "Master p2p hApp upgrade/revert propagation (mixed-version peers talk, no big-bang rolls) before builders; every fleet roll is friction evidence."
metadata: 
  node_type: memory
  title: P2P upgrade/rollback propagation is the north star; wall-clock friction is first-class signal
  type: feedback
  originSessionId: 1e20b084-3df6-41bb-98c2-98477eff8411
  modified: 2026-08-31T16:19:10.618Z
---

**Operator correction (2026-08-31, mid carried-election shift).** When asked to guess his arising priorities, the agent ranked: (1) cure-vs-bypass of the conductor DHT plane, (2) making election authority real, (3) steward-operable runtime surface. He named 1–3 **biased mistakes**: with proper contextual understanding and *modeled test-fixtures* — the system deploying and driving a **simulacra without an agent embedded in the runtime** — those concerns are holdable/resolvable in stride. (4) living membership plane: valid. (5) dev-loop wall-clock: the real one, but structurally, not as mesh tweaks.

**What is actually surfacing for him (his words, compressed):** *"Mastering p2p rollback/upgrade of hApps, so upgrades aren't a big bang every time, and diverse peers can continue to communicate — incremental scale/velocity from localdev to deployment."* He is climbing the **"trust, reliability, performance" ladder of the P2P dataplane**, refining primitives into an **SDK** for himself and other app developers. *"Full support for upgrade propagation revert/upgrade over p2p is another engineering feat to achieve before I'd feel confident for people to start trying to use it. Holochain encrypts data to ONE hApp bundle — this 'feature not a bug' requires the network itself to agree on how it evolves; this seam cannot be upgraded by any external push."* This is the CRUX the p2p-storage-plane work has been building toward. Timing: "still a little early to make the jump," but it is the direction.

**Why:** The multi-hour dev cycle (fleet rolls to change anything, arcs resetting, saturation windows, discovery-by-reboot) and the token burn on "what should be simple" (dev workspace understood as a peer device of matthew's storage runtime, driving a simulacra on a test-fixture network) is the wall-clock that **threatens delivery of the whole system**.

**How to apply:**
- Treat every fleet roll, churn window, and multi-hour verification loop as **first-class friction evidence** — feel it, count it, and let it steer, exactly as the operator does. Reaching for a `[build:edge]` roll to answer a question is itself a finding.
- Hold the conductor-plane/authority/operator-surface concerns as *fixture-resolvable* — don't promote them to roadmap heads on agent-felt friction alone.
- Direction after local-participation-in-election validates: design/brainstorm **p2p upgrade+revert propagation** grounded in what already exists — dna-upgrade-governance (network-seed ladder, lineage), the coordinator hot-swap proven live 2026-08-31 (admin update_coordinators, no re-key), the additive-wire mixed-version discipline (carried-election payload), and hApp bundles carried as content on the storage plane. The simulacra/fixture-network capability is the proving ground and also what makes 1–3 holdable.
- **Debt-snowball ordering (operator-set 2026-08-31):** pay the SMALLEST atomic-discipline debts first (coordinator hot-swap vehicle → conductor-workload split → staggered rolls → runtime config); the complex p2p upgrade-propagation change goes deliberately LAST — it needs the most CI/CD proof, so it gets the cheapest possible iterations. Cycle-time per change class is the arc's own measure: re-baseline after each rung and harvest the deltas in sprint planning. Ladder lives in genesis/data/timeline/backlog/upgrade-propagation-p2p-design-arc.md.
- Related: [[feedback_local_mesh_first_cadence]], [[project_pipeline_dispatch_ordering]], [[user_operator_resource_reality_and_thesis]].

**2026-09-06 (operator, on the storage-binary "Out of MVP" cut):** "we want to be able to handle whole network
upgrades on ALL our components THROUGH our own architecture.. we're trying to build everything developers
would need to not just build on the architecture but to live, and grow there as well." So the definition of
done is the FULL class matrix carried by election with fleet receipts — coordinator zome (proven household,
fleet leg open) · runtime config · hApp bundle · storage binary · conductor binary (Nachalah: ark-activated
rung-5 artifact with rollback) · integrity/DNA-hash move (rung 6, happ-lineage-migration) — not the
2026-09-01 spec §9 MVP cut. §9's "fleet binaries self-updating over p2p — Out" is a trust-stage GATE on a
ladder (local → hybrid T3 → mesh → cluster; each rung's receipt admits the next; the binary vehicle opens
past `NetworkStage::Simulacra` on soak evidence), never a permanent exclusion. **How to apply:** when a
class is called "out of scope", name which RUNG it is waiting on and what receipt admits it; never present
the MVP cut as the architecture's ceiling. The k8s image roll is the legacy path for the last k8s-hosted
peers, not the design.
