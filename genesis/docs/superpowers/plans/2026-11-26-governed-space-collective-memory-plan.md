---
title: Governed space and collective memory — the stations
id: governed-space-collective-memory-plan
status: active
class: process-meta
process_subdomain: memory
date: 2026-11-26
steward: human:matthew
author: "Claude (session 96b46344, Fable 5.2), at the steward's direction; plan approved by the steward 2026-11-26"
serves: [acts-attributed-to-participants, memory-tended-once, devices-share-one-berth, workspace-device-arrival, agents-arrive-briefed]
refines: native-dev-tooling-over-peers-plan
cites:
  - "governed-space-share-boundary-collective-memory-design | the design whose seven prose stations this plan restates as claimable ones; its section 15 postures are not the steward's rulings | sha256:e684da3e7abc4153 | path: genesis/docs/superpowers/specs/2026-11-26-governed-space-share-boundary-collective-memory-design.md"
  - "native-dev-tooling-over-peers-plan | the epic's plan this is a child of; it carries CD-no-fingerprint, Crossing-drop-seen, Daemon-policy-current, concern rows 13 and 14 and the F6 stations that this plan points at and does not restate | sha256:585dc2fb9260d496 | path: genesis/docs/superpowers/plans/2026-10-29-native-dev-tooling-over-peers-plan.md"
  - "host-compatibility-foundation-plan | carries station K11, a method on the artifact path that is not a build, which GS-5 needs | sha256:2ca85fdeda855326 | path: genesis/docs/superpowers/plans/2026-11-10-host-compatibility-foundation-plan.md"
  - "artifact-store-task-e4-report | names the follow-on E4b, the payload store's current format by default, which no later plan carried | sha256:f107b996f90a7ea0 | path: genesis/docs/superpowers/sprints/2026-11-07-artifact-store/task-e4-report.md"
  - genesis/data/timeline/backlog/steward-device-arrival-open-items.md
  - genesis/data/timeline/backlog/arch-native-dev-plane-backlog.md
  - genesis/data/timeline/backlog/native-peer-dev-channel-and-collective-memory-seed.md
---

# Governed space and collective memory — the stations

The governed-space design (`genesis/docs/superpowers/specs/2026-11-26-governed-space-share-boundary-collective-memory-design.md`)
describes its work as seven stations in prose. `epr flow project` mints from `- [ ]` stations in
documents under `plans/` and `specs/` and reads no prose, so nothing in that design was claimable.
This file restates its stations as claimable ones, in the form the epic's plan uses, and adds
nothing to the design.

It is a child of [the native developer tooling plan](epr:native-dev-tooling-over-peers-plan), the
epic's plan. The blob store, the artifact tier and the peer link that this work rides are that
epic's; this plan carries only what the memory design adds.

Two things a reader should know before claiming a station:

- The design is a Draft. Its section 15 gives a working posture on each open decision, and none of
  those postures is the steward's ruling. A station whose State line says it waits on a ruling is
  not claimable until that ruling exists.
- Several things the design needs are already promised by another plan. They are listed under
  "Carried elsewhere" and have no station here, because the epic's plan rules that a promise
  another plan carries is not minted twice.

Written on 2026-11-26 from the design at that date, in a worktree at dev `19bee2055`. Nothing in
it was re-measured; each State line repeats the design or a tracker it names.

Gap ids are positional. New stations are appended at the end; stations are ticked, not reordered
or removed. A tick is a claim that a line is met, and still awaits verification.

## Words used

- The steward's three devices are named shem, ethosengine and thinkpad. All three exist and exchange with one another today, so a Verify line that names three devices needs no new device.
- The **channel** is what the steward's devices carry for one another over the peer link. The **channel's list** is the list of record kinds it will carry, `berth.channel.crosses` in `genesis/agentic/pool-policy.json`; a kind not on it is not carried.
- A **head** is one device's signed statement of the tree it holds, announced every 30 seconds. To **fold** is to read a sibling's new records, check them and keep them.
- A **chain root** is the first device in a person's roster, from which their other devices were authorised. Two chain roots means two people.
- Station ids that do not begin with GS (CD-no-fingerprint, K11, F6 and the like) are stations in the other plans named beside them. They are named here so a reader can find them, and are explained there.

## Stations

- [ ] **GS-1 — the acts on a memory contribution (who contributed it, who reviewed it, what it supersedes) carry a measure and a kind, are carried between the steward's devices, and the memory index reads the acts carried from siblings** (serves acts-attributed-to-participants)

State: not built. The entry bytes reach every device by git; the acts stay in each device's own untracked flow store, so a second device reads every shared contribution as unattributed (269 of 269 on shem, read on 2026-11-26 with `epr flow memory project --index`). Memory acts carry no `measure:` slot, so the channel's list cannot name them. Extending that list is a change to `genesis/agentic/pool-policy.json`, a governance path: it refuses sibling reuse for the push that carries it. Design section 8 gives the steward's framing that these records graduate under his standing consent; the statement of that consent in his words is still owed (design section 15, decision 1). The first backfill has to be batched, because one fold reads at most 1024 records.

Verify: each act names one measure and one kind, and the channel's list names them. A contribution imported on one of the steward's devices reads as attributed on the other two with no git operation carrying the act. A verdict made on one device is honoured on the others as the same steward's verdict and adds nothing to a count of distinct people.

- [ ] **GS-1b — each memory contribution carries a scope marker that records where it sits (this device, the steward's space, the collective's space, personal, stale) and what its author said about sharing it** (serves agents-arrive-briefed)

State: not built. No note or contribution says anything about its own scope today. A draft reading of the 135 notes on one device proposed 57 for the collective, 36 for the steward, 23 for the device, 13 stale and 6 personal, and found that the existing `type` field predicts the scope for 66 of them (design section 15, decision 3). Open for whoever claims this: whether the marker is a field on the contribution or a slot on its act.

Verify: a new contribution cannot be recorded without a marker, the marker is shown wherever the contribution is listed, and a contribution marked for this device is not offered to a sibling.

- [ ] **GS-2 — a memory ceremony records a tending attestation keyed on what it read, and a sibling device accepts that attestation in place of its own ceremony or says why it did not** (serves memory-tended-once)

State: not built. No record exists that a ceremony ran over a given state of memory, so each device runs its own. The key the attestation needs is the one the `memory-ceremony` gate lacks today (`pool-policy.json` withholds its reuse for "no step fingerprint"); that key is the epic plan's station CD-no-fingerprint and has no station here. This station is the attestation record, its carriage between devices, and the acceptance rule of design section 9: the signer is another admitted device, the key matches what this device holds, a recorded red refuses, an item the attestation did not see refuses.

Verify: a ceremony run on one device is accepted on a second device holding the same memory, and the second device's ceremony does not run. With one item on the second device that the first did not hold, the second refuses and names that item. With both siblings silent, a device still tends.

- [ ] **GS-3 — the count that says a memory tending is due is derived from records every holder has, and an accepted tending attestation resets it** (serves memory-tended-once)

State: not built. The count is derived on each device from that device's own hook observations, and its reset is a local observation (`cleanup-pressure-reset@1`), so each device believes its own ceremony is still due. Depends on GS-2, and through it on the epic plan's station CD-no-fingerprint.

Verify: the `cleanup:` line of the session headline reads the same on three devices holding the same memory, before and after one of them tends.

- [ ] **GS-4 — a space's boundary node is addressed by its content, holds the steward's consent clause for the kinds of record it covers, and each device reports the boundary version it has applied against the newest it holds** (serves devices-share-one-berth)

State: not built. The control surface for memory is paths in one checkout (`.epr-meta/collective.json`, `.claude/memory/.epr-meta`, the ceremony package), and `.epr-meta` cascades by directory ancestry inside one checkout. Nothing lets a steward give standing consent for a kind of record; the nearest existing shape is the session covenant's `variesRule` clause. Until this station exists, the channel's list in `pool-policy.json` is edited by hand.

Verify: a change to the boundary made on one device is reported as pending on the others until they accept the tending that introduced it, and a boundary version that arrives without an accepted tending is not applied. The channel's list is produced from the boundary's clauses, and a hand edit to it is detected.

- [ ] **GS-5 — memory contributions are held in the device payload store and fetched between the steward's devices, the working files are a projection each device renders, and git stops carrying memory** (serves workspace-device-arrival)

State: not built. The design's posture is to hold items on the existing artifact path, which stores arbitrary bytes at the level beneath `epr artifact put`. That needs a method that is not a build (host-compatibility plan station K11) and the payload store in its current format by default (the open follow-on named E4b in the artifact-store sprint, now row 19 of the native dev plane backlog); neither has a station here. This station is the producer for a hand-contributed item, a fetch that does not assume a binary to install, the projection of working files from held items, and the retirement of git as the carrier. Open for whoever claims this: how a hand edit to a projected file becomes a new contribution, since today's hooks treat the file as the source. Depends on GS-1 to GS-4.

Verify: a memory ceremony run once on one of the steward's three devices is accepted by the other two without a re-run, and an entry contributed on one device appears attributed on the other two with no git operation carrying it. On a device whose checkout has no memory entries in its tracked tree, every entry is still rendered from the items that device holds, and a new entry made after the cut-over appears in no commit.

- [ ] **GS-5b — the peer link's serving scope classifies a space's root, refuses a root outside the asking device's boundary, and spaces that share the same holders are announced under one head** (serves devices-share-one-berth)

State: not built. The tier's tree verbs reach only a device's own channel projection; a root that is neither a channel root nor an indexed artifact is classified unknown and refused. Each device announces one head over one tree every 30 seconds, and that tree has no place for a space. Depends on GS-4, which is what gives a space a boundary to refuse against.

Verify: an admitted device asking for a space root outside its boundary is refused with a typed reason. After a second space is created for the same three devices, each device still announces one head per interval.

- [ ] **GS-6 — each act on a memory contribution is signed by the device that made it, and a receiving device checks that signature without trusting the device that relayed the record** (serves acts-attributed-to-participants)

State: not built; integrity-class. The epic's plan rules that a station of this class needs the steward's ruling and an adversarial review before it is switched on. Today attribution is the session's own report and a carried record is admitted by which device relayed it. That is accepted among one steward's devices and is not enough once a second person is involved. Depends on GS-1.

Verify: a record whose signature does not match the device it names is refused and counted on the receiving device, and the refusal says which of absent, unverifiable and refused it was.

- [ ] **GS-7 — a test runs peer daemons under two different chain roots in one container, each admitting only its own devices** (serves devices-share-one-berth)

State: not built. The peer link's tests start two daemons under one chain root. The harness takes the root as a parameter. Whether two roots run side by side is untested, and finding out is this station's work. This is the first step toward exercising a collective with the fixture steward Adam, who can be given a roster and a device of his own by the ordinary enrolment commands and a second device key.

Verify: two daemons under two chain roots run in one container, each folds its own siblings' heads and neither admits a device of the other root.

- [ ] **GS-8 — a collective's space admits the devices of its current members, and a new member's agent arrives briefed from that space alone** (serves agents-arrive-briefed)

State: not started. It waits on GS-6 and GS-7, and on what the epic's plan already carries for membership and standing at wider reach (its concern rows 13 and 14 and its F6 stations). The collective's own declaration is the members' to make: its charter today grants no network membership, so a gate that admits by membership has nothing to read. With the fixture steward the result is read as bootstrap and not as peer validation.

Verify: a fresh session for the fixture developer, admitted by membership and holding only the collective's space, can do a first task without being walked through it, and can fetch nothing from the steward's own space.

## Carried elsewhere, no station here

Each of these is something the design needs and another plan or tracker already promises.

- A key for the `memory-ceremony` gate, so its result can be judged for reuse: station CD-no-fingerprint in [the epic's plan](epr:native-dev-tooling-over-peers-plan), open. GS-2 is a consumer of it.
- The roster read from a published ref and not from a daemon's own checkout: row 77 of the `steward-device-arrival-open-items` backlog, open. The design adds the collective's case to it (delta 4 of that row).
- A status line when a device's loaded policy does not list a kind another device sent: stations Crossing-drop-seen and Daemon-policy-current in the epic's plan.
- A method on the artifact path that is not a mesh build: station K11 in [the host-compatibility plan](epr:host-compatibility-foundation-plan), open. GS-5 needs it to admit a hand-contributed item.
- Membership and standing at reach wider than one steward: concern rows 13 and 14 and the F6 stations in the epic's plan, not started.
- A ring on the contribution record: named as later work in section 6.2 of the workspace arrival spec (`elohim/lvi/docs/specs/2026-10-10-workspace-arrival-as-declared-device.md`). GS-1b's scope marker is the nearest thing here and does not replace it: the marker says where an entry sits, and a ring would say how far it may travel.

Concerns the design raised that no plan carried were added as backlog rows on 2026-11-26. Row 19 of the `arch-native-dev-plane-backlog` (the payload store's current format by default) is needed by GS-5. Row 23 there (acts made through a device later revoked) and row 16 of the `arch-confidentiality-plane-backlog` (encryption at rest) have to be answered before GS-8. Rows 20 to 22 (the bin-store beside the payload store, a record of who carries a root, a failure domain on each roster row) block no station here.
