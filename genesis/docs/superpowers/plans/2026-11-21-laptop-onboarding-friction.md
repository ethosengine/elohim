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

## Next horizon — another developer joins

Walk one incoming developer through the same journey with their own identity and scoped standing. Start with one real contribution. They should be able to find the governing decisions and evidence, understand their role, reconcile local work with the current shared state, and submit a contribution with provenance. Show who can review and accept it and whether it is awaiting review or accepted. Independent review comes from another admitted principal under the space's rules. Devices belonging to one steward share that steward's standing.

Let developers move from personal working context into collective memory by deliberately offering what belongs in the shared space. Attach the purpose, evidence, and provenance so another person can review and reconcile it. Personal drafts, private feedback, and recall session records stay private. Test that a useful contribution can be shared without exposing the private work that led to it.

**The long-term entry point is one click.** An invitation opens the relevant workspace and explains who invited the developer, what they can do now, and what requires consent or approval. It starts a resumable journey through the existing consent and admission paths. While approvals are pending, the first useful view offers context already within the developer's reach and a next action. Prove the journey through the two pilots before implementing the invitation link.
