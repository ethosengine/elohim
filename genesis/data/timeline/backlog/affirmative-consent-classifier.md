---
id: "backlog-affirmative-consent-classifier"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Affirmative-consent classifier: surface only what needs a human's yes, never a TOS stream"
slug: "affirmative-consent-classifier"
written: "2026-09-26"
author: "agent:orchestrator@claude-opus-5-5 (operator-directed, human:matthew)"
status: "backlog"
priority: "high"
tags: [governance, consent, qahal, stewardship, permissions, capture, ergonomics]
relatedNodeIds: [feedback_consent_fatigue_route_by_stakes, feedback-decide-clear-calls-not-over-ask, feedback_legacy_consumers_governed_gifts, feedback_human_loop_not_terminal_authority]
cites:
  - bridges/CLAUDE.md
  - .claude/epr-meta/policies.yaml
  - elohim/eprfs/epr-cli/src/flow/memory/supersession.rs
---

# Affirmative-consent classifier

**Operator, 2026-09-26:** "these approvals are too tedious … we can't be asking for approvals
constantly … humans don't want to be reading the TOS. What's surfacing suggests an important concern
of the elohim protocol (we probably need our own permission classifiers to help catch what really
needs affirmative consent)."

## The concern

The distinct-Steward rule is sound: nobody approves their own fold, offer or election. But in one
session, sound primitives produced a stream of consent requests. There was the tier registration,
two command permissions, the Jenkins offer verdict, the steward-of-record import and three
memory-fold verdicts. The operator answered most of them with "yes, record it for me".

Consent asked constantly degrades into rubber-stamping. A rubber stamp is a capture vector, and it
looks like governance while carrying none. It is the TOS failure mode, and the protocol must not
reproduce it for its own people.

## What the classifier does

It routes each pending verdict by stakes, not by primitive:

- **Affirmative consent (always surfaced, one at a time):**
  - anything crossing the network: settlement to token or fiat, an offer to a consumer we don't own;
  - identity and keys;
  - irreversible or outward-facing acts;
  - anything a fixture Steward cannot validate.
- **Standing delegation (recorded, legible, contestable, never re-asked):**
  - curatorial and in-tree verdicts under a delegation the human granted once, e.g. "approve
    memory folds that keep their members' substance";
  - each act still lands as a verdict naming the delegation's CID. The human can review the
    stream after the fact, and revoke the delegation, which is itself a verdict.
- **Batched review (surfaced together, one decision):** same-kind verdicts that fall outside
  every delegation.

The delegation is a VF Agreement between the human Steward and the agent seat. It carries a
scope, a `retire-when:`, and the same distinct-Steward rule for its own grant. It is not a bypass.

## First slice

Delegated verdicts for memory supersession:
- `supersession.rs` accepts a verdict whose steward is a human and whose actor is an agent seat
  holding a live delegation for `memory-fold`.
- The index projection names the delegation beside each fold it activated.

Receipt: a curator's fold activates without a prompt, the index names the delegation, and revoking
the delegation re-pends the folds it activated.
