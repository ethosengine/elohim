//! Provide-loop reconciler (Slice 2b) — the P1 controller that turns
//! caught-up commons pins into notarized `replicates-commons` Commitments.
//!
//! ## Desired vs actual (P1 reconciliation)
//!
//! - **desired** = active `item` pins whose acquisition tracker reports
//!   caught-up AND whose content has `reach == "commons"` (you can only offer
//!   to the commons what you have fully fetched and what the commons may hold).
//! - **actual** = this provider's non-revoked `replicates-commons` Commitments,
//!   keyed by the LOGICAL key `(provider, head_ref)` (head_ref == recipient on
//!   the commitment row).
//!
//! The diff authors a Commitment only when a desired key has NO live actual
//! row, and authors a revocation only when a live actual row has NO desired key
//! (un-pin → withdraw the offer; the revocation arm itself lands in T10).
//!
//! ## Restart safety (the load-bearing property)
//!
//! The in-memory `latch` is a pure optimisation. On process restart it is
//! empty, but the actual set is re-derived from the DHT projection
//! (`live_commons_provides_for_provider`), so a key already provided before the
//! restart is found in `actual` and is NOT re-authored. LOGICAL-KEY dedup —
//! not the latch — is the author-once guarantee. The latch only suppresses
//! redundant authoring attempts *within* a process lifetime.
//!
//! Authoring rides a [`CommitmentAuthor`] seam so the dedup/idempotency/restart
//! logic is unit-testable without a live conductor (mirrors `CommitmentFetcher`).
//!
//! Spec: 2026-06-08-epr-acquisition-slice2b-provide-loop-design.md.

use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::DbPool;
use crate::error::StorageError;

/// Lifecycle of one logical provide `(provider, head_ref)` as the reconciler
/// observes it. Category C (in-memory; recomputed on restart from the
/// projection + pin tables). `Projected`/`Active` distinguish a freshly
/// authored-but-ungraduated commitment from one a provide event has graduated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvideStage {
    /// Desired but no live commitment yet — the next tick will author.
    NeedsCommitment,
    /// Author call issued this lifetime; awaiting projection.
    Authoring,
    /// A live `replicates-commons` row exists (proposed).
    Projected,
    /// Announce step in flight (reserved for the gossip-announce follow-on).
    Announcing,
    /// A live row exists and has graduated to 'active'.
    Active,
    /// The pin was removed; a revocation has been authored.
    Revoked,
}

/// A single logical provide the reconciler decided to author or revoke.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvideAuthorRequest {
    pub provider: String,
    pub head_ref: String,
}

/// A revocation the reconciler decided to author for a removed pin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvideRevokeRequest {
    /// action_hash of the live commitment to revoke (the pin's commitment_cid).
    pub target_cid: String,
    pub head_ref: String,
}

/// Author seam — the conductor write of a `replicates-commons` Commitment or a
/// `revokes-commitment`. Production impl wraps `conductor_writes`; tests use
/// [`MockAuthor`] to assert exactly-once authoring across ticks and restarts.
#[async_trait]
pub trait CommitmentAuthor: Send + Sync {
    /// Author a replicates-commons Commitment for the given provide. Returns
    /// the new commitment action_hash (the back-reference stored on the pin).
    async fn author_commons(&self, req: &ProvideAuthorRequest) -> Result<String, StorageError>;

    /// Author a revokes-commitment targeting `req.target_cid`.
    async fn revoke_commons(&self, req: &ProvideRevokeRequest) -> Result<(), StorageError>;
}

/// In-memory author for unit tests. Records every author/revoke call so a test
/// can assert exactly-once semantics.
#[derive(Debug, Default)]
pub struct MockAuthor {
    pub authored: Mutex<Vec<ProvideAuthorRequest>>,
    pub revoked: Mutex<Vec<ProvideRevokeRequest>>,
    next_cid: std::sync::atomic::AtomicU64,
}

impl MockAuthor {
    pub fn new() -> Self {
        Self::default()
    }
    pub async fn authored_keys(&self) -> Vec<(String, String)> {
        self.authored
            .lock()
            .await
            .iter()
            .map(|r| (r.provider.clone(), r.head_ref.clone()))
            .collect()
    }
}

#[async_trait]
impl CommitmentAuthor for MockAuthor {
    async fn author_commons(&self, req: &ProvideAuthorRequest) -> Result<String, StorageError> {
        self.authored.lock().await.push(req.clone());
        let n = self
            .next_cid
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(format!("uhCkk-mock-commit-{n}"))
    }
    async fn revoke_commons(&self, req: &ProvideRevokeRequest) -> Result<(), StorageError> {
        self.revoked.lock().await.push(req.clone());
        Ok(())
    }
}

/// One desired provide derived from a caught-up commons pin.
#[derive(Debug, Clone)]
pub struct DesiredProvide {
    pub pin_id: i32,
    pub head_ref: String,
    /// Pre-existing commitment_cid back-reference (set on a prior tick).
    pub commitment_cid: Option<String>,
}

/// The provide-loop controller. Holds the per-process stage latch keyed by the
/// LOGICAL key `(provider, head_ref)`.
pub struct ProvideReconciler {
    /// `(provider, head_ref)` → stage. Pure optimisation — emptied on restart.
    latch: Arc<Mutex<HashMap<(String, String), ProvideStage>>>,
}

impl Default for ProvideReconciler {
    fn default() -> Self {
        Self::new()
    }
}

impl ProvideReconciler {
    pub fn new() -> Self {
        Self {
            latch: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Read the current stage for a logical key (test/diagnostic accessor).
    pub async fn stage(&self, provider: &str, head_ref: &str) -> Option<ProvideStage> {
        self.latch
            .lock()
            .await
            .get(&(provider.to_string(), head_ref.to_string()))
            .copied()
    }

    /// One reconcile pass over the supplied desired set against the DHT-projected
    /// actual set, authoring/back-filling/revoking through `author`.
    ///
    /// `self_provider` is this peer's identity (the commitment provider).
    /// `desired` is the caught-up commons pin set (already filtered by the
    /// caller — see [`Self::derive_desired`]). The actual set is read fresh from
    /// the projection so dedup survives a restart.
    ///
    /// Returns the number of NEW author calls issued this pass (for metrics/tests).
    pub async fn reconcile_provides<A: CommitmentAuthor + ?Sized>(
        &self,
        pool: &DbPool,
        author: &A,
        self_provider: &str,
        desired: &[DesiredProvide],
    ) -> Result<usize, StorageError> {
        // ── actual: live (non-revoked) replicates-commons logical keys ──────
        let actual_rows = {
            let mut conn = pool
                .get()
                .map_err(|e| StorageError::Internal(format!("pool: {e}")))?;
            crate::db::mishpat_commitments::live_commons_provides_for_provider(
                &mut conn,
                self_provider,
            )
            .map_err(|e| StorageError::Database(e.to_string()))?
        };
        // logical key (provider, head_ref==recipient) → its row state.
        let mut actual: HashMap<(String, String), String> = HashMap::new();
        for row in &actual_rows {
            actual.insert(
                (row.provider.clone(), row.recipient.clone()),
                row.state.clone(),
            );
        }

        let desired_keys: HashSet<(String, String)> = desired
            .iter()
            .map(|d| (self_provider.to_string(), d.head_ref.clone()))
            .collect();

        let mut authored = 0usize;
        let mut latch = self.latch.lock().await;

        // ── author arm: desired keys with no live actual row ────────────────
        for d in desired {
            let key = (self_provider.to_string(), d.head_ref.clone());

            if let Some(state) = actual.get(&key) {
                // A live commitment already exists (restart re-derivation lands
                // here even with an empty latch) — never re-author.
                let stage = if state == "active" {
                    ProvideStage::Active
                } else {
                    ProvideStage::Projected
                };
                latch.insert(key, stage);
                continue;
            }

            // No live row. If we already issued an author this lifetime and are
            // awaiting its projection, do not re-issue (within-process dedup).
            if matches!(latch.get(&key), Some(ProvideStage::Authoring)) {
                continue;
            }

            latch.insert(key.clone(), ProvideStage::Authoring);
            let req = ProvideAuthorRequest {
                provider: self_provider.to_string(),
                head_ref: d.head_ref.clone(),
            };
            match author.author_commons(&req).await {
                Ok(cid) => {
                    authored += 1;
                    // Back-fill the pin's commitment_cid back-reference.
                    if let Ok(mut conn) = pool.get() {
                        let _ = crate::db::acquisition_pins::set_commitment_cid(
                            &mut conn, d.pin_id, &cid,
                        );
                    }
                }
                Err(e) => {
                    // Roll the latch back so the next tick retries.
                    latch.insert(key, ProvideStage::NeedsCommitment);
                    tracing::warn!(
                        target: "elohim_storage::provide",
                        head_ref = %d.head_ref,
                        error = %e,
                        "provide reconcile: author_commons failed; will retry next tick"
                    );
                }
            }
        }

        // ── revoke arm: live actual rows with no desired key (un-pinned) ─────
        // (Full pin→commitment_cid revocation flow lands in T10; here we revoke
        // a stranded live commitment whose logical key left the desired set.)
        for row in &actual_rows {
            let key = (row.provider.clone(), row.recipient.clone());
            if desired_keys.contains(&key) {
                continue;
            }
            let req = ProvideRevokeRequest {
                target_cid: row
                    .dht_anchor_hash
                    .clone()
                    .unwrap_or_else(|| row.cid.clone()),
                head_ref: row.recipient.clone(),
            };
            if let Err(e) = author.revoke_commons(&req).await {
                tracing::warn!(
                    target: "elohim_storage::provide",
                    head_ref = %row.recipient,
                    error = %e,
                    "provide reconcile: revoke_commons failed; will retry next tick"
                );
                continue;
            }
            latch.insert(key, ProvideStage::Revoked);
        }

        Ok(authored)
    }

    /// Observe-only pass: re-derive the per-key stage latch from the live DHT
    /// projection WITHOUT authoring or revoking. This is the controller's
    /// read-half — it is what restores restart-safe stages when the in-memory
    /// latch is empty (e.g. on the first tick after process start, before any
    /// conductor author seam is wired). For each desired key it latches
    /// `Active`/`Projected` if a live commitment row exists, else
    /// `NeedsCommitment` (the next authoring tick, once an author seam is
    /// threaded, will act on those). Returns the number of desired keys that
    /// still need a commitment (no live actual row).
    ///
    /// Authoring proper goes through [`Self::reconcile_provides`] with a live
    /// [`CommitmentAuthor`]; this method exists so the loop can keep the latch
    /// current even where the conductor author seam is not yet composed.
    pub async fn observe(
        &self,
        pool: &DbPool,
        self_provider: &str,
        desired: &[DesiredProvide],
    ) -> Result<usize, StorageError> {
        let actual_rows = {
            let mut conn = pool
                .get()
                .map_err(|e| StorageError::Internal(format!("pool: {e}")))?;
            crate::db::mishpat_commitments::live_commons_provides_for_provider(
                &mut conn,
                self_provider,
            )
            .map_err(|e| StorageError::Database(e.to_string()))?
        };
        let mut actual: HashMap<(String, String), String> = HashMap::new();
        for row in &actual_rows {
            actual.insert(
                (row.provider.clone(), row.recipient.clone()),
                row.state.clone(),
            );
        }

        let mut needs = 0usize;
        let mut latch = self.latch.lock().await;
        for d in desired {
            let key = (self_provider.to_string(), d.head_ref.clone());
            match actual.get(&key) {
                Some(state) if state == "active" => {
                    latch.insert(key, ProvideStage::Active);
                }
                Some(_) => {
                    latch.insert(key, ProvideStage::Projected);
                }
                None => {
                    needs += 1;
                    // Do not overwrite an in-flight Authoring latch from this
                    // process lifetime with NeedsCommitment.
                    if !matches!(latch.get(&key), Some(ProvideStage::Authoring)) {
                        latch.insert(key, ProvideStage::NeedsCommitment);
                    }
                }
            }
        }
        Ok(needs)
    }

    /// Derive the desired provide set: active `item` pins that are caught-up
    /// (acquisition byte-arrival complete) AND whose content is `reach=="commons"`.
    /// Caught-up ids come from the live `AcquisitionState` rollup; the caller
    /// passes the set of head_refs the acquisition stream reports complete.
    pub fn derive_desired(
        pins: &[crate::db::models::AcquisitionPin],
        caught_up_head_refs: &HashSet<String>,
        commons_head_refs: &HashSet<String>,
    ) -> Vec<DesiredProvide> {
        pins.iter()
            .filter(|p| p.kind == "item" && p.status == "active")
            .filter(|p| caught_up_head_refs.contains(&p.head_ref))
            .filter(|p| commons_head_refs.contains(&p.head_ref))
            .map(|p| DesiredProvide {
                pin_id: p.id,
                head_ref: p.head_ref.clone(),
                commitment_cid: p.commitment_cid.clone(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::NewMishpatCommitment;
    use crate::test_util::test_pool;

    fn desired(pin_id: i32, head_ref: &str) -> DesiredProvide {
        DesiredProvide {
            pin_id,
            head_ref: head_ref.to_string(),
            commitment_cid: None,
        }
    }

    fn seed_commons_row(pool: &DbPool, cid: &str, recipient: &str, state: &str, revoked: bool) {
        let mut conn = pool.get().expect("conn");
        crate::db::mishpat_commitments::upsert_with_anchor(
            &mut conn,
            NewMishpatCommitment {
                cid: cid.to_string(),
                action: "replicates-commons".to_string(),
                scope: "replicates-commons".to_string(),
                provider: "agent:self".to_string(),
                recipient: recipient.to_string(),
                bounds_json: r#"{"rate_per_minute":6,"reach_ceiling":"commons"}"#.to_string(),
                valid_from: "2026-06-01T00:00:00Z".to_string(),
                valid_until: "2026-12-01T00:00:00Z".to_string(),
                revoked_at: if revoked {
                    Some("2026-06-10T00:00:00Z".to_string())
                } else {
                    None
                },
                state: state.to_string(),
                dht_anchor_hash: Some(format!("anchor-{cid}")),
            },
        )
        .expect("seed commons row");
    }

    #[tokio::test]
    async fn authors_once_per_logical_key() {
        let pool = test_pool();
        let author = MockAuthor::new();
        let r = ProvideReconciler::new();
        let desired = vec![desired(1, "epr:album-1"), desired(2, "epr:album-2")];

        let n = r
            .reconcile_provides(&pool, &author, "agent:self", &desired)
            .await
            .expect("first pass");
        assert_eq!(n, 2, "two unprovided keys → two authors");
        assert_eq!(author.authored.lock().await.len(), 2);

        // Latch now says Authoring for both — a second pass with NO projection
        // landing must NOT re-author (within-process dedup).
        let n2 = r
            .reconcile_provides(&pool, &author, "agent:self", &desired)
            .await
            .expect("second pass");
        assert_eq!(n2, 0, "Authoring-latched keys must not re-author");
        assert_eq!(author.authored.lock().await.len(), 2);
    }

    #[tokio::test]
    async fn restart_rederives_from_projection_no_double_author() {
        // Simulate: a prior process authored + the projection landed (a live
        // proposed row exists). A FRESH reconciler (empty latch == restart)
        // must find the actual row and NOT re-author.
        let pool = test_pool();
        seed_commons_row(&pool, "cid:already", "epr:album-1", "proposed", false);

        let author = MockAuthor::new();
        let fresh = ProvideReconciler::new(); // empty latch — the restart case
        let n = fresh
            .reconcile_provides(&pool, &author, "agent:self", &[desired(1, "epr:album-1")])
            .await
            .expect("restart pass");

        assert_eq!(
            n, 0,
            "a key already live in the projection must not re-author"
        );
        assert!(
            author.authored.lock().await.is_empty(),
            "restart re-derivation = zero author calls"
        );
        assert_eq!(
            fresh.stage("agent:self", "epr:album-1").await,
            Some(ProvideStage::Projected),
            "an existing proposed row latches Projected"
        );
    }

    #[tokio::test]
    async fn graduated_row_latches_active() {
        let pool = test_pool();
        seed_commons_row(&pool, "cid:grad", "epr:album-1", "active", false);
        let author = MockAuthor::new();
        let r = ProvideReconciler::new();
        r.reconcile_provides(&pool, &author, "agent:self", &[desired(1, "epr:album-1")])
            .await
            .expect("pass");
        assert_eq!(
            r.stage("agent:self", "epr:album-1").await,
            Some(ProvideStage::Active),
            "an active projection row latches Active"
        );
        assert!(author.authored.lock().await.is_empty());
    }

    #[tokio::test]
    async fn revoked_projection_row_is_not_actual_so_key_reauthors() {
        // A revoked commitment is NOT a live actual row — a still-desired key
        // must author a fresh commitment (the old offer was withdrawn).
        let pool = test_pool();
        seed_commons_row(&pool, "cid:was-revoked", "epr:album-1", "proposed", true);
        let author = MockAuthor::new();
        let r = ProvideReconciler::new();
        let n = r
            .reconcile_provides(&pool, &author, "agent:self", &[desired(1, "epr:album-1")])
            .await
            .expect("pass");
        assert_eq!(n, 1, "a revoked row does not satisfy the desired key");
        assert_eq!(author.authored.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn stranded_live_row_with_no_desired_key_is_revoked() {
        // A live commitment whose logical key left the desired set (un-pinned)
        // gets a revocation authored.
        let pool = test_pool();
        seed_commons_row(&pool, "cid:stranded", "epr:gone", "proposed", false);
        let author = MockAuthor::new();
        let r = ProvideReconciler::new();
        // desired set is empty for that key.
        r.reconcile_provides(&pool, &author, "agent:self", &[])
            .await
            .expect("pass");
        let revoked = author.revoked.lock().await;
        assert_eq!(revoked.len(), 1, "stranded live row → one revocation");
        assert_eq!(revoked[0].target_cid, "anchor-cid:stranded");
        assert_eq!(revoked[0].head_ref, "epr:gone");
    }

    #[tokio::test]
    async fn observe_latches_stage_from_projection_without_authoring() {
        // observe() is the controller's read-half: it must latch stages from the
        // live projection (Active/Projected/NeedsCommitment) without ever
        // authoring or revoking.
        let pool = test_pool();
        seed_commons_row(&pool, "cid:obs-active", "epr:active", "active", false);
        seed_commons_row(&pool, "cid:obs-proposed", "epr:proposed", "proposed", false);
        // epr:gap has no live row → NeedsCommitment, but no author call.
        let r = ProvideReconciler::new();
        let desired = vec![
            desired(1, "epr:active"),
            desired(2, "epr:proposed"),
            desired(3, "epr:gap"),
        ];

        let needs = r
            .observe(&pool, "agent:self", &desired)
            .await
            .expect("observe");
        assert_eq!(needs, 1, "exactly one desired key lacks a live actual row");

        assert_eq!(
            r.stage("agent:self", "epr:active").await,
            Some(ProvideStage::Active)
        );
        assert_eq!(
            r.stage("agent:self", "epr:proposed").await,
            Some(ProvideStage::Projected)
        );
        assert_eq!(
            r.stage("agent:self", "epr:gap").await,
            Some(ProvideStage::NeedsCommitment),
            "a desired key with no live row latches NeedsCommitment"
        );
    }

    #[test]
    fn derive_desired_filters_non_caught_up_and_non_commons() {
        use crate::db::models::AcquisitionPin;
        let pin = |id: i32, head: &str, status: &str| AcquisitionPin {
            id,
            agent_pub_key: "local-device".to_string(),
            head_ref: head.to_string(),
            kind: "item".to_string(),
            closure_rule_json: None,
            priority: 1,
            status: status.to_string(),
            created_at: "t".to_string(),
            updated_at: "t".to_string(),
            commitment_cid: None,
        };
        let pins = vec![
            pin(1, "epr:ready-commons", "active"),
            pin(2, "epr:not-caught-up", "active"),
            pin(3, "epr:caught-up-not-commons", "active"),
            pin(4, "epr:paused", "paused"),
        ];
        let caught: HashSet<String> = [
            "epr:ready-commons",
            "epr:caught-up-not-commons",
            "epr:paused",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let commons: HashSet<String> = ["epr:ready-commons", "epr:not-caught-up", "epr:paused"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let d = ProvideReconciler::derive_desired(&pins, &caught, &commons);
        assert_eq!(
            d.len(),
            1,
            "only the caught-up commons active pin is desired"
        );
        assert_eq!(d[0].head_ref, "epr:ready-commons");
    }
}
