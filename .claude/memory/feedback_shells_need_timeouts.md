---
name: Shell commands need explicit timeouts
description: Long-running shell/cargo/build commands history of never ending; always set a timeout and kill on overrun
type: feedback
originSessionId: a343d895-dee4-491c-a4db-adda4c79312f
---
Long-running shell commands — especially `cargo test`, `cargo build`, `pnpm build`, justfile recipes wrapping any of those — have a history in this repo of getting stuck and never returning. The wedged sync-bench at 86 minutes silent (May 2026) is the canonical case but not the only one.

**Why:** Cargo serializes on its target/ lock; one wedged subprocess can hang the whole shell. Bench tests with no internal timeout (e.g. `read_frame_default` without `tokio::time::timeout` wrap) sit forever. Even non-bench tests can deadlock waiting for peer events.

**How to apply:**
- Always pass an explicit `timeout` to the Bash tool when running cargo/build/test commands. Default to **600000ms (10 min)** for builds + tests; **300000ms (5 min)** for known-fast commands like single test invocations.
- For `run_in_background`, attach a watchdog: schedule a wakeup at `timeout + 60s` to verify completion and kill the process tree if not done.
- When polling, never use `until ! pgrep`-style busy waits without a max-iteration cap.
- If a command exceeds its budget, kill the process tree (`kill <PID>; sleep 2; kill -9 <PID>`) — don't sit hoping it'll resolve.
- Before running a long bench, capture the expected duration from prior runs; flag if it's already taken 2-3× that and intervene rather than waiting.
