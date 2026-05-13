---
name: Check existing compute-reporting foundation before adding probes
description: elohim-storage already has fs4 + heartbeat::measure_free_pct + cluster.rs Member.capacity_bytes + custodians total_capacity_bytes; new probes synthesize these, never duplicate
type: feedback
originSessionId: 72a4534a-dd50-4984-be17-9d287ef54e6b
---
Before adding new compute/storage/memory reporting primitives to elohim-storage, grep for the existing foundation. Failure to do this in M1 Task 2 of topology-substrate-completion meant the implementer wrote raw `libc::statvfs` while the codebase standard is `fs4`.

**Why:** The codebase has multiple compute-reporting surfaces accumulated over prior sprints. Reinventing them produces parallel implementations that drift and confuse readers about which is canonical. Synthesizing — building on top of — keeps the foundation unified.

**How to apply:**
- Before writing any filesystem capacity probe, check `heartbeat::measure_free_pct` and the `fs4` import (cross-platform statvfs/GetDiskFreeSpaceEx wrapper, already in deps).
- Before writing storage-capacity types, check `cluster.rs:Member.capacity_bytes` and `views.rs:total_capacity_bytes` (custodians API).
- Before writing process metrics, check whether the heartbeat's `LiveProbe`/`LiveState` already covers the concern.
- Useful greps: `fs4::`, `capacity_bytes`, `statvfs`, `measure_free_pct`, `LiveProbe`.
- New utility module = fine if it **synthesizes** existing primitives (e.g., calls `fs4::total_space` + adds memory/CPU probes).
- New utility module that **duplicates** primitives (e.g., raw `libc::statvfs` when fs4 covers it) = regression. Roll it back.

The user's framing: "complementary concerns handled elegantly and efficiently is perfectly fine; duplication is not." Apply that test.
