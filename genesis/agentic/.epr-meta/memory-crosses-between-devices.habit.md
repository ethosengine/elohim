---
epr-habit-version: 1
id: memory-crosses-between-devices
invariant: >-
  A correction a steward gives on one of their devices is in force on their other admitted
  devices before each one's next session starts, carried device to device and with no git pull.
  What crosses is the entry that was said, attributed to the device and session that wrote it;
  a session transcript, a recall receipt and anything declared `applies-to: device` stay where
  they were made. An entry that crosses folds on the sibling as that device's entry, and merging
  it with a local one is a consolidation someone judges.
status: unwired
active: false
checks: []
first_move: >-
  Run the P2P design gate on one entity, the room: a shared session whose roster is its
  participants, with an entry crossing between one person's two devices as the one-person case.
  Its answers go in the collective-memory seed. Only then is the crossing kind added to
  `berth.channel.crosses`, where a memory kind is one row on a default-deny list. The
  precondition is in place from the same change that declared this habit: an entry says what it
  applies to (`applies-to: device | steward | repository`), so the channel can tell what may
  cross from what may not.
refs:
  - genesis/research/memorilla-claude-dreaming-cross-pollination-2026-11-18.md
  - genesis/data/timeline/backlog/native-peer-dev-channel-and-collective-memory-seed.md
  - genesis/docs/superpowers/specs/2026-10-23-workspaces-cooperate-as-devices-design.md
  - genesis/a2o/features/devflow/collective-memory.feature
  - genesis/agentic/.epr-meta/devices-share-one-berth.habit.md
  - genesis/agentic/pool-policy.json
retire-when: >-
  a memory entry is composed at a declared ring and carried by the same reach machinery as any
  other EPR, so that crossing between one person's devices is a consequence of its reach and
  needs no rule of its own. Until then this habit is what says the steward's direction follows
  the steward and not the repository.
---

DELTA 2026-11-18 (born UNWIRED; declared on the steward's approval of the "Memory: first rung" plan, step 3): nothing carries memory between devices today. Entries travel by git push to the forge, which is wider and slower than the steward channel, and the channel's crossing list (`berth.channel.crosses`) names no memory kind. What exists to build on: the channel itself, with roster admission, a default-deny crossing list, folding a sibling's records as the sibling's and an acknowledgement vector (habit `devices-share-one-berth`, green). What landed with this declaration: entries declare `applies-to`, required at birth by `memory-frontmatter-at-birth@2`, read by import, and shown in the projected index as whose each entry is. No check exists, so the status is unwired and may not be flipped on intention. The first check is the two-device observation the invariant states.
