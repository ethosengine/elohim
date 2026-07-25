---
id: "backlog-epr-meta-unregistered-validators"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "15 .epr-meta rules name validators that are not registered — the invariants are prose, not predicates"
slug: "epr-meta-unregistered-validators"
written: "2026-07-25"
author: "shift-dataplane-agility-card-and-concerns"
status: "backlog"
priority: "medium"
themes: [epr-meta, governance, agent-agency, validators, dev-tooling]
relatedNodeIds:
  - ".claude/scripts/_lib/epr_meta.py"
  - ".claude/scripts/_lib/epr_seal.py"
  - "elohim/elohim-storage/src/.epr-meta"
---


# Unregistered `.epr-meta` validators

Fifteen rules across seven manifests declare `validator: epr:validator-*` for a validator that
exists in neither `REFERENCE_VALIDATORS` nor `RUNTIME_SCOPED_VALIDATORS`
(`.claude/scripts/_lib/epr_meta.py`). Nothing evaluates them — their text reaches the agent as an
advisory, and no predicate ever fires.

| manifest | rules |
|---|---|
| `elohim/.epr-meta` | `interface-first-reuse-{rs,ts,mjs,py}` (matches every `.rs`/`.ts`/`.mjs`/`.py` under `elohim/`) |
| `elohim/elohim-storage/src/.epr-meta` | `peer-fallback-invariant`, `heal-fills-never-moves`, `dataplane-guide-star` |
| `elohim/lvi/.epr-meta` | `co-resident-safety-on-sandbox-edits`, `mount-dont-ship-on-materialization-edits`, `authorization-reuse-on-grant-edits`, `no-derived-bytes-in-commons-on-state-edits` |
| `genesis/orchestrator/manifests/.epr-meta` | `webrtc-ice-servers-camelcase` |
| `bridges/did/.epr-meta` | `did11-conformance-on-wire-shape-edits` |
| `scripts/ci/.epr-meta` | `deploy-scripts-bash-coreutils-only` |
| `elohim/holochain/tests/sweettest/.epr-meta` | `unique-content-id-in-tests` |

## Why this is now visible (and no longer urgent)

Until `4d90bf276` these rules did not merely fail to check — they **blocked**. The
unresolvable-validator branch returned `ask` regardless of a rule's declared class, so all fifteen
(every `class: inject`) escalated into blocking permission prompts. Because a PreToolUse hook's
`ask` sits above the permission layer, no auto-mode setting could clear it: an unattended session
editing any Rust file under `elohim/` simply hung waiting for a human. That is very likely the
mechanism behind the recurring "overnight session stalls on a permission prompt" class
(memory: `feedback_overnight_permission_stalls`).

`4d90bf276` clamps an unresolvable validator to the rule's DECLARED class — `ask`/`deny` still
route to review (the S5 soundness law in `epr_seal.py` is preserved and now pinned by tests in
both directions), while advisory rules stay advisory. So the bleeding stopped. What remains is
the honest gap below.

## The actual gap

Two of these guard real, expensive, previously-lived defect classes and deserve genuine predicates:

- **`peer-fallback-invariant`** — a `NotFound` arm on the EPR/blob resolution path that omits
  `race_fetch` re-creates the "App not found" class. Mechanizable: match added `NotFound` arms in
  resolution-path files and require a peer-fallback call or an EPR-head-aware syncing body.
- **`heal-fills-never-moves`** — `StampMode::Declare` widened onto a heal path moved the canonical
  head BACKWARDS twice in July 2026 (edge #1188; the 2,838-row resurrection). Mechanizable: flag a
  diff that introduces `Declare` outside the known canonical channels.

The rest are genuinely narrative (`dataplane-guide-star` is a guide-star statement with no
mechanical predicate) and should probably DROP their phantom `validator:` key rather than pretend
to one — a rule that names a validator it does not have misrepresents itself to every reader.

## Definition of done

Either register a real predicate in `REFERENCE_VALIDATORS`, or remove the `validator:` key from
rules that are narrative-only. No rule should name a validator that does not exist.
