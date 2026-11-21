

### 18:35Z: the storage gate beside a test loop, and why its total proves nothing

`just gate elohim-storage` ran from 17:02:56Z beside the eprfs test loop and ended green after
5,534 s, against 3,535 s alone on the morning's landing. The two are not comparable: the morning
run compiled 43 crates and the afternoon run 803, because its test build was cold (547 crates
against 1). One phase is like for like, the iroh lint (342 and 345 crates): 15m33s alone, 15m45s
beside a three-thread test run. Memory peaked at 37% of the limit with both running. That is the
whole of what the steward's "two, then measure" has so far; a like-for-like total waits for the
first landing that carries the storage gate.

### 16:22Z to 20:12Z: the thread count is bounded by the I/O guard

Seventeen runs of the eprfs workspace tests at four, three and two threads (arrival backlog row
53, delta, has the table). Four threads: five clean of five, 604 to 656 s against about 1,620 s
at one, the guard pausing three. Three threads: eight clean of ten, the guard pausing four and
two of those failing on a timeout shorter than the 36 s freeze. Two threads: one clean of two;
the failure was a timing assumption in a test (row 66), diagnosed by Codex astra as a test
defect and fixed on a branch. No thread change landed. The cause of the writes is row 65, and an
Opus agent is cutting them in the test file.

### 18:53Z: the carrier contract is on dev, in shadow

`2a9816390..e83d0931b`, 974 s: address-profile-lint 21 s, orchestrator 93 s, dialogue-stasis
125 s, orchestrator-tree 422 s, agentic-shift 252 s, eprfs-live-corpus 61 s. orchestrator-tree
was about 250 s on the day's earlier landings; a test loop ran beside this one and the landing
adds hook tests, and which of the two the extra 170 s is has not been measured. On shem
afterwards `berth room cargo` answered admit (two seats, one build running, both guards ok) and
`gate-carry.mjs plan` offered `elohim-storage` to `second-seat` and kept eprfs, steward-node and
orchestrator inline with a reason each.

### 19:05Z: the sibling device

The steward asked where the epic stands (3 of 15 stations ticked in the peers plan; about six of
its sixteen concerns working between his own devices) and offered the idle ethosengine device.
The brief handed to him for it: first a second-device run of the day's gate-plane landings from
a fresh worktree, then the input-class change in rakia and brit that takes pipeline files out of
device gate keys natively (peers plan §9, "Interfaces", fourth row), stopping before any
publish, release or pin move.
