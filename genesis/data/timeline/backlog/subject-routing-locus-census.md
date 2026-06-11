---
id: "backlog-subject-routing-locus-census"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Subject-routing locus census — declare the ~20 missing monorepo loci into the cite + routing discipline (lamad landed as the reference template)"
slug: "subject-routing-locus-census"
written: "2026-06-11"
author: "operator + locus-census workflow (46-agent fan-out, adversarially verified)"
status: "refined"
priority: "medium"
tags: [subject-routing, cite-discipline, decomposition, loci, cascade, managed-surfaces]
cites:
  - .claude/subject-routing.yaml
  - app/lamad/CLAUDE.md
  - elohim/sdk/domains/lamad/CLAUDE.md
  - .claude/scripts/_lib/subject_routing.py
---

# Subject-routing locus census

A 46-agent fan-out (`locus-census` workflow, adversarially verified) confirmed which in-tree monorepo
sub-trees are **subject loci** missing from the cite + decomposition-routing discipline. Submodules
(`brit`, `rakia`, `rust-ipfs`, `sophia`) were excluded — they own their own cascade (see
[[task: parent-agnostic submodule composition]]).

**Result: 25 candidates · 19 true loci · 22 need routing work.** `app/lamad` is **done** this session as the
reference template; this entry is the work-list for the rest. Each row's routing action mirrors the lamad
pattern: **(a)** id-anchor the subject home's gospel, **(b)** cite-rail consumers to it, **(c)** stamp+decompose
island docs, **(d)** declare a `.claude/subject-routing.yaml` sub-manifest only where a real home-remap is needed.

## Cascade prerequisites (two findings — one fixed, one deferred)

The first sub-manifest (`app/lamad/.claude/subject-routing.yaml`) exercised the cascade resolver for the first
time and exposed two latent issues:

1. **`find_repo_root` shadowing — FIXED 2026-06-11.** It treated any `subject-routing.yaml` as a repo-root
   signal, so the first sub-manifest *shadowed* the root constitution instead of composing on top of it
   (`classes` lost). Fix: the `.git` boundary is authoritative; a sub-tree manifest is a cascade member.
   Regression-tested in `.claude/scripts/_lib/__tests__/subject_routing_test.py` (3 new asserts). Bonus: this
   makes submodules bound at their own `.git` — parent-agnostic by construction.
2. **`_merge_into` shallow-replace — DEFERRED (blocks functional per-locus remaps).** `base.classes[cls] = spec`
   is a FULL replace, not a deep merge — so a *partial* `classes.<x>:` override in a sub-manifest clobbers the
   root's `write_location`/`status_modes`/`discard`. Until the resolver deep-merges, sub-manifests must stay
   **declarative** (locus identity + `default_class`), NOT override `classes`. The lamad sub-manifest documents
   this. **Action: add deep-merge to `_merge_into` (per-key, nearest-wins) so a locus can pin `pillar:` etc.
   without re-stating the whole class.**

## Locus registry (corrected truth = verifier's adjudication, not the census over-claim)

Priority = leverage: subject-home anchors first (pillars cite them), then big island-doc trees.

| # | Locus | Verdict | Class | Corrected truth | Gospel | Action | Pri |
|---|---|---|---|---|---|---|---|
| ✅ | `elohim/sdk/domains/lamad` | true-locus | protocol-canonical | self | **id'd `lamad-domain-gospel`** | DONE (template) | — |
| 1 | `elohim/sdk/domains/qahal` | true-locus | protocol-canonical | self | plain | id-anchor + cite `2026-05-21-qahal-architecture-vision` | high |
| 2 | `elohim/sdk/domains/imagodei` | true-locus | protocol-canonical | self | plain | id-anchor (`imagodei-domain`) | high |
| 3 | `elohim/sdk/domains/shefa` | true-locus | protocol-canonical | self | **cited** | confirm id; consumer-rail app/shefa | low |
| 4 | `app/.../app/qahal` | true-locus | protocol-canonical | self + sdk/domains/qahal | plain | seal `QAHAL_API_SPECIFICATION_v1.0.md`; cite-rail; sub-manifest | high |
| 5 | `app/.../app/elohim` | true-locus | protocol-canonical | **self** (`models/` canonical, not sdk) | plain | id (`elohim-protocol-core`); decompose `ELOHIM_PROTOCOL_ARCHITECTURE.md`, `ARCHITECTURE.md` | high |
| 6 | `app/.../app/imagodei` | consumer-only | protocol-canonical | sdk/domains/imagodei | plain | consumer-rail (cite #2) | med |
| 7 | `app/.../app/shefa` | consumer-only | protocol-canonical | sdk/domains/shefa | plain | consumer-rail; route `README-EXCHANGE/-INSURANCE-MUTUAL/banking-bridge` | med |
| 8 | `app/.../app/avodah` | true-locus | protocol-canonical | sdk/domains/avodah (D1 demo) | plain | id + consumer-rail preamble | low |
| 9 | `app/elohim-app` (shell) | **consumer/delivery** | protocol-canonical | **NOT self** — sdk/domains + elohim-storage | cited | **MOVE mis-placed `ELOHIM_PROTOCOL_ARCHITECTURE.md`, `QAHAL_API_SPECIFICATION.md`, `ARCHITECTURE.md` out of shell root → pillar loci**; multi-pillar sub-manifest | high |
| 10 | `elohim/elohim-storage` | true-locus | protocol-canonical | self | plain | id; route `P2P-ARCHITECTURE`, `EDGE-ARCHITECTURE`, `REACH` | med |
| 11 | `elohim/holochain` | true-locus | protocol-canonical | **derived_from** genesis/docs/content/elohim-protocol | plain | id (`holochain-substrate-notary`) **as derived_from, not truth:self**; route 11 ARCHITECTURE docs | med |
| 12 | `doorway/doorway-service` | true-locus | protocol-canonical | **refinements OF** genesis resilience spec (derived_from) | plain | id; route FEDERATION/RECOVERY-PROTOCOL as derived, not canonical | med |
| 13 | `elohim/elohim-cache-core` | true-locus | protocol-canonical | self | plain | id; cite `2026-03-29-elohim-cache-core-extraction-cache-design` | med |
| 14 | `elohim/sdk` (root) | true-locus | **mixed** | self | plain | id; per-subcomponent split (schemas/storage-client-ts=process-meta; domains/src=protocol-canonical — root already routes `schema-sdk→process-meta`) | med |
| 15 | `steward/node` | true-locus | protocol-canonical | self | plain | id; route `ARCHITECTURE`, `P2P-COMPUTE-FOOTPRINT` | low |
| 16 | `steward/device` | true-locus | protocol-canonical | self + genesis | plain | id; cite-rail genesis protocol docs | low |
| 17 | `bridges` | true-locus | protocol-canonical | Wave3 valueflows-hrea spec | plain (root-cited L96) | confirm: join loci or stay infra-coupled-only | low |
| 18 | `app/.../projects/elohim-service` | consumer-only | mixed | storage-client generated types | plain | consumer-rail; CLI=dev-tooling | low |
| 19 | `app/.../projects/perseus-plugin` | consumer-only | protocol-canonical | lamad manifest (sophia-quiz-json) | plain | consumer-rail (lamad's renderer) | low |
| R | `elohim/elohim-token` | **research-locus** | protocol-canonical | genesis/plans + protocol-spec | plain | id research docs OR decline gospel; **defer island retirement until impl** | low |
| — | `app/elohim-elements` | true-locus | protocol-canonical | self | cited | **already adequate** (but stale hash in consumers → re-bless `35a70e2664d5ec3f`) | hygiene |
| — | `app/.../projects/graphos` | true-locus | protocol-canonical | parent `elohim-library` gospel | none | **parent-gospeled — no child gospel**; `src/imported`=narrative-context (unrouted) | skip |
| ?? | `elohim/elohim-agent` | — | — | — | — | **RE-CENSUS** (empty result) | recensus |
| ?? | `doorway/doorway-app` | — | — | — | — | **RE-CENSUS** (empty result) | recensus |

## Dispositions worth not re-deriving

- **Truth ≠ self for delivery/implementation layers.** The verifier corrected three over-claims: the
  `elohim-app` shell, `holochain`, and `doorway-service` *serve/implement* but do not *own* canonical truth.
  Their gospels should carry `derived_from:`/consumer-rail cites to the genesis/sdk/storage truth, not `truth: self`.
- **The shell root is a dumping ground.** `ELOHIM_PROTOCOL_ARCHITECTURE.md` + `QAHAL_API_SPECIFICATION_v1.0.md`
  sit in `app/elohim-app/` AND in the pillar subdirs — they belong in the pillar/sdk loci, not the shell. Row 9
  is the highest-leverage cleanup.
- **Don't manufacture loci.** `graphos` is gospeled from its parent; `elohim-elements` is adequate. A cite-seal
  there would be duplication, not discipline.
- **`elohim-token` is research, not a discipline locus** — verdict downgraded to research-locus; its theory docs
  stay as design-context appendices until implementation begins.

## Done this session (the template)

`elohim/sdk/domains/lamad/CLAUDE.md` → `id: lamad-domain-gospel`; `app/lamad/CLAUDE.md` cite-rails it with a
relationship hint + a **code-citation discipline** section (`// subject: lamad-domain-gospel` breadcrumbs +
generated-provenance-as-citation); `app/lamad/.claude/subject-routing.yaml` declared as the monorepo's first
cascade sub-manifest; resolver shadow-bug fixed + regression-tested. Replicate per row.

## Progress — 2026-06-11

**Subject-vocabulary homes anchored** (lamad template replicated across `elohim/sdk/domains/*`):

| Home | id | cites design canon | Consumer pillar cite-rail |
|---|---|---|---|
| lamad ✅ | `lamad-domain-gospel` | — | `app/lamad` (`lamad-bundle-gospel`) |
| qahal ✅ | `qahal-domain-gospel` | `qahal-architecture-vision` | `qahal-pillar-gospel` |
| imagodei ✅ | `imagodei-domain-gospel` | `imagodei-surfaces` | `imagodei-pillar-gospel` |
| shefa ✅ | `shefa-domain-gospel` | Shefa whitepaper (legacy-path, un-id'd) | `shefa-pillar-gospel` |

Each: sdk/domains home id-anchored + cites its design canon (drift-tracked); the app pillar consumer
(`<x>-pillar-gospel`) cite-rails the home + carries the **code-citation discipline** section
(`// subject: <x>-domain-gospel` breadcrumbs). All cites verified resolvable.

**Remaining subject homes:** the no-gospel domains (`avodah`/`elohim`/`infrastructure`/`mishpat` — create-or-decline,
sibling task) and the implementation-layer truth homes (`elohim-storage`/`elohim-cache-core`/`holochain`/
`doorway-service`/`steward` + `sdk` root) — larger, truth-disputed island trees (handle per the corrected-truth column).

### Implementation-layer homes anchored — 2026-06-11

| Home | id | truth | cites canon |
|---|---|---|---|
| `elohim/sdk` | `elohim-sdk-gospel` | derived | `elohim-sdk-architecture` |
| `elohim/elohim-storage` | `elohim-storage-gospel` | self | `tiered-quilt-stewardship-design` |
| `elohim/elohim-cache-core` | `elohim-cache-core-gospel` | derived | `tiered-quilt-stewardship-design` |
| `doorway/doorway-service` | `doorway-service-gospel` | derived | `resilience-protocol-spec` |
| `elohim/holochain/dna` | `holochain-integrity-layer-gospel` | derived | `elohim-protocol-specification` |
| `steward/device` | `steward-device-gospel` | derived | `iroh-libp2p-complementarity` |

Also **id'd 3 foundational canon docs** so the home→canon cites are drift-tracked envelopes (and the corpus
gains first-class anchors): `elohim-protocol-specification` (protocol-specification.md), `resilience-protocol-spec`
(resilience/README.md), `elohim-sdk-architecture` (architecture/elohim-sdk.md).

**`steward/node` has NO gospel** (only ARCHITECTURE.md) → folded into the no-gospel create-or-decline set
(`avodah`/`elohim`/`infrastructure`/`mishpat`/`steward-node`). Island docs still to route+retire:
elohim-storage {EDGE/P2P-ARCHITECTURE, REACH}, doorway-service {ARCHITECTURE, FEDERATION, SCALING, RECOVERY-*,
EDGE-DESIGN}, holochain {LINK_ARCHITECTURE}.
