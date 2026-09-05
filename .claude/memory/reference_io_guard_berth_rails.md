---
name: reference_io_guard_berth_rails
title: io-guard and berth rails
description: io-guard sheds compile trees past a disk-WRITE budget; berth is the claim registry (claim cargo|mesh before heavy work) — bites before any storage build or mesh start
metadata: 
  node_type: memory
  title: io-guard write budget + berth session claims
  type: reference
  originSessionId: 08dda108-5eac-4580-8178-d1bade78f0ab
  modified: 2026-09-03T16:18:44.658Z
---

Two operator-requested rails landed on local dev 2026-09-03 (elohim-0d), in `genesis/agentic/bin/`:

- **`io-guard`** — continuous disk-WRITE budget over 30 s windows: soft 60 / high 100 / hard 160 MB/s.
  high = SIGSTOP the newest compile tree; hard = kill tier-1 then pause seeders; critical processes never
  touched; auto-resume when quiet. Born because a 0.7 mesh + prologue seed + a storage build drove the
  shared NVMe to 130–270 MB/s and swapped the pod (see [[project_devspace_recovery]]). A `cargo test`
  alone was measured at 215 MB/s (HARD) — a storage-crate test run is itself a write storm.
- **`berth`** — the session/claim registry io-guard reads: `berth moor --session <id> --model <model-id>
  --lab <vendor> --runtime claude-code --principal <human> --task '<task>' --writes <paths>`; then
  `berth claim cargo` before a heavy build and `berth claim mesh` before taking the household mesh (a live
  holder is refused by name); `berth release`, `berth say`, `berth status|ledger`.

**How to apply:** before any heavy cargo run or mesh start, moor + claim; never rely on peer-session chat
("mesh taken"/"mesh free") alone — that is how two incarnations of one session both drove station 3b on
2026-09-03. Verify the binaries still exist before recommending (`ls genesis/agentic/bin/io-guard berth`).
See [[project_tevah_compute_envelope_canonized]] (mesh-run write budget), [[project_cargo_pvc_disk_discipline]].
