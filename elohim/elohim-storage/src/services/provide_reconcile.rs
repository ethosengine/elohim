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

/// Pin `kind` the provide loop authors offers for (single-item pins). Cluster
/// pins are a later slice and never enter the desired set.
const PIN_KIND_ITEM: &str = "item";
/// Pin `status` that is eligible for a commons offer (removed/paused pins are
/// excluded; a removed pin's offer is revoked, not re-authored).
const PIN_STATUS_ACTIVE: &str = "active";

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
    /// The CONTENT's own reach (Stage B), threaded from the desired pin's
    /// content row. The author writes this as the commitment's top-level
    /// `reach` (the `reach_ceiling` bound stays "commons"); the projection scopes
    /// the `content:<reach>` provide row by it. Defaults to "commons" when the
    /// caller has no per-content reach (back-compat with the commons loop).
    pub reach: String,
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

/// Reach-aware provide-eligibility seam.
///
/// Off-commons, "may this node provide content X at reach R?" is a privacy
/// question, not just a presence question: a household-reach record is not
/// openly providable; the node must have **embodied responsibility** for the
/// scope (the same predicate that decides receiver-side pre-authorization — you
/// may provide a scope iff you have standing to hold it). Commons stays openly
/// providable.
///
/// The seam takes `(head_ref, reach)` candidates (caught-up AND locally present)
/// and returns the head_refs the node is eligible to provide. Extracting it
/// behind a trait (mirroring [`CommitmentAuthor`]/[`MockAuthor`]) lets the
/// admit/reject decision be unit-tested without a live DB pool — the production
/// resolver reads `peer_identity_bindings`; the mock injects the eligible set.
pub trait ProvideEligibility: Send + Sync {
    /// Filter `candidates` (`(head_ref, reach)`) to the head_refs this node is
    /// eligible to provide.
    fn eligible_head_refs(&self, candidates: &[(String, String)]) -> HashSet<String>;
}

/// Production eligibility resolver: builds the scope topic per candidate and
/// runs [`classify_pre_authorization`](crate::p2p::classify_pre_authorization).
///
/// `node_has_embodied_responsibility` is resolved ONCE (Stage 1 ignores the
/// topic — it answers "does this node have any standing at all"); Stage 2/3
/// will thread per-scope graph walks through the same call without changing
/// this seam. The pillar is supplied here (no `pillar` column on content);
/// `community` reach additionally carries the collective segment when known
/// (none at Stage 1 — `None`), riding the topic ladder so Stage 2/3 tightening
/// flows through with no call-site rewrite.
///
/// Gated on the `p2p` feature: the classifier + topic machinery live in the
/// `p2p` module. The `ProvideEligibility` trait itself is ungated so a
/// non-p2p build (and the unit tests) can still inject a mock resolver.
#[cfg(feature = "p2p")]
pub struct ClassifierEligibility {
    pool: Arc<DbPool>,
    pillar: String,
}

#[cfg(feature = "p2p")]
impl ClassifierEligibility {
    pub fn new(pool: Arc<DbPool>, pillar: impl Into<String>) -> Self {
        Self {
            pool,
            pillar: pillar.into(),
        }
    }
}

#[cfg(feature = "p2p")]
impl ProvideEligibility for ClassifierEligibility {
    fn eligible_head_refs(&self, candidates: &[(String, String)]) -> HashSet<String> {
        use crate::p2p::{classify_pre_authorization, topics::topic_for, PreAuthorizationDecision};

        if candidates.is_empty() {
            return HashSet::new();
        }
        // Resolve the embodied-responsibility boolean once. NOTE (flag, not
        // fixed here): this resolver fails OPEN on pool/query error (returns
        // true), so under DB stress it can admit a node that lacks standing —
        // existing Stage 1 behavior, tracked with the receiver-side gate, not
        // re-designed in this change.
        let has_responsibility =
            crate::p2p::reach_authorization::node_has_embodied_responsibility(&self.pool, "");

        candidates
            .iter()
            .filter(|(_head_ref, reach)| {
                // Stage B DEGRADE-OPEN: a reach OUTSIDE the schema-8 DNA
                // vocabulary (`local`, `household`, `neighborhood`, …) does not
                // parse to `elohim_epr::Reach` and `topic_for` is `Reach`-typed,
                // so we cannot build its canonical topic. Rather than DROP it
                // (the Stage-A storage projection already reads such reaches
                // through, so dropping here would silently starve household/local
                // content of a provide author — the regression this prevents), we
                // ADMIT it. Consistent with the storage-side degrade-open: the
                // schema-8 vocabulary makes no claim about the others.
                let Some(reach_enum) = parse_reach_for_topic(reach) else {
                    return true;
                };
                let topic = topic_for(&self.pillar, reach_enum, None);
                matches!(
                    classify_pre_authorization(&topic, has_responsibility),
                    PreAuthorizationDecision::Standing
                )
            })
            .map(|(head_ref, _reach)| head_ref.clone())
            .collect()
    }
}

/// Parse a reach string to [`elohim_epr::Reach`] for topic construction.
/// Mirrors the storage-side read-through (`mishpat_projection::parse_reach`):
/// membership in the canonical reach vocabulary is "the parse succeeds".
#[cfg(feature = "p2p")]
fn parse_reach_for_topic(reach: &str) -> Option<elohim_epr::Reach> {
    serde_json::from_value::<elohim_epr::Reach>(serde_json::Value::String(reach.to_string())).ok()
}

/// One desired provide derived from a caught-up pin.
#[derive(Debug, Clone)]
pub struct DesiredProvide {
    pub pin_id: i32,
    pub head_ref: String,
    /// The CONTENT's own reach (Stage B), carried from the eligibility
    /// candidates `(head_ref, reach)` map so the authored commitment declares the
    /// content's reach rather than a hardcoded "commons". Defaults to "commons"
    /// when the caller supplies no per-content reach map.
    pub reach: String,
    /// Pre-existing commitment_cid back-reference (set on a prior tick).
    ///
    /// Read by the T10 un-pin revoke path (`http::handle_remove_pin`), which
    /// targets this CID directly to author a `revokes-commitment` when the pin is
    /// removed. Now wired — kept here so the desired set carries the back-ref
    /// even for callers that only observe.
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
                reach: d.reach.clone(),
            };
            match author.author_commons(&req).await {
                Ok(cid) => {
                    authored += 1;
                    // Back-fill the pin's commitment_cid back-reference. The T10
                    // un-pin revoke path reads this, so a silent failure here
                    // would later strand the offer (un-pin couldn't target it) —
                    // warn loudly. The reconciler's stranded-row revoke arm is
                    // the eventual backstop, but the back-ref is the fast path.
                    match pool.get() {
                        Ok(mut conn) => {
                            if let Err(e) = crate::db::acquisition_pins::set_commitment_cid(
                                &mut conn, d.pin_id, &cid,
                            ) {
                                tracing::warn!(
                                    target: "elohim_storage::provide",
                                    pin_id = d.pin_id,
                                    commitment_cid = %cid,
                                    error = %e,
                                    "provide reconcile: set_commitment_cid back-fill failed; \
                                     un-pin revoke will fall back to the stranded-row arm"
                                );
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                target: "elohim_storage::provide",
                                pin_id = d.pin_id,
                                error = %e,
                                "provide reconcile: pool.get failed for commitment_cid back-fill"
                            );
                        }
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

        // Prune the per-process latch. It is a pure optimisation, but without a
        // sweep it accumulates a key for every (provider, head_ref) ever seen —
        // notably terminal `Revoked` keys from un-pinned offers, which otherwise
        // never leave. Keep only keys still desired or still live in `actual`; a
        // key that is re-needed next pass is re-inserted by the walks above.
        let live: HashSet<(String, String)> = actual.keys().cloned().collect();
        latch.retain(|k, _| desired_keys.contains(k) || live.contains(k));

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
    /// (acquisition byte-arrival complete) AND that this node is **eligible to
    /// provide** at their content's reach. Caught-up ids come from the live
    /// `AcquisitionState` rollup; the eligible set is computed by the caller via
    /// the reach-aware [`ProvideEligibility`] seam (commons is openly providable;
    /// non-commons requires embodied responsibility for the scope). Staying pure
    /// — the caller resolves both sets and passes them in — keeps the diff/dedup
    /// logic unit-testable without a live pool.
    ///
    /// `reach_by_head_ref` carries the CONTENT's own reach per head_ref (Stage B,
    /// from the eligibility `(head_ref, reach)` candidate map) so the authored
    /// commitment declares the content's reach. A head_ref missing from the map
    /// defaults to "commons" (back-compat: the commons loop never built one).
    pub fn derive_desired(
        pins: &[crate::db::models::AcquisitionPin],
        caught_up_head_refs: &HashSet<String>,
        provide_eligible_head_refs: &HashSet<String>,
        reach_by_head_ref: &HashMap<String, String>,
    ) -> Vec<DesiredProvide> {
        pins.iter()
            .filter(|p| p.kind == PIN_KIND_ITEM && p.status == PIN_STATUS_ACTIVE)
            .filter(|p| caught_up_head_refs.contains(&p.head_ref))
            .filter(|p| provide_eligible_head_refs.contains(&p.head_ref))
            .map(|p| DesiredProvide {
                pin_id: p.id,
                head_ref: p.head_ref.clone(),
                reach: reach_by_head_ref
                    .get(&p.head_ref)
                    .cloned()
                    .unwrap_or_else(|| "commons".to_string()),
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
            reach: "commons".to_string(),
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
    fn derive_desired_filters_non_caught_up_and_non_eligible() {
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
            pin(1, "epr:ready-eligible", "active"),
            pin(2, "epr:not-caught-up", "active"),
            pin(3, "epr:caught-up-not-eligible", "active"),
            pin(4, "epr:paused", "paused"),
        ];
        let caught: HashSet<String> = [
            "epr:ready-eligible",
            "epr:caught-up-not-eligible",
            "epr:paused",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        // The eligible set (reach-aware, computed by the caller's seam) is the
        // third filter. Only a pin that is BOTH caught-up AND eligible is desired.
        let eligible: HashSet<String> = ["epr:ready-eligible", "epr:not-caught-up", "epr:paused"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        // Reach map (Stage B): the surviving pin carries its content's own reach.
        let reaches: HashMap<String, String> = [("epr:ready-eligible", "household")]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        let d = ProvideReconciler::derive_desired(&pins, &caught, &eligible, &reaches);
        assert_eq!(
            d.len(),
            1,
            "only the caught-up AND eligible active pin is desired"
        );
        assert_eq!(d[0].head_ref, "epr:ready-eligible");
        assert_eq!(
            d[0].reach, "household",
            "the desired provide carries the content's own reach from the map"
        );
    }

    #[test]
    fn derive_desired_reach_defaults_to_commons_when_unmapped() {
        use crate::db::models::AcquisitionPin;
        let pin = AcquisitionPin {
            id: 1,
            agent_pub_key: "local-device".to_string(),
            head_ref: "epr:no-reach-map".to_string(),
            kind: "item".to_string(),
            closure_rule_json: None,
            priority: 1,
            status: "active".to_string(),
            created_at: "t".to_string(),
            updated_at: "t".to_string(),
            commitment_cid: None,
        };
        let caught: HashSet<String> = ["epr:no-reach-map"].iter().map(|s| s.to_string()).collect();
        let eligible = caught.clone();
        let empty_reaches: HashMap<String, String> = HashMap::new();
        let d = ProvideReconciler::derive_desired(&[pin], &caught, &eligible, &empty_reaches);
        assert_eq!(d.len(), 1);
        assert_eq!(
            d[0].reach, "commons",
            "an unmapped head_ref defaults to commons (back-compat)"
        );
    }

    /// Mock eligibility seam — admit a fixed allow-list, reject the rest. Mirrors
    /// `MockAuthor`: it lets admit/reject be asserted without a live DB pool.
    struct MockEligibility {
        admit: HashSet<String>,
    }
    impl ProvideEligibility for MockEligibility {
        fn eligible_head_refs(&self, candidates: &[(String, String)]) -> HashSet<String> {
            candidates
                .iter()
                .filter(|(head_ref, _reach)| self.admit.contains(head_ref))
                .map(|(head_ref, _)| head_ref.clone())
                .collect()
        }
    }

    #[test]
    fn eligibility_seam_admits_allowlisted_rejects_rest() {
        let resolver = MockEligibility {
            admit: ["epr:household-eligible"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        };
        let candidates = vec![
            (
                "epr:household-eligible".to_string(),
                "household".to_string(),
            ),
            (
                "epr:household-no-standing".to_string(),
                "household".to_string(),
            ),
        ];
        let eligible = resolver.eligible_head_refs(&candidates);
        assert!(
            eligible.contains("epr:household-eligible"),
            "a head_ref the node has standing for is admitted"
        );
        assert!(
            !eligible.contains("epr:household-no-standing"),
            "a head_ref the node lacks standing for is rejected (privacy gate)"
        );
        assert_eq!(eligible.len(), 1);
    }

    /// Stage B DEGRADE-OPEN: the production classifier admits commons
    /// unconditionally (commons is openly providable). A reach OUTSIDE the
    /// schema-8 vocabulary (`household`, `local`) does not parse to a topic, so
    /// the classifier now ADMITS it (degrade-open) rather than dropping it — the
    /// regression fix that lets household/local content get a provide author.
    /// A schema-8 *bound-tier* reach (e.g. `trusted`) still requires embodied
    /// responsibility and is rejected without standing.
    #[cfg(feature = "p2p")]
    #[test]
    fn classifier_eligibility_admits_commons_and_degrades_open_for_household() {
        use crate::test_util::test_pool;
        let pool = std::sync::Arc::new(test_pool());
        let resolver = ClassifierEligibility::new(pool, "lamad");
        let candidates = vec![
            ("epr:commons-item".to_string(), "commons".to_string()),
            ("epr:household-item".to_string(), "household".to_string()),
            ("epr:local-item".to_string(), "local".to_string()),
            ("epr:trusted-item".to_string(), "trusted".to_string()),
        ];
        let eligible = resolver.eligible_head_refs(&candidates);
        assert!(
            eligible.contains("epr:commons-item"),
            "commons is openly providable — always admitted"
        );
        assert!(
            eligible.contains("epr:household-item"),
            "household is outside schema-8 → degrade-open admits it (Stage B)"
        );
        assert!(
            eligible.contains("epr:local-item"),
            "local is outside schema-8 → degrade-open admits it (Stage B)"
        );
        assert!(
            !eligible.contains("epr:trusted-item"),
            "trusted IS schema-8 (bound-tier); requires embodied responsibility; none seeded → rejected"
        );
    }
}
