# F2r producer half: shem, 2026-11-09

`just mesh build storage` from `.worktrees/as-seeder` at dev 6a82e3c44, household down,
`EPR_BIN=/projects/.claude-config/tools/bin-store/epr/bafyreiexj56thxwc46nc24ulxjfsdcij6q67nmydcdjevmk6iyrzcy3zki/epr`
(source 86e1395e0, clean, local gate). Peer daemon restarted on the same epr at 05:11Z.

| | |
|---|---|
| start → end | 05:12:41Z → 05:24:50Z |
| wall clock (`/usr/bin/time -v`) | 12:09.24 |
| peak RSS | 7,557,700 KB |
| fetch first | `fetch gap: no-record: no admitted sibling device has recorded a build for this key; build here` |
| output (byte CID) | `bafkr4iex6zxq5uy6lpbbuudkvn7a72tmgnvib26e6bpmb4male772toz54` |
| manifest | `bafyreiehhz6ytzyerq4qkhrxzlxbwszm7cvy2dbvrzovpoybj7pqkauajq` |
| key | `bafyreibz2qaq4oux44wbsgagykjpfxtfhewuog3m2niycjk5fpns4mn5v4` (record crosses) |
| bytes | 468,343,856 |
| installed at | `/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/mesh-bin/elohim-storage` |
| crossing | ethosengine acks shem's head to seq 45363 right after the put |
| toolchain in the key | cargo 1.96.1 (356927216 2026-06-26); rustc 1.96.1 commit 31fca3adb283…, host x86_64-unknown-linux-gnu; CC=gcc |

12 min is probably not the cold cost: the pool slot is the gate's, and the log shows 36 crates
compiled, so most dependencies were already built. A cold storage build here has measured 77 min. The fetching half compares against its own local rebuild time, not this one. Raw log
`f2r-produce-shem.log` (gitignored, kept on shem).

**The fetching half (ethosengine)** is at the same dev commit with an F2-capable epr and its peer
daemon on it: `just mesh build storage` should print `(fetched from a sibling)`. Then compare the
binary byte for byte against the output CID above.
