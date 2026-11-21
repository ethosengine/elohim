---
id: laptop-onboarding-friction-20261121
title: "From one device to distributed developer participation"
date: 2026-11-21
status: proposed
habits: [workspace-device-arrival]
cites:
  - .epr-meta/workspace-device-arrival.habit.md
  - "steward-device-arrival-journal | The laptop runbook establishes the enrollment and verified exchange still owed | sha256:1c9dde7401bbc78d | path: genesis/docs/superpowers/sprints/2026-11-16-steward-device-arrival/journal.md"
  - "native-dev-tooling-over-peers-plan | The existing hand-over and evidence rules explain what changes across principals | sha256:0ffa5c75af02fc2a | path: genesis/docs/superpowers/plans/2026-10-29-native-dev-tooling-over-peers-plan.md"
  - "collective-memory-integration | The memory integration plan grounds selective contributions, provenance, and review at each audience | sha256:00249865b48968ac | path: genesis/docs/superpowers/plans/2026-09-09-collective-memory-integration.md"
---

# From one device to distributed developer participation

A developer should be able to arrive with local work, see what landed since their last session, reconcile their changes, find the decisions that matter, and make a first contribution. Start by exercising that journey across Matthew's own devices. Carry what works into onboarding another developer, where consent, standing, independent review, and the reach of shared memory become explicit.

## What the laptop exposed

- The legacy Brit executable reported version `0.52.0`, which looked newer than the intended Brit `0.3.0`. EPR builds from different commits both reported `0.3.3`. Source commit, release CID, and executable location resolved the ambiguity.
- Building EPR from `21a8060` took 9½ minutes. Installing through the native launcher, declaring durable home, and restarting the daemon brought the CLI and daemon to the same release.
- The laptop could serve with local enrollment while its siblings still lacked the shared roster row. At the end of the run, neither sibling had exchanged a verified head. The local feedback store also needed a separate format migration, after which Brit recorded feedback successfully.

## First pilot — one steward, two devices

1. **Finish admission.** The integrator lands the existing laptop enrollment from `c6a5f57f` through the normal `dev` gate. One of Matthew's peers advances the checkout its daemon serves and allows the daemon to reload the roster. Run `epr peer status --json` on both devices. Require fresh verified heads in both directions within the configured freshness window, with the sibling's `acked` reaching the laptop's starting `ownSeq`. Restart the laptop daemon and repeat the exchange with the same device key.
2. **Remove the build from the usual first install.** Follow arrival backlog ruling R5. Supply the matching `epr` and `bin-launch` as a forge release asset and verify the pair against a steward-authenticated pin before execution. Reuse the native store and launcher. Show the executable path, source commit, and release CID, and keep a local build available for unsupported hosts. After admission, exercise a routine update through sibling artifact reuse. Keep feedback migration explicit and local, after legacy writers have stopped.
3. **Exercise a real hand-over.** Start one small task on one device and resume it on the other. Use the existing `self/*` hand-over, arrival status, and governed recall to recover the intended outcome, current base, working changes the author chose to share, prior decision, evidence, and next action. Exercise the offline case too. The developer can continue local work, and status names the pending publication, peer exchange, or other action and who can take it. A sibling's result can save work under the same steward's authority.

Record time to local readiness, time to first verified exchange, and the manual steps the hand-over required against `workspace-device-arrival`. Repeat with a legacy Brit on PATH, an unpublished roster row, and a published row with the sibling offline. A supported first install should reach local readiness without Cargo.

## A public library as the front door to an open value network

Think of a public library on a town square. The square helps people find the places that welcome them. Inside the library, anyone can read the material on the public shelves, orient themselves, and ask a question at the reference desk. They can leave with something useful without joining. A workshop or reading room has a stated purpose, a host, and terms for taking part. The library gives us the civic welcome of the square, the human scale of a front porch, and a way to ask for help without spending the visit in a waiting room.

The library is where you enter an **open value network**. The makerspace lets people build, repair, and learn together. The coworking space lets someone find collaborators and join useful work already under way. The community resource center helps people find tools, context, and a person to ask. The mutual-aid hub makes it possible to ask for help and offer what you can. In developer terms, someone can bring a problem, offer a code review, run a build, or contribute a guide that makes the next person's arrival easier.

Openness includes discovery, participation, inclusion, and evolution. People can see what is available, find a way to take part, and propose changes to the terms as needs change. Peers negotiate what they share, what they can carry, and who can make decisions in each space. A person can contribute in one space, visit another, and decline an invitation while keeping their existing relationships and work.

Ostrom's principles apply to the capacity of these spaces as well as to what they share. Many people can read the same published guide. Hosting, relaying, answering requests, and reviewing contributions consume time and capacity. Let the people who bear those costs set visible boundaries on what their space offers and how much it can carry. They should be able to see demand, response load, and available capacity at the appropriate reach, and revise the rules together. Keep ordinary reading open wherever the material already permits it. A request for scarce help can be narrowed, deferred, or declined.

Each space should be able to govern its own reception within the wider commons' rules. Start with a notice when someone oversteps, use graduated responses for repeated misuse, and offer a nearby way to resolve disputes or appeal. Start with the smallest space able to answer the request. Wider spaces can help route a visitor and recognize locally made rules. Measure whether the people receiving requests can sustain the work without turning public discovery into surveillance. These are questions to test in the pilots, informed by [Ostrom's lecture on polycentric governance](https://www.nobelprize.org/uploads/2018/06/ostrom_lecture.pdf).

## The shelves hold collective intelligence

The public shelves hold what the network has chosen to make widely useful. Special collections inside the library hold the decisions, evidence, and practices of a particular working group or community. Their value comes from the context that goes with them and the people who keep that context current. A newcomer can start with a public guide, then ask for access to a relevant collection when they need more depth. Discovery shows a collection's existence only where the space has chosen to advertise it.

Native agentic memory gives knowledge a path from private work to the collections where it can help. An agent can help a developer extract a lesson from private work and draft a contribution for the immediate collaborators. The developer chooses what to offer. The receiving group reviews it alongside what it already knows, retains relevant disagreement, and decides what belongs in its collection. A lesson can later reach another group or the public shelves after renewed review for that audience. Each step carries the purpose, provenance, and evidence that audience may see. Private source material stays with its holder, and a useful lesson may stay in a special collection indefinitely.

Agentic stewards can help people move through the knowledge they already have access to. They can point a reader to the applicable decision, flag a stale guide or competing account, and prepare a proposed revision or wider contribution. People acting under the space's rules decide admission, acceptance, and wider release. Exercise one lesson from private work through a shared collection to a public guide. Verify that a reader can recover why it was accepted, find an authorized correction, and learn what they can use without exposing the private work or a restricted collection.

## Discover in low-trust channels and continue by consent

Start with the developer's immediate need. For example, the laptop needs a steward peer who can confirm roster pickup. A device resuming a task might need a copy of the prior gate result. A new contributor might need the decision behind a change or someone authorized to review it. Surface relevant offers, roles, and published context within the developer's current reach. Show who made each offer, what it covers, and whether it is current. A visible peer is evidence of presence. An offer or declared role gives the developer a reason to ask.

Use a low-trust discovery channel for what people or spaces deliberately advertise at that reach. A newcomer can see what a space works on, the help it offers, and where to send a first request. Show the source and freshness of those claims. Keep private rosters, device details, work in progress, and restricted memory outside that view.

Let the developer review one scoped request before sending it to a chosen recipient. Say what is needed, why, what evidence the recipient can see, and when a reply would still help. Sending reserves no capacity and creates no obligation. The recipient can accept within their authority, decline, or let the request expire without giving a reason. Show the requester what happened and what they can do locally while waiting. Keep a pending request visible across reconnects and send it only once. A decline or timeout ends the request unless the developer chooses to ask again.

Move the next step into a more trusted channel when the people involved consent and the existing admission or grant covers it. Carry forward the purpose and any references the requester chose to share. Show which space the developer is joining, what becomes visible, and which actions their standing permits. A first request can open a conversation. Device coordination stays inside the steward's admitted channel, and collective review and memory follow the receiving space's rules.

In the two-device pilot, discover an offer for one gate result or artifact and ask the sibling for it. Exercise acceptance, refusal, and an offline sibling. In the next pilot, start with a newcomer who can discover a space and ask for context or review. Verify that restricted context stays hidden while the request is pending or declined, and that approval opens only the scope granted. Carry automated requests and offers across principals only when the existing admission and reach work is ready.

## Next horizon — another developer joins

Walk one incoming developer through the same journey with their own identity and scoped standing. Start with one real contribution. They should be able to find the governing decisions and evidence, understand their role, reconcile local work with the current shared state, and submit a contribution with provenance. Show who can review and accept it and whether it is awaiting review or accepted. Independent review comes from another admitted principal under the space's rules. Devices belonging to one steward share that steward's standing.

**The long-term entry point is one click.** An invitation opens the relevant workspace and explains who invited the developer, what they can do now, and what requires consent or approval. It starts a resumable journey through the existing consent and admission paths. While approvals are pending, the first useful view offers context already within the developer's reach and shows how to ask for what is missing. Prove the journey through the two pilots before implementing the invitation link.
