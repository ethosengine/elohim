---
title: Acceptance-aware native reconciliation
id: acceptance-plan
kind: plan
status: DONE_WITH_CONCERNS
written: 2026-09-09
serves: dev-system-equilibrium
cites:
  - "native-process-reconciliation-grounding | Native grounding for reconciliation, search, and governed algorithms | sha256:957b98e39076ff32 | path: genesis/docs/analysis/2026-09-09-native-process-reconciliation-grounding.md"
---

# Acceptance-aware native reconciliation

Implement the operator-approved local proof: native context separates fulfillment, technical review and appointed experiential acceptance; the memory ceremony consumes that distinction for one declared concern. No DHT or new EPR kind; existing local append-only FlowEvents carry additive note slots. Local governance and actor claims establish accountable appointment, not cryptographic network qualification. No stock accounting changes or retroactive feature acceptance.

- [ ] A scoped independent acceptor can be appointed and can record evidence-pinned acceptance or requested changes without altering historical note bytes.
- [ ] Native context exposes all scoped assertions with fulfillment, review, acceptance, conflicts, historical candidates and deterministic remaining-work presentation.
- [ ] The memory ceremony and an executable devflow story distinguish technical completion from accepted experience using the native projection.
- [ ] Independent technical review and practical acceptance prove the CLI on isolated fixtures and the doorway chain, and a bounded ceremony readout preserves historical uncertainty.

## Interface contract

Extend `flow note` with `--appoint <actor-claim-cid>` on rulings, and optional verdict metadata `--purpose acceptance --appointment <ruling-cid> --fulfillment <event-cid> --review <verdict-cid> --report <path> [--supersedes <acceptance-cid>]`. Appointment targets one exact Commitment. Acceptor must have an exact registered claim different from implementer. Existing notes and technical verdicts remain byte-identical; note action stays Cite, unit run-note, fulfills empty. Metadata slots: appoint:, purpose:acceptance, appointment:, fulfillment:, review:, report-path:, report-cid:, scope-cid:, supersedes:. Slot order after verdict, before actor/steward trailers. Report body CID via existing codec. Report JSON fields: intent, revision, environment, actions (nonempty string array), observations (nonempty string array), evidence (nonempty array of {path,cid}), limitations (string array). Evidence paths confined to root, files must resolve and body hashes match. Report and evidence metadata never establishes semantic fitness on its own.

Admission and reader share helper validation. For acceptance, actor pin must equal appointment; technical verdict is approved, on same commitment, and not acceptance; fulfillment is Produce linking that commitment. Existing old review without revision binding remains a technical fact; acceptor explicitly names exact review and fulfillment examined. Reject malformed references before append. Explicit supersession is on same commitment; stale branches remain contested on read, never choose by wall-clock. No supersession invents a terminal discharge.

Reader module `flow/reconciliation.rs`: one borrowed records snapshot, per-scope assertions and commitments, authored state retained, history never erased. APIs coordinate between implementers. Add `reconciliation` JSON field; human context renders derived statuses. Statuses acceptance-unestablished, accepted, changes-requested, revalidation-required, contested; fulfillment/review are independent evidence facets. Missing or contradictory evidence cannot yield accepted. Directly scoped orphan commitments remain visible; historical label candidates explicitly unverified. Source body change and scenario content change cannot silently inherit acceptance. Report pins rechecked on reads. Preserve legacy JSON keys and generic stock/walk contracts.

## Acceptance and gates

Owning gate `just gate eprfs`, native pool and berth required. Full package verification after package-first memory-ceremony edit; scoped projection only. Story owning gates and context-blind story review. Fixture CLI coverage: no review/fulfillment/acceptance combinations, appointment mismatch/self-acceptance, evidence changes/missing paths, revised source/scenario, contradictory events, supersession/conflict, deterministic text/JSON and explicit truncation, old CID preservation, duplicate/concurrent admission.

Practical acceptor gets only user intent, usage, fixture/live entry paths and scoped appointment; exercises CLI, locates contrary evidence, states what remains. Does not accept doorway UI. Existing ceremony canonical-rewrite approval remains required only if a substantive rewrite is actually proposed. End with bounded readout, evidence delta and habit projection, no status flip.
