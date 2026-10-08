---
title: "Permaculture DAO / Prometheus — Verification-First Ecological Evidence on Holochain: Cross-Pollination and Partnership Gap Analysis"
id: permaculture-dao-prometheus-cross-pollination-2026-10-08
status: Capture
date: 2026-10-08
sovereignty-frame: bridge-legibility
---

# Permaculture DAO / Prometheus Cross-Pollination — October 2026

**[Permaculture DAO](https://github.com/Permaculture-DAO)** (Permaculture DAO LLC, Florida; one public maintainer, Uwohali Tuccio, Italy) is building **Prometheus**: a verification-first evidence and claims control plane for regenerative land projects, with a single-DNA Holochain hApp ("hearth") as the pilot runtime, a Python ingestion pipeline, a GPG-signed canon, and a Sicily field pilot in pre-registration. It came to our attention on 2026-10-08 through an inbound email from Holo's head of growth asking about "what you're doing with Prometheus"; resolving which Prometheus was meant led to the [hAppRadar listing](https://happradar.com/projects/prometheus-happ) and this survey. The name guard below exists because the other reading of that question, the Prometheus metrics exporter in our conductor fork (`holochain_metrics/src/prometheus.rs`), is at least as likely to be what was asked. It is the closest external mirror we have found for three things at once: the **evidence ladder** (claim → observation → evidence → provenance → admissibility → value), the **canon-versus-runtime split** we practise as docs-lead-code-follows, and **ValueFlows on a Holochain DHT** for something other than exchange.

**One-line verdict: Prometheus is a sibling in discipline and a stranger in substrate. Their honesty rails are the thing to lift; their runtime is a single-cell pilot that our observation plane, attestation consolidation and release machinery already generalise. The partnership shape is a consuming app on our bridges seam: they bring the domain, the sites and the reviewers; we bring the plane. The next thing we do is write to them.**

Method: two grounded-research agents (Sonnet) read every public repo in the org via the GitHub API (no clones; disk was at 97%), with exact quotes, file paths and UNVERIFIED markers; one Explore agent mapped the Elohim seams the verdicts adjudicate against (file:line); the orchestrating session wrote this survey; a separate, context-isolated agent then read it *from their side* (final section). Verification key: ✅ verified in source · ◐ single-source/plausible · ⚠ web-only/unverified.

The `sovereignty-frame` above is declared because their constitutional texts use "Syntropic sovereignty" and the pre-v1.1 guide an "Agent Sovereignty" eternity clause. We quote those as bridge vocabulary only. Elohim's identity is imago dei, backstopped by community and institutional expression; agency comes from the community backstopping the person, never from a self-asserted key. Where their texts converge on that ("Human dignity is greater than any score, signal, token record, wallet history…"), we say so; where they lean on a key as the root, we do not import it.

---

## ⚠️ Name guard (read first)

| Name | What it is here | What it is not |
|---|---|---|
| **Prometheus** | Permaculture DAO's regenerative-evidence project (h•eart•h intelligence) | Prometheus the metrics system: our conductor fork carries a text-exposition exporter for the conductor's OpenTelemetry instruments (`HOLOCHAIN_PROMETHEUS_LISTEN`, the `hc_*` series the performance deep dive reads) |
| **hearth** | Their single DNA (`dnas/hearth/dna.yaml`), app id `hearth_prometheus` | `topeuph-ai/hearth`, the family-care-record hApp featured on hAppRadar (Holochain 0.7.0, v0.3.4) ✅ |
| **Genesis** | Their "Genesis Institutional Canonical Line" v1.1 and the Sicily Genesis pilot | Our `genesis/` content tree |
| **Permaculture DAO** | A Florida LLC with one public member; "DAO" is a name, not a token or chain | A Hypha-style DAO; there is no token, chain, or governance contract anywhere in the org ✅ |
| **RAVEL** | Their risk layer (Risk Allocation, Vulnerability, Exposure & Loss), shadow mode only | Anything of ours |

---

## Subjects surveyed (compressed; full dossier in the method note)

**The org.** Ten public repos, all 0 stars, created from 2026-03-06. Three live and pushed on 2026-10-08: `prometheus-canon` (the white paper and release discipline), `prometheus-happ` (the Rust hApp), `prometheus-runtime` (Python, FastAPI-shaped, PostgreSQL, "S0 sanitized candidate; production admission gated", `gate_status.json` G2 "RED / NO-GO", `production_admitted: false`). One signed-release registry, one org profile, five frozen `legacy-*-pre-v1.1` archives from the earlier "Rigenera" pilot. The six sibling repos the hApp README names (bridge, console, evaluation-stack, ops-docs, pilot-handoff-pack, mock-backend) are **private** ("remain private pilot workspaces until their own publication, licensing, privacy, and release gates are cleared"). ✅

**The canon.** Source of truth is a 4.1 MB DOCX, with a strict Markdown derivation (59,343 lines) kept as provenance; signed `v1.1.2-genesis` under one GPG key with the custody disclosed as "substantially single-source, single-authority signature". A `v1.2.0-rc.2` candidate (77 KB, "AI-authored (Claude Code) … not ratified") reorders the ontology: **OHE** (One Human Ecosystem, the minimum deployable unit) → **MRV** (observation, evidence, provenance, confidence, admissibility) → **PRU** (an analytical index "like a rating", with a maturity ladder R0 conceptual … R7 legally structured) → **RAVEL** → **RAP** (portfolio aggregation, M4 … M7) → legal wrapper → capital. **TRBK** has drifted between releases from "consumptive access credit" to "external to the canon; no Prometheus rights"; **HoloFuel** is "an operational network resource for transaction/coordination only. Not a value or admissibility input." The controlling expression is `V_PRU_adj = V_base × LegalGate × MRVGate × (1 + U_max × g(Z)) × Conf_total`, with "if PilotEvidence = 0, then U_max_capital = 0". The normative stack rule: "canon → claims register → methods/MRV → schemas → hApp integrity rules → runtime/bridge/API → console/website → deck, data room and marketing. A downstream artefact may not contain a stronger semantic claim than its upstream controlling source." ✅

**The hApp.** Holochain 0.6.1 / hdk 0.6.1, UNLICENSED, one DNA, six public entry types, five link types, fourteen coordinator externs (create and get only, no update or delete). The integrity zome's `validate` checks only `StoreEntry` Create and Update; links, deletes and agent joins all return `Valid`; there is no membrane and no author or capability check. What it does validate is sharp:
- `MrvEvidenceEntry.evidence_class` is `TEST` or `REAL`, never interchangeable. TEST subjects must be `TEST-`-prefixed. REAL requires a 64-hex `calibration_hash`, and then **the integrity zome refuses it unconditionally** with `REAL_DATA_PERSISTENCE_GATE_CLOSED` ("This rule belongs to the DNA, not a replaceable coordinator. A correctly shaped hash is not proof of registry membership"). No REAL evidence can be persisted by any coordinator on this DNA. ✅
- "capture evidence cannot self-declare reviewer; use separate attestation workflow". ✅
- `ClaimEntry` tiers are an ordered enum `Architectural < Methodological < Hypothesis < PilotObserved < ThirdPartyReviewed < LegallyAdmitted < MarketAdmitted`; an `investor_facing` claim below `PilotObserved` is refused as "overclaiming". "THE LOWEST UNRESOLVED GATE CONTROLS THE PERMITTED CLAIM. Gates are BINARY. No PRU value is ever computed in the runtime." ✅
- `RavelAssessmentEntry` is valid only with `vrrc == 0.0`, `vrrc_status == "not_admitted"`, `mode == "shadow_underwriting"`, `authority_boundary == "evaluation_not_certification"`; `RavelBrakeSignalEntry.autonomous_enforcement` must be false. The invariants of a layer that is not allowed to act are written into the DNA. ✅
- No-double-counting: a domain identity tuple `(subject_id, indicator, observed_at)` checked in a `BTreeSet`, and a separate durable idempotency anchor `evidence_identity:{subject}:{sensor}:{indicator}:{observed_at}`, by their own comment "a single gateway/agent durable idempotency boundary… not a universal cross-agent duplicate-proof claim". ✅
- A hand-rolled ValueFlows module (`Agent`, `ResourceSpec`, `Process`, `EconomicEvent{Consume, Produce, Use, Work}`) is compiled and unit-tested but **never persisted as entries**. No hREA. ✅

**The ingress.** A Node gateway on a Raspberry Pi 5: LoRaWAN → Mosquitto → `prometheus/<subject>/<indicator>` → `evidence.mjs` (a 27-indicator allowlist "from MRV_BASELINE_PROTOCOL_SICILY.md": soil moisture, precipitation, soil organic matter, bulk density, pH, NPK, infiltration rate, NDVI, biodiversity count, water/fertiliser/fuel/labour inputs, yield per species, land equivalent ratio, geo photo…; per-indicator ranges; an explicit boolean `test` flag required; HMAC-SHA256 over a canonical-JSON envelope, compared with `timingSafeEqual`) → one zome call per reading, no batching. `PROMETHEUS_ALLOW_REAL=0` forces every reading to TEST; the go-live checklist in `.env.example` names reviewer appointment, frozen pre-registration with a GPG-signed SHA256, TEST-only commissioning, a subject allowlist. "The baseline is the FIRST real evidence after this gate opens… collection != admissibility." ✅ The `MRV_BASELINE_PROTOCOL_SICILY.md` it cites is in no public repo. ⚠

**The runtime.** `prometheus-runtime` is a FastAPI, PostgreSQL and nginx compose stack (Python 3.12) with a Holochain profile disabled by default; its bridge exposes only `/health`. It **never commits to Holochain**: an evidence batch carries `holochain_reference.mode` as the constant `"simulated_reference_only"`, `commit_performed` constant `false`, `entry_ref` constant `null`. Stages S0 (freeze and rotate secrets; a CI guard strips the conductor's wildcard origin), S2 (publication checklist), S3 (JSON-schema data contracts for device registry, decoder registry, sensor event, observation, evidence package, evidence batch, claim, admissibility decision, relationship assertion, review attestation, disturbance), S4 (adapters `ttn_mock`, `manual_mock`, `lab_mock`; the intake endpoint always answers HTTP 423), S5 (normalisation; event identity `"evt_" + sha256(source_system, source_event_id, device_id, sensor_id, observed_at, raw_payload_sha256, decoder_id, decoder_version)`), S6 (deterministic batch hash). There is no S1 anywhere. A later "evidence spine" adds observation → evidence_package → review_attestation → admissibility_decision with conflict-of-interest rules and a `claim_uid` registry. What they measure, from the contracts: soil moisture (percent VWC), soil organic matter, soil and air temperature, humidity, rainfall, battery voltage; device classes soil sensor, weather sensor, gateway, irrigation controller; sources LoRaWAN via TTN, lab PDFs, manual CSV, Open-Meteo, Copernicus STAC, Home Assistant, Node-RED, Rain Bird; disturbances drought, heatwave, flooding, pest, irrigation interruption. The canon lineage is two tracks "mapped, not merged": the runtime boots fail-closed against four internal `v7.0.x` documents that are not public, while canon authority is the v1.1.2 line; a July pull request that flipped the pointer was reverted. The recurring rule: "Holochain remains a provenance and batch-reference layer, not a raw telemetry database." An open PR #17 is a "Holochain 0.6.1 to 0.7 compatibility gate" ("pending a complete compatibility matrix"). ✅

**The lineage.** The pre-v1.1 hApp (February to May 2026) registered exactly one entry type, `Message{content}`, with no `validate` callback and no links, behind an unrestricted capability grant; the June 2026 rewrite is where every rule quoted above was born, in a fortnight of Claude Code pull requests. The legacy "pilot handoff pack" has only ever contained two README files; the domain model, API contract and validation rules it advertises were never published, and nothing public names the Rigenera site, partners, crops or sensors. ✅

**The evidence of their own evidence.** Six runtime "proofs" from May and June 2026 prove only that `hello_benchmark_layer` returns a sentinel string and a health endpoint answers. The 2026-10-02 conductor receipt in `RUNTIME_BOUNDARY.md` records TEST persisted, REAL refused, and says of itself "coverage across two runs, not a claimed single final 4/4 suite run". Sweettest conductor tests are "a separate manual gate and are not CI evidence". The signed v1.1.3 WP4 B0–B8 run's evidence directory is in a private repo. The `bridge/server.mjs` health check hard-codes a May DNA hash that no longer matches either later bundle. ✅

**The people and the AI.** One public member. Commits from "Claude (canon lane)" with `Co-Authored-By: Claude Opus 5.5`; fifteen hApp PRs and six canon PRs "Generated with Claude Code"; Codex PRs "while Claude is absent until October 10"; internal AI reviewers "Astra" and "Sol" named in PR bodies and defined nowhere public ⚠. Their own disclosure is exact: "the agent prepares and verifies but never signs"; "AI holds no authority"; "Independent assurance requires a competent, identifiable, conflict-disclosed reviewer." No CLAUDE.md, AGENTS.md or agent config in any repo. ✅ No funding, grant or investor statement anywhere; the sites say "pre-pilot, non-offering, evaluation-only" and accept only optional ETH donations. ✅ Licence: runtime code proprietary and UNLICENSED under Italian law, with a stated *intent* to open methodology code under CAL-1.0 and prose under CC-BY-SA-4.0 (NOTICE.md still says CAL-1.0). ✅

---

## Grounded Elohim reality (what the verdicts adjudicate against)

Seam map by an Explore agent, 2026-10-08, paths under the monorepo root. Status: **shipped** / **partial** / **design-only**.

- **Attestation is one consolidated shape** — a `Content` entry with `content_type = "attestation:<subtype>"`, 24 kinds, subject linked by `AttestationToSubject`, metadata carrying `proof_evidence.class ∈ {witness, audit, proof, confirmation}` (`elohim/holochain/dna/elohim/zomes/content_store_integrity/src/attestation_validator.rs`). Floors F1, F5, F7, F8 enforced; **F2 issuer authorization, F4 uniqueness and F6 eligibility are accept-all `TODO(C.3)`**. The proof-evidence schema, the validator's floor 8 and the coordinator's `proof_class` input disagree with each other, and callers pass `"social-witness"`, `"operator-report"`, `"intimate_recovery"` outside the enum. Adding a kind moves the DNA hash. **shipped / partial**.
- **No entry anywhere distinguishes fixture evidence from real evidence.** The distinction lives in `NetworkStage {Simulacra, Bootstrap, Coordinated, Enforced}` (network-wide, `crates/seam-contracts/src/freshness.rs`), the jenkins bridge's "bootstrap (fixture co-steward)" stakes string, and the a2o act and lane tags. `arch-dataplane-borrows-backlog.md:213`: "No action carries a writer marker". **absent**.
- **Claim tiers live off-chain.** `EpistemicStatus {Emergent, Reviewed, Contested, Canon, Superseded}` is a derived fold in `elohim/epr-rea` with `Canon` requiring a mishpat `Precedent`; it has no consumer. The seven verbs CLAIM→…→RENEW exist in `elohim/sdk/README.md` only; `ClaimInstance`, `TransformationRecord`, `RenewalEvent` are named gaps. **design-only / partial**.
- **The observation plane is real.** `Observation {observer_cid, log_cid, log_offset, seq, observation_kind, subject_cid, payload_json, diversity tags, signature}` as an append-only per-observer log gossiped on the `observation-log` ALPN over libp2p and iroh, projected to SQLite with a diversity view, graduating by diversity threshold into an attestation or a summary `EconomicEvent` (`elohim/elohim-storage/src/observation/`, `graduation/`). Kinds are manifest-declared (`elohim/sdk/domains/*/manifest.json` `observation_kinds`). HTTP-ingested rows are **unsigned** ("the ack says so"). **No physical-world ingress exists**: no MQTT, no sensors. **shipped / signing partial / sensors absent**.
- **REA on the DHT is broad and shipped**: `EconomicEvent` with 25 `REA_ACTIONS`, `Process`, `Intent`, `Commitment`, `Agreement`, `Claim`, `Settlement`, projected by `rea_projection.rs` ("DHT is the truth. Storage is the index"). The VF-GraphQL bridge is an M1 fixture; hREA is not wired. `epr-rea` has `Scopes` as a per-axis containment forest whose own doc example is "a farm sits in a watershed (ecological), a county (political), and a co-op (economic)", and `ExternalClaim {source: service:<name>, provenance, SignatureStatus, in_scope_of}` for outside evidence. **shipped / partial**.
- **Stewardship, custody and collectives are shipped** in imagodei (`StewardshipGrant` with `authority_basis`, `evidence_hash`, `verified_by`, mandatory `expires_at`/`review_at`; `Collective`, `Membership`, `CollabAgreement`), with six defects filed 2026-10-08 on `custodial-authority-answerable`. The commons pool is **design-only, held**.
- **Release provenance is strong and off-chain for DNA.** rakia's `release-manifest.schema.json` carries artifacts by blob CID and sha256, `roleBinding {dnaHash, coordinatorWasmHashes, migrateFrom, lineage, constitutionRoot}`, full `provenance`, and `adoptionDiscipline {soakSecs, attestationThreshold, canaryOrder}`; "A builder's own attestation never suffices to EARN its release (C1)". DNA hashes are guarded in `dna-hashes.baseline` by CI, stamped on the conductor StatefulSet, not notarized. brit's covenant trailers carry structure, "not proof that a claim is true or that its signer has authority". **shipped / partial**.
- **The habit register is our claims discipline**: `status` flips "require evidence (build #, live probe, test run) — never edit status from memory or intention"; the a2o ladder T0–T4 is ascend-only; a household green never flips a fleet habit ("a flip needs a build number, not a commit"). **shipped as tooling**.
- **Bridges**: "Consuming apps: served, never in control" (`bridges/CLAUDE.md`): gifts go out; observations come in only on the observation plane as external claims with `service:<app>` provenance, signature status and lowest reach; every gift is a governed offer approved by a non-author Steward. The seam disambiguator: add a *manifest* → SDK seam; add a *crate* → bridge seam. **shipped as discipline; atproto/activitypub bridges design-only**.
- **Mishpat** has `Precedent`, `ChallengeOutcome` ("accountability without consequence is theater") and `Commitment` (`delegates-compute`, `replicates-*`); it has **no evidence admissibility concept** and no appeal of evidence. **shipped / absent**.
- **Toolchain**: we run the pinned Holochain 0.7 fork line (`elohim/holochain-conductor`, `elohim-0.7`), with the 0.6→0.7 break list and review checklist in the `holochain-hdk-0-7` skill.

---

## The mapping (concern by concern)

| Concern | Prometheus (theirs) | Elohim (ours) | Verdict |
|---|---|---|---|
| Fixture vs real evidence | `evidence_class TEST|REAL` on the entry; TEST namespaced subjects; REAL needs a calibration hash and is refused in integrity until a registry exists | Network-wide `NetworkStage`, bridge stakes strings, a2o lane tags; nothing on the entry | **LIFT NOW** (see 1) |
| Capture vs review | "capture evidence cannot self-declare reviewer" refused in integrity | `validation_method` computed; F2/F6 accept-all | **LIFT NOW** (2) |
| Claim tiers | Ordered enum on chain; `investor_facing` below `PilotObserved` refused | `EpistemicStatus` fold with no consumer; `proof_evidence.class` drifting across schema, validator and coordinator | **LIFT NOW** the drift fix (3); **HOLD** an on-chain tier (H1) |
| Lowest unresolved gate controls the claim | Integrity rule + a report-only claims-discipline linter on prose (FAIL "guaranteed return", "passive income"…) | Habit register rule 4; a2o admissibility rule; no linter on public prose | **LIFT NOW** (4) |
| No-double-counting | Domain tuple (subject, indicator, time) + durable anchor (adds sensor); single-agent honesty note | `(observer_cid, log_cid, offset)` is the universal reference; writer identity is the observer | **LEAVE** (ours is stronger); lift the domain/transport key naming (5) |
| Sensor ingress | MQTT gateway, HMAC envelope, per-indicator ranges, allowlist, one call per reading | Observation plane with manifest kinds, diversity, graduation; no sensor path; HTTP rows unsigned | **HOLD** (H2): the partnership slice |
| Evidence maturity ladders | PRU R0–R7, RAP M4–M7, claim tiers | Reach 8 levels; seven verbs in docs; `MasteryLevel` in lamad only | **HOLD** (H1) |
| Canon vs runtime | DOCX canon, strict MD derivation, "downstream may not claim more than upstream" | docs-lead-code-follows; content-addressed cites with fingerprints; `.epr-meta` rules | **LEAVE** their mechanism; **offer** ours |
| Release signing | One GPG key, custody disclosed, `RATIFICATION.md` naming what is unmet | Release manifest with blob CIDs, lineage, `attestationThreshold`, C1 anti-self-election, soak | **LEAVE** (ours generalises); lift the ratification honesty line (6) |
| ValueFlows | Hand-rolled structs, never persisted | 25 REA actions on the DHT, `Scopes`, `ExternalClaim` | **LEAVE** theirs; **offer** ours |
| Risk layer (RAVEL) | Shadow-mode invariants in the DNA; brake signals that may never enforce | Algedonic signals (latch, no reset); `ConstitutionalLimit` | **WATCH** (W1) |
| AI custody | "prepares and verifies but never signs"; AI reviewers named, undefined | `WitnessId {Claude, Codex, Gemini, Human}`; Co-Authored-By; operator signs; evidence rule for flips | **CONVERGENT**; lift the disclosure phrasing (7) |
| Validation surface | Only `StoreEntry`; links, deletes, joins `Valid`; no membrane | Floors F1–F8 with three accept-all; link validation present | **WATCH** (W2) and a thing to give |
| Toolchain | hdk 0.6.1; open 0.6→0.7 gate PR | 0.7 fork line; break list in a skill | **GIVE** (G1) |
| Licence | Proprietary runtime, CAL-1.0 intent, CC-BY-SA prose | MIT OR Apache-2.0 for building blocks per the p2panda program; copyfarleft stated where intended | **HOLD** (H3) before any crate crosses |

---

## What we lift now

None of these mints a DHT entry type. Each is small, has a home, and cites this survey.

1. **An `evidence_class` on evidence, not on the network.** Add `evidence_class: fixture | real` to the observation payload contract (`observation-kind.schema.json`) and to `attestation-metadata.schema.json`, required, with the rule that a `fixture` observation can graduate only into a `fixture` attestation and never into a summary `EconomicEvent` that settles. Today the only marker is network-wide `NetworkStage`, which cannot say that one row on the alpha fleet came from a seeder. Their namespacing rule (TEST subjects carry a `TEST-` prefix) is the cheap version; ours should be the field, because our subjects are CIDs. **Home:** `elohim/sdk/schemas/v1/manifest/observation-kind.schema.json`, `elohim/sdk/schemas/v1/attestation/attestation-metadata.schema.json`, `elohim-storage` graduation evaluator. **Hash cost:** none until the integrity floor is added at the next `[dna:migrate]`; the schema and the storage-side refusal land first. **Cluster:** `measure-family-borrows-backlog`.
2. **Capture never self-reviews.** An integrity floor that refuses an attestation whose `validation_method` or reviewer field names its own author, folded into the F2/F6 work that is already `TODO(C.3)`. Their one-line rule is the right shape and the right place (integrity, not coordinator). **Home:** `attestation_validator.rs` floors; **Cluster:** `arch-authority-in-integrity-backlog` (their `REAL_DATA_PERSISTENCE_GATE_CLOSED` is the exemplar of that cluster's thesis: the rule belongs to the DNA, not a replaceable coordinator).
3. **Fix our own proof-class drift.** One enum, declared once: `proof_evidence.class ∈ {witness, audit, proof, confirmation}` in the schema, the validator and the coordinator input, with the out-of-band callers (`social-witness`, `operator-report`, `intimate_recovery`) mapped or refused. The comparison exposed it; it is ours to close. **Home:** `proof-evidence.schema.json`, `floor8_proof_class`, the three callers. **Cluster:** `arch-authority-in-integrity-backlog`.
4. **A claims-discipline linter on public prose.** Their `ci/claims_discipline_check.sh` (FAIL terms "risk-free", "guaranteed return", "passive income", "investable asset class"; WARN "revolutionary", "proven", "validated", "best-in-class"; negation-marker exemption; report-only) is a two-hour port onto `genesis/docs/content/elohim-protocol/` and the manifesto tree, as an `.epr-meta` inject rule rather than a CI stage. The register already refuses a status flip without evidence; this refuses the sentence. **Home:** `genesis/docs/content/.epr-meta`. **Cluster:** `design-legibility-borrows-backlog`.
5. **Name the two identity keys.** Their split between a *domain* identity (subject, indicator, time: "two sensors may not both count the same observation") and a *transport* idempotency key (adds the sensor) is vocabulary we lack. Our `(observer_cid, log_cid, offset)` is the transport key; a declared per-kind domain key is what lets graduation de-duplicate across observers. **Home:** `observation-kind.schema.json` gains an optional `domain_identity: [field…]`. **Cluster:** `measure-family-borrows-backlog`.
6. **Say what a release did not satisfy.** Their `RATIFICATION.md` ("Ratified by the sole active steward at this time… multi-stakeholder ratification is not yet satisfied; this runtime pointer is therefore substantially single-steward and revisable") belongs as a required `custody` note in the rakia release manifest: signers present, threshold declared, threshold met or not. `adoptionDiscipline.attestationThreshold` already exists; the manifest should state the shortfall rather than imply it. **Home:** `elohim/rakia/schemas/v1/release-manifest.schema.json` (a rakia PR; attested pin). **Cluster:** `arch-workspace-discipline-backlog`.
7. **The disclosure sentence.** "The agent prepares and verifies but never signs" is our practice; it is not written anywhere a reader would find it. One line in the habit covenant (`.epr-meta/habits-covenant.md`) and in `genesis/research/README.md`'s method conventions. **Cluster:** none (prose).

## What we hold (gated)

- **H1. An on-chain claim tier.** Their ordered `ClaimStatus` and PRU R0–R7 are the clearest statement we have seen of "the lowest unresolved gate controls the permitted claim". We have the better substrate for it (`EpistemicStatus` as a *derived* fold, canon requiring a mishpat `Precedent`) and no consumer. Hold until a second consumer of `EpistemicStatus` exists; then the tier is a projection, never a stored column, which is exactly the lesson of the reach-enum drift.
- **H2. Sensor ingress as a bridge crate.** The partnership slice. Their gateway is one Node process on a Pi; ours would be `bridges/mqtt-sensor` (translate an external protocol → bridge seam) producing signed `Observation` rows of a manifest-declared `mrv:<indicator>` kind (declare a vocabulary → SDK seam, `elohim/sdk/domains/mrv/manifest.json`), with the 27-indicator allowlist and ranges as manifest data, `evidence_class: fixture` forced until a reviewer-signed registry says otherwise, and diversity tags from the device archetype. Gated on: a real device stream (even TEST) from a partner, the observation plane's HTTP signing gap closed, and the local-first check (a reading must land on the household peer with no doorway present).
- **H3. Licence crossing.** No crate of theirs may enter our tree while the runtime is UNLICENSED; the CAL-1.0 intent is a copyleft decision our building-block policy has not made. Hold until their methodology tier is actually released under a licence and ours states its copyfarleft boundary.

## What we leave behind, and why

- **The DOCX canon.** A 4 MB binary as the controlling source, with a 59k-line strict Markdown derivation kept as provenance, is the shape our cite graph exists to replace: content-addressed documents with fingerprinted cites that survive moves. Their own rc.2 record ratified "Markdown is the source" (CCD-001) and proved byte-identical pandoc builds; they are mid-migration to where we are. Nothing to borrow; something to offer.
- **The single-cell runtime.** One DNA, one coordinator, one gateway, one key, one steward, `validate` that accepts every link and delete. Our attestation consolidation, observation plane, doorway on-ramp, release soak and household-to-fleet ladder generalise all of it. Borrowing their runtime would be regression.
- **Hand-rolled ValueFlows.** Compiled, tested, never persisted. Our DHT REA with `Scopes` and `ExternalClaim` is where their "a farm sits in a watershed, a county, a co-op" sentence already lives.
- **hdk 0.6.1.** We left it; their open compatibility gate is the first gift.
- **The benchmark intelligence layer** (PRE/PJE/NBE, GO/NO_GO/MANUAL_REVIEW receipts, a 0–100 score). Superseded in their own tree by the evidence spine; "evaluation, not certification" survived as the only invariant worth keeping, and it is already in the RAVEL entry.

## What we watch (their failure modes, live in our tree)

- **W1. A brake that may never act.** `autonomous_enforcement` must be false, forever, by integrity rule. It is honest about a layer that is not yet trusted, and it is also our algedonic module's state: a latch with a SET condition and no reset, signals with no wired emission. Both projects have declared a nervous system and forbidden it to move. Watch whether theirs gains a reset and a reader before ours does.
- **W2. Honesty concentrated in one entry type.** Their sharpest rules all sit on `MrvEvidenceEntry`; links, deletes and agent joins are `Valid`. Our equivalent is three accept-all floors behind a `TODO(C.3)`. The failure shape is the same: the thing the project is proudest of is validated; the surface around it is not.
- **W3. Proofs that prove the sentinel.** Six dated "runtime proofs" that a hello function returns a string, plus a health endpoint pinned to a stale DNA hash. We have had our own version (readiness probes that lie, `household-mesh-harness-honest-readiness`). The cure on both sides is the same rule: a proof names what it did not prove.
- **W4. AI reviewers without names.** "Astra" and "Sol" approve PRs and are defined nowhere public. Our `WitnessId` enum and Co-Authored-By trailers are the floor; the register's "a flip needs a build number" is the ceiling. Keep the distance between those two visible.
- **W5. A canon that drifts between releases.** TRBK changed meaning across v1.1.2, A-002 and rc.2 while the org profile kept the oldest wording. Our reach enum drifted across five subsystems the same way. Their runtime also answers to a second, non-public `v7.0.x` document track "mapped, not merged" onto the canon, which is two sources of truth by another name. The cure they are converging on (claims keyed by an immutable `claim_uid`, labels as display aliases) is the cure we applied (content-addressed cites).

## The one hard rule

**Evidence class is a property of the row, never of the network, and it can only descend.** A fixture observation may graduate into a fixture attestation and nothing further; a real observation may be refused, reviewed, or admitted, and nothing a coordinator does can promote fixture to real. Their DNA states this for one entry type with a closed gate. Ours must state it for every observation kind and every attestation subtype, in integrity, before any sensor stream from any partner touches the plane.

## Verdict

Prometheus is what a verification-first project looks like when one careful person and several AI agents build the honesty rails first and the substrate second. The rails are excellent and small: a two-valued evidence class the DNA refuses to blur, a capture that cannot review itself, a claim ladder where the lowest open gate controls the sentence you may say, a linter that refuses "guaranteed return", a ratification note that names what is unmet. Every one of those is a cheap lift into a tree that already has the plane, the attestation shape, the reach ladder, the release manifest and the evidence ladder they are still building toward. Where they are ahead of us it is in *saying* things: the evidence class on the row, the reviewer separation, the custody shortfall. Where we are ahead it is in everything that moves: observation gossip with diversity, graduation, multi-DNA composition, a hosted on-ramp, release soak with a threshold, and a household-to-fleet ladder that prices a flip in build numbers. Lift the sentences; keep the plane.

---

## Conclusion: the next thing we do is write to them

The report would be incomplete as a reading list. The shape of the work on both sides says what a collaboration is, and it is concrete enough to propose in one message.

**What they hold that we lack.** A domain where evidence has physical stakes (soil, water, biomass) and a legal perimeter that forces honesty about claims; a 27-indicator MRV vocabulary tied to a published baseline protocol; a field pilot in pre-registration with a reviewer role the design already demands; a claims register with immutable ids; and a constitutional text ("Human dignity is greater than any score, signal, token record, wallet history, governance participation, reputation trace or AI classification") that reads as a sibling of stewardship-over-sovereignty.

**What we hold that they lack.** The plane their README asks for ("Holochain remains a provenance and batch-reference layer, not a raw telemetry database") already built as an observation log with observer diversity and graduation; attestation as one consolidated shape with subject links and revocation; a hosted on-ramp that does not require a Pi on every farm; release manifests with thresholds and soak; the 0.7 line and its break list; a2o scenarios as the specification; and a cite graph that would end their DOCX custody problem.

**Who takes what.**

| Lane | Prometheus takes | Elohim takes |
|---|---|---|
| Vocabulary | The `mrv` domain manifest: indicators, ranges, units, the domain identity key per kind, the evidence-class rule | Hosting it under `elohim/sdk/domains/mrv/`, codegen, the observation-kind schema it validates against |
| Ingress | The field side: devices, LoRaWAN, the gateway on the Pi, the HMAC envelope, the allowlist policy | `bridges/mqtt-sensor` as the translation crate; signed observation rows; the local-first proof that a reading lands with no doorway present |
| Review | The reviewer role and the batch-attestation ceremony their go-live checklist already names | F2/F6 floors in integrity; `attestation:mrv-review` as a subtype; revocation and expiry already shipped |
| Claims | The claim ladder semantics, the claims-discipline word list, the "lowest gate" rule as prose and as test | `EpistemicStatus` as the derived tier, mishpat `Precedent` as canon, the prose linter as an `.epr-meta` rule |
| Release | Their ratification honesty | The release manifest, soak and threshold machinery; a `custody` field they can read their shortfall from |
| Toolchain | Their 0.6.1→0.7 compatibility matrix PR | The fork line, the break list, a sweettest shape that runs in CI rather than "a separate manual gate" |
| Canon | The white paper and the ontology stay theirs, licensed as they choose | The cite tooling (`epr flow cites`) offered as a method, not a dependency |

**What we would ask for first.** One TEST-class sensor stream from a real device, even a bench rig, and one named human in the reviewer role, so the F2/F6 floors are exercised by someone who is not us. In return we run their 27 indicators through the household mesh as a manifest-declared kind and hand back a sprint report in the a2o shape, which is the evidence currency both projects already speak.

**What we would not ask for.** Their runtime code (proprietary, and we do not need it), the private pilot repos, or any change to their canon. The bridges rule holds in both directions: served, never in control.

**Why now.** Their 0.7 gate is open this week; their Sicily pre-registration is not frozen; their evidence class is the one rule in their DNA that every partner would have to honour, and ours does not exist yet. The two trees are at the moment where a vocabulary can still be shared rather than translated.

**The message** is short and public-safe (their boundary docs welcome "inspection, verification, and defensive publication"): who we are, this report's link, the three gifts (the 0.7 break list for PR #17, the observation-plane prototype of their indicator list on our household mesh, the cite method for their canon), the one ask (a TEST stream and a reviewer), and a note that our evidence rule matches theirs: nothing flips without a build number. Contact per their sites: Telegram `t.me/uwohali77`, the org email, or an issue on `prometheus-happ`.

---

## The view from the other side

*This section is written by a separate, context-isolated agent asked to read the survey above and the public Prometheus corpus as a member of the Permaculture DAO project would, and to answer: what would a partnership look like from where you stand, what would you take, what would you refuse, and what would you want from Elohim first. It is their argument, not ours; disagreements are left standing.*

### Where the survey reads us right, and where it does not

It reads the rules right. TEST and REAL are never interchangeable; the REAL gate is closed in `zome_integrity`, not in a coordinator; capture cannot name its own reviewer; the lowest unresolved gate controls the permitted claim; the agent prepares and verifies but never signs. The name guard is correct. W3 is fair: six proofs that a sentinel string comes back are not runtime evidence, and `RUNTIME_BOUNDARY.md` says so itself.

It reads the shape wrong. "Their runtime is a single-cell pilot" and "borrowing their runtime would be regression" treat narrowness as a gap. Narrowness is the gate. `clone_limit: 0`, no update or delete externs, six entry types, REAL refused: that is what a runtime looks like when the canon says "No PRU value is ever computed in the runtime" and X.15 puts hApp integrity rules below methods and schemas. Width is downstream of canon for us. "Hand-rolled ValueFlows, never persisted" is true as a fact and wrong as a verdict. An `EconomicEvent` is a value-bearing artefact; persisting one before admissibility exists is exactly the overclaim our stack rule forbids. We are not allowed to yet.

The "lift now" list calls our rules "small" and "sentences". Their own seam map says no Elohim entry distinguishes fixture from real, F2 and F6 are accept-all, and claim tiers are "design-only". Three rules the other side lacks at the integrity layer are not small. "Where they are ahead of us it is in saying things" undersells it: saying it in the DNA is the whole thesis.

What is fair against us: links, deletes and agent joins all return `Valid`; there is no membrane. PR #30 and #33 are the open work.

### What I would take, tested against our gates

The 0.7 break list and review checklist: take now. Our PR #17 holds 0.6.1 "pending a complete compatibility matrix"; a break list is an input to that matrix, not the matrix, and crosses no licence line.

The observation plane: shipped, and close to what our README asks for. But `stream.rs` says "observations are unsigned until the signing graduation", and our gateway requires an HMAC envelope. A TEST stream may ride it today; a REAL row never may, because an unsigned row has no provenance by our §I.

The attestation shape: shipped, but F2 issuer authorization is `TODO(C.3)`. A reviewer attestation whose issuer is not checked is not a "separate attestation workflow" by our standard. Not admissible until F2 lands.

The cite graph: take as method, since CCD-001 ratified Markdown as source. Caution: our self-hash trap rule applies to fingerprints too.

The release manifest `custody` field: take the vocabulary. It restates our `RATIFICATION.md` shortfall; it does not grow us out of single-key custody.

Not admissible yet, by the seam map's own labels: `EpistemicStatus` (no consumer), the seven verbs (docs only), VF-GraphQL (M1 fixture), hREA (not wired), the commons pool (held), and `bridges/mqtt-sensor`, which the table assigns to Elohim and which does not exist.

### What I would refuse or defer

Graduation of our rows into a "summary `EconomicEvent`". Path 2 of their graduation module turns observation aggregates into economic events. Under X.15 that is a downstream artefact claiming more than its upstream. Their "one hard rule" covers fixture; it still lets a real row be "admitted", and admission is ours.

Any move of REAL admission out of `zome_integrity`. No Elohim attestation of any subtype opens `REAL_DATA_PERSISTENCE_GATE_CLOSED`.

The Sicily pre-registration. It is not frozen; G-SIC-01 is suspended; nothing a partner does is an input to it, and no TEST run on their mesh is pilot evidence. The 27-name list in `evidence.mjs` is public; `MRV_BASELINE_PROTOCOL_SICILY.md` is not, and stays so until freeze.

Licence crossings. Our runtime is UNLICENSED and our open intent is CAL-1.0; their policy is MIT OR Apache-2.0. H3 is right. Prose and indicator names cross under CC-BY-SA-4.0; code does not.

Dependence on a hosted doorway. §O: "Prometheus thinks locally, remembers locally, recovers locally". Their local-first skill agrees ("A doorway hosts; it does not own"); I would hold them to it: a reading must land on the Pi with `doorway.elohim.host` down.

Identity framing. Imago dei with community backstop versus our Syntropic Sovereignty Header: both say dignity is greater than any score. The difference bites only if a reviewer's authority were derived from an Elohim community grant instead of our appointment. For a TEST stream it blocks nothing.

### Who takes what, amended

Vocabulary: accept, with a correction. We hand over the public 27 names, ranges and units; the evidence-class rule stays in our DNA. Ingress: the Pi stays our gateway and writes to our conductor; their crate becomes a second subscriber, TEST only, once it exists. Review: reviewer appointment is ours; F2/F6 is their work; `attestation:mrv-review` moves their DNA hash, their cost to price. Claims: port the word list, not the script; no claim of ours maps onto their `EpistemicStatus`. Release, toolchain, canon rows: accept as written.

Rows they left out. Reviewer appointment: ours, one named human, before anything else. Pre-registration freeze: ours alone, no date promised. Calibration registry: ours; Elohim never populates it. Legal perimeter: Permaculture DAO LLC under Italian law; an a2o sprint report is "internal" under X.16, never independent assurance. Data residency: farm data stays on the Pi and in our DNA; TEST rows may gossip on their fleet once we know where it is; REAL rows never leave until residency is written. Gateway operator: us. Signing: Uwohali signs our releases, their operator signs their reports, neither signs the other's, AI signs nothing.

### First ask, first offer

Ask first: the break list as a document, and one line stating where the household mesh runs and what leaves it. Offer first: the indicator list as JSON under CC-BY-SA, the HMAC envelope spec, and one bench-rig TEST stream. Sized to one person at G2 RED with `production_admitted: false`: two weeks, TEST only, nothing on the Sicily path.

### Risk, and the one condition

The survey names the shape itself: "a consuming app on our bridges seam". Hosting the `mrv` manifest under `elohim/sdk/domains/` makes their copy the referent and ours the derivation. Graduation turns our evidence into their economy. Their register today reads 19 red of 35 with `dataplane-convergence` red; a pre-registration pilot cannot carry a red fleet's name. The condition for yes: a written rule in both trees, with a runnable check, that no Elohim artefact may carry a Prometheus claim stronger than our claims register, and that admissibility is emitted only by our integrity zome. That is X.15 extended across the seam. Write it as a habit and we talk.

## Reconciliation (what we accept from the other side)

Three of their corrections stand and change the survey's wording, not its verdicts.

- **"Single-cell pilot" and "hand-rolled ValueFlows, never persisted" were facts read as gaps.** Their narrowness is the gate: six entry types, no update or delete externs, REAL refused, and no persisted `EconomicEvent` because persisting a value-bearing artefact before admissibility exists is exactly the overclaim their stack rule forbids. We keep "leave behind" for the runtime as a substrate choice, and withdraw it as a judgment of discipline.
- **"Small" and "sentences" undersold the lifts.** The three rules we lift are the three things our seam map says we lack at the integrity layer. Saying it in the DNA is the thesis, and it is the thesis of our own `arch-authority-in-integrity-backlog`. The verdict's "where they are ahead of us it is in saying things" should be read as: ahead of us in the integrity zome, which is the only place that counts.
- **`bridges/mqtt-sensor` does not exist** and the table assigned it to us as if it did. It is H2, gated, and the Pi stays their gateway writing to their conductor; our crate, when it exists, is a second subscriber, TEST only.

Two of their refusals are the real seam, and they are right about both:

- **Graduation into a summary `EconomicEvent` (Path 2) is where our plane would claim more than their canon allows.** Any `mrv:*` observation kind must declare graduation to attestation only, never to a settling event, and that declaration belongs in the manifest where the kind is declared, not in a coordinator.
- **Admissibility is emitted only by their integrity zome.** No Elohim attestation subtype opens `REAL_DATA_PERSISTENCE_GATE_CLOSED`, and our "one hard rule" (evidence class only descends) is necessary but not sufficient: it governs fixture-ness, not admission.

Their condition for yes is a habit, and it is the right first joint artefact: **a runnable rule in both trees that no Elohim artefact carries a Prometheus claim stronger than their claims register, and that admissibility is emitted only by their integrity zome.** On our side that is a habit atom born `unwired` under `bridges/.epr-meta/` beside `consuming-apps-serve-never-control`, with `retire-when` the day their claims register is content-addressed and the check reads it directly. That atom, not the message, is the next thing we do; the message cites it.

Their reputational point stands too: our register reads 19 red of 35 with `dataplane-convergence` active and red. A pre-registration pilot should not carry a red fleet's name, which is why the first exchange is TEST only, on the household mesh, with nothing on the Sicily path, and why the reach of anything they send us is the lowest level until they say otherwise.

---

## Outputs

- **This survey** — `genesis/research/permaculture-dao-prometheus-cross-pollination-2026-10-08.md`.
- **Canon** — `genesis/docs/content/elohim-protocol/observability/epic.md` (the Observability epic this survey's lift 1 and name guard fed).
- **`research-manifest.json`** — `prometheus-happ`, `prometheus-canon`, `prometheus-runtime` registered with relevance notes (not cloned; disk).
- **README enrichment** — new Research Index section "The Evidence Problem — Prometheus (Permaculture DAO)".
- **Mint pass** — lifts 1 and 5 → [measure-family-borrows-backlog](epr:measure-family-borrows-backlog); lifts 2 and 3 → [arch-authority-in-integrity-backlog](epr:arch-authority-in-integrity-backlog); lift 4 → [design-legibility-borrows-backlog](epr:design-legibility-borrows-backlog); lift 6 → [arch-workspace-discipline-backlog](epr:arch-workspace-discipline-backlog). H2 waits on the outreach answer and is not minted.
- **Still open (the operator's):** the outreach message itself; the joint habit the other side names as its condition (born `unwired` under `bridges/.epr-meta/`, cited by the message); whether to submit elohim to hAppRadar (a one-field form taking a GitHub URL); the licence stance H3.

## Method note

Two grounded-research agents (Sonnet) read the ten public repos and the three founder repos via `gh api` raw reads (two passes: hApp and canon; runtime, release registry, org profile and legacy archives) and tree listings, dumping issues, PRs and commit lists to a scratchpad; one Explore agent produced the Elohim seam map with file:line evidence; the orchestrating session (Fable) wrote the mapping and verdicts; a fourth agent wrote the other-side section after reading the survey, the dossier and the seam map, and spot-checking Elohim's claims about itself in the tree. Corrections the passes caught are kept inline: hAppRadar's "Hearth" is a different project; `prometheus-bridge` and five siblings are private, not missing; the valueflows module is never persisted; the REAL gate is closed in integrity, not merely in the gateway; the Sicily baseline protocol the gateway cites is not public; "Astra" and "Sol" are undefined. The deprecation sentinel fired nine times on quoted external text; all were closed or pruned as false positives from their files, not ours.

## Credit

Uwohali Tuccio and Permaculture DAO LLC built Prometheus in the open with unusual honesty about what each artefact does and does not prove, disclosed their AI authorship and single-steward custody in writing, and published a licence that names its own limits. The canon's verification progression and the hApp's closed REAL gate are the two ideas this survey is built around.
