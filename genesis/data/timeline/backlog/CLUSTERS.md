---
id: "backlog-clusters-index"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Backlog Clusters — the index of subject-scoped idea sinks"
slug: "backlog-clusters-index"
written: "2026-08-04"
author: "claude (cluster discipline, operator-directed)"
status: "backlog"
priority: "medium"
tags: [backlog, clusters, index, provenance, research-mint-pass]
---

# Backlog Clusters — subject-scoped idea sinks

The backlog's job is to **graduate concerns into active sprints**, not to accumulate. Not every
entry will ever be acted on — so like concerns cluster into subject files, each the *single*
re-surfacing point for its subject: one ranked table, per-item graduation targets, items lifted
into shifts individually. A standalone entry is for an operationally-atomic concern (one bug, one
incident, one bounded task); anything with siblings belongs in a cluster. Research engagements
(surveys, retrospectives, confrontations) close with a **mint pass**: surviving take-items fold
into the matching cluster citing the survey's `epr:` slug — takes not worth a cluster row die
honestly in the survey prose.

**Re-surfacing:** groom this table when a cluster changes; `grep -c '^| [0-9]' <file>` counts a
cluster's open rows. Status lives in each cluster's frontmatter + per-row notes — this index is
the map, never the content.

| Cluster | Subject | Rows | Groomed | Notes |
|---|---|---|---|---|
| [arch-dataplane-refactor-backlog](epr:arch-dataplane-refactor-backlog) | Dataplane *internal* reshaping — reuse, IoC seams, `p2p/mod.rs` decomposition chain (10→12→15), dev QoL, head-plane scale | 18 | 2026-09-05 | Row 18 = `projection_reconcile.rs` LoC-ceiling decomposition (finding `c933fddb1025`, plan in [projection-reconcile-loc-ceiling-decomposition](epr:projection-reconcile-loc-ceiling-decomposition)), blocked on the in-flight drain-cure batch; Row 17 (content_store root/version-listing extern) from the 2026-09-04 rung-5 c3 shift; p2p-design-gate mandatory on pickup |
| [arch-workspace-discipline-backlog](epr:arch-workspace-discipline-backlog) | Crate/workspace discipline — lints, licensing, versioning, CI gates, extraction sequence (p2panda-derived) | 14 | 2026-08-23 | Items 2 + 9 need operator decisions (license policy; root LICENSE absent) |
| [arch-dataplane-borrows-backlog](epr:arch-dataplane-borrows-backlog) | *Externally-sourced* dataplane mechanisms from surveys (Holepunch/SSB/p2panda/sedimentree/backstitch) — each p2p-design-gated | 11 | 2026-08-15 | Sibling of refactor cluster: borrows vs reshaping. Row 10 (schema-as-content-addressed-EPR) is **jointly designed with** [governance-native-dna-upgrade-path](epr:governance-native-dna-upgrade-path) — wire-format skew and DNA-hash skew are one problem at two layers |
| [arch-confidentiality-plane-backlog](epr:arch-confidentiality-plane-backlog) | The unbuilt encryption layer (§3.13) — fail-closed classifier, KeyEnvelope, p2panda-encryption candidate, X25519 substrate, ciphertext relay, witnessed-harm limit | 8 | 2026-08-09 | #1 is immediate; #2's audit-check gates the design fork; #8 is council-gated (position TBD) |
| [measure-family-borrows-backlog](epr:measure-family-borrows-backlog) | *Externally-sourced* observation procedures + their invariants, and the fold they ride — measure families over EPRs, unit-agnostic by design (Playnet-derived) | 23 | 2026-08-23 | Row 1 (harden `epr-rea`'s fold) is the prerequisite for every other row; composes with [middot](epr:middot-measure-primitive-design). Rows 12–15 are Meadows-derived **dynamics**; rows 16–18 are the **confidence-interval ontology** (operator dispatch); **rows 19–23 are the 2026-08-23 succession/monetary mint** — coordination-cost reduction, abundance-type (pairs with 13–14), the Cantillon detector (Amplified set, pointed at us), WIR counter-cyclicality, and the efficiency–resilience window (⚠ gated on re-deriving the coordinate). **Landed and tested:** row 12 (`MeasureKind::Rate{per}` + dimensional algebra `divide`/`combine_additive`), row 13 (harvest/regeneration, named) and row 14 (turnover time) as the new `Stock{level, inflow, outflow}` primitive in `elohim/epr-rea/src/stock.rs` (slice 2, 2026-08-11 — an open question is recorded, not fixed: turnover's time unit isn't type-carried yet), and rows 16–17 (interval-inside-the-hash, `fold::with_uncertainty`). Rows 15 (aggregation-facing global fold) and 18 (uncertainty work-queue) remain design-only, exactly as their Gate/blocker columns say. **P2P gate recorded in-file: zero new DHT entry types** |
| [design-legibility-borrows-backlog](epr:design-legibility-borrows-backlog) | *Externally-sourced* devices for making a quantity felt — palette registry, tension render, area-preserving surface (Playnet-derived) | 7 | 2026-08-05 | Render-half sibling of the measure cluster; rows 2–3 must land paired with their measure halves |
| [commons-holonic-stewardship-backlog](epr:commons-holonic-stewardship-backlog) | How a holon holds standing — custody≠ownership (VF rights/custody split), steward-as-path, nested per-holon elohim ceilings, credential-as-lens | 26 | 2026-08-27 | Rows 1–2 are Playnet borrows; 3–8 our design frontier. **Row 26 (custodial account class — kids/wards/IDD/seniors) is time-sensitive**: `StewardshipGrant`/`StewardshipAppeal` face caller-count pruning at imagodei Stage G with zero consumers above the DNA; it is row 2a's `subjectStanding` at its sharpest, and its first deliverable is an a2o scenario, not code. Row 1 has **zero** implementation in-tree. **Row 2a (`subjectStanding`) + the row-2 carve-out are red-team-derived and operator-gated** — see the inalienable red-team spec. Rows 9–12 are Meadows-derived: **row 9 (World3-as-lens over the aggregated dataset) is the Global Orchestra's deliberation surface** and row 10 (per-fold anonymity) blocks its network rung. **Rows 17–23 are the 2026-08-23 succession/monetary mint** — liability-absorption ledger, credit-limit invariant, negotiated personal equilibrium, derivable-projection invariant, `binds-policy` (the blocker for all of them), currency-as-Mishpat-lens with its eight never-rules, and the three-function MoE/SoV/unit-of-account decomposition. Row 13–15 are the operator's 2026-08-11 dispatch — index lenses (World3/GNP/GDP/Donut) as a named epic deliverable, statistical-method application as a **ceiling** deliverable composing onto graduated-rigor, and the "fruits" thesis kept where the work is. **Rows 16/16a are the operator's 2026-08-15 dispatch** — agent-capability attestation as a policy lens over imported benchmark measures (the blind-reader/scribe `claude-opus-4-6` pin is the named degenerate case), paired 2/2a-style with the inclusion constraint (lamad/avodah earnable paths + plural DID authorship) |
| [algedonic-phase2-network-phase3-dedupe](epr:algedonic-phase2-network-phase3-dedupe) | The algedonic feedback-signal arc past local-first — network graduation (CI/CD as producer) + slice-2 protocol-wiring burn-down | 12 | 2026-08-11 | Phase 1 (EPR-level, local-first) landed; this is the single re-surfacing point for phase 2/3. Phase-2 row 6 (Meadows respite/response ratio) is proposed as the graduation criterion past threshold-firing |
| [arch-frontend-bundle-seams-backlog](epr:arch-frontend-bundle-seams-backlog) | Frontend bundle seams — pillar/core placement, the SDK↔bundle gospel chain, silent renderer-wiring gaps | 10 | 2026-08-11 | Ceremony-derived (2026-08-11 substrate-currency, four-lens + two verify lenses). Row 1 (lamad houses the cross-pillar content substrate; core→pillar dependency arrow) is an **architect decision** with three candidate resolutions; row 2 (two ContentServices) is separable and worth doing regardless. Rows 3-7 are gospel drift the drift-graph *should* have caught — row 7 (two SDK surfaces carry no `cites:` frontmatter) is why it didn't |
| [agentic-harness-borrows-backlog](epr:agentic-harness-borrows-backlog) | *Externally-sourced* context/state disciplines for long-horizon agent runs (OpenAI harness engineering, Anthropic science/harness/managed-agents, Arize, Symphony, backstitch) | 10 | 2026-08-15 | Minted from the [context-engineering survey](epr:context-engineering-primary-sources-cross-pollination-2026-08-13). External-borrows sibling of the internal [agentic-context-tooling-consolidation-queue](epr:agentic-context-tooling-consolidation-queue). Rows 2, 5, 6 are **mintable now** (zero/near-zero tooling; row 6 is a package-governed ceremony-gate edit via `plant-eprfs-skill`); row 1 (per-turn run-state projection) must ship with a `retire-when:`; row 3 is a **measured live defect** (root `AGENTS.md` 42,327 B vs Codex's 32 KiB budget, truncating at `AGENTS.md:308`) whose repair goes through package authority; row 4 carries the §4.1a **plane-typing caution** (ephemeral scheduler reservation ≠ durable REA commitment); row 7 composes with [measure-family-borrows](epr:measure-family-borrows-backlog) rows 12–14 and is operator-gated on habit admission. **Design pass ran 2026-08-13**: rows 1/2/5 → [Spec A](epr:run-plane-projection-observation-events), row 7 → [Spec B](epr:dev-system-equilibrium-stocks), row 4 → [Spec C](epr:commitment-dispatch-puller) (decision-track), rows 3/6/8 + all tasks → [the implementation plan](epr:agentic-harness-borrows-implementation-plan). **Implementation began same day** (operator go): T1 `epr flow note` (9082a42) + T2 `epr flow stocks` (cae50c1) landed, `dev-system-equilibrium` habit flipped RED on live evidence (commitments +22/wk vs 0 drain); row 9 (rate-based stasis criterion for the loops) minted from the first real authored run-note, jointly designed with row 4 |
| [arch-dataplane-sdk-proposal](epr:arch-dataplane-sdk-proposal) | Dataplane SDK surface (Artifact 2 of the 2026-06-11 review) | — | 2026-06-11 | Proposal-shaped, not a ranked table |
| [agentic-context-tooling-consolidation-queue](epr:agentic-context-tooling-consolidation-queue) | Agentic tooling consolidation | — | 2026-08-12 | Queue-shaped, not a ranked table. Items 16–18 are Meadows-derived. **Item 16 (`retire-when:` removal conditions) DELIVERED 2026-08-11** (`63e81325c`) — mechanism live on both `.epr-meta` gates + 8 hooks, two-implementation hash-exclusion invariant, `intervenor_census.py` meter; remainder is the backfill (58 of 99 intervenors still lack an exit, live via `placement-audit.py --epr-meta`) — stays open until drained. Items 17–18 (capacity-as-vector delegation, cost-per-verified-result) remain design-only |
| [arch-scale-risk-backlog](epr:arch-scale-risk-backlog) | **Risk rows** — landed-code shapes that grow badly with chain length, peer count or migration count; each pinned to a file, a measurable trigger, a horizon and the change that retires it (tag `risk`; discipline in CONVENTIONS.md §Risks) | 6 | 2026-09-04 | Born from the Holochain Evolution Epic code read (quadratic export walk, O(W²) witness validation, chain doubling + held-carry fan-out, dual-cell window memory, per-entry idempotency reads, controller sweep load). Rows 1–4 also sit in the `happ-lineage-migration` habit's `guard:`; a row that FIRES flips to `regression` + a chronicle entry. Sibling: `dht-scale-envelope-…` holds the planetary question |
| [mesh-prologue-cast-and-env-gaps](epr:mesh-prologue-cast-and-env-gaps) | Act I local-mesh Prologue — cast (named conductors), env legs CI sets that `just mesh` must set, per-host bundle staging, household fixture manifest; siblings: `doorway-warm-shell-local-archive-mesh-provable` (mongod leg, landed), `seam-registry-schema-invalid-row-silently-dropped` | 8 | 2026-08-21 | Born from the first full a2o inventory + saga run on the mesh (17/2 on the saga; 25 code-reds, 9 fixed same day). Each row is an env/cast leg, not a product defect — closes when `just mesh start` + one seed verb yields the saga green through ch10 |

Candidates for future clustering (standalone entries with visible siblings): the `a2o-*` family;
the `alpha-*` incident family (some are chronicle-shaped, not backlog-shaped); the recovery/identity
`agent-peer-binding-*` pair.

## The provenance chain (storytelling as compression)

A concern that travels the whole pipeline leaves a walkable story, each hop content-addressed:

```
research (messy desk)          survey closes with a MINT PASS
   └─ cites →  cluster row     graduates when picked for work
        └─ cites →  spec/plan  (p2p-design-gated where it touches entities; decomposes to gaps)
             └─ cites →  code + a2o scenario   (shift lands it; story-harvest preserves constraints)
                  └─ cites →  chronicle entry  (historian compresses the arc at close)
```

Every hop cites *backward* via `epr:` slugs (content-addressed — survives file moves), so the
chronicle entry at the end is the **compressed story**: reading it and following cites reconstructs
the full why — which survey found it, which cluster held it, which spec shaped it, which commits
landed it, which scenario guards it. The row's `status` (the shared delivery-status axis) is the
chain's live position; `epr flow walk` renders the same chain as a valueflow. A row with no forward
cite is *waiting*, not lost; a spec citing no row is the smell (where did it come from?); a landed
change with no chronicle is a story not yet compressed. Enforcement is deliberately light: the
`.epr-meta` here nudges cluster-first at birth; the rest is convention until a measure earns its
headline token.
