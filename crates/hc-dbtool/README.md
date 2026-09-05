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

Two words the output uses: a **warrant** is the DHT record one peer publishes to
accuse an op of being invalid; the **warrantee** is the agent the warrant accuses.
A block cites the warrant, and the party it lands on is the warrantee when the
warrant itself validated (details under "What hash a block actually cites").

The sections "Verbs" and "Building" are how to use the tool. Everything after
them is how it works inside, for when the output needs interpreting.

## Verbs

All reads open the database `SQLITE_OPEN_READ_ONLY`, so they are safe against a
running conductor. Every command takes `--databases <dir>` — the peer's data root
holding `conductor.db`, `db.key` and one `dht-<dna>.db` per DNA — and
`--passphrase <text>` for `db.key` (default `test`, the local household mesh's
passphrase; a real peer passes the lair passphrase its conductor was started with).
A real peer's data root is the `data_root_path` in its `conductor-config.yaml`
(`databases/` beneath it). `apps` lists each role's DNA hash — that is where a
`--dna` value comes from, and `blocks` prints the `dna:agent` pair `unblock` takes.

```bash
DB=elohim/holochain/local-dev/james/databases   # substitute your own peer's directory

hc-dbtool --databases $DB apps
#   this conductor's own agent key per installed app, and each role's DNA

hc-dbtool --databases $DB blocks
#   every BlockSpan row, decoded to dna:agent + reason + interval, each joined
#   through the DNA's DHT database to the WARRANT it cites and to the rejected
#   ops the warrantee authored

hc-dbtool --databases $DB rejected --dna uhC0k…
#   rejected ops in that DNA (ChainOp = integrated, LimboChainOp = still in
#   validation limbo), joined to their authors, plus the Warrant rows that
#   carry the accusation
```

The one write verb refuses while any live process holds `conductor.db` open, and
refuses without `--yes`:

```bash
./app/elohim-app/scripts/hc-mesh.sh blocks james    # 1. see it
./app/elohim-app/scripts/hc-mesh.sh stop            # 2. stop the conductors. hc-dbtool itself needs only the
                                                    #    blocked peer's own conductor.db closed; hc-mesh.sh is
                                                    #    the household mesh's lifecycle script and stops all
                                                    #    three — outside that mesh, stop your one conductor
hc-dbtool --databases $DB unblock --cell <dna>:<agent> --yes   # 3. lift it
./app/elohim-app/scripts/hc-mesh.sh start           # 4. bring the mesh back
./app/elohim-app/scripts/hc-mesh.sh blocks james    # 5. confirm: no rows; then watch the space's
                                                    #    gossip metrics — completed_rounds climbing and
                                                    #    storageArc no longer null means the peer rejoined
```

Lifting is not a cure by itself: whatever wrote the rejected op (in the first
observed case, a `CapGrant` written after a chain close) re-earns the block on its
next write. Fix the writer first. Quote the `*` in `--cell <dna>:'*'` so the shell
does not expand it.

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
