---
id: "roadmap-reconciling-networks-that-grew-apart"
kind: "roadmap"
contentType: "roadmap-item"
contentFormat: "markdown"
title: "Reconciling networks that grew apart: history made under one identity, brought to the person it belongs to"
slug: "reconciling-networks-that-grew-apart"
written: "2026-10-05"
author: "human:matthew, recorded by claude-fable-5-1"
status: "proposed"
target_window: "open-ended"
themes: [identity, provisioning, reconciliation, local-first]
relatedNodeIds:
  - "backlog-identity-chain-merge-reconcile-kinship-lineage"
  - "genesis/a2o/features/auth/device-provisioning-paths.feature"
  - "genesis/a2o/features/auth/device-consent-grant.feature"
  - "memory:feedback-trust-and-affirm-later-never-blocking"
tags: [identity, device-provisioning, reconciliation, bespoke-network, pre-provisioned-node, epic, seam]
---

# Reconciling networks that grew apart

## Where the seam surfaced

Found on 2026-10-05 while designing how a node joins its person's identity (branch
`feat/device-authorization-grant`). Operator's words: reconciling bespoke networks is an
epic-level concern of the protocol, and this is where that seam surfaces.

A node can begin an identity of its own before anyone tells it whose it is: a workspace that
configured itself, a blade slid into a rack, a device set up offline. When it later meets the
person it belongs to, there are two cases, and they are not the same size.

- **Nothing developed under the identity it began.** Giving that identity up, deleting its key and
  making a new one is fine. The joining story covers this as the recommended default, with a
  confirmation on the node so it knows what is about to happen to it.
- **History developed by accident.** Content, relationships, acts, perhaps other peers who relied
  on them. This is the case that needs care. The joining story's default leaves that history
  exactly as it was and says so; it does not bring it into the person's identity.

## Floor and ceiling

Operator's framing (2026-10-05): this follows the protocol's usual shape, a deterministic floor
and an elohim ceiling.

- **Floor: forget and re-join.** Mechanical, always available, needs no judgment and no other
  party. The node gives up what it had, makes a new key and joins. It is the right answer when
  nothing developed, and it remains available when something did, provided the node is told what
  will be lost before it confirms.
- **Ceiling: formal reconcile.** Judgment over what developed: what is the person's, what others
  relied on, what relationship the two identities should have. This is where an attending elohim
  works, and it is the surface this entry names as unbuilt.

Refined the same day into three options, by what is at stake:

| Option | Stakes | What happens | State |
|---|---|---|---|
| Forget and start fresh | Low | The node gives up what it had, makes a new key and joins. | In the joining story; not built. |
| Remember and just join (adopt) | Low | The node keeps its key and what it made, and joins as it is. Informal and pro-social: nobody is asked to justify anything. Nothing is brought across; nothing is lost. | In the joining story; the "as it is" join is being built first. |
| Reconcile | High | What joins in kind, and how the things with a lot resting on them are recognized, elevated and given standing. | This entry. Unbuilt. |

Adopt destroys nothing and leaves reconciliation reachable later.

## The missing node, in mintable shape

- **Chain:** identity provisioning (`device-provisioning-paths.feature`)
- **Between:** "a node that already has an identity and history joins its person's identity"
  → "the person holds, in their own identity, what that node made before it was theirs"
- **Missing node:** a peer-to-peer reconciliation surface. Assertion: two identities (or two
  networks of records) that grew apart can be brought into a declared relationship by the parties
  to them, record by record or as a whole, with nothing wiped, merged or re-keyed that a party did
  not confirm. Probe: none yet.
- **Current state:** unbuilt. The joining ceremony recognizes the node and leaves its earlier
  history untouched and unreconciled.

## What achieving it would feel like

A person who finds that a node of theirs spent a month acting under an identity nobody intended
can look at what it made, decide what is theirs, and have the network agree, without losing the
node's key, its history, or anyone's trust in what was already relied on. The same surface serves
two households that each ran their own small network and now want to join them, and an elohim
node operator bringing a pre-provisioned blade under its operator of record with no person
present.

## What is already decided, and what is not

Decided by the operator (2026-10-05):

- A recommended click-through default, not a forced decision each time.
- The pre-provisioned node always confirms what will happen to it.
- Deletion and re-keying are acceptable when no history developed.
- History that developed by accident is never wiped, merged or re-keyed without care.

Not designed, and to be designed here and nowhere smaller:

- What record connects two identities, who signs it, and what it does and does not confer. The
  authority ruling keeps recognition (whose node) apart from standing (who may change a subject);
  a connecting record must not move standing by itself. Correlating identities is a consent act.
- How history is offered, reviewed and accepted: whole, in part, or not at all.
- What peers who already relied on the earlier identity are told.
- How this meets identity-chain fork and merge resolution
  (`backlog-identity-chain-merge-reconcile-kinship-lineage`), which is the same question asked of
  one identity's key lineage.

An earlier draft of the joining story carried a scenario for a signed record connecting the two
identities. It was removed so that a slice-sized story would not settle an epic-sized design.
