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

## One composition, several kinds of runtime

A devspace is one kind of runtime. An integration job is another: it takes a proposed change,
builds it and proves it. A person's tooling (a language server, an agent, a preview) is a third.
Kubernetes taught this lesson with pods: one way of composing and placing a unit of work serves
the editor, the tools beside it and the pipeline that integrates the result.

lvi carries that lesson to the protocol's own concepts. An `ark` holds a running unit and a berth
is where it runs. The kinds, as analogues and not as a design:

| Kind | What it is for | The pod it resembles |
|---|---|---|
| Devspace | A person, or an agent, working on source | The workspace pod of Eclipse Che |
| Tooling | Services beside the work: language servers, previews, agents | A sidecar or tooling pod |
| Integration | Building and proving a proposed change, then recording the result | A CI job pod |

What differs is where the composition is resolved. Under Kubernetes a controller decides and a
registry such as Harbor supplies the images. Here the composition is resolved from the peer
dataplane and the runtimes peers already hold: the seed names what is needed, admission says which
devices can carry it, placement finds one that has agreed, and the bytes arrive as verified
artifacts. This is the module's thesis, hyperscale powers brought to peers, applied to more than
the editor. EPR supplies what makes that responsible at scale: identity for every part, declared
reach, attribution, and an account of who supplied what.

"Describe before running" still holds for the person opening a link. It does not mean lvi starts
nothing. When someone asks for a build, or a change is proposed under rules that call for proof,
lvi composes and starts the integration runtime the same way it would start the devspace. Each
start is asked for, admitted and accounted.

## From one person's variant to everyone's affordance

The grandmother's larger text is where this starts, not where it ends. Diversity of this kind is
something the network can afford each person because making a variant is a primitive of
development, not an exception to it. In the network's mature form the same primitive carries the
change outward:

1. **A variant for herself.** She keeps it. Nobody else is asked.
2. **A signal.** Her variant, and her reason, are evidence that the original did not fit her. So
   is the same change made by others. Someone close to her may also raise what she would not ask
   for herself. A grandson who sees her squinting can do it either way: as a peer of equal
   standing, raising in his own name what he sees of her experience, or as the steward of her
   device, under a stewardship grant, if she would sooner not deal with it at all. The network
   rewards and encourages that care in both forms. Because elohim are present and can read the
   context, the response can fit the real needs, wishes and ambitions of the people involved,
   not a general case.
3. **A collective.** When signals gather around one need, the people who share it, with their
   elohim, can form a collective for that space: large print, here. It has members, a purpose and
   rules like any other collective in the network, and it has a lifecycle. It forms, it works, and
   it can dissolve when the need is met.
4. **An architectural change.** The collective's work is not a pile of patches. With their elohim
   they can work on the deeper affordance that would give every client accessible, well-composed
   large print, and on how the network should evolve to provide it.
5. **Reach, graduated.** What they produce earns wider reach by the same ladder as anything else:
   from one person, to those who trust her, to a community, to the commons. Elohim facilitate
   each integration along the way; the people and rules at each rung decide.

So the grandmothers are not only users whose feedback is collected. They are collaborators in the
network's evolution, at the depth they choose, with the elohim doing the work of finding, drafting,
aggregating and repairing. This is what human flourishing means inside the development lifecycle:
the way software is made answers to the particular people it is for, and caring for one of them is
itself a contribution.

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
7. **Runtime kinds.** lvi's design describes the devspace. The tooling and integration kinds, and
   how one seed composes them over `ark` and a berth, are not designed. `ark task run` and the
   placed orchestrator gate are the nearest existing pieces.
8. **From signals to a collective.** How variants and concerns are gathered into evidence of a
   shared need, how a collective forms around it and ends, and how its work graduates in reach,
   are not designed here. They belong with the collective and reach designs, and this vision is a
   consumer of them.

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
