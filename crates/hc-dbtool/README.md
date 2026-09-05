# hc-dbtool

See a Holochain 0.7 cell block, read the warrant (the accusation record) and the rejected op behind it, and lift it.

## Why this exists

Holochain 0.7's `integrate_dht_ops_workflow` blocks the **author's cell** from
`Timestamp::now()` to `Timestamp::max()` when one op that author wrote integrates
as invalid (`CellBlockReason::InvalidOp`). The peer store then drops that agent's
`agent_info` records, and gossip with that agent never starts again.

0.7 ships no way back: no unblock admin call, no unblock HDK host function. One
rejected op per author is enough to partition a household **permanently**, and
from the outside it is indistinguishable from an unreachable peer — the log says
nothing, the peer's storage arc stays null and its completed gossip rounds stay at
zero (both are read from the conductor's `dump_network_metrics`, shown at the end
of "Lifting a block").

A household has to be able to see the refusal, read the evidence behind it, and
decide. That is the whole tool.

Two words the output uses: a **warrant** is the DHT record one peer publishes to
accuse an op of being invalid; the **warrantee** is the agent the warrant accuses.
A block cites the warrant. If the warrant itself passed validation the block lands
on the warrantee; if the warrant was rejected it lands on the warrant's own author
(mechanics under "What hash a block actually cites").

"The household mesh" below is this repository's local three-conductor development
network (`app/elohim-app/scripts/hc-mesh.sh`); the tool itself works on any 0.7
conductor's data directory.

**Prerequisites:** a built `hc-dbtool` binary (see "Building" — build first, then
come back; `hc-mesh.sh blocks` finds it for you on the household mesh), the peer's
`databases/` directory, its `db.key` passphrase, for the household-mesh path the
mesh script itself, and for the confirm step a `genesis/a2o` with its Node
dependencies installed (`pnpm install` once). Hashes written `uhC0k…` / `uhCAk…` in the examples are
truncated placeholders — paste the full hash the tool prints.

The sections "Verbs", "Lifting a block" and "Building" are how to use the tool.
Everything after them is how it works inside, for when the output needs interpreting.

## Verbs

Every command takes `--databases <dir>` — the peer's data root holding
`conductor.db`, `db.key` and one `dht-<dna>.db` per DNA (a real peer's is the
`data_root_path` in its `conductor-config.yaml`, `databases/` beneath it) — and
`--passphrase <text>` for `db.key` (default `test`, the household mesh's; a real peer
passes the lair passphrase piped to `holochain --piped` at launch, held by whoever
launched it — a unit file or secret store — and not recoverable from disk; without it
no verb can open the databases).

All reads open the database `SQLITE_OPEN_READ_ONLY`, so they are safe against a
running conductor:

```bash
DB=elohim/holochain/local-dev/james/databases   # substitute your own peer's directory

hc-dbtool --databases $DB apps
#   this conductor's own agent key per installed app, and each role's DNA hash
#   (the --dna value the next verb takes)

hc-dbtool --databases $DB blocks
#   every BlockSpan row, decoded to dna:agent + reason + interval, each joined
#   through the DNA's DHT database to the WARRANT it cites and to the rejected ops
#   of whichever party the block landed on (the warrantee when the warrant validated,
#   the warrant's author when it did not — the output says which); the dna:agent
#   pair is what `unblock --cell` takes

hc-dbtool --databases $DB rejected --dna uhC0k…
#   rejected ops in that DNA (ChainOp = integrated, LimboChainOp = still in
#   validation limbo), joined to their authors, plus the Warrant rows that
#   carry the accusation
```

## Lifting a block

`BlockSpan` is the **only** table this tool ever writes; source chains, `Action`,
`ChainOp`, `LimboChainOp`, `Warrant` and every other DHT row are read-only at every
code path. The one write verb, `unblock`, refuses while any live process holds
`conductor.db` open and refuses without `--yes` — run it first WITHOUT `--yes` to see
what would be deleted. `--cell '<dna>:*'` (quoted, so the shell keeps the `*`) lifts
every agent blocked in that DNA.

**On the household mesh.** Only the blocked peer's own conductor needs to be down;
`hc-mesh.sh stop` is the mesh's lifecycle script and stops all three, which is the
convenient way here (to stop just one, use the single-peer sequence below against
that peer's conductor).

```bash
DB=elohim/holochain/local-dev/james/databases        # this peer's databases/ (james = your peer's mesh name)
./app/elohim-app/scripts/hc-mesh.sh blocks james    # 1. see it — prints the dna:agent pair step 3 takes
./app/elohim-app/scripts/hc-mesh.sh stop            # 2. stop the conductors
hc-dbtool --databases $DB unblock --cell <dna>:<agent>          # 3a. preview: what would be deleted
hc-dbtool --databases $DB unblock --cell <dna>:<agent> --yes    # 3b. lift it
./app/elohim-app/scripts/hc-mesh.sh start           # 4. bring the mesh back
./app/elohim-app/scripts/hc-mesh.sh blocks james    # 5. confirm: no rows — then the check below
```

**On a single peer** (no mesh script), the same sequence:

```bash
DB=<your peer's data_root_path>/databases            # from its conductor-config.yaml
ARGS=$(ps -o args= -p "$(pgrep -f 'holochain --piped')")   # the running conductor's full argv —
[ -n "$ARGS" ] || echo "no running holochain found: take the argv from the unit file that launches it"
#   capture it NOW: step 4 relaunches with exactly this argv and step 2 kills the process
hc-dbtool --databases $DB --passphrase '<lair passphrase>' blocks                                  # 1. see it
PID=$(pgrep -f 'holochain --piped'); kill -INT "$PID"   # 2. stop THIS peer's conductor (SIGINT = graceful)
while kill -0 "$PID" 2>/dev/null; do sleep 1; done   #    …and wait until it has exited (unblock refuses while it holds the db)
hc-dbtool --databases $DB --passphrase '<lair passphrase>' unblock --cell <dna>:<agent>            # 3a. preview
hc-dbtool --databases $DB --passphrase '<lair passphrase>' unblock --cell <dna>:<agent> --yes      # 3b. lift it
nohup $ARGS <<< '<lair passphrase>' >> conductor.log 2>&1 &   # 4. start it again with the same argv, in the
                                                              #    background (foreground would hold this shell)
hc-dbtool --databases $DB --passphrase '<lair passphrase>' blocks                                  # 5. no rows — then the check below
```

**Confirm the peer rejoined** (both cases): the admin interface's `dump_network_metrics`
shows, per DNA, the local agent's `storage_arc` (null while blocked-out, a full range
once a gossip round has completed) and `peer_meta[*].completed_rounds` climbing. One
literal call, run from `genesis/a2o` because the `@holochain/client` dependency is
installed there (`ADMIN` = the peer's admin websocket port — household mesh: matthew
4444, jessica 4454, james 4464; elsewhere the `admin_interfaces` port in
`conductor-config.yaml`; `DNA` = the hash from `apps`):

```bash
cd genesis/a2o && ADMIN=4464 DNA=uhC0k… npx tsx -e '
import { AdminWebsocket, decodeHashFromBase64 } from "@holochain/client";
const a = await AdminWebsocket.connect({ url: new URL(`ws://127.0.0.1:${process.env.ADMIN}`), wsClientOptions: { origin: "elohim" } });
const m = await a.dumpNetworkMetrics({ dna_hash: decodeHashFromBase64(process.env.DNA), include_dht_summary: false });
const s = (typeof m === "string" ? JSON.parse(m) : m)[process.env.DNA];
console.log("storage_arc", s.local_agents[0].storage_arc, "completed_rounds", Object.values(s.gossip_state_summary.peer_meta).map(p => p.completed_rounds));
process.exit(0);'
```

Lifting is not a cure by itself: whatever wrote the rejected op (in the first
observed case, a `CapGrant` written after a chain close) re-earns the block on its
next write. Fix the writer first: `rejected --dna <hash>` names the op's type,
author, sequence and time, which is enough to find the client or service that
committed it.

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
