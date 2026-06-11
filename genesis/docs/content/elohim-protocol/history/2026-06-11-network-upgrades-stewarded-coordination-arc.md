---
title: "History: The NETWORK_UPGRADES stewarded-coordination arc (Dec 2025 – Jun 2026)"
id: network-upgrades-stewarded-coordination-arc
type: history-gotcha
status: noted
tier: history
created: 2026-06-11
topic: [holochain, dna, upgrades, lineage, network-seed, manifest-hygiene, rna, governance, design-arc]
# Provenance breadcrumb: the retiring island doc this record distills.
derived_from:
  - elohim/holochain/dna/NETWORK_UPGRADES.md  # retired to git 2026-06-11 (holochain dna/ island recompose)
canonical:
  - genesis/docs/content/elohim-protocol/architecture/2026-06-11-dna-upgrade-governance.md
  - elohim/holochain/rna/README.md
cites:
  - elohim/holochain/rna/README.md
  - elohim/holochain/tests/manifest-hygiene/tests/manifest_hygiene.rs
  - elohim/holochain/tests/manifest-hygiene/README.md
  - elohim/holochain/dna/elohim/dna.yaml
  - elohim/holochain/dna/elohim/Cargo.toml
  - elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs
  - cluster3-substrate-signal-migration-governance-signal-flow-design | where the upgrade constraint first priced a real schema decision — the alpha-pair reinstall + pre-field lineage fork | sha256:b758c9c0959c0fef | path: genesis/docs/superpowers/specs/2026-06-09-cluster3-substrate-signal-migration-governance-signal-flow-design.md
  - rno-reference-implementation-positioning | carries the wave-1-era positioning after the wave-1 execution plan left the tree | sha256:1964362b51d396b9 | path: genesis/docs/content/elohim-protocol/history/2026-06-02-rno-reference-implementation-positioning.md
---

# History: The NETWORK_UPGRADES stewarded-coordination arc (Dec 2025 – Jun 2026)

> **Hot-context pointer (the one sentence to remember):**
> A doc's philosophy and mechanics can each graduate to better homes (rna/README;
> an enforcing test crate) while the file quietly becomes the *only* copy of the
> policy in between — and its STATUS note can call a crate "backburner" that is
> compiled into every DNA build. Check residue AND liveness before retiring prose.

## The question and the answer (2025-12-14)

NETWORK_UPGRADES.md was born alongside the RNA migration toolkit
(`a815c944c`, 2025-12-14, "feat: add Holochain RNA migration toolkit") asking one
question: is "DNA hash = network identity" a feature or a bug? Its honest answer —
a trade-off optimized for adversarial trustless networks, which a stewarded network
pays for without the full benefit — resolved into the doc's thesis: **"The Elohim
ARE the bridge."** Stewarded coordination: constitutional stewards run both DNA
versions during a migration window, provide the coordination signal pure P2P lacks,
and cannot change the rules — only facilitate transitions.

## The philosophy graduated (and evolved past its origin)

That thesis did not stay in the doc. Its living home is elohim/holochain/rna/README.md
(§Constitutional Evolution onward), where it grew past the original framing: DNA
immutability as a *governance primitive* ("no one can unilaterally change the rules
everyone operates under"), the elohim as **consensus-finders** translating agreement
across global/constitutional/local/personal levels, and the hash change reframed as
the **constitutional checkpoint** that makes agreement legible. No later canon
*inverted* the source doc's claims — the philosophy-inversion check came back clean —
but the center of gravity moved: the origin doc argued stewards solve a coordination
*weakness*; the successor text treats the constraint itself as the governance
*feature* the stewards serve. Same answer, inverted valence.

## The mechanics were appended, then enforced (2026-04-21)

Sixteen weeks later the doc acquired a second life: the Forward-Compat Policy
("Wave 1 §7") was appended (`c1a23bb2a`, 2026-04-21) — additive-vs-breaking rules,
the lineage ledger, the `_alpha → _beta → prod` seed ladder — and the same day the
manifest-hygiene crate landed (`948b02ad6`) to enforce it as a 0.01s pre-push
schema-contract test, with "Manifest hygiene (Wave 1 / §7)" comment blocks stamped
into all five dna.yaml files + happ.yaml. Prose became contract. (The appended policy
also contained a contradiction its own table caught — serde-default entry fields and
new link types listed as "no hash bump" when integrity-zome code changes always bump —
resolved only in the 2026-06-11 successor seed, §2 there.)

## The regression was recorded in place (2026-04-24)

Three days later the policy's centerpiece regressed: Holochain 0.6 gates the
`lineage` manifest field behind the `unstable-migration` cargo feature, and stable
`hc dna pack` rejects it. The field was stripped from every manifest, the hygiene
check asserting it was deleted (`9855133d2`, 2026-04-24), and the regression was
documented at all three layers — the doc's STATUS banner, a tombstone comment where
the test had lived (manifest_hygiene.rs:165-170), and each dna.yaml's hygiene block.
Today's mechanics: upgrade history reconstructed from git + network_seed rollover.

## The retirement shape (2026-06-11)

By recompose time the doc's three lives had three different fates:

- **Philosophy** — graduated to rna/README.md; the seed only summarizes and points.
- **Mechanics** — enforced by manifest-hygiene; but the "wave-1 execution plan §7"
  that the crate's README still cites left the tree (the path at
  tests/manifest-hygiene/README.md:68 resolves to nothing; the plan's positioning
  survives in the 2026-06-02 rno-reference-implementation-positioning history
  record). The residue test on NETWORK_UPGRADES came back **positive**: the
  forward-compat policy, lineage ladder, and seed-suffix contract were homed
  nowhere else. The new seed
  (architecture/2026-06-11-dna-upgrade-governance.md) is now the policy's doc home
  and the target the dna.yaml comments resolve to.
- **The STATUS note's liveness claim** — wrong by omission. It called the rna/
  module "currently on the backburner," and the migration *workflow* is indeed
  unwired (no import/transform consumer, templates and TS package zero-consumer).
  But the `hc-rna` crate is a workspace dependency of the shipping DNA
  (dna/elohim/Cargo.toml:15): the integrity zome validates entries through
  `hc_rna::SelfHealingEntry` (content_store_integrity/src/lib.rs:4272) and the
  coordinator boots a `FlexibleOrchestrator` from it at init. "Backburner" prose
  over a live compile-time dependency survived 6+ weeks because nobody renders a
  Cargo graph while reading a markdown STATUS banner.

Meanwhile the policy's sharpest edge went live elsewhere: the alpha pair's
`ALLOW_DNA_REINSTALL` + pre-field lineage decision is an operator-owned fork in the
cluster3 substrate-signal spec (§2.6, §8) — the first time the upgrade-governance
constraint priced a real schema decision ("the metadata_json route makes this
tradeoff vanish entirely").

## The lesson (one line)

**Prose policy rots in three directions at once — philosophy outgrows it, tests
out-enforce it, and liveness claims out-date it; before retiring a doc, test what
it's the last copy of, and verify every "dormant" label against the build graph.**
