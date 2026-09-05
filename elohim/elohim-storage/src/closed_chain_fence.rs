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
/// predecessor for precisely this reason). The last three are the additional
/// write-free node-registry externs that STORAGE calls on a v1 cell:
/// `known_agents` from the trailing lineage-bridge sweep
/// ([`crate::services::lineage_bridge`]), and the two export walks the bridge
/// and carry paths read pages through. Every one was checked write-free in the
/// coordinator source before being listed.
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
    /// 1. **Persisted credentials win, closed or open.** The grant they name is
    ///    already on the chain; re-minting would author a second one for no
    ///    gain. This is what makes a `storage-restart` write-free.
    /// 2. **A closed cell with no persisted credentials is refused by name.**
    ///    Nothing reaches the conductor.
    /// 3. **Otherwise mint once and persist immediately**, so the next restart
    ///    takes branch 1.
    pub fn decide(&self, cell: &CellId, label: &str) -> AuthorizeDecision {
        let key = cell_key(cell);
        if let Some(stored) = self.load_credentials(&key) {
            return AuthorizeDecision::Reuse(Box::new(stored));
        }
        if let Some(record) = self.closed_record(cell) {
            return AuthorizeDecision::Refuse(format!(
                "refusing to authorize signing credentials on a CLOSED chain (cell {cell}, \
                 {why}) for '{label}': authorize_signing_credentials COMMITS a CapGrant, and a \
                 post-close action is warranted by every neighbour into a permanent cell block \
                 that holochain 0.7 cannot lift. No pre-close credentials are persisted for this \
                 cell, so there is no write-free way to reach it — this role stays unconnected \
                 (503) rather than partitioning the mesh.",
                cell = record.cell,
                why = record.why,
            ));
        }
        AuthorizeDecision::Mint
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
fn write_private(path: &Path, body: &str) {
    if let Err(e) = std::fs::write(path, body) {
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

    #[test]
    fn cell_keys_separate_dna_from_agent() {
        assert_ne!(cell_key(&test_cell(1, 2)), cell_key(&test_cell(2, 1)));
        assert_eq!(cell_key(&test_cell(1, 2)), cell_key(&test_cell(1, 2)));
    }
}
