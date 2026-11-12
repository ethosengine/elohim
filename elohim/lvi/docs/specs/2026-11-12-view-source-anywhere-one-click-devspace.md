# View source, anywhere — the one-click devspace

**Date:** 2026-11-12
**Status:** Vision. A horizon item (roadmap H6). It schedules nothing and changes no v1 scope.
**Author:** Matthew Dowell (the vision, stated 2026-11-12) + Claude Fable 5.2 (this write-up)
**Reads with:** `2026-07-20-elohim-native-devspace-design.md` (the runtime),
`2026-10-10-workspace-arrival-as-declared-device.md` (arrival), and the host-compatibility plan
`genesis/docs/superpowers/plans/2026-11-10-host-compatibility-foundation-plan.md` (the checks this
vision reuses, named there as item X3).

## The experience

A browser lets anyone right-click a page and choose "view source". That gesture made the early web
learnable: what you used, you could read. It stopped there. You could read the page but not change
it, build it, or offer the change back.

The one-click devspace carries the gesture the rest of the way. Anything a person uses or
experiences in a client can be opened from that client as something they can develop: the screen,
a card on it, the lesson inside the card, the rule that decided what the card showed.

Three people show the range.

**A developer** is reading a component in the repository explorer. They follow its link. A space
opens on that component's source, with the directory's own rules and context loaded. The space
tells them what the component is, that this laptop can build and run it, and how. They change it,
and an agent shows where the change will go next: which build, which queue, who reviews.

**A contributor on a tablet** opens the same link. The space tells them what the component is and
that this device cannot build it, and why. It offers to start the workspace on another device that
can: their own desktop at home, or a friend's that has agreed to host. They carry on in the
browser, as Eclipse Che allows today.

**A grandmother** is using a recipe app and the text is too small on one screen. Today she can
complain, if she can find where. Here she opens the thing she is looking at and says what she
wants. Her elohim (the agent that works alongside her) finds the element's source, makes the
change as a variant, and shows it to her. If she likes it, she keeps it for herself. If she wants,
it is offered back as a proposal that carries her name and her reason. She did not write code. She
did more than give feedback: she suggested something that could be acted on, and it was.

## What the space says before anything runs

Opening a link describes; it does not start anything. This keeps the rule in the runtime design
that following a source branch must not implicitly warm or spawn a runtime. Before the person asks
for a build or a run, the space answers five questions:

1. **What is this?** The component, what it is part of, who stewards it, and the rules that govern
   changes to it (its `.epr-meta`).
2. **Can this device carry it?** Whether it can build and run here, and how, or the specific reason
   it cannot (the wrong CPU features, too little memory, a missing toolchain).
3. **What does it depend on?** Its dependencies as recorded artifacts available from peers, with no
   checkout of their source required.
4. **Where else could it run?** Devices that could host it and have agreed to: the person's own
   first, then others who have offered.
5. **What happens to a change?** The path from an edit to a build, a queue, a review and a landing,
   and what this device could contribute to those queues while idle.

## What it is built from

Little here is new machinery. The vision is a consumer of primitives that exist or are planned.

| The space needs | The primitive | Where it lives |
|---|---|---|
| A link from any rendered thing to its source | The EPR link and the page-info surface (`/epr/<id>`) | `elohim/epr`, the client chrome |
| The environment itself | `DevspaceSeed`, mounted and not shipped | this module |
| "Can this device carry it?" | `Requires`, `Provides` and `admit` | `crates/elohim-host-contract` |
| Dependencies without a checkout | Method and input records, fetch by key or family | the artifact store in `elohim/eprfs` |
| The right build for this device | Variant sets and conformance | host-compatibility plan, lane V |
| Hosting on another device | Placement eligibility and the `delegates-compute` commitment | `elohim/placement`, this module's design |
| An honest account of who supplied what | Commitment and fulfilment, in kind among one steward's devices | host-compatibility plan, ruling of 2026-11-12 |
| A change offered back | Attributed proposals on a shared working version | the working-version SDK standard |
| A refusal that teaches | Typed refusals sent back as feedback | host-compatibility plan, lane F |

The host-compatibility work exists so that a job, and the peer under it, can know before a run
whether it will fit. The same answer, shown to a person, is question 2 above.

## Principles

- **Reachable from the client.** A person should not need to know that a repository exists to find
  the source of what they are looking at. The link is on the thing.
- **Local first.** The first answer is what this device can do alone. Another device is an offer,
  made when this one cannot carry the work or the person prefers it.
- **Describe before running.** Opening shows; building and running are asked for.
- **A variant for yourself comes first.** A person can change their own experience without anyone
  else's agreement. Offering the change to others is a separate, attributed act under the
  component's own rules.
- **The elohim works alongside.** It finds, explains, drafts and repairs. Admission to a team, and
  acceptance of a proposal, stay with the people and rules that hold them. Presence in a space is
  not authorization.
- **Checks before judgment.** What a program can check (will it fit, are the inputs the same, is
  the record signed) is checked by a program, so that an elohim's attention goes to questions of
  integrity, governance and care.
- **Accounted and legible at scale.** With a thousand agents on a hundred machines, a person who
  drops in can still see what is being built, by whom, on whose device, and why.

## What is missing

Named so that none of it is assumed:

1. **Element to source.** A rendered element in a client app does not yet resolve to the EPR of its
   source. The page-info surface resolves pages and content, not components.
2. **A source tree composed from artifacts.** Today a workspace is a full checkout. The space needs
   one component's source with its dependencies supplied as built artifacts. This is roadmap H1
   (the dependency substrate) seen from the person's side.
3. **The describing view.** No surface yet shows the five answers. `admit` returns a verdict to a
   program; nothing renders it for a person.
4. **Hosting by offer.** Placement across peers is roadmap H2, and metering is H4. Hosting on one's
   own second device needs neither a market nor a meter, and could come first.
5. **A variant kept for oneself.** How a person's own variant of a component is held, shown in
   their client, and kept as the original moves on, is not designed.
6. **The non-developer path.** The conversation in which a person says what they want and an
   elohim turns it into a variant is not designed, and it needs its own story before any code.

## Order of approach

This is a suggestion, to be settled when the item is planned:

1. The describing view for a developer on a device that can build (questions 1, 2 and 5), over a
   full checkout. It uses only what the host-compatibility plan lands.
2. The same view on a device that cannot build, offering the person's own other device.
3. Dependencies as artifacts (H1).
4. Element to source in one client app.
5. The variant for oneself, and the non-developer path, each starting from a story.

Entities this would add (a personal variant, an element-to-source binding, a hosting offer) go
through the P2P design gate when the item is planned. None is designed here.
