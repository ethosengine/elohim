---
id: artifact-store-f2r-runbook
status: active
habits: [sprint-lands-each-closure-once]
---

# F2r runbook: the 454 MB sibling fetch, shem ⇄ ethosengine

This is the receipt sprint 2 owes (RESULT.md, owed item 1). One device builds the mesh storage
binary and records it (a `Produce`). The other runs the same build, and the build resolves the
record and fetches the binary over the tier instead of compiling it (a `Use`). The figures go into
`genesis/orchestrator/.epr-meta/sprint-lands-each-closure-once.habit.md` as a DELTA.

## Preconditions (both devices)

- An `epr` carrying the F2 verbs (`epr artifact describe|key|fetch|put`). On shem:
  `/projects/.claude-config/tools/bin-store/epr/bafyreiexj56thxwc46nc24ulxjfsdcij6q67nmydcdjevmk6iyrzcy3zki/epr`
  (source 86e1395e0, clean, local gate). Check with `"$EPR_BIN" artifact` (it prints its usage).
- `epr peer serve` running on that `epr`, so the tier serves the artifact store (E3). Restart it
  with the devfile env, `EPR_PEER_RELAY_URL` included, or the sibling goes silent.
- `epr peer status` on each side hears the other (`ageS < 90`).
- Both checkouts at the **same dev commit**. The key is the closure at HEAD, so any selected-source
  change between the two builds is a miss, by design.
- The household mesh is down on the building device. The build uses the gate's pool slot and jobs=1.

## Run

On the producing device, from a worktree at dev:

```bash
export EPR_BIN=<the F2-capable epr>
MESH_BUILD_DRY_RUN=1 just mesh build storage    # read the fetch/put lines first
/usr/bin/time -v just mesh build storage 2>&1 | tee genesis/docs/superpowers/sprints/2026-11-07-artifact-store/evidence/f2r-produce-<device>.log
```

Expect a fetch miss (no record yet), then a cargo build, then
`[mesh build] … artifact put … --cross` printing the record. Note the wall-clock, the binary's
bytes (`stat -c %s`) and its byte CID.

On the fetching device, at the same commit:

```bash
export EPR_BIN=<the F2-capable epr>
/usr/bin/time -v just mesh build storage 2>&1 | tee genesis/docs/superpowers/sprints/2026-11-07-artifact-store/evidence/f2r-fetch-<device>.log
```

Expect `[mesh build] ok … (fetched from a sibling)`. Record the wall-clock and bytes moved, and run
`b3sum` on both sides; they must be byte-identical. Any gap (no record, key mismatch, daemon down,
refused, deadline, integrity, transport) prints a typed reason and builds locally. That is a
result to record, not a failure of the lane.

Before the fetch, compare the two devices' `tool:` lines in the method JSON
(`<pool>/<slot>/mesh-bin/.method-elohim-storage.json`). The key carries the resolved cc/ld paths and
versions, so different toolchains are a legitimate miss.

Then repeat in the reverse direction for `doorway`, and run a second successive storage build to
measure byte-weighted reuse.

## What to record (the DELTA)

Fetch vs rebuild wall-clock and bytes; chunk count; whether the binary was byte-identical; the
per-transport result (iroh measured; libp2p and dual are typed gaps); and the key's basis. The key
is still `basis-key:git-tree`, the interim; Track A's projected view is the F1r frontier, so this
receipt does not close F2.
