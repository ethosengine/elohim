---
id: "backlog-native-peer-dev-channel-and-collective-memory-seed"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Campaign seed: a private, reach-governed native channel between the operator's own devices, carrying jobs, conversation and collective agent memory"
slug: "native-peer-dev-channel-and-collective-memory-seed"
written: "2026-10-01"
author: "claude-opus-5-5 (recording the operator's direction; not a design)"
status: "open"
priority: "high"
area: "devspace peers / rakia / ark / berth / brit / agent memory"
domain: "protocol"
relatedNodeIds:
  - "habit:dataplane-convergence"
  - "habit:reach-enforced-everywhere"
cites:
  - genesis/docs/superpowers/sprints/2026-10-01-campaign-1.4-closing-definition.md
  - genesis/docs/superpowers/specs/2026-09-30-claim-backed-credentials-and-contextual-authority-design.md
  - genesis/docs/superpowers/specs/2026-09-08-peer-executed-stage-design.md
  - genesis/data/timeline/backlog/reach-delivery-subsystem-solutioning-bootstrap.md
tags: [seed, design-session-input, devspace, peer-channel, jobs, collective-memory, reach, device-binding, trustful-self]
---

# Seed: the native peer dev channel

This records what the operator asked for on 2026-10-01, in the order he gave it. It is input to a
design session. It proposes no entry types, routes or protocols; those go through the P2P design
gate first.

## The need

There are now two Che workspaces: one on ethosengine, one on shem (running on the x86-64-v2 image
flavor). Today the operator carries every message between them by hand.

> "right now, I'm going to have to facilitate all cross communication between workspaces, we need
> a matthew (me) private reach-governed elohim native peer dev communication channels.. pretty
> quickly on top of these device registration epr-authorization grants"

## What it rests on

Device registration and EPR authorization grants, as settled by campaign 1.4. A channel private to
the operator is a channel whose members are devices bound to the real Matthew. The channel is the
first real consumer of that work, not a separate identity system.

## What rides it

The operator's direction is that this is one concern, not several:

- **Conversation** between agent sessions on his devices.
- **Jobs.** Revisit with rakia, ark and berth: "we should be able to post those jobs, alongside the
  communication channel".
- **Brit.** Pull it in: the tree with root integrity anywhere on the network is what the peers are
  collaborating over.
- **The native agent memory harness.** "this would be a 'collective' memory graduation": memory
  that today is local to one workspace becomes shared between the operator's devices.

## Sequence the operator set

1. Campaign 1.4 closes: household mechanism proof, one push, the real Che device on the fleet.
2. This campaign is designed and built.
3. The shem workspace updates to head and launches a conductor under a witness-authorized grant.
4. Collaboration between the two workspace peers drives the shakeout.

The shem workspace is deliberately not a second device in campaign 1.4.

## Constraints already on record, to carry into the design

- **Fixtures and agents are an attribution boundary** (reach solutioning brief, §6a, 2026-10-01).
  Messages and job results between the operator's devices are agentic labor accountable to his
  stewardship. Each should carry which device and which agent session produced it, so it never
  reads as a second human.
- **Thoughts are private; only what is said or done is judged** (constitutional ruling,
  2026-09-10). Private stores are never imported. A collective memory graduation has to say what
  crosses between devices and what stays private to a session.
- **Secrets never cross.** The credential contract already forbids copying keys, keystores or
  signing credentials between devices.
- **Device revocation.** Withdrawing a device must end its place in the channel without touching
  the human or the other devices, as the credential contract requires for publishing.
- **shem is CPU-constrained and shared.** Its 24 older threads also host fleet conductors that
  request an eighth of a core each. Jobs posted to the shem workspace can starve them; the
  workspace devfile is capped at CPU limit 8 and request 2 for that reason.

## What exists to compose, not yet audited for fit

- The peer-executed stage (`just measure <feature> --on <peer>`): one peer sends work to another
  and receives the completion.
- Berth leases and moorings, ark envelopes, rakia manifests.
- Private reach in the reach vocabulary.
- The governed recall session and memory import path in `epr flow memory`.
- The lvi devspace peer-runtime spec.

A quick search of the zomes on 2026-10-01 found no existing peer-signalling or message entry
type. That was a search, not an audit.

## Open questions for the design session

1. What is a message, a job and a memory entry in the P2P classification: notarized, linked,
   private, attested-private or ephemeral? They need not share one answer.
2. How does a receiving device prove that the sender is a device bound to the same human, and at
   what cost per message?
3. What is the head-plane cost of a channel that carries agent conversation at working volume?
4. What does "collective" memory mean between devices of one human, as against between humans in
   a collective, and does the first generalize to the second?
5. What replaces the operator as courier in the meantime, if anything? He declined a git
   coordination branch.

## The operator's observation: convergence is already a rudimentary channel

> "we get the ability to perform atomic updates to the peer network that converge in this next
> push.. that.. is a rudementary communicatino channel itself."

A root the operator authors, updated on one device and converged on the other, is a message: it
is signed by a device bound to him, it carries its own history, and delegation decides who else
may write to it. Once campaign 1.4's push lands, this answers question 5.

Its limits, so the rudimentary form is not mistaken for the finished one:

- **Not private.** Content is readable by the peers that hold it; the confidentiality plane is
  unbuilt, and HTTP reads through a doorway currently return content to any authenticated caller
  regardless of reach (`http-reach-enforcement-gap.md`). Suitable for coordination that could be
  public; not for anything sensitive.
- **Slow.** The convergence deadline is 75 seconds per update.
- **Every message is a head.** Volume is a head-plane cost.

This reframes the campaign: it may be mostly about what to add on top of converging roots
(privacy, latency, volume) and little about inventing a transport.
