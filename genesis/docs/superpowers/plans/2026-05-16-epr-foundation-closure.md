# EPR Foundation Closure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the EPR foundation sprint by (a) producing an evidence-backed per-scenario disposition for the 12 remaining `@wip` scenarios in the two EPR a2o feature files, (b) lifting the scenarios whose substrate is ready and documenting backlog reasons for the rest, (c) recording the D4 `GetDocument` decision (defer to graph-native or escalate now) with citations to specific scenarios, (d) landing the conditional `AgentPeerBinding` IntegrityNotify arm IFF iroh Phase 12 caller-identity is live, and (e) writing the sprint-result memory entry that hands clean state to the graph-native sprint.

**Architecture:** Five sequential tasks. **Task 1** is an Opus-authored per-scenario walk over the 12 `@wip` scenarios — for each scenario, decide lift / defer-with-evidence / restructure, and produce a one-paragraph rationale. The output is a markdown disposition table that drives Tasks 2 + 3 and also answers D4 (GetDocument escalation) by citing which scenarios — if any — genuinely require it. **Tasks 2 + 3** execute the dispositions: lift eligible scenarios in `epr-content-addressing.feature` and `epr-cross-peer-resolution.feature`, leaving documented backlog comments on deferrals. **Task 4** checks iroh Phase 12 status and, if green, lands the small `AgentPeerBinding` arm in `epr_atom_service.rs` (mirrors the existing `RevocationAttestation` arm). **Task 5** writes the sprint-result memory entry, ticks the foundation-completion plan's remaining checkboxes, and confirms the closing-condition list from the EPR delivery master.

**Tech Stack:** Cucumber/Gherkin (a2o), Playwright (`@browser-only` step defs in `genesis/a2o/steps/ui/epr-content.steps.ts`), Rust (elohim-storage IntegrityNotify pipeline at `epr_atom_service.rs`), libp2p 0.54 (cross-peer EPR-atom protocol), MessagePack via rmp_serde, Diesel + SQLite (no schema changes in this plan).

**Spec / parent master:** `genesis/docs/superpowers/plans/2026-05-11-epr-delivery-master.md` (master, §D4 + §closing-condition) + `genesis/docs/superpowers/plans/2026-05-15-epr-foundation-completion.md` (foundation-completion, Tasks 8–11 remain) + `genesis/docs/plans/2026-05-15-epr-wave0-audit-results.md` (audit, §D7 `epr_2b_batch_a_full_loop` gate).

**Wave 0 (audit) is complete. Foundation Bands A + B (substantive) are complete.** This plan is the closure sweep: per-scenario walk, conditional Phase 12 follow-on, sprint-result memory. No new code beyond the optional AgentPeerBinding arm.

---

## P2P Design Gate Output

This plan does not introduce new data entities — every change is either an a2o tag edit, a documentation backlog comment, an existing-pattern Rust match-arm addition, or a memory file. No new DHT entry types, no new SQLite tables, no new HTTP routes, no new wire contracts. The single conditional Rust change (Task 4) consumes the already-defined `AgentPeerBindingMessage` wire shape from iroh Phase 12 (Category C, operational projection of the Category A `peer_identity_bindings` DHT entries on the infrastructure DNA).

**Anti-pattern check:** ✓ Zero new entities. ✓ No CID-as-FK. ✓ No silent fold-in of out-of-scope graph-native items (the plan's whole point is to enumerate which @wip scenarios escalate to graph-native).

---

## File Structure

### New files
| Path | Responsibility |
|------|----------------|
| `genesis/docs/plans/2026-05-16-epr-wip-disposition.md` | Task 1 deliverable. Per-scenario disposition table (12 rows: scenario name, current @wip reason, decision = lift / defer-with-evidence / restructure, rationale citing step-def state + substrate readiness). Includes a §"D4 — GetDocument escalation answer" subsection that cites which scenarios require `GetDocument` and recommends accept (escalate) or defer (stay in graph-native). |
| `.claude/memory/project_epr_foundation_closure_2026_05_16.md` | Sprint-result memory entry (Task 5). Surface: non-obvious discoveries during the @wip walk, D4 final disposition, plan-tracking debt cleared, the substrate state that graph-native inherits. |

### Modified files (a2o feature files)
| Path | What changes |
|------|--------------|
| `genesis/a2o/features/content/epr-content-addressing.feature` (4 `@wip` at lines 95, 112, 128, 144) | Per Task 1 disposition: strip `@wip` from scenarios where step defs are ready; rewrite the inline backlog comment block (currently lines 91–94, 108–111, 124–127, 140–143) on retained deferrals to cite the graph-native sprint as the destination and name the specific subsystem that gates the lift (e.g., "lifts when sophia-quiz-json renderer carries shefa-context propagation — graph-native sprint Task X"). |
| `genesis/a2o/features/federation/epr-cross-peer-resolution.feature` (8 `@wip` at lines 112, 128, 142, 158, 170, 184, 196, 209) | Same per-scenario lift-or-rewrite-backlog pattern. The existing line 99–109 multi-paragraph backlog block (the "5 missing pieces" inventory) gets updated to reflect post-Task-1 truth: which pieces are now landed, which moved to graph-native, which moved to a different downstream sprint. |

### Modified files (Rust storage — conditional on Task 4 Step 1)
| Path | What changes |
|------|--------------|
| `elohim/elohim-storage/src/epr_atom_service.rs` (after the `RevocationAttestation` arm closes around line 453, before the `other_kind` catch-all) | IFF Phase 12 is green: insert `"AgentPeerBinding"` match arm reading `AgentPeerBindingMessage` from request bytes (mirrors `RevocationAttestation` arm shape at `:393–448`). Wire dedupe via existing `recent_integrity_notifies` cache using key `format!("AgentPeerBinding:{}:{}", subject_cid, issuer)`. |
| `elohim/elohim-storage/src/p2p/` (new file `agent_peer_binding_message.rs`) | IFF Phase 12 is green: define `AgentPeerBindingMessage` wire struct with `from_bytes` / `to_bytes` per the existing pattern in `p2p/revocation_attestation_message.rs`. Schema source: `elohim/sdk/schemas/v1/dna-signals/agent-peer-binding.schema.json` (if missing, schema-first per `feedback_schema_first_ioc` — write the schema first, then the wire struct). |

### Modified files (plan-tracking debt)
| Path | What changes |
|------|--------------|
| `genesis/docs/superpowers/plans/2026-05-15-epr-foundation-completion.md` | Tick the remaining `[ ]` checkboxes on Tasks 8, 9, 10, 11. Append a "Closed: 2026-05-16" line to the Goal section. The plan is the destination for the closure record; this plan (this file) is its operational successor. |

### Test files
| Path | Responsibility |
|------|----------------|
| `genesis/a2o/steps/ui/epr-content.steps.ts` | IFF a scenario disposition from Task 1 is "lift after wiring helpers", add the helper functions called out in that disposition. Do NOT add new step definitions for scenarios disposed as "defer-with-evidence". |
| `elohim/elohim-storage/src/epr_atom_service.rs::tests` | IFF Task 4 lands the AgentPeerBinding arm: add `integrity_notify_agent_peer_binding_acks_received_true` + `_dedup_returns_duplicate_reason`, mirroring the existing RevocationAttestation tests around lines 599–660. |

### Out-of-scope (explicit carve-outs)
- **W2C `GetDocument` implementation.** Task 1 produces the *answer* to D4, not the implementation. If the answer is "escalate", a follow-on plan implements it in the graph-native sprint kickoff.
- **The graph-native sprint brainstorm itself.** A separate `superpowers:brainstorming` session, after this plan closes.
- **The 1-week cross-stack soak from foundation-completion Task 11.** Operator-driven; outside the agentic shape.
- **Any new `RevocationAttestation` / `KeyRotation` / `KeyRevocation` arm work** — all three already landed in prior bands.

---

## Sequencing (band order)

Tasks 1 → 2 → 3 are sequential because Task 1's disposition table is the input to Tasks 2 and 3.

Task 4 (Phase 12 / AgentPeerBinding) can run in parallel with Tasks 2 + 3 since it touches a different layer (Rust storage, not a2o features). Dispatch as a parallel agent IFF Task 4 Step 1 returns "Phase 12 green"; otherwise close the task in 10 minutes as "deferred-pending-Phase-12" and proceed to Task 5.

Task 5 closes when 1–4 are all in their terminal states (lifted/deferred-with-evidence/landed/Phase-12-pending).

**Closing-condition check (from EPR delivery master §closing-condition):** after Task 5, walk the 8-item checklist in the master and confirm each item maps to a green outcome or a documented deferral in this plan's sprint-result memory file. The master's checklist is the gate, not this plan's tasks.

---

## Task 1: Opus per-scenario walk + D4 GetDocument disposition

**Files:**
- Read: `genesis/a2o/features/content/epr-content-addressing.feature` (lines 91–154 — the 4 @wip scenarios + inline backlog notes)
- Read: `genesis/a2o/features/federation/epr-cross-peer-resolution.feature` (lines 64–222 — the 8 @wip scenarios + the line 99–109 backlog inventory)
- Read: `genesis/a2o/steps/ui/epr-content.steps.ts` (entire file — establish what step-defs already exist)
- Read: `elohim/elohim-storage/src/p2p/epr_protocol.rs:40–60` (EprRequest variants — confirm Resolve/ResolveBatch/Announce/GetDocument shapes)
- Read: `elohim/elohim-storage/src/epr_atom_service.rs:200–460` (handle_resolve / handle_get_document — confirm what each variant actually delivers today)
- Create: `genesis/docs/plans/2026-05-16-epr-wip-disposition.md`

This task MUST be performed by a Sonnet or Opus subagent (per `feedback_a2o_narrative_is_opus_work` — Haiku produces scenario-shaped objects without interpretability). Recommended: Opus. The output is load-bearing: it gates D4 and drives the next two tasks.

- [ ] **Step 1: Read the 12 scenarios + their inline backlog notes**

Use the Read tool on each scenario range:

```
epr-content-addressing.feature lines  91–106  (Scenario: EPR popover surfaces all three pillars when present)
epr-content-addressing.feature lines 108–122  (Scenario: Following an EPR link transfers reading context to the destination)
epr-content-addressing.feature lines 124–138  (Scenario: EPR link to a versioned-since-authored CID degrades gracefully)
epr-content-addressing.feature lines 140–154  (Scenario: EPR Head signature is verifiable end-to-end)
epr-cross-peer-resolution.feature lines 112–126 (Scenario: Community-reach guide accessible only to consented collective members)
epr-cross-peer-resolution.feature lines 128–140 (Scenario: Trusted-reach content requires standing relationship with steward)
epr-cross-peer-resolution.feature lines 142–156 (Scenario: Attestation-gated content requires prerequisite mastery)
epr-cross-peer-resolution.feature lines 158–168 (Scenario: Recognition distributes proportionally to stewards on P2P delivery)
epr-cross-peer-resolution.feature lines 170–180 (Scenario: Policy ceiling blocks content above the device's reach level max)
epr-cross-peer-resolution.feature lines 184–194 (Scenario: Steward sees recognition land for content delivered cross-peer)
epr-cross-peer-resolution.feature lines 196–207 (Scenario: Cross-peer fetch surfaces transient peer-offline as a soft state)
epr-cross-peer-resolution.feature lines 209–222 (Scenario: Identity binding allows cross-peer fetches to attribute reach correctly)
```

For each scenario, note: (a) what concrete user moment it asserts, (b) what step verbs/nouns it uses, (c) the inline backlog reason already documented in the comment block above the @wip tag.

- [ ] **Step 2: Read the step-def file and grep for each scenario's verbs**

```bash
grep -nE "shefa stewardship summary|qahal reach level|popover shows|origin context|back-affordance|superseded|historical|Ed25519|canonical envelope bytes" /projects/elohim/genesis/a2o/steps/ui/epr-content.steps.ts
grep -nE "consented member|collective membership|trusted relationship|prerequisite mastery|recognition events|reach_level_max|recognition feed|fetching from another steward|AgentPeerBinding|PeerIdentityMap" /projects/elohim/genesis/a2o/steps/ui/epr-content.steps.ts /projects/elohim/genesis/a2o/steps/**/*.steps.ts
```

For each scenario, classify the step-def state as one of:
- **READY** — all step verbs have matching definitions; only @wip strip required
- **WIRE-NEEDED** — verbs need 1–3 small helper additions in the existing step-def file (≤30 lines per scenario)
- **SUBSYSTEM-MISSING** — verbs require a substantive new subsystem (renderer affordance, fixture builder, cross-peer disconnect simulator, signature-verify library import)

- [ ] **Step 3: For each SUBSYSTEM-MISSING scenario, name the missing subsystem and its likely sprint owner**

The likely sprint owners are:
- **graph-native** — anything touching `experience-story` / `experience-moment` / supersedence UI / origin-context tracking / vouch sponsor UI / shefa-context propagation through renderer
- **doorway-full-facilitator** (project `_doorway_full_facilitator_sprint`) — cross-peer disconnect simulation + multi-steward failover at the HTTP edge
- **iroh-phase-12-followon** — `AgentPeerBinding`-mediated cross-peer fetch attribution (scenario 12)
- **a2o-tooling** — DAG-CBOR + Ed25519 verification harness in step-defs (could land standalone; no protocol gate)

Cite the sprint owner in the disposition row.

- [ ] **Step 4: Specifically answer D4 — does any scenario require `EprAtomRequest::GetDocument`?**

`GetDocument` is the variant for fetching document-tier content body (markdown, sophia-quiz-json, html5-app — anything that lives as a Content entry on the DNA, not as a blob in pantry storage) directly over the libp2p EPR-atom protocol, bypassing HTTP. Today the substrate uses HTTP `/api/v1/content/{cid}` via doorway for document-tier reads and HTTP `/blob/{cid}` for blob reads; both are HTTP-edge surfaces, not peer-to-peer.

For each scenario in Step 1, classify body-fetch path as one of:
- **HTTP-VIA-DOORWAY** — current substrate (`/api/v1/content/{cid}` or `/blob/{cid}`); no GetDocument required
- **EPR-ATOM-PROTOCOL** — scenario explicitly requires libp2p-direct document fetch (would need GetDocument or a new variant)
- **AMBIGUOUS** — scenario could be satisfied by either; preference deferred

Then state the D4 verdict in the disposition file:
- If **zero scenarios are EPR-ATOM-PROTOCOL**: D4 → **defer GetDocument to graph-native** as originally planned. The substrate doesn't need it for the foundation a2o lift. Graph-native sprint can pick it up when `experience-story` / cross-peer document fetch over libp2p becomes load-bearing.
- If **≥1 scenarios are EPR-ATOM-PROTOCOL**: D4 → **escalate** — author a small follow-on plan implementing GetDocument before graph-native dispatches, scoped to the cited scenarios.
- If **all are AMBIGUOUS**: D4 → **defer** with a note that graph-native must decide based on the renderer architecture chosen for `experience-story`.

The preliminary read (from this plan's pre-write walk) is that scenario 12 ("Identity binding allows cross-peer fetches to attribute reach correctly") is the most likely **EPR-ATOM-PROTOCOL** candidate because its step verb explicitly says "fetches via the EPR-atom protocol." However, the existing `Resolve` variant returns the EPR Head, after which an HTTP fetch of the body could carry the binding context via headers. Confirm or refute in Step 4.

- [ ] **Step 5: Write the disposition file**

Create `genesis/docs/plans/2026-05-16-epr-wip-disposition.md` with this skeleton:

```markdown
# EPR @wip Disposition — Foundation Closure Walk

**Date:** 2026-05-16
**Audited against:** dev @ <commit-sha-here>
**Walker:** <agent name + model>
**Source files:**
- genesis/a2o/features/content/epr-content-addressing.feature (4 @wip)
- genesis/a2o/features/federation/epr-cross-peer-resolution.feature (8 @wip)

## Per-scenario dispositions

| # | Scenario | Line | Step-def state | Disposition | Backlog destination | Rationale |
|---|---|---|---|---|---|---|
| 1 | EPR popover surfaces all three pillars when present | epr-content-addressing.feature:96 | <READY/WIRE-NEEDED/SUBSYSTEM-MISSING> | <lift/defer-with-evidence/restructure> | <sprint owner or n/a> | <one-sentence rationale citing concrete state> |
| 2 | Following an EPR link transfers reading context | epr-content-addressing.feature:113 | … | … | … | … |
| 3 | EPR link to a versioned-since-authored CID degrades gracefully | epr-content-addressing.feature:129 | … | … | … | … |
| 4 | EPR Head signature is verifiable end-to-end | epr-content-addressing.feature:145 | … | … | … | … |
| 5 | Community-reach guide accessible only to consented collective members | epr-cross-peer-resolution.feature:113 | … | … | … | … |
| 6 | Trusted-reach content requires standing relationship with steward | epr-cross-peer-resolution.feature:129 | … | … | … | … |
| 7 | Attestation-gated content requires prerequisite mastery | epr-cross-peer-resolution.feature:143 | … | … | … | … |
| 8 | Recognition distributes proportionally to stewards on P2P delivery | epr-cross-peer-resolution.feature:159 | … | … | … | … |
| 9 | Policy ceiling blocks content above the device's reach level max | epr-cross-peer-resolution.feature:171 | … | … | … | … |
| 10 | Steward sees recognition land for content delivered cross-peer | epr-cross-peer-resolution.feature:185 | … | … | … | … |
| 11 | Cross-peer fetch surfaces transient peer-offline as a soft state | epr-cross-peer-resolution.feature:197 | … | … | … | … |
| 12 | Identity binding allows cross-peer fetches to attribute reach correctly | epr-cross-peer-resolution.feature:210 | … | … | … | … |

## D4 — GetDocument escalation answer

**Verdict:** <DEFER-TO-GRAPH-NATIVE / ESCALATE-NOW / DEFER-WITH-RENDERER-DEPENDENCY>

**Evidence:**
- Scenarios classified HTTP-VIA-DOORWAY: <list scenario numbers>
- Scenarios classified EPR-ATOM-PROTOCOL: <list scenario numbers, or "none">
- Scenarios classified AMBIGUOUS: <list scenario numbers>

**Reasoning:** <2–3 paragraphs citing concrete step verbs + substrate paths. If verdict is ESCALATE-NOW, name the scenario(s) that require GetDocument and sketch the minimum-viable implementation shape. If DEFER-TO-GRAPH-NATIVE, name the renderer-architecture decision graph-native must make before reconsidering.>

## Backlog destinations summary

- **graph-native sprint:** <count> scenarios — <list>
- **doorway-full-facilitator sprint:** <count> scenarios — <list>
- **iroh-phase-12-followon:** <count> scenarios — <list>
- **a2o-tooling (standalone):** <count> scenarios — <list>
- **Lifted in this sprint (no backlog):** <count> scenarios — <list>

## Sanity-check counts

- Total @wip at sprint start: 12 (4 content + 8 federation)
- Lifted by this walk: <N>
- Retained @wip with new backlog citations: <12 − N>
- D4 decision: <one-line restatement of verdict>
```

- [ ] **Step 6: Commit**

```bash
git add genesis/docs/plans/2026-05-16-epr-wip-disposition.md
git commit -m "$(cat <<'EOF'
docs(epr): @wip disposition walk — D4 GetDocument answered

Per-scenario walk of the 12 remaining @wip scenarios in
epr-content-addressing.feature + epr-cross-peer-resolution.feature.
Disposition table cites step-def state + substrate readiness for
each scenario; backlog destinations route to graph-native /
doorway-full-facilitator / iroh-phase-12-followon / a2o-tooling.

D4 GetDocument verdict recorded with evidence from scenario walk:
classifies each body-fetch path as HTTP-VIA-DOORWAY,
EPR-ATOM-PROTOCOL, or AMBIGUOUS, and recommends defer/escalate
accordingly.

Drives Tasks 2 + 3 of the foundation-closure plan
(2026-05-16-epr-foundation-closure.md).
EOF
)"
```

---

## Task 2: Lift / rewrite-backlog on `epr-content-addressing.feature`

**Files:**
- Modify: `genesis/a2o/features/content/epr-content-addressing.feature` (4 scenarios at lines 95, 112, 128, 144)
- Modify: `genesis/a2o/steps/ui/epr-content.steps.ts` (only IFF a Task-1 disposition is WIRE-NEEDED — add the named helper functions)
- Read: `genesis/docs/plans/2026-05-16-epr-wip-disposition.md` (Task 1 output — drives every action in this task)

This task is a per-scenario walk. For each of the 4 scenarios, the action is determined by the Task-1 disposition. Do NOT freelance — the disposition file is the source of truth.

- [ ] **Step 1: For each scenario disposed as LIFT, strip the @wip tag and the inline backlog comment**

For a scenario with `Disposition: lift`, in `epr-content-addressing.feature`:
1. Locate the @wip line (e.g., `  @wip @browser-only` at line 95)
2. Locate the comment block above it (typically 4–6 lines of `# @wip retained: …`)
3. Delete the comment block AND the @wip tag (replace `  @wip @browser-only` with `  @browser-only`, or delete the line entirely if no other tags)
4. Do NOT modify the Scenario: line or any of its steps

- [ ] **Step 2: For each scenario disposed as WIRE-NEEDED, add the helpers first, then lift**

In `genesis/a2o/steps/ui/epr-content.steps.ts`, add the helper functions named in the Task-1 disposition rationale. Follow the existing patterns in the file (the same step-def file has helpers for the 5 already-passing scenarios). Then strip @wip + inline comment as in Step 1.

Run the scenario locally to confirm it passes before stripping @wip:
```bash
cd /projects/elohim/app/elohim-app
HUSKY=0 pnpm run cypress:run --spec '<scenario file>' --grep '<scenario name>'
```
If FAIL, keep @wip and downgrade the disposition to SUBSYSTEM-MISSING for that scenario — update the disposition file inline before moving on.

- [ ] **Step 3: For each scenario disposed as DEFER-WITH-EVIDENCE, rewrite the inline backlog comment**

Replace the existing 4–6 line `# @wip retained: …` block with a structured backlog comment that follows this template:

```gherkin
  # @wip retained: <one-line subsystem name> not yet built.
  # Backlog destination: <graph-native sprint / doorway-full-facilitator sprint / a2o-tooling>
  # Citation: 2026-05-16-epr-wip-disposition.md row <N>
  # Gate condition: <one-line precondition for lift>
  @wip @browser-only
  Scenario: <unchanged>
```

- [ ] **Step 4: For each scenario disposed as RESTRUCTURE, propose new prose to operator**

If a scenario's verbs don't map cleanly to *any* substrate path, the scenario was written speculatively and needs a rewrite. In that case STOP this task and report BLOCKED — restructuring scenario prose is Opus work that needs the operator's eye on whether the underlying human moment still matches the manifesto. Do NOT silently rewrite scenarios.

- [ ] **Step 5: Verify the file still parses + lints**

```bash
cd /projects/elohim
npx tsx app/elohim-app/scripts/scan-coverage.ts --feature genesis/a2o/features/content/epr-content-addressing.feature
```
Expected: clean exit, no parse errors, scenario count unchanged.

- [ ] **Step 6: Commit**

```bash
git add genesis/a2o/features/content/epr-content-addressing.feature genesis/a2o/steps/ui/epr-content.steps.ts genesis/docs/plans/2026-05-16-epr-wip-disposition.md
git commit -m "$(cat <<'EOF'
feat(a2o): EPR content-addressing @wip closure — N lifted, M deferred

Per Task 1 disposition (2026-05-16-epr-wip-disposition.md):
- Lifted: <N> scenarios — step-defs ready / helpers wired
- Deferred-with-evidence: <M> scenarios — backlog destinations
  cited inline (graph-native / doorway-full-facilitator / a2o-tooling)

Drops the previous "@wip retained" prose blocks in favor of
structured backlog comments that name the gate condition and
the destination sprint per row of the disposition table.
EOF
)"
```

---

## Task 3: Lift / rewrite-backlog on `epr-cross-peer-resolution.feature`

**Files:**
- Modify: `genesis/a2o/features/federation/epr-cross-peer-resolution.feature` (8 scenarios at lines 112, 128, 142, 158, 170, 184, 196, 209)
- Modify: `genesis/a2o/steps/**/*.steps.ts` (IFF dispositions specify federation step-defs — note that some federation verbs may live in `genesis/a2o/steps/federation.steps.ts` if that file exists; check with `grep -lE "consented member|reach gate|cross-peer" genesis/a2o/steps/**/*.steps.ts` before assuming epr-content.steps.ts is the only file)
- Read: `genesis/docs/plans/2026-05-16-epr-wip-disposition.md`

Same shape as Task 2; the only differences are file paths and that the federation feature has a multi-paragraph backlog block at lines 99–109 (the "5 missing pieces" inventory) that may need updating after the dispositions land.

- [ ] **Step 1: For each of the 8 scenarios, apply the disposition**

Walk through scenarios 5–12 from the Task-1 table. For each:
- LIFT → strip @wip, no other changes
- WIRE-NEEDED → add helpers in the appropriate step-def file, run scenario, then strip @wip
- DEFER-WITH-EVIDENCE → rewrite the inline comment block using the template from Task 2 Step 3
- RESTRUCTURE → STOP and report BLOCKED

- [ ] **Step 2: Update the multi-paragraph backlog block at lines 99–109**

The block currently lists "5 missing pieces" the federation step-defs need (cross-peer recognition tracking, disconnect simulator, etc.). After Task 1's walk, some of those pieces may be in-flight or landed elsewhere. Update each bullet to reflect post-walk truth:
- If a piece is now landed → strike it from the inventory with a `# now landed: <commit-or-file>` note
- If a piece remains missing → keep it but cite the disposition file row that depends on it
- If a piece is moved to graph-native → cite the destination

- [ ] **Step 3: Verify the file still parses + lints**

```bash
cd /projects/elohim
npx tsx app/elohim-app/scripts/scan-coverage.ts --feature genesis/a2o/features/federation/epr-cross-peer-resolution.feature
```
Expected: clean exit, no parse errors, scenario count unchanged.

- [ ] **Step 4: Run the federation pipeline smoke locally for any lifted scenarios**

```bash
cd /projects/elohim/app/elohim-app
HUSKY=0 pnpm run cypress:run --spec 'genesis/a2o/features/federation/epr-cross-peer-resolution.feature'
```
Expected: all previously-passing scenarios still pass; all newly-lifted scenarios pass; all retained @wip scenarios skip cleanly.

If a lifted scenario fails, keep @wip and downgrade the disposition row.

- [ ] **Step 5: Commit**

```bash
git add genesis/a2o/features/federation/epr-cross-peer-resolution.feature
# include step-def files only if helpers were added
git commit -m "$(cat <<'EOF'
feat(a2o): EPR cross-peer-resolution @wip closure — N lifted, M deferred

Per Task 1 disposition (2026-05-16-epr-wip-disposition.md):
- Lifted: <N> scenarios — step-defs + fixture helpers ready
- Deferred-with-evidence: <M> scenarios — backlog cited inline

Multi-paragraph backlog block at lines 99–109 updated to reflect
post-walk truth: <list pieces now landed / pieces moved to
graph-native / pieces remaining for doorway-full-facilitator>.
EOF
)"
```

---

## Task 4: iroh Phase 12 readiness check + conditional AgentPeerBinding arm

**Files:**
- Read: `genesis/docs/superpowers/plans/2026-05-10-iroh-phase12-peer-transport-manifest.md` (status header + task checkboxes)
- Read: `elohim/elohim-storage/migrations/2026-05-10-120000_peer_transport_manifest/up.sql` (migration shape — already on disk per prior check)
- Read: `elohim/elohim-storage/src/p2p/revocation_attestation_message.rs` (template for the new wire struct)
- Read: `elohim/elohim-storage/src/epr_atom_service.rs:393–460` (template for the new match arm)
- Conditionally Create: `elohim/elohim-storage/src/p2p/agent_peer_binding_message.rs`
- Conditionally Create: `elohim/sdk/schemas/v1/dna-signals/agent-peer-binding.schema.json` (IFF the schema is not already on disk — schema-first per `feedback_schema_first_ioc`)
- Conditionally Modify: `elohim/elohim-storage/src/epr_atom_service.rs` (insert new arm before `other_kind` catch-all)
- Conditionally Modify: `elohim/elohim-storage/src/p2p/mod.rs` (add `pub mod agent_peer_binding_message;`)

This task is gated. Run Step 1 first; the outcome decides whether Steps 2–7 execute or whether the task closes as "deferred-pending-Phase-12".

- [ ] **Step 1: Check iroh Phase 12 status**

Run all of:
```bash
grep -E "^\*\*Status:|^## Task " /projects/elohim/genesis/docs/superpowers/plans/2026-05-10-iroh-phase12-peer-transport-manifest.md | head -20
grep -nE "peer_transport_manifest|PeerTransportManifest" /projects/elohim/elohim/elohim-storage/src/iroh*/*.rs /projects/elohim/elohim/elohim-storage/src/p2p/*.rs 2>/dev/null | head -20
git log --oneline --all -- 'elohim/elohim-storage/migrations/2026-05-10-120000_peer_transport_manifest/*' 'elohim/elohim-storage/src/p2p/peer_transport_manifest*' 2>/dev/null | head -10
```

Interpret:
- **GREEN** = Phase 12 plan all `[x]`, OR ≥4 iroh adapter files reference `peer_transport_manifest`, OR explicit "Phase 12 LANDED" commit in git log
- **AMBER** = migration on disk but adapters partially wired (≤3 adapter refs)
- **RED** = migration on disk only; no adapter wiring

If **GREEN** → continue to Step 2.
If **AMBER** or **RED** → write a one-paragraph deferral note into `.claude/memory/project_epr_foundation_closure_2026_05_16.md` (created in Task 5) under §"AgentPeerBinding deferred" with the evidence above, and SKIP to Task 5. Do not write any Rust code.

- [ ] **Step 2: Confirm or write the schema**

```bash
ls /projects/elohim/elohim/sdk/schemas/v1/dna-signals/agent-peer-binding.schema.json 2>&1
```
If present, Read it and confirm fields: `subjectCid`, `issuer`, `issuedAt`, `signature`, `metadata.agentCid`, `metadata.peerId`, `metadata.transportKind`, `metadata.boundAt`.

If absent, write the schema first (per `feedback_schema_first_ioc`).

**Source-of-truth declaration (P2P Design Gate, co-located with artifact):**
- **Entity:** `AgentPeerBinding` wire envelope (consumed by the IntegrityNotify direct-notify path; never persisted as a new entry type)
- **Category:** **C** — operational projection / transport envelope only
- **Category A authority lives on:** `peer_identity_bindings` DHT entries on the **infrastructure DNA** (Kitsune2/tx5 canonical agent identity + identity-binding gossip topic per `peer_transport_manifest` spec lines 496–498)
- **Why no new entry type:** The wire envelope is read at the receiving peer, dedupe-checked against `recent_integrity_notifies`, and consumed to update local `peer_identity_bindings` projection rows. If the envelope conflicts with the DHT, the DHT wins (matches the existing `peer_identity_bindings` precedent — Category C, keyed on `agent_cid`, no `dht_anchor_hash`).
- **Schema artifact rationale:** the schema file is the wire contract for the libp2p IntegrityNotify codec — it is **not** a new storage schema. It is a transport envelope specification analogous to `revocation-attestation.schema.json` (which has the same Category-C transport-envelope status). Consumed by `agent_peer_binding_message.rs` (Step 3 of this task).

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "agent-peer-binding.schema.json",
  "title": "AgentPeerBinding DNA signal envelope",
  "description": "Notifies recipients that an agent has bound to a peer transport identity. Category C operational projection over Category A peer_identity_bindings DHT entries on the infrastructure DNA.",
  "type": "object",
  "additionalProperties": false,
  "required": ["type", "attestationKind", "subjectCid", "issuer", "issuedAt", "signature", "metadata"],
  "properties": {
    "type": { "const": "agentPeerBinding" },
    "attestationKind": { "const": "attestation:agent-peer-binding-emit" },
    "subjectCid": { "type": "string", "description": "CID of the peer_identity_bindings entry on infrastructure DNA" },
    "issuer": { "type": "string", "description": "base64-STANDARD ed25519 pubkey of the emitting elohim" },
    "issuedAt": { "type": "string", "format": "date-time" },
    "signature": { "type": "string", "description": "base64-STANDARD ed25519 signature over canonical envelope bytes" },
    "metadata": {
      "type": "object",
      "additionalProperties": false,
      "required": ["agentCid", "peerId", "transportKind", "boundAt"],
      "properties": {
        "agentCid": { "type": "string" },
        "peerId": { "type": "string", "description": "libp2p PeerId or iroh NodeId, depending on transportKind" },
        "transportKind": { "type": "string", "enum": ["libp2p", "iroh"] },
        "boundAt": { "type": "string", "format": "date-time" }
      }
    }
  }
}
```

- [ ] **Step 3: Write the wire struct**

Create `elohim/elohim-storage/src/p2p/agent_peer_binding_message.rs` following the template in `revocation_attestation_message.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentPeerBindingMessage {
    pub kind: String,
    pub subject_cid: String,
    pub issuer: String,
    pub issued_at: String,
    pub signature: String,
    pub metadata: AgentPeerBindingMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentPeerBindingMetadata {
    pub agent_cid: String,
    pub peer_id: String,
    pub transport_kind: String,
    pub bound_at: String,
}

impl AgentPeerBindingMessage {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(bytes)
    }
    pub fn to_bytes(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec_named(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_preserves_all_fields() {
        let msg = AgentPeerBindingMessage {
            kind: "AgentPeerBinding".into(),
            subject_cid: "bafyTEST".into(),
            issuer: "BASE64ISSUER==".into(),
            issued_at: "2026-05-16T10:00:00Z".into(),
            signature: "BASE64SIGNATURE==".into(),
            metadata: AgentPeerBindingMetadata {
                agent_cid: "agent-matthew".into(),
                peer_id: "12D3KooWPEER".into(),
                transport_kind: "libp2p".into(),
                bound_at: "2026-05-16T09:55:00Z".into(),
            },
        };
        let bytes = msg.to_bytes().expect("encode");
        let decoded = AgentPeerBindingMessage::from_bytes(&bytes).expect("decode");
        assert_eq!(decoded, msg);
    }
}
```

Then add `pub mod agent_peer_binding_message;` to `elohim/elohim-storage/src/p2p/mod.rs` in the alphabetical block where other message modules are listed.

- [ ] **Step 4: Run the round-trip test → PASS**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/elohim/elohim__elohim-storage/dev cargo test --lib agent_peer_binding_message::tests::round_trip_preserves_all_fields
```
Expected: 1 passed.

- [ ] **Step 5: Write failing tests for the IntegrityNotify arm**

In `elohim/elohim-storage/src/epr_atom_service.rs::tests`, add two tests mirroring the RevocationAttestation pair around lines 599–660:

```rust
#[tokio::test]
async fn integrity_notify_agent_peer_binding_acks_received_true() {
    let msg = crate::p2p::agent_peer_binding_message::AgentPeerBindingMessage {
        kind: "AgentPeerBinding".to_string(),
        subject_cid: "bafySUBJECT".to_string(),
        issuer: "BASE64ISSUER==".to_string(),
        issued_at: "2026-05-16T10:00:00Z".to_string(),
        signature: "BASE64SIG==".to_string(),
        metadata: crate::p2p::agent_peer_binding_message::AgentPeerBindingMetadata {
            agent_cid: "agent-matthew".to_string(),
            peer_id: "12D3KooWPEER".to_string(),
            transport_kind: "libp2p".to_string(),
            bound_at: "2026-05-16T09:55:00Z".to_string(),
        },
    };
    let bytes = msg.to_bytes().expect("encode");
    let request = EprAtomRequest::IntegrityNotify {
        kind: "AgentPeerBinding".into(),
        payload: bytes,
    };
    let service = build_test_service().await;
    let response = service.handle_integrity_notify(request).await.expect("handle");
    match response {
        EprAtomResponse::IntegrityNotifyAck { received, .. } => assert!(received, "expected received=true"),
        other => panic!("unexpected response: {:?}", other),
    }
}

#[tokio::test]
async fn integrity_notify_agent_peer_binding_dedup_returns_duplicate_reason() {
    let msg = crate::p2p::agent_peer_binding_message::AgentPeerBindingMessage {
        kind: "AgentPeerBinding".to_string(),
        subject_cid: "bafySUBJECT".to_string(),
        issuer: "BASE64ISSUER==".to_string(),
        issued_at: "2026-05-16T10:00:00Z".to_string(),
        signature: "BASE64SIG==".to_string(),
        metadata: crate::p2p::agent_peer_binding_message::AgentPeerBindingMetadata {
            agent_cid: "agent-matthew".to_string(),
            peer_id: "12D3KooWPEER".to_string(),
            transport_kind: "libp2p".to_string(),
            bound_at: "2026-05-16T09:55:00Z".to_string(),
        },
    };
    let bytes = msg.to_bytes().expect("encode");
    let request_a = EprAtomRequest::IntegrityNotify {
        kind: "AgentPeerBinding".into(),
        payload: bytes.clone(),
    };
    let request_b = EprAtomRequest::IntegrityNotify {
        kind: "AgentPeerBinding".into(),
        payload: bytes,
    };
    let service = build_test_service().await;
    service.handle_integrity_notify(request_a).await.expect("first");
    let response = service.handle_integrity_notify(request_b).await.expect("second");
    match response {
        EprAtomResponse::IntegrityNotifyAck { received, reason, .. } => {
            assert!(!received, "expected duplicate to return received=false");
            assert_eq!(reason.as_deref(), Some("duplicate AgentPeerBinding direct-notify — dropped"));
        }
        other => panic!("unexpected response: {:?}", other),
    }
}
```

If `build_test_service()` does not exist in the file's test module, look at the existing RevocationAttestation tests (around lines 599–660) for the test-fixture pattern and reuse it verbatim.

- [ ] **Step 6: Run failing tests → FAIL**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/elohim/elohim__elohim-storage/dev cargo test --lib integrity_notify_agent_peer_binding
```
Expected: 2 FAIL with "unexpected response: IntegrityNotifyAck { received: false, reason: Some(\"unsupported kind: AgentPeerBinding\") ..." or similar — the catch-all arm rejects the unknown kind.

- [ ] **Step 7: Insert the match arm**

In `elohim/elohim-storage/src/epr_atom_service.rs`, locate the `RevocationAttestation` arm (around line 393–448) and insert an `AgentPeerBinding` arm immediately after it, before the `other_kind` catch-all:

```rust
"AgentPeerBinding" => {
    match crate::p2p::agent_peer_binding_message::AgentPeerBindingMessage::from_bytes(
        &payload,
    ) {
        Ok(msg) => {
            let dedupe_key = format!(
                "AgentPeerBinding:{}:{}",
                msg.subject_cid, msg.issuer
            );
            if self.recent_integrity_notifies.contains(&dedupe_key).await {
                tracing::debug!(
                    subject_cid = %msg.subject_cid,
                    issuer = %msg.issuer,
                    "duplicate AgentPeerBinding direct-notify — dropped"
                );
                return Ok(EprAtomResponse::IntegrityNotifyAck {
                    received: false,
                    reason: Some(
                        "duplicate AgentPeerBinding direct-notify — dropped".into(),
                    ),
                    accepted_at: chrono::Utc::now().to_rfc3339(),
                });
            }
            self.recent_integrity_notifies.insert(dedupe_key).await;
            tracing::info!(
                subject_cid = %msg.subject_cid,
                issuer = %msg.issuer,
                agent_cid = %msg.metadata.agent_cid,
                peer_id = %msg.metadata.peer_id,
                transport_kind = %msg.metadata.transport_kind,
                "W2: Received AgentPeerBinding via direct-notify"
            );
            Ok(EprAtomResponse::IntegrityNotifyAck {
                received: true,
                reason: None,
                accepted_at: chrono::Utc::now().to_rfc3339(),
            })
        }
        Err(err) => {
            tracing::warn!(
                error = %err,
                "W2: Failed to decode AgentPeerBindingMessage from direct-notify"
            );
            Ok(EprAtomResponse::IntegrityNotifyAck {
                received: false,
                reason: Some(format!("decode error: {err}")),
                accepted_at: chrono::Utc::now().to_rfc3339(),
            })
        }
    }
}
```

- [ ] **Step 8: Run tests → PASS**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/elohim/elohim__elohim-storage/dev cargo test --lib integrity_notify_agent_peer_binding
```
Expected: 2 passed.

- [ ] **Step 9: Run clippy + fmt on the changed files**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/elohim/elohim__elohim-storage/dev cargo clippy --lib -- -D warnings
cargo fmt --check
```
Expected: clean exit on both. If fmt complains, run `cargo fmt` (no `--check`) and re-run the check.

- [ ] **Step 10: Commit**

```bash
git add elohim/elohim-storage/src/p2p/agent_peer_binding_message.rs elohim/elohim-storage/src/p2p/mod.rs elohim/elohim-storage/src/epr_atom_service.rs elohim/sdk/schemas/v1/dna-signals/agent-peer-binding.schema.json
git commit -m "$(cat <<'EOF'
feat(epr): Band B Task 8 — AgentPeerBinding IntegrityNotify arm

Phase 12 caller-identity is live; D5 gate cleared. The
AgentPeerBinding arm mirrors the RevocationAttestation arm
shape (subject_cid+issuer dedupe key; received-true on first;
received-false with "duplicate" reason on repeat; decode errors
return received-false with the decode error string).

Schema added at elohim/sdk/schemas/v1/dna-signals/
agent-peer-binding.schema.json (Category C: operational
projection over Category A peer_identity_bindings DHT entries
on the infrastructure DNA).

Tests:
- agent_peer_binding_message round-trip (lib)
- integrity_notify_agent_peer_binding_acks_received_true
- integrity_notify_agent_peer_binding_dedup_returns_duplicate_reason
EOF
)"
```

---

## Task 5: Sprint-result memory + close the foundation-completion plan

**Files:**
- Create: `.claude/memory/project_epr_foundation_closure_2026_05_16.md`
- Modify: `.claude/memory/MEMORY.md` (add one-line index entry)
- Modify: `genesis/docs/superpowers/plans/2026-05-15-epr-foundation-completion.md` (tick remaining `[ ]` boxes on Tasks 8, 9, 10, 11; append "Closed: 2026-05-16")
- Read: `genesis/docs/superpowers/plans/2026-05-11-epr-delivery-master.md` (lines 315–326 — the closing-condition checklist; this task confirms each item)

- [ ] **Step 1: Walk the EPR delivery master closing-condition checklist**

The master's §"Closing condition" lists 7 items. For each, record evidence in the sprint-result memory file:

1. **Audit complete:** All 479 latent checkboxes — confirm via `grep -c "^- \[x\]" genesis/docs/superpowers/plans/2026-05-11-epr-delivery-master.md` and adjacent plans
2. **Phase 4 landed:** `grep -nE "TODO\(Phase 4 follow-up\)" elohim/ doorway/ steward/ | wc -l` should return 0
3. **Runtime gaps closed:** `record_predecessor` at `api/epr.rs:189–191`; IntegrityNotify Stage 2; D4 GetDocument recorded in disposition file
4. **A2o coverage lifted:** Cite final lift/defer counts from Tasks 2 + 3 of this plan
5. **Pre-push hook validates** — run `HUSKY=0 git status` then `pnpm run schema:codegen:ts && pnpm run schema:validate && cargo test schema_contract` from `/projects/elohim/elohim/elohim-storage`
6. **Push lands on origin/dev** — operator-driven; record as "ready for push" if Tasks 1–4 closed
7. **Cross-stack integration** — Aunt-and-rage-bait passing on both transports; cite the Jenkins build URL if green

- [ ] **Step 2: Write the sprint-result memory entry**

Create `.claude/memory/project_epr_foundation_closure_2026_05_16.md` with this exact frontmatter and structure:

```markdown
---
name: epr-foundation-closure-2026-05-16
description: EPR foundation sprint closure (2026-05-16) — @wip walk, D4 GetDocument verdict, AgentPeerBinding status, what graph-native inherits
metadata:
  type: project
---

EPR foundation sprint closed 2026-05-16 with the @wip disposition walk + Band B Task 8 follow-on.

**@wip disposition outcome:** lifted <N>/12 scenarios; deferred <12−N>/12 with cited backlog destinations (graph-native: <X>; doorway-full-facilitator: <Y>; iroh-phase-12-followon: <Z>; a2o-tooling: <W>). Full table at genesis/docs/plans/2026-05-16-epr-wip-disposition.md.

**D4 GetDocument verdict:** <DEFER-TO-GRAPH-NATIVE | ESCALATE-NOW | DEFER-WITH-RENDERER-DEPENDENCY>. Evidence: <one-line summary citing scenario classifications from the disposition file>.

**AgentPeerBinding arm:** <LANDED at <commit-sha> | DEFERRED pending iroh Phase 12 — adapter wiring status: <RED/AMBER per Task 4 Step 1>>.

**What graph-native inherits:**
- Foundation phases P1, P2A, P2B, P2C, P3, P3.5, LUG, Phase 4 — all green
- W2A record_predecessor + W2B KeyRotation + Band B RevocationAttestation arm + RecoveryFlowProjector — all landed
- Substrate-level reach gating, identity binding, cross-peer Resolve/ResolveBatch, EPR Head signing — all live
- Out-of-scope carve-outs per EPR delivery master §"Out-of-scope" remain explicit: full social-reach nervous system, experience-story/moment/story-point, VF-GraphQL application layer, elohim-mediated reach matchmaking, full vouch sponsor UX
- + <count> @wip scenarios with documented backlog destinations pointing at graph-native

**Non-obvious discoveries:**
- <list any surprises from the @wip walk — e.g., "scenario X actually maps cleanly to existing Resolve and didn't need GetDocument" or "scenario Y's verbs require subsystem Z that wasn't on any backlog">
- <list any plan-tracking debt found and ticked>
- <list any contract renegotiations on pre-existing artifacts>

**Closing-condition checklist (EPR delivery master §closing-condition):**
1. Audit complete — <evidence>
2. Phase 4 landed — <evidence>
3. Runtime gaps closed — <evidence>
4. A2o coverage lifted — <evidence>
5. Pre-push hook validates — <evidence>
6. Push lands on origin/dev — <evidence or "ready">
7. Cross-stack integration — <evidence or "operator-driven soak pending">
```

- [ ] **Step 3: Add one-line index entry to MEMORY.md**

Per the memory system rules, append a single line under ~150 chars to `.claude/memory/MEMORY.md`. Pick a placement near the other EPR-related entries:

```markdown
- [EPR foundation closure (2026-05-16)](project_epr_foundation_closure_2026_05_16.md) — @wip walk + D4 verdict; N/12 lifted; AgentPeerBinding <status>; graph-native inherits clean substrate.
```

Confirm the entry total stays under the 24.4KB warning threshold:
```bash
wc -c /projects/.claude-config/projects/-projects-elohim/memory/MEMORY.md
```
If over, trim the most stale entry in the same edit (don't add over a warning).

- [ ] **Step 4: Tick the remaining boxes on the foundation-completion plan**

In `genesis/docs/superpowers/plans/2026-05-15-epr-foundation-completion.md`, walk Tasks 8, 9, 10, 11. For each `- [ ] **Step N:`, tick to `[x]` IFF the step was actually done by this closure plan. For steps that this closure plan deferred (e.g., Task 11 cross-stack soak is operator-driven), leave `[ ]` and add an inline note `<!-- deferred: operator-driven; see project_epr_foundation_closure_2026_05_16.md -->`.

Append at the bottom of the Goal section: `**Closed: 2026-05-16** — succeeded by 2026-05-16-epr-foundation-closure.md; sprint-result at .claude/memory/project_epr_foundation_closure_2026_05_16.md.`

- [ ] **Step 5: Commit**

```bash
git add .claude/memory/project_epr_foundation_closure_2026_05_16.md /projects/.claude-config/projects/-projects-elohim/memory/MEMORY.md genesis/docs/superpowers/plans/2026-05-15-epr-foundation-completion.md
git commit -m "$(cat <<'EOF'
docs(epr): close foundation sprint — sprint-result + plan ticking

Closes the EPR foundation sprint 2026-05-16. The successor plan
2026-05-16-epr-foundation-closure.md ran the @wip walk, recorded
the D4 GetDocument verdict with evidence, landed (or deferred)
the AgentPeerBinding arm per Phase 12 status, and walks the EPR
delivery master's 7-item closing-condition checklist.

Sprint-result memory entry captures: lift/defer counts, D4
verdict, AgentPeerBinding status, what graph-native inherits,
and non-obvious discoveries from the @wip walk.

Foundation-completion plan boxes ticked where landed; deferred
items annotated inline with pointer to the sprint-result memory.
EOF
)"
```

---

## Self-Review

**Spec coverage:**
- ✅ The three user-named open items are addressed: W2C GetDocument (Task 1 produces the verdict); Wave 5 @wip lift (Tasks 1–3 walk + lift + defer-with-evidence); Task 8 AgentPeerBinding (Task 4 conditional on Phase 12 status check).
- ✅ The "what we're waiting for" set is mapped: Phase 12 status check is Task 4 Step 1; M4 status is irrelevant to this plan (M4 owns its own closure); graph-native brainstorm is explicit out-of-scope and is the natural successor.
- ✅ Closing condition from EPR delivery master is walked in Task 5 Step 1.

**Placeholder scan:**
- No "TBD" / "fill in" — every step has its content.
- Where the disposition outcomes drive counts (`<N>`, `<M>`), those are deliberate — they are filled in by the agent executing Task 1 and cited downstream. The commit message templates use `<N>` / `<M>` as substitution slots, not as plan placeholders.
- The conditional schema body (Task 4 Step 2) is fully specified.
- The conditional Rust arm (Task 4 Step 7) is fully specified.

**Type consistency:**
- `AgentPeerBindingMessage` / `AgentPeerBindingMetadata` field names match between the schema (Task 4 Step 2), the wire struct (Step 3), the test fixtures (Step 5), and the match arm (Step 7).
- Dedupe key format `"AgentPeerBinding:{}:{}"` (subject_cid, issuer) is consistent between the test expectations (Step 5) and the implementation (Step 7).
- The disposition file's column names (Step-def state / Disposition / Backlog destination) are reused verbatim in Tasks 2 and 3 step instructions.

**One known soft spot:** the cypress invocation in Task 2 Step 2 and Task 3 Step 4 assumes the elohim-app cypress runner accepts `--grep` for scenario filtering. If the actual runner uses a different flag (e.g., `--tags`), the executing agent will discover this on first invocation and adapt — the plan should NOT pretend to know without checking. Marked here so the executor doesn't waste cycles fighting it.
