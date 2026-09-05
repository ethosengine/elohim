# hc-dbtool

See a Holochain 0.7 cell block, read the warrant and the rejected op behind it, and lift it.

## Why this exists

Holochain 0.7's `integrate_dht_ops_workflow` blocks the **author's cell** from
`Timestamp::now()` to `Timestamp::max()` when one op that author wrote integrates
as invalid (`CellBlockReason::InvalidOp`). The peer store then drops that agent's
infos, and gossip with them never starts again.

0.7 ships no way back: no unblock admin call, no unblock HDK host function. One
rejected op per author is enough to partition a household **permanently**, and
from the outside it is indistinguishable from an unreachable peer — the log says
nothing, the arc stays null, and the round count stays at zero.

A household has to be able to see the refusal, read the evidence behind it, and
decide. That is the whole tool.

## Verbs

All reads open the database `SQLITE_OPEN_READ_ONLY`, so they are safe against a
running conductor.

```bash
DB=elohim/holochain/local-dev/james/databases

hc-dbtool --databases $DB apps
#   this conductor's own agent key per installed app, and each role's DNA

hc-dbtool --databases $DB blocks
#   every BlockSpan row, decoded to dna:agent + reason + interval, each joined
#   through the DNA's DHT database to the WARRANT it cites and to the rejected
#   ops the warrantee authored

hc-dbtool --databases $DB rejected --dna uhC0k…
#   rejected ops in that DNA (ChainOp + LimboChainOp), joined to their
#   authors, plus the Warrant rows that carry the accusation
```

The one write verb refuses while any live process holds `conductor.db` open, and
refuses without `--yes`:

```bash
./app/elohim-app/scripts/hc-mesh.sh blocks james    # 1. see it
./app/elohim-app/scripts/hc-mesh.sh stop            # 2. stop the conductors
hc-dbtool --databases $DB unblock --cell <dna>:<agent> --yes   # 3. lift it
./app/elohim-app/scripts/hc-mesh.sh start           # 4. bring the mesh back
```

`--cell <dna>:*` lifts every agent blocked in that DNA. Omitting `--yes` prints
what would be deleted and changes nothing.

`BlockSpan` is the **only** table this tool ever writes. Source chains, `Action`,
`ChainOp`, `LimboChainOp`, `Warrant` and every other DHT row are read-only at
every code path.

## Building

The `crates/` siblings are each their own workspace root, so this crate has its
own pool slot:

```bash
cd crates/hc-dbtool
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/crates/dev \
  RUSTFLAGS="" cargo build
```

`hc-mesh.sh blocks` picks the binary up from that slot automatically
(`DBTOOL_BIN` overrides it).

## How it reads an encrypted database

`db.key` is base64url-nopad of `nonce[24] || secretbox(key[32])[48] || salt[16]`;
the cipher key is recovered with argon2id over the passphrase and that salt. The
database is then opened with SQLCipher 4 in holochain's exact configuration —
`cipher_salt` supplied out-of-band with `cipher_plaintext_header_size = 32`.

`src/key.rs` is a faithful port of `holochain_data 0.7.0`'s `DbKey`, not a call
into it: `apply_pragmas`, `key_hex` and `salt_hex` are `pub(crate)` upstream and
cannot be reached from outside the crate at all, and `DbKey::load` alone would
drag in `holochain_types`, `holochain_conductor_api`, `sqlx` and a tokio runtime
for forty lines of libsodium. The port is pinned from both sides: a
generate-then-load round trip in `key.rs`, and the live conductor databases that
wrote the real files.

## What hash a block actually cites

`CellBlockReason::InvalidOp` carries the **warrant op's** hash, not the hash of
the rejected chain op that provoked it.
`integrate_dht_ops_workflow.rs:46-65` builds the block from a warrant op's
summary — it pushes `(target, s.op_hash)` where `s` is the *warrant* op, and
which party gets blocked depends on the warrant's own validation status: an
accepted warrant blocks the **warrantee**, a rejected one blocks the **warrant's
author**.

So the cited hash is found under `WARRANTS`, never under `REJECTED OPS`. `blocks`
does that hop for you — it opens the DNA's DHT database, resolves the warrant,
and prints the warrantee, the cause, and the rejected ops that warrantee
authored, so one screen answers "who is refused, by whom, and for what".

## Hash rendering

The DHT tables store the **36-byte core** of a hash (32-byte digest + 4-byte DHT
location) without its 3-byte type prefix — the prefix is implied by the column. A
`BlockSpan` payload, by contrast, decodes to a typed `HoloHash` carrying all 39
bytes. `fmt::hash_b64_kind` re-attaches the column's prefix so both render as the
same `uhCAk…` / `uhCQk…` string the conductor logs use; without it a block cannot
be grepped back to the warrant it cites.
