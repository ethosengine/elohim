---
id: artifact-store-f2r-fetch-ethosengine
status: active
habits: [sprint-lands-each-closure-once]
---

# F2r fetching half: ethosengine, 2026-11-09 (first attempt: typed gap, not run to completion)

`just mesh build storage` from `.worktrees/f2r-fetch` at dev c4c09bab4 (the view is byte-identical
to shem's 6a82e3c44: `git diff 6a82e3c44 c4c09bab4 -- <every method view path>` is empty),
`EPR_BIN=/projects/.claude-config/tools/bin-store/epr/bafyreid2oejfnqkwvg6utmwm5xsopzlki43k6gk4cpvnwkdalaaxe2lvk4/epr`
(source 7fe827e28, clean, local gate; no artifact-code change since shem's 86e1395e0).

| | |
|---|---|
| fetch first | `fetch gap: no-record: no admitted sibling device has recorded a build for this key; build here` |
| local build | stopped by the operator's session right after the gap (not a fetch measurement) |
| key here | `bafyreif23t6xkf7tdooopq5u2yg435vlmenmacaproleurx5p6twbkycvm` |
| method here | `bafyreigj7dasqhq7tf4de6uvuvsd2kqnsejtifzz2baytl3e4m33qkollm` |
| input here | `bafkr4ife6jsgaeaf4jivtqluc3prcby2lpyzjpunjfrw7hbtmiufajf6xq` (basis-key:git-tree, dirty []) |
| key on shem | `bafyreibz2qaq4oux44wbsgagykjpfxtfhewuog3m2niycjk5fpns4mn5v4` |
| tools here | cargo 1.96.1 (356927216 2026-06-26); rustc 1.96.1 31fca3adb283; CC=gcc (GCC 14.3.1); PATH /opt/rust/cargo/bin:/usr/local/bin:/usr/bin:/bin; LIBCLANG_PATH /usr/lib64 |

Two independent findings, both of which keep the fetch from resolving:

1. **The record had not reached this device's flow store as of 09:30Z.** `epr peer artifacts --key <shem's key>
   --root /projects/elohim` answers "no sibling holds an artifact", and `.eprfs/status/flows.jsonl`
   holds no `produce` event from shem's device (did:key:z6Mkm5de…). This device's peer daemon runs
   the landed epr `bafyreibzey…` (source cbc56bd7c, 2026-11-04), which predates F2: it folds
   shem's heads (it acked seq 45363) but does not project `artifact-produce@1` into the flow store.
   The runbook precondition "`epr peer serve` running on that `epr`" is unmet here. Restarting the
   device daemon on an F2-capable epr is an operator-level act on this device (it interrupts the
   live cross-device channel); it was not done in this session.
2. **The keys differ** with an identical view. Initialising `elohim/rakia` (the one submodule path
   in the view) does not change the key, and the input is git-tree based, so the difference is
   expected in the method (a tool string or a non-path input). Shem was asked for its
   method/input CIDs to localise the slot; open.

Cure order: land an F2-capable epr as this device's current and restart `epr peer serve` on it,
confirm `epr peer artifacts --key bafyreibz2qaq…` lists shem's record, then compare method CIDs.
B3 (branch `artifact-store/b3-closure-staleness`) writes the `.derivation-<bin>.json` projection
that makes the method/input split visible on every later build.
