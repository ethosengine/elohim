//! The post-close write fence (Holochain Evolution Epic MVP, Task 32).
//!
//! **A close is a sealing act. Nothing is written on a closed cell by anyone
//! afterwards — conductor-side capability grants included.**
//!
//! # The failure this module exists to make impossible
//!
//! `AdminWebsocket::authorize_signing_credentials` reads like a handshake and
//! is not one: it generates an ed25519 keypair locally and then calls
//! `grant_zome_call_capability`, which **commits a `CapGrant` entry on the
//! cell's source chain**. Storage did that on every (re)connect, for the role
//! cell and for the mishpat and imagodei cells besides.
//!
//! On 2026-09-05 the household mesh measured what that costs after a chain
//! close (Task 30, `hc-dbtool`, commit `6968baf1e`): matthew authored a
//! `CapGrant` `Create` at seq 1052 and jessica one at seq 732, both **after**
//! Station 8's `seal_close` had closed their node-registry v1 chains. Holochain
//! 0.7 lets the author's own conductor write it (a close is not self-enforcing
//! — Probes A/B), and then every neighbour validates it, warrants the author
//! with *"No more actions are allowed after a chain close"*, and turns the
//! accepted warrant into a `Timestamp::max()` cell block. Three block rows
//! landed inside a 30 ms window; 0.7 offers no unblock. The mesh partitioned
//! itself, permanently, over a credential handshake.
//!
//! Storage is the writer that restarts most often, so storage is where the
//! fence belongs.
//!
//! # The three rails
//!
//! 1. **Mint ONCE per cell, ever — then persist.** A `CapGrant` already on the
//!    chain keeps authorizing its assigned key across restarts, so the
//!    credentials it minted stay valid forever. [`ClosedChainFence::authorize`]
//!    reuses persisted credentials for a cell whenever they exist and only
//!    reaches the conductor on a genuine first mint. This alone removes the
//!    per-restart chain write that produced the household's blocks — including
//!    on a chain closed by somebody else, provided the mint happened before the
//!    close.
//! 2. **A closed cell gets NO mint, by name.** When no credentials are
//!    persisted and the cell is recorded closed, [`ClosedChainFence::authorize`]
//!    refuses with a message that names the cell, the close, and the
//!    consequence — instead of writing the action that would earn a permanent
//!    block. A refused role stays unconnected and its routes answer 503, which
//!    is a recoverable state; a block is not.
//! 3. **A closed cell serves READS only.** [`ClosedChainFence::refuse_zome_call`]
//!    refuses any zome function not named in [`CLOSED_CHAIN_READ_FNS`], because
//!    the cost of guessing wrong in that direction is a permanent block on
//!    every neighbour rather than a failing call. The allowlist mirrors the a2o
//!    harness rail (`genesis/a2o/steps/delivery/happ-lineage-migration.steps.ts`,
//!    `CLOSED_CHAIN_READ_FNS`) so the two seats cannot drift into different
//!    ideas of what a closed chain still answers.
//!
//! # Why the ledger is keyed by CELL and lives on disk
//!
//! [`crate::lineage_roles::LineageRoles`] holds `RoleLineage::closed` in
//! process memory and is rebuilt at base for every role on each boot. So the
//! in-memory answer to "is this role's reading cell closed?" is `false` on
//! exactly the code path that matters — the restart. This ledger is the
//! DURABLE projection of that flag, written the moment `seal_close` succeeds
//! (`services::release_adoption::apply`'s `ChainSealer` impl), and keyed by
//! `CellId` because a `CellId` is what the conductor writes to and it survives
//! a restart that loses the role map.
//!
//! # What this fence does NOT know
//!
//! The headline above is the GUARANTEE, not the coverage. Two limits, both
//! real, and both the next seat's business:
//!
//! * **The ledger records only closes THIS peer performed.** It is written from
//!   the `ChainSealer` impl, so a chain closed by an operator, by a neighbour,
//!   or by an earlier a2o run leaves no entry here. Rail 1 covers most of that
//!   in practice — credentials minted on an earlier boot are reused forever, so
//!   a later close by anyone never provokes a mint — but a peer that meets an
//!   **already-closed cell with a fresh data dir** has neither a ledger entry
//!   nor a persisted credential, takes the `Mint` branch, and earns the block.
//!   Closing that gap is a bounded follow-on: a boot-time `dump_full_state`
//!   close-scan per supervised role, mirroring the a2o Background's
//!   `closeChainSeqOf`.
//! * **Durability here equals `storage_dir`'s durability.** Both rails live
//!   under the storage data dir while the sealed chain lives in the conductor's.
//!   A deployment that gives storage an EPHEMERAL data dir beside a persistent
//!   conductor dir makes every restart a fresh-data-dir case, and this fence
//!   silently degrades to nothing.
//!
//! A third consequence worth stating where an operator meets it: rail 2 is a
//! read-disable as well as a write-fence. A closed cell with no pre-close
//! credentials never connects, so it serves none of the
//! [`CLOSED_CHAIN_READ_FNS`] either — a zome read needs a capability grant too.
//! That is forced rather than chosen, and it is the one place the sunset
//! ruling's "routing, not a disable" does not hold; the refusal message says so
//! in as many words.
//!
//! # Storage layout
//!
//! ```text
//! <storage_dir>/closed-chain-fence/
//!   closed-cells.json                       # the ledger (0600)
//!   credentials/<dna39hex>-<agent39hex>.json # one per cell (0600), dir 0700
//! ```
//!
//! Both are node-local operational state (Category C): they are never
//! notarized, never gossiped, and never leave this host. The credential file
//! holds a signing keypair and a capability secret, so it is written 0600 under
//! a 0700 directory.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

use holochain_client::CellId;
use serde::{Deserialize, Serialize};

/// Zome functions that only READ, and are therefore still legal on a closed
/// chain — *"each closed v1 chain is still readable by every peer"* rides on
/// exactly these.
///
/// Anything NOT named here is treated as an author on a closed chain and
/// refused. The list is an ALLOW-list rather than a deny-list on purpose: a new
/// write extern added to any DNA is refused by default, whereas a deny-list
/// would let it through until somebody remembered to add it.
///
/// The first five mirror the a2o harness rail exactly (`seal_close` itself
/// calls `my_chain_activity`, `get_record_at` and `get_signed_action` on the
/// predecessor for precisely this reason).
///
/// The last three are additional node-registry externs, each verified
/// write-free in the coordinator source and RESERVED for the v1 read paths —
/// not a claim that storage calls all three today. Only `known_agents` is
/// storage's own call on a v1 cell (`lineage_bridge.rs`'s neighbour sweep);
/// `export_records` / `export_held_records` / `agent_activity_of` are read
/// INSIDE `carry_from`, which runs on the v2 side app. They are listed because
/// an over-broad READ allow-list costs nothing while a missing entry is a
/// silent refusal on a path that was always legal.
pub const CLOSED_CHAIN_READ_FNS: &[&str] = &[
    "export_records",
    "my_chain_activity",
    "get_record_at",
    "get_signed_action",
    "get_closes_for",
    "known_agents",
    "export_held_records",
    "agent_activity_of",
];

/// The ledger file, under [`FENCE_DIR`].
const LEDGER_FILE: &str = "closed-cells.json";
/// The per-cell credential directory, under [`FENCE_DIR`].
const CREDENTIALS_DIR: &str = "credentials";
/// The fence's own subdirectory of the storage data dir.
pub const FENCE_DIR: &str = "closed-chain-fence";

/// Stable identity of one cell on disk and in the maps: the DNA hash and the
/// agent key, each as the hex of its raw 39 bytes.
///
/// Hex rather than the `uhC0k…` base64 rendering because it is filename-safe on
/// every platform without a second escaping rule, and because a hex string
/// cannot be confused with a hash an operator might paste from a warrant.
pub fn cell_key(cell: &CellId) -> String {
    format!(
        "{}-{}",
        hex::encode(cell.dna_hash().get_raw_39()),
        hex::encode(cell.agent_pubkey().get_raw_39())
    )
}

/// One closed cell, as recorded at the moment its chain was sealed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClosedCellRecord {
    /// `cell_key(cell)` — the ledger's primary key.
    pub cell: String,
    /// DNA hash, hex of the raw 39 bytes.
    pub dna_hash: String,
    /// Agent public key, hex of the raw 39 bytes.
    pub agent_pub_key: String,
    /// The role whose chain this was, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// The installed app id the cell belonged to, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_id: Option<String>,
    /// Human-readable provenance of the close — quoted back in every refusal
    /// so an operator reading a 503 learns why without opening a database.
    pub why: String,
    /// Unix seconds at which this peer recorded the close.
    pub closed_at: i64,
}

/// Persisted signing credentials for one cell.
///
/// The three fields are exactly `holochain_client::SigningCredentials`, stored
/// as hex so the file is inspectable and so a length mismatch is caught at read
/// time rather than by a panic inside `from_raw_39`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredCredentials {
    /// The SIGNING agent key the grant was assigned to (raw 39, hex) — not the
    /// cell's own agent.
    pub signing_agent_key: String,
    /// The ed25519 signing key's 32 secret bytes, hex.
    pub keypair: String,
    /// The 64-byte capability secret, hex.
    pub cap_secret: String,
    /// Unix seconds at which these credentials were minted.
    pub minted_at: i64,
}

/// Why the fence refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FenceError {
    /// The cell's chain is closed and no pre-close credentials are persisted,
    /// so there is no way to reach it that does not author on a sealed chain.
    ClosedChain(String),
    /// The conductor refused the mint (this is the underlying admin error,
    /// passed through unchanged).
    Authorize(String),
    /// Persisted credentials exist but could not be turned back into a usable
    /// `SigningCredentials` (bad hex, wrong length, truncated file).
    Corrupt(String),
}

impl std::fmt::Display for FenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FenceError::ClosedChain(msg) => write!(f, "{msg}"),
            FenceError::Authorize(msg) => write!(f, "authorize_signing_credentials failed: {msg}"),
            FenceError::Corrupt(msg) => write!(f, "persisted signing credentials unusable: {msg}"),
        }
    }
}

impl std::error::Error for FenceError {}

/// What the fence will do about one cell's signing credentials — the decision,
/// separated from its enactment so it can be asserted without a conductor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorizeDecision {
    /// Credentials already exist for this cell. **Nothing is authored.**
    Reuse(Box<StoredCredentials>),
    /// The cell's chain is closed and no pre-close credentials exist.
    /// **Nothing reaches the conductor**; the string is the named refusal.
    Refuse(String),
    /// The cell is open and unminted: authorize once, then persist.
    Mint,
}

/// The durable closed-cell ledger and per-cell credential vault.
///
/// Constructed with a directory and nothing else, so every test drives a real
/// one over a temp dir rather than a mock.
#[derive(Debug)]
pub struct ClosedChainFence {
    root: PathBuf,
    closed: RwLock<BTreeMap<String, ClosedCellRecord>>,
    /// In-memory mirror of the credential files. Populated lazily on read and
    /// eagerly on mint, so the steady state costs no filesystem call.
    creds: RwLock<BTreeMap<String, StoredCredentials>>,
    /// Cells whose credentials have already been healed once in this process
    /// (see [`ClosedChainFence::discard_stale_credentials`]). Not persisted: the
    /// bound is per-process on purpose, so a genuine repair survives a restart
    /// while a rejection LOOP cannot.
    healed: RwLock<std::collections::BTreeSet<String>>,
}

/// The one wording of the closed-cell refusal, so the two arms that produce it
/// cannot drift into describing the same fact differently.
///
/// `detail` says WHY there is no way in; the rest is invariant.
fn closed_refusal(record: &ClosedCellRecord, label: &str, detail: &str) -> String {
    format!(
        "refusing to authorize signing credentials on a CLOSED chain (cell {cell}, {why}) for \
         '{label}': authorize_signing_credentials COMMITS a CapGrant, and a post-close action is \
         warranted by every neighbour into a permanent cell block that holochain 0.7 cannot \
         lift. {detail} This role therefore stays UNCONNECTED (503) — which, note, also means it \
         serves none of the reads a closed chain would otherwise still answer, because a zome \
         read needs a capability grant too. An unconnected role is recoverable; a blocked cell \
         is not.",
        cell = record.cell,
        why = record.why,
    )
}

impl ClosedChainFence {
    /// Open (and create) the fence under `storage_dir`.
    ///
    /// A ledger that fails to parse is reported LOUDLY and treated as empty:
    /// the alternative — refusing every cell — would take a node off the mesh
    /// over a corrupt operational file, and the credential vault (rail 1) still
    /// keeps a restart write-free wherever a mint already happened.
    pub fn open(storage_dir: &Path) -> Self {
        let root = storage_dir.join(FENCE_DIR);
        if let Err(e) = std::fs::create_dir_all(root.join(CREDENTIALS_DIR)) {
            tracing::warn!(
                dir = %root.display(),
                error = %e,
                "closed-chain fence: could not create its directory — the ledger and the \
                 credential vault will not persist across this restart"
            );
        }
        harden_dir(&root);
        harden_dir(&root.join(CREDENTIALS_DIR));

        let ledger_path = root.join(LEDGER_FILE);
        let closed = match std::fs::read_to_string(&ledger_path) {
            Ok(raw) => match serde_json::from_str::<Vec<ClosedCellRecord>>(&raw) {
                Ok(records) => records
                    .into_iter()
                    .map(|r| (r.cell.clone(), r))
                    .collect::<BTreeMap<_, _>>(),
                Err(e) => {
                    tracing::warn!(
                        path = %ledger_path.display(),
                        error = %e,
                        "closed-chain fence: LEDGER UNREADABLE — proceeding with an empty ledger. \
                         Any chain closed by an earlier run is invisible to the fence until this \
                         file is repaired; persisted credentials still keep a restart write-free."
                    );
                    BTreeMap::new()
                }
            },
            Err(_) => BTreeMap::new(),
        };

        if !closed.is_empty() {
            tracing::info!(
                closed_cells = closed.len(),
                path = %ledger_path.display(),
                "closed-chain fence armed with a non-empty ledger — no credential will be minted \
                 on these cells"
            );
        }

        Self {
            root,
            closed: RwLock::new(closed),
            creds: RwLock::new(BTreeMap::new()),
            healed: RwLock::new(std::collections::BTreeSet::new()),
        }
    }

    /// A fence with no directory behind it — every write is dropped and nothing
    /// persists. Used only where a data dir genuinely does not exist.
    #[cfg(test)]
    fn in_memory() -> Self {
        Self {
            root: PathBuf::new(),
            closed: RwLock::new(BTreeMap::new()),
            creds: RwLock::new(BTreeMap::new()),
            healed: RwLock::new(std::collections::BTreeSet::new()),
        }
    }

    /// Record that `cell`'s chain has been closed. Idempotent: a second call
    /// for the same cell keeps the FIRST record, because the first is the one
    /// contemporaneous with the close.
    pub fn record_closed(
        &self,
        cell: &CellId,
        role: Option<&str>,
        app_id: Option<&str>,
        why: &str,
        now: i64,
    ) {
        let key = cell_key(cell);
        {
            let mut closed = self.closed.write().unwrap_or_else(|e| e.into_inner());
            if closed.contains_key(&key) {
                return;
            }
            closed.insert(
                key.clone(),
                ClosedCellRecord {
                    cell: key.clone(),
                    dna_hash: hex::encode(cell.dna_hash().get_raw_39()),
                    agent_pub_key: hex::encode(cell.agent_pubkey().get_raw_39()),
                    role: role.map(str::to_string),
                    app_id: app_id.map(str::to_string),
                    why: why.to_string(),
                    closed_at: now,
                },
            );
        }
        self.persist_ledger();
        tracing::warn!(
            cell = %key,
            role = role.unwrap_or("-"),
            app_id = app_id.unwrap_or("-"),
            why,
            "closed-chain fence: CELL SEALED — this node will author nothing further on it, \
             capability grants included"
        );
    }

    /// The record for `cell`, when its chain is closed.
    pub fn closed_record(&self, cell: &CellId) -> Option<ClosedCellRecord> {
        self.closed
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&cell_key(cell))
            .cloned()
    }

    /// Every closed cell this node knows of, for the operator surfaces.
    pub fn closed_cells(&self) -> Vec<ClosedCellRecord> {
        self.closed
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .cloned()
            .collect()
    }

    /// Refuse a zome call that would author on a closed chain, by name.
    ///
    /// `None` means "not refused" — either the cell is open, or the function is
    /// one of the reads a closed chain still answers.
    pub fn refuse_zome_call(&self, cell: &CellId, zome: &str, fn_name: &str) -> Option<String> {
        let record = self.closed_record(cell)?;
        if CLOSED_CHAIN_READ_FNS.contains(&fn_name) {
            return None;
        }
        Some(format!(
            "refusing {zome}/{fn_name} on a CLOSED chain (cell {cell}, {why}). A close is a \
             sealing act: every neighbour validates a post-close action as invalid, warrants its \
             author, and turns the warrant into a permanent cell block that holochain 0.7 cannot \
             lift. Reads are still served here ({reads}); a write belongs on the successor cell.",
            cell = record.cell,
            why = record.why,
            reads = CLOSED_CHAIN_READ_FNS.join(", "),
        ))
    }

    /// What [`Self::authorize`] will do for `cell`, decided WITHOUT a conductor.
    ///
    /// Split out from the async call on purpose: "a closed cell produces no
    /// `authorize_signing_credentials` call" is a claim about control flow, and
    /// a test that can only observe the call by mocking an `AdminWebsocket`
    /// cannot make it. Here the claim is a value.
    ///
    /// Order is load-bearing:
    ///
    /// 1. **USABLE persisted credentials win, closed or open.** The grant they
    ///    name is already on the chain; re-minting would author a second one
    ///    for no gain. This is what makes a `storage-restart` write-free.
    /// 2. **An UNUSABLE persisted credential is discarded on an OPEN cell and
    ///    re-minted; on a CLOSED cell it is refused.** See below — this
    ///    asymmetry is the whole point of the branch.
    /// 3. **A closed cell with no usable credentials is refused by name.**
    ///    Nothing reaches the conductor.
    /// 4. **Otherwise mint once and persist immediately**, so the next restart
    ///    takes branch 1.
    ///
    /// # Why an unusable credential must not brick an OPEN cell
    ///
    /// Before this fence existed, storage re-minted on every connect, so a
    /// truncated, hand-edited or half-written credential file simply could not
    /// happen — and if the conductor's grant went missing, the next connect
    /// replaced it. Rail 1 removes that per-connect write, which also removes
    /// that self-healing. A `Reuse` returned for a file that cannot be turned
    /// back into `SigningCredentials` would be retried forever by
    /// [`crate::hc_client_registry::HcClientRegistry::connect_role_forever`],
    /// which never gives up — the role would stay down until an operator
    /// deleted the file by hand.
    ///
    /// So on an OPEN cell an unusable credential is DISCARDED (cache and file)
    /// with a named WARN and the decision falls through to `Mint`: one extra
    /// `CapGrant` on a chain that accepts them, which is exactly the
    /// pre-Task-32 behaviour and exactly what the fence exists to allow on an
    /// open chain.
    ///
    /// On a CLOSED cell it is NOT discarded and NOT minted. There, minting is
    /// the thing that must never happen, and an unusable credential is simply
    /// the "no way in" case — refused by name, with the file left in place for
    /// an operator to inspect.
    pub fn decide(&self, cell: &CellId, label: &str) -> AuthorizeDecision {
        let key = cell_key(cell);
        let closed = self.closed_record(cell);

        if let Some(stored) = self.load_credentials(&key) {
            match to_signing_credentials(&stored) {
                // The steady state: a grant already on the chain, reused.
                Ok(_) => return AuthorizeDecision::Reuse(Box::new(stored)),
                Err(e) if closed.is_none() => {
                    tracing::warn!(
                        cell = %key,
                        label,
                        error = %e,
                        "closed-chain fence: persisted signing credentials are UNUSABLE on an \
                         OPEN cell — discarding them and minting once. A CapGrant on an open \
                         chain is safe and this is the pre-fence self-healing behaviour; the \
                         same fault on a CLOSED cell is refused instead."
                    );
                    self.discard_credentials(&key);
                }
                Err(e) => {
                    // Closed: say so precisely rather than reporting "no
                    // credentials", which would send an operator looking for a
                    // file that is right there and broken.
                    return AuthorizeDecision::Refuse(closed_refusal(
                        closed.as_ref().expect("closed arm"),
                        label,
                        &format!(
                            "The credentials persisted for this cell are UNUSABLE ({e}) and are \
                             left in place for inspection; they are NOT re-minted, because \
                             minting is the one thing a sealed chain must never receive."
                        ),
                    ));
                }
            }
        }

        if let Some(record) = closed {
            return AuthorizeDecision::Refuse(closed_refusal(
                &record,
                label,
                "No pre-close credentials are persisted for this cell, so there is no \
                 write-free way to reach it.",
            ));
        }
        AuthorizeDecision::Mint
    }

    /// Forget the persisted credentials for `key`, in memory and on disk.
    ///
    /// Reached from two places, both of which have established that the cell is
    /// OPEN: [`Self::decide`] on an unusable file, and
    /// [`Self::discard_stale_credentials`] on a runtime authorization failure.
    /// Nothing calls it for a closed cell.
    fn discard_credentials(&self, key: &str) {
        self.creds
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(key);
        if self.root.as_os_str().is_empty() {
            return;
        }
        let path = self.credentials_path(key);
        if let Err(e) = std::fs::remove_file(&path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "closed-chain fence: could not remove the unusable credential file — the \
                     next connect will read it again and discard it again"
                );
            }
        }
    }

    /// A zome call on `cell` failed in a way that says the conductor no longer
    /// honours our persisted grant. Discard it so the NEXT connect mints a
    /// fresh one.
    ///
    /// # The shape this exists for
    ///
    /// The credential file can be perfectly well-formed while the `CapGrant` it
    /// names is gone from the chain — a conductor database restored from an
    /// older snapshot, with the same `CellId`. `decide` cannot see that: the
    /// file parses, the key is valid, `Reuse` is correct as far as it can tell.
    /// The only place the truth appears is the first zome call, which comes
    /// back unauthorized.
    ///
    /// Returns `true` when something was discarded.
    ///
    /// # Two bounds, both deliberate
    ///
    /// * **Never on a closed cell.** Discarding there would invite the next
    ///   connect to mint on a sealed chain — the exact action this module
    ///   exists to prevent. A closed cell whose grant has vanished is simply
    ///   unreachable, and that is the correct answer.
    /// * **Once per cell per process.** The heal is a repair, not a retry loop:
    ///   after one discard the next connect mints, and if THAT grant is also
    ///   rejected the fault is not a stale file. Bounding it here means the
    ///   worst case is one extra `CapGrant` per cell per process — strictly
    ///   fewer than the pre-Task-32 one-per-connect.
    pub fn discard_stale_credentials(&self, cell: &CellId, reason: &str) -> bool {
        if self.closed_record(cell).is_some() {
            return false;
        }
        let key = cell_key(cell);
        {
            let mut healed = self.healed.write().unwrap_or_else(|e| e.into_inner());
            if !healed.insert(key.clone()) {
                return false;
            }
        }
        if self.load_credentials(&key).is_none() {
            return false;
        }
        self.discard_credentials(&key);
        tracing::warn!(
            cell = %key,
            reason,
            "closed-chain fence: the conductor rejected our persisted signing credentials on an \
             OPEN cell — discarded ONCE so the next connect mints a fresh grant. A second \
             rejection on this cell will not be healed again; the fault would not be a stale file."
        );
        true
    }

    /// The ONE gate every `authorize_signing_credentials` call site goes
    /// through. The decision is [`Self::decide`]; this only enacts it.
    pub async fn authorize(
        &self,
        admin_ws: &holochain_client::AdminWebsocket,
        cell: &CellId,
        label: &str,
    ) -> Result<holochain_client::SigningCredentials, FenceError> {
        let key = cell_key(cell);
        match self.decide(cell, label) {
            AuthorizeDecision::Reuse(stored) => {
                let creds = to_signing_credentials(&stored)?;
                tracing::info!(
                    cell = %key,
                    label,
                    "closed-chain fence: REUSING persisted signing credentials — no CapGrant \
                     authored"
                );
                Ok(creds)
            }
            AuthorizeDecision::Refuse(reason) => Err(FenceError::ClosedChain(reason)),
            AuthorizeDecision::Mint => {
                let credentials = admin_ws
                    .authorize_signing_credentials(
                        holochain_client::AuthorizeSigningCredentialsPayload {
                            cell_id: cell.clone(),
                            functions: None,
                        },
                    )
                    .await
                    .map_err(|e| FenceError::Authorize(e.to_string()))?;
                self.store_credentials(&key, &credentials);
                tracing::info!(
                    cell = %key,
                    label,
                    "closed-chain fence: minted signing credentials ONCE and persisted them — \
                     every later connect for this cell reuses them and authors nothing"
                );
                Ok(credentials)
            }
        }
    }

    // ---- persistence -----------------------------------------------------

    fn credentials_path(&self, key: &str) -> PathBuf {
        self.root.join(CREDENTIALS_DIR).join(format!("{key}.json"))
    }

    fn load_credentials(&self, key: &str) -> Option<StoredCredentials> {
        if let Some(hit) = self
            .creds
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(key)
            .cloned()
        {
            return Some(hit);
        }
        if self.root.as_os_str().is_empty() {
            return None;
        }
        let path = self.credentials_path(key);
        let raw = std::fs::read_to_string(&path).ok()?;
        match serde_json::from_str::<StoredCredentials>(&raw) {
            Ok(stored) => {
                self.creds
                    .write()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(key.to_string(), stored.clone());
                Some(stored)
            }
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "closed-chain fence: persisted credentials unreadable — a fresh mint will be \
                     attempted, which is a chain write and is refused outright on a closed cell"
                );
                None
            }
        }
    }

    fn store_credentials(&self, key: &str, creds: &holochain_client::SigningCredentials) {
        let stored = StoredCredentials {
            signing_agent_key: hex::encode(creds.signing_agent_key.get_raw_39()),
            keypair: hex::encode(creds.keypair.to_bytes()),
            cap_secret: hex::encode(creds.cap_secret.as_ref()),
            minted_at: unix_now(),
        };
        self.creds
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key.to_string(), stored.clone());
        if self.root.as_os_str().is_empty() {
            return;
        }
        let path = self.credentials_path(key);
        match serde_json::to_string_pretty(&stored) {
            Ok(body) => write_private(&path, &body),
            Err(e) => tracing::warn!(
                path = %path.display(),
                error = %e,
                "closed-chain fence: could not encode credentials — the next restart will re-mint \
                 (one extra CapGrant on an OPEN chain; refused outright on a closed one)"
            ),
        }
    }

    fn persist_ledger(&self) {
        if self.root.as_os_str().is_empty() {
            return;
        }
        let records = self.closed_cells();
        let path = self.root.join(LEDGER_FILE);
        match serde_json::to_string_pretty(&records) {
            Ok(body) => write_private(&path, &body),
            Err(e) => tracing::error!(
                path = %path.display(),
                error = %e,
                "closed-chain fence: COULD NOT PERSIST THE LEDGER — a restart will not know this \
                 chain is closed"
            ),
        }
    }
}

/// Rebuild `holochain_client::SigningCredentials` from the persisted hex.
fn to_signing_credentials(
    stored: &StoredCredentials,
) -> Result<holochain_client::SigningCredentials, FenceError> {
    let agent = hex::decode(&stored.signing_agent_key)
        .map_err(|e| FenceError::Corrupt(format!("signingAgentKey is not hex: {e}")))?;
    if agent.len() != 39 {
        return Err(FenceError::Corrupt(format!(
            "signingAgentKey is {} bytes, expected 39",
            agent.len()
        )));
    }
    let key_bytes = hex::decode(&stored.keypair)
        .map_err(|e| FenceError::Corrupt(format!("keypair is not hex: {e}")))?;
    let key_bytes: [u8; 32] = key_bytes.try_into().map_err(|v: Vec<u8>| {
        FenceError::Corrupt(format!("keypair is {} bytes, expected 32", v.len()))
    })?;
    let secret_bytes = hex::decode(&stored.cap_secret)
        .map_err(|e| FenceError::Corrupt(format!("capSecret is not hex: {e}")))?;
    let cap_secret = holochain_types::prelude::CapSecret::try_from(secret_bytes.as_slice())
        .map_err(|_| {
            FenceError::Corrupt(format!(
                "capSecret is {} bytes, expected 64",
                secret_bytes.len()
            ))
        })?;

    Ok(holochain_client::SigningCredentials {
        signing_agent_key: holochain_types::prelude::AgentPubKey::from_raw_39(agent),
        keypair: ed25519_dalek::SigningKey::from_bytes(&key_bytes),
        cap_secret,
    })
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Write `body` to `path` with owner-only permissions.
///
/// The file is CREATED at 0600 rather than created-then-chmodded: a plain
/// `fs::write` followed by `set_permissions` leaves a credential readable at
/// `0666 & ~umask` for the window between the two syscalls. The chmod stays as
/// belt-and-braces for a file that already existed at a looser mode.
fn write_private(path: &Path, body: &str) {
    use std::io::Write;

    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let written = options
        .open(path)
        .and_then(|mut file| file.write_all(body.as_bytes()));
    if let Err(e) = written {
        tracing::warn!(path = %path.display(), error = %e, "closed-chain fence: write failed");
        return;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)) {
            tracing::warn!(path = %path.display(), error = %e, "closed-chain fence: chmod 0600 failed");
        }
    }
}

fn harden_dir(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700));
    }
    #[cfg(not(unix))]
    let _ = path;
}

// ---------------------------------------------------------------------------
// Process-wide handle
// ---------------------------------------------------------------------------

static FENCE: OnceLock<ClosedChainFence> = OnceLock::new();

/// Arm the process-wide fence under `storage_dir`. Called ONCE from the
/// composition root, before any conductor connect. A second call is a no-op.
pub fn init(storage_dir: &Path) -> &'static ClosedChainFence {
    FENCE.get_or_init(|| ClosedChainFence::open(storage_dir))
}

/// The process-wide fence, or `None` on a build that never armed one.
///
/// `None` restores the exact pre-Task-32 behaviour at every call site, which is
/// what keeps unit tests and library consumers unaffected. Production arms it
/// in `main`.
pub fn fence() -> Option<&'static ClosedChainFence> {
    FENCE.get()
}

#[cfg(test)]
mod tests {
    use super::*;
    use holochain_types::prelude::{AgentPubKey, DnaHash};

    fn test_cell(dna_seed: u8, agent_seed: u8) -> CellId {
        CellId::new(
            DnaHash::from_raw_32(vec![dna_seed; 32]),
            AgentPubKey::from_raw_32(vec![agent_seed; 32]),
        )
    }

    fn tmp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "elohim-fence-{name}-{}-{}",
            std::process::id(),
            unix_now()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn an_open_cell_is_not_closed_and_refuses_nothing() {
        let fence = ClosedChainFence::in_memory();
        let cell = test_cell(1, 2);
        assert!(fence.closed_record(&cell).is_none());
        assert!(fence
            .refuse_zome_call(&cell, "node_registry_coordinator", "register_node")
            .is_none());
    }

    #[test]
    fn a_closed_cell_refuses_a_write_by_name_and_still_serves_the_reads() {
        let fence = ClosedChainFence::in_memory();
        let cell = test_cell(3, 4);
        fence.record_closed(
            &cell,
            Some("node_registry"),
            Some("elohim"),
            "sealed by seal_close",
            1_700_000_000,
        );

        let refusal = fence
            .refuse_zome_call(&cell, "node_registry_coordinator", "register_node")
            .expect("a write on a closed chain must be refused");
        assert!(refusal.contains("register_node"), "{refusal}");
        assert!(refusal.contains("CLOSED chain"), "{refusal}");
        assert!(refusal.contains("sealed by seal_close"), "{refusal}");

        for read in CLOSED_CHAIN_READ_FNS {
            assert!(
                fence
                    .refuse_zome_call(&cell, "node_registry_coordinator", read)
                    .is_none(),
                "{read} is an allow-listed read and must still be served"
            );
        }
    }

    #[test]
    fn the_read_allowlist_is_the_one_the_a2o_rail_declares() {
        // The a2o harness (happ-lineage-migration.steps.ts CLOSED_CHAIN_READ_FNS)
        // names these five. Storage may add write-free externs of its own but
        // must never DROP one of the five, or the two seats disagree about what
        // a closed chain answers.
        for shared in [
            "export_records",
            "my_chain_activity",
            "get_record_at",
            "get_signed_action",
            "get_closes_for",
        ] {
            assert!(
                CLOSED_CHAIN_READ_FNS.contains(&shared),
                "{shared} is in the a2o rail and must stay in this allowlist"
            );
        }
        // And the whole list is pinned, so storage's own three cannot vanish
        // under a rename without a failing test naming them.
        assert_eq!(
            CLOSED_CHAIN_READ_FNS,
            &[
                "export_records",
                "my_chain_activity",
                "get_record_at",
                "get_signed_action",
                "get_closes_for",
                "known_agents",
                "export_held_records",
                "agent_activity_of",
            ]
        );
    }

    #[test]
    fn recording_a_close_is_idempotent_and_keeps_the_first_record() {
        let fence = ClosedChainFence::in_memory();
        let cell = test_cell(5, 6);
        fence.record_closed(&cell, Some("node_registry"), None, "first", 100);
        fence.record_closed(&cell, Some("node_registry"), None, "second", 200);
        let record = fence.closed_record(&cell).expect("recorded");
        assert_eq!(record.why, "first");
        assert_eq!(record.closed_at, 100);
        assert_eq!(fence.closed_cells().len(), 1);
    }

    #[test]
    fn the_ledger_survives_a_restart() {
        let dir = tmp_dir("ledger");
        let cell = test_cell(7, 8);
        {
            let fence = ClosedChainFence::open(&dir);
            fence.record_closed(&cell, Some("node_registry"), Some("elohim"), "sealed", 42);
            assert!(fence.closed_record(&cell).is_some());
        }
        // A brand new process: the in-memory LineageRoles map is gone, and the
        // ledger is the only thing that still knows.
        let reopened = ClosedChainFence::open(&dir);
        let record = reopened.closed_record(&cell).expect("survives a restart");
        assert_eq!(record.role.as_deref(), Some("node_registry"));
        assert_eq!(record.why, "sealed");
        assert!(reopened
            .refuse_zome_call(&cell, "node_registry_coordinator", "heartbeat")
            .is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_first_mint_persists_and_the_next_process_reuses_it() {
        let dir = tmp_dir("creds");
        let cell = test_cell(9, 10);
        let key = cell_key(&cell);

        // Stand in for what `authorize` stores after the one and only mint.
        let minted = holochain_client::SigningCredentials {
            signing_agent_key: AgentPubKey::from_raw_32(vec![11; 32]),
            keypair: ed25519_dalek::SigningKey::from_bytes(&[7u8; 32]),
            cap_secret: holochain_types::prelude::CapSecret::from([3u8; 64]),
        };
        {
            let fence = ClosedChainFence::open(&dir);
            fence.store_credentials(&key, &minted);
        }

        let reopened = ClosedChainFence::open(&dir);
        let stored = reopened
            .load_credentials(&key)
            .expect("persisted credentials are found by a fresh process");
        let rebuilt = to_signing_credentials(&stored).expect("rebuild");
        assert_eq!(rebuilt.signing_agent_key, minted.signing_agent_key);
        assert_eq!(rebuilt.keypair.to_bytes(), minted.keypair.to_bytes());
        assert_eq!(rebuilt.cap_secret.as_ref(), minted.cap_secret.as_ref());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(reopened.credentials_path(&key))
                .expect("stat")
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600, "the credential file holds a secret");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_closed_cell_with_persisted_credentials_needs_no_conductor_at_all() {
        // The branch that makes a `storage-restart` write-free: credentials
        // minted BEFORE the close are still valid after it, so `authorize`
        // never reaches branch 2 or 3 and never touches an AdminWebsocket.
        let dir = tmp_dir("closed-with-creds");
        let cell = test_cell(12, 13);
        let key = cell_key(&cell);
        let fence = ClosedChainFence::open(&dir);
        fence.store_credentials(
            &key,
            &holochain_client::SigningCredentials {
                signing_agent_key: AgentPubKey::from_raw_32(vec![14; 32]),
                keypair: ed25519_dalek::SigningKey::from_bytes(&[15u8; 32]),
                cap_secret: holochain_types::prelude::CapSecret::from([16u8; 64]),
            },
        );
        fence.record_closed(&cell, Some("node_registry"), None, "sealed", 1);

        let stored = fence.load_credentials(&key).expect("reused, not re-minted");
        assert!(to_signing_credentials(&stored).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_credentials_are_named_not_silently_re_minted_into_a_grant() {
        let bad = StoredCredentials {
            signing_agent_key: hex::encode([1u8; 39]),
            keypair: hex::encode([2u8; 8]), // wrong length
            cap_secret: hex::encode([3u8; 64]),
            minted_at: 0,
        };
        match to_signing_credentials(&bad) {
            Err(FenceError::Corrupt(msg)) => assert!(msg.contains("keypair"), "{msg}"),
            other => panic!("expected a named Corrupt error, got {other:?}"),
        }
    }

    #[test]
    fn a_closed_cell_with_no_pre_close_credentials_produces_no_authorize_call() {
        // The deliverable's first half, as a claim about control flow: the
        // decision is `Refuse` and it is reached with no conductor in the
        // picture at all, so there is no path from here to a CapGrant.
        let fence = ClosedChainFence::in_memory();
        let cell = test_cell(20, 21);
        fence.record_closed(&cell, Some("node_registry"), Some("elohim"), "sealed", 1);
        match fence.decide(&cell, "node_registry") {
            AuthorizeDecision::Refuse(reason) => {
                assert!(reason.contains("CLOSED chain"), "{reason}");
                assert!(reason.contains("COMMITS a CapGrant"), "{reason}");
                assert!(reason.contains("node_registry"), "{reason}");
            }
            other => panic!("a closed cell must never mint: {other:?}"),
        }
    }

    #[test]
    fn an_open_unminted_cell_still_mints_exactly_as_before() {
        let fence = ClosedChainFence::in_memory();
        assert_eq!(
            fence.decide(&test_cell(22, 23), "lamad"),
            AuthorizeDecision::Mint,
            "an open role must be byte-for-byte the pre-Task-32 behaviour"
        );
    }

    #[test]
    fn a_restart_reuses_rather_than_re_minting_open_or_closed() {
        let dir = tmp_dir("decide-reuse");
        let cell = test_cell(24, 25);
        let key = cell_key(&cell);
        {
            let first_boot = ClosedChainFence::open(&dir);
            assert_eq!(
                first_boot.decide(&cell, "node_registry"),
                AuthorizeDecision::Mint,
                "the FIRST boot mints"
            );
            first_boot.store_credentials(
                &key,
                &holochain_client::SigningCredentials {
                    signing_agent_key: AgentPubKey::from_raw_32(vec![26; 32]),
                    keypair: ed25519_dalek::SigningKey::from_bytes(&[27u8; 32]),
                    cap_secret: holochain_types::prelude::CapSecret::from([28u8; 64]),
                },
            );
        }

        // Second boot, chain still open: reuse, no chain write.
        let second_boot = ClosedChainFence::open(&dir);
        assert!(
            matches!(
                second_boot.decide(&cell, "node_registry"),
                AuthorizeDecision::Reuse(_)
            ),
            "a storage-restart must author nothing"
        );

        // Third boot, after somebody sealed the chain: still reuse — the grant
        // is already on the chain, so the closed cell stays reachable for the
        // reads it still serves.
        second_boot.record_closed(&cell, Some("node_registry"), None, "sealed", 2);
        assert!(matches!(
            ClosedChainFence::open(&dir).decide(&cell, "node_registry"),
            AuthorizeDecision::Reuse(_)
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A credential file that parses as JSON but cannot become
    /// `SigningCredentials` — the shape a truncated write or a hand-edit
    /// leaves behind.
    fn write_unusable_credentials(fence: &ClosedChainFence, key: &str) {
        fence
            .creds
            .write()
            .unwrap()
            .insert(key.to_string(), unusable_stored());
        if !fence.root.as_os_str().is_empty() {
            write_private(
                &fence.credentials_path(key),
                &serde_json::to_string(&unusable_stored()).unwrap(),
            );
        }
    }

    fn unusable_stored() -> StoredCredentials {
        StoredCredentials {
            signing_agent_key: hex::encode([1u8; 39]),
            keypair: hex::encode([2u8; 8]), // wrong length — cannot be a SigningKey
            cap_secret: hex::encode([3u8; 64]),
            minted_at: 0,
        }
    }

    #[test]
    fn an_unusable_credential_on_an_open_cell_is_discarded_and_re_minted_once() {
        // I1. Before the fence, a per-connect re-mint healed this shape; rail 1
        // removed that, so the heal has to be explicit or an OPEN role stays
        // down forever behind `connect_role_forever`'s infinite retry.
        let dir = tmp_dir("stale-open");
        let fence = ClosedChainFence::open(&dir);
        let cell = test_cell(30, 31);
        let key = cell_key(&cell);
        write_unusable_credentials(&fence, &key);

        assert_eq!(
            fence.decide(&cell, "node_registry"),
            AuthorizeDecision::Mint,
            "an open cell must heal, not brick"
        );
        assert!(
            fence.load_credentials(&key).is_none(),
            "the unusable file is discarded from cache AND disk, so the mint is not undone \
             by the next read"
        );
        assert!(!fence.credentials_path(&key).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unusable_credential_on_a_closed_cell_is_refused_and_never_minted() {
        // The asymmetry is the point: on a sealed chain a mint is the one thing
        // that must never happen, so the broken file is left in place and named.
        let dir = tmp_dir("stale-closed");
        let fence = ClosedChainFence::open(&dir);
        let cell = test_cell(32, 33);
        let key = cell_key(&cell);
        write_unusable_credentials(&fence, &key);
        fence.record_closed(&cell, Some("node_registry"), None, "sealed", 1);

        match fence.decide(&cell, "node_registry") {
            AuthorizeDecision::Refuse(reason) => {
                assert!(reason.contains("UNUSABLE"), "{reason}");
                assert!(reason.contains("NOT re-minted"), "{reason}");
                assert!(reason.contains("CLOSED chain"), "{reason}");
            }
            other => panic!("a closed cell must never mint: {other:?}"),
        }
        assert!(
            fence.credentials_path(&key).exists(),
            "the file stays for an operator to inspect"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_rejected_grant_heals_exactly_once_on_an_open_cell() {
        // The stale-grant shape: the file is perfectly good, but the CapGrant
        // it names is gone from a restored conductor database. Only the first
        // zome call can see it, so the heal is driven from there.
        let dir = tmp_dir("stale-grant");
        let fence = ClosedChainFence::open(&dir);
        let cell = test_cell(34, 35);
        let key = cell_key(&cell);
        fence.store_credentials(&key, &good_credentials());

        assert!(
            fence.discard_stale_credentials(&cell, "unauthorized"),
            "the first rejection heals"
        );
        assert!(fence.load_credentials(&key).is_none());
        assert_eq!(
            fence.decide(&cell, "node_registry"),
            AuthorizeDecision::Mint
        );

        // Bounded: a second rejection is not a stale file, and a heal loop
        // would author a CapGrant per failed call.
        fence.store_credentials(&key, &good_credentials());
        assert!(
            !fence.discard_stale_credentials(&cell, "unauthorized"),
            "the heal is once per cell per process"
        );
        assert!(fence.load_credentials(&key).is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_rejected_grant_never_heals_on_a_closed_cell() {
        // Discarding here would invite the next connect to mint on a sealed
        // chain — the exact action this module exists to prevent.
        let dir = tmp_dir("stale-grant-closed");
        let fence = ClosedChainFence::open(&dir);
        let cell = test_cell(36, 37);
        let key = cell_key(&cell);
        fence.store_credentials(&key, &good_credentials());
        fence.record_closed(&cell, Some("node_registry"), None, "sealed", 1);

        assert!(!fence.discard_stale_credentials(&cell, "unauthorized"));
        assert!(
            fence.load_credentials(&key).is_some(),
            "an unreachable closed cell is the correct answer, not a fresh grant"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn good_credentials() -> holochain_client::SigningCredentials {
        holochain_client::SigningCredentials {
            signing_agent_key: AgentPubKey::from_raw_32(vec![38; 32]),
            keypair: ed25519_dalek::SigningKey::from_bytes(&[39u8; 32]),
            cap_secret: holochain_types::prelude::CapSecret::from([40u8; 64]),
        }
    }

    #[test]
    fn cell_keys_separate_dna_from_agent() {
        assert_ne!(cell_key(&test_cell(1, 2)), cell_key(&test_cell(2, 1)));
        assert_eq!(cell_key(&test_cell(1, 2)), cell_key(&test_cell(1, 2)));
    }
}
