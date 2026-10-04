---
name: local-first
description: Use when designing or reviewing anything a person touches — a flow, a ceremony, a screen, a client API, a sync stream — to check that it works for one person on their own device with nobody else present, and that adding anyone or anything else never becomes a condition for what already worked. Triggers - "does this work offline?", "does this need a doorway?", "does this have a working version?", "what happens when the hub is asleep?", a design that names a server before it names the device, a flow whose first step is a network call, "the native path" listed as later.
metadata:
  sourceRuntime: claude
  master: package
  governance: "epr:elohim-agent/skills/local-first"
---

# Local-First

A reading of a design from the person's side. It asks what works when they are alone, and whether
each thing that arrives — another device, a peer, an always-on copy, a doorway, a pool — adds
something without becoming required.

Run it beside the `p2p-design-gate`, which decides where truth lives and what it costs. This skill
decides what the person can do before any of that is reachable.

## When to use it

- A flow, ceremony or screen a person acts through.
- A client or SDK surface.
- Anything that syncs, or that names a doorway, a hub or a pool.

Not for: a change with no person on either end (a reconcile sweep, a build step); a single
component's appearance.

## 1. Read the design along four lines

Four things vary independently. Read the design along each one separately. No line is a ladder,
and no combination is a tier.

| Line | The cases |
|---|---|
| Who holds the person's key | the device · a doorway, for them · nobody (a visitor) |
| Who else is present | nobody · the person's other devices · other people · an always-on copy · a pool |
| How far the thing may travel | its reach, from `private` to `commons` |
| Whether the thing has a working version | none · the author's alone · shared by those who may edit |

**The rule on every line:** moving along it adds strength, reach or capacity. It never becomes a
condition for what worked before the move, and when what arrived goes away again, what worked
before still works.

## 2. Alone

State what the feature does with one device and nothing else reachable. Then say it for each case
of who holds the key.

| The key is | Alone, the device can |
|---|---|
| on the device | read every published thing it holds; keep a working version; fix a version, address it and sign it; the network learns of it when a peer is reachable |
| held by a doorway | read every published thing it holds, checked against its address; keep a working version; fix and address a version; signing waits for the doorway, and the person is told it has not been saved to their account |
| nobody's | read what it holds; nothing is kept past the visit, and the person is told |

A published thing is fixed and addressed, so any device holding its bytes can serve it offline and
prove it is the right thing. That is owed in every case.

**An act that needs another party** (a consent someone must receive, an agreement two people
sign, a vote in a collective) is not exempt. Alone, the person's side is prepared, fixed and kept.
It completes when the other party is reachable. "It waits" is acceptable only for the part that is
someone else's. A doorway acting for the person is not another party for any step the person's own
key could take: that step is their own side of the act and must be runnable without the doorway.

**A doorway hosts; it does not own.** Design each step so the person's own node can run it, and
have the doorway mount that same step for the people it hosts. Both are designed in the same
pass: a design that proves the doorway path first and leaves the person's own node for later has
the order backwards. For a hosted person the doorway holds their key. It can see what passes
through it and it can sign as them: say both, and never describe it as unable to read.

A line or a row that does not apply is answered "does not apply" with the reason. Without a
reason it counts as blank. A feature whose
purpose is to obtain authority from somewhere (a login, a grant) still has an alone case: what the
person's side prepares and keeps, and which of their own devices could give the authority without
any host.

## 3. What kind of state is it

Skip to §4 if the feature has no working version and keeps nothing between acts; say so in the
output. Otherwise use the names that exist. Do not mint others.

| What it is | Its home | Gate class |
|---|---|---|
| What this device is showing (selection, scroll) | this device; never sent | not an entity |
| What is passing (who is here, a cursor, a drag) | a separate unstored channel between those present | not an entity |
| A working version | the holder's document store, on the merging layer; values only, no authority | see the working-version standard; drafts are near Private (B) |
| A published version | the DHT declares and elects it; storage projects it; amber until anchored, green after | Notarized (A), Linked (A2), Attested-Private (B2) |

- **A working version is a declared choice.** A content type either has one or publishes in one
  act. Say which before designing the client.
- **A draft starts with its author alone.** The published thing's reach is a ceiling for the
  working version, not its starting audience. Widening is an act the person takes.
- **Republishing is a submission.** The root author's new version becomes the head of their own
  chain; anyone else's is a candidate in an election and can lose. Show what the election says
  now (pending, current, not current) and never show it as final.
- **A working version is its own document.** It is never the document published values are
  projected into, and who may edit it is a signed record, never a field inside it.
- **Merging is agreement, not legitimacy.** What peers merge is unauthenticated until a version is
  declared and elected. Never show merged state with the mark of an anchored one.
- **A gesture passes, then lands.** State that changes many times a second is passing state; the
  working version gets one change when the gesture ends.
- **A heartbeat hints; the record decides.**
- **An agent works on a draft.** The person merges its work whole or in part, attributed. An
  agent never writes the shared copy directly.
- **Some things never travel.** A person's attention, revealed preferences and private notes stay
  on their device and are never pooled.

Design contract: `genesis/docs/superpowers/specs/2026-10-04-working-version-sdk-standard-design.md`.

## 4. When others arrive

For each thing that can be present — another of the person's devices, another person, an
always-on copy, a doorway, a pool — answer three questions:

1. **What does it add?**
2. **What does it learn?** Even a copy that cannot read learns sizes, timing and who asks. Say
   whether it stands inside or outside the circle the thing belongs to.
3. **What still works when it is gone?** Including gone for good.

Answer for what the design names, and always for the person's other devices: could one of them
do what the design asks a host to do?

Adding the person's own device should cost them nothing. Adding a relationship should cost one
act of trust, not one per item. If a design joins a person to everyone with no act at all, say so:
that is a way to be captured.

For a pool: `genesis/docs/superpowers/specs/2026-10-04-commons-pool-as-collective-party-design.md`
§5 lists what a pool owes the devices that carry it. A holder that serves a draw knows who drew
and which bytes, so nothing a person would not want its holders to see them read belongs in a
pool.

## 5. What the person is shown

- What is only on this device.
- For each other copy: caught up, behind or unknown (the sync-state contract's terms).
- Whether a submitted version is pending, elected or lost.
- That a wait is a wait, and on what.

## Red flags

- Any step before the person's own device or node has acted is a request to a server.
- A store only a doorway has holds the ceremony's state.
- Any path that needs no host, however it is named, is later or out of scope. "Later" with no
  named slice counts as out of scope.
- The design serves only hosted people. Listing the person's own node as out of scope or later
  counts as not stating it.
- An act is called "one step" to avoid saying what is kept when it cannot complete.
- A feature disappears offline when it could degrade.
- A content type with a working version has nowhere to keep it while the network is away.
- Passing state is stored, or the working version is written at the rate of a drag.
- Merged state is shown as anchored.
- The shared copy is the only copy: no draft, no way to work unwatched.
- An agent writes to the shared copy directly.
- A doorway is trusted to be the same doorway next time.
- Something is required from the person that must never leave their device.

Any of these is a reason to redesign, not a risk to note. Say which parts of the design stand as
they are, and name the new home each one needs: sound engineering in the wrong place is the usual
finding.

## Output

Present this before the design gate's output.

```
## Local-first reading: {feature}

- **Alone, key on the device:** {what works, what is kept}
- **Alone, key held by a doorway:** {what works, what waits, what the person is told}
- **Alone, no key:** {what works}
- **Working version:** none | author | shared — {who may edit, what republishes}
- **Each kind of state:** {shown here / passing / working / published}
- **Arrivals:** {for each: adds, learns, survives its loss}
- **Shown to the person:** {the states they can see}
- **Refusals:** {red flags found and how the design changed}
- **Stands as is:** {what is sound, and the home each part needs}
```

A missing line counts as blank.

**Back-fill check.** These cannot be answered from a design that started at a server. If one is
blank, the reading was not done.

1. Delete every doorway. What does the person's own node run?
2. Close the laptop mid-change and reopen it with no network. What is there?
3. Name the first thing in the flow that waits on someone else. Why can it not be prepared alone?
