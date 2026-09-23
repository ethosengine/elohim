//! `hc-dbtool` — see a Holochain 0.7 conductor's blocks, read the rejected ops
//! that caused them, and lift them.
//!
//! Holochain 0.7's `integrate_dht_ops_workflow`
//! (`holochain-0.7.0/src/core/workflow/integrate_dht_ops_workflow.rs`) blocks the
//! *author's cell* from `Timestamp::now()` to `Timestamp::max()` when an op that
//! author wrote integrates as invalid. The peer store then drops that agent's
//! infos and gossip with them never starts again. 0.7 ships no way back: there is
//! no unblock admin call and no unblock host function, so a single rejected op
//! per author partitions a household permanently.
//!
//! This crate gives the operator seat the moves it needs:
//!
//! * `apps` — this conductor's own agent key per installed app, and each role's
//!   DNA. Without it a block row cannot be read as "I refuse X" rather than
//!   "X is me".
//! * `blocks` — decode every `BlockSpan` row in `conductor.db`.
//! * `rejected --dna <hash>` — the rejected ops and warrants in that DNA's DHT
//!   database, joined to their authors.
//! * `unblock --cell <dna>:<agent> --yes` — delete the matching `BlockSpan` rows,
//!   and nothing else.
//!
//! Deliberate limits, so the tool can never be the thing that corrupts a node:
//!
//! * Reads open the database `SQLITE_OPEN_READ_ONLY`.
//! * The only table ever written is `BlockSpan`. Source chains, `Action`,
//!   `ChainOp`, `LimboChainOp`, `Warrant` and every DHT row are read-only to this
//!   tool at every code path.
//! * `unblock` refuses while a live process holds the database open, and refuses
//!   without `--yes`.

pub mod apps;
pub mod blocks;
pub mod db;
pub mod fmt;
pub mod key;
pub mod rejected;
