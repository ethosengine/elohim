//! Compute-payload custody under the hosting promise.
//!
//! A stewarded runner (e.g. `agent:ci-runner-jenkins-matthew`) is a HOSTED
//! agent: its cell runs on a pool peer's conductor and it has no storage peer
//! of its own. A peer-executed stage still needs its input chunks held where a
//! provider can draw them over the native blob race, and its output chunks
//! drawn back. The pool peer that SPONSORS the runner already promised to host
//! it — a `hosted-cell` `delegates-compute` commitment this peer issued, with
//! the runner as recipient. This module lets that same promise cover holding
//! the runner's compute chunks, and nothing wider.
//!
//! # Who may act, and under what
//!
//! Two arms reach `/api/v1/compute/payloads/{cid}`, and they never mix
//! ([`classify_payload_caller`]):
//! - **local**: the node-local capability token, no performer — the operator's
//!   own workspace tools, exactly as before;
//! - **hosted**: a doorway-verified performer (`x-elohim-verified-performer`,
//!   which the doorway strips from every inbound request and re-injects from
//!   its verified JWT). When a performer is present the token is NOT
//!   consulted, so a doorway-forwarded call can never act under the node-local
//!   capability (the doorway does not forward the token header either).
//!
//! The hosted arm is admitted only when THIS peer issued an ACTIVE `hosted-cell`
//! grant naming the performer as recipient, verified at the acting node the
//! way compute launch is ([`authorize_custody`]): projection lookup filtered by
//! provider == own cell key, the exact signed record fetched and authenticated
//! as authored by this cell, the shared bounds validator (window, rotation,
//! reach, rate), and the provider-authored lifecycle links (not
//! revoked/cancelled/sunset). Every admitted use is recorded before it acts
//! (emit-then-act), so the grant's `rate_per_hour` is a real, fail-closed
//! ceiling on volume.
//!
//! # What a runner may touch
//!
//! Leases are namespaced to the runner ([`hosted_owner`]): the client's owner
//! (a task CID, as `payloads.mjs` sends it) is held as
//! `hosted:<performer>:<owner>`. A runner can write, read and release only its
//! own leases. The one widening is read-back of a task's OUTPUTS, which the
//! provider published on its own store and this peer must draw over the blob
//! race: allowed only for a chunk listed in the completion receipt of a task
//! whose requester is the runner and whose task CID is the named owner
//! ([`task_output_admits`]), named by the task's request action hash
//! (`x-compute-request`).
//!
//! Source of truth: the notarized `hosted-cell` commitment and its lifecycle
//! links (A); the payload leases themselves stay operational custody (C).
use crate::{
    db::{AppContext, DbPool},
    error::StorageError,
    hc_client::HcClient,
    services::conductor_writes::CommitmentStateLink,
};
use serde_json::Value;
use std::sync::Arc;

/// The scope whose recipients this peer holds compute chunks for.
pub const HOSTED_CELL_SCOPE: &str = "hosted-cell";

/// Header naming the task (request action hash) whose outputs a runner reads.
pub const REQUEST_HEADER: &str = "x-compute-request";

/// Prefix of a runner-namespaced lease owner.
const HOSTED_OWNER_PREFIX: &str = "hosted:";

/// Lease owners are capped at 256 bytes by the payload store.
const OWNER_LIMIT: usize = 256;

/// The longest a sponsor holds one runner chunk lease. The local arm's `0`
/// means "until released"; custody a sponsor gives a hosted runner is never
/// open-ended, so `0` and anything longer are held to this bound (a stage's own
/// retention policy is 86400 s; a week covers a slow provider and a retry).
pub const HOSTED_MAX_RETAIN_SECONDS: u64 = 7 * 86400;

/// The lease lifetime a hosted call actually gets. Pure.
pub fn hosted_retention(requested_seconds: u64) -> u64 {
    if requested_seconds == 0 || requested_seconds > HOSTED_MAX_RETAIN_SECONDS {
        HOSTED_MAX_RETAIN_SECONDS
    } else {
        requested_seconds
    }
}

/// Who is calling the payload route. Pure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayloadCaller {
    /// The node-local capability, no performer.
    Local,
    /// A doorway-verified performer; the hosted-cell arm decides.
    Hosted(String),
    /// Neither.
    Refused,
}

/// Classify a payload call. A present performer ALWAYS selects the hosted arm —
/// the local token is never consulted for it, so nothing forwarded by a doorway
/// acts under the node-local capability.
pub fn classify_payload_caller(performer: Option<&str>, local_token_ok: bool) -> PayloadCaller {
    match performer.map(str::trim).filter(|p| !p.is_empty()) {
        Some(p) => PayloadCaller::Hosted(p.to_string()),
        None if local_token_ok => PayloadCaller::Local,
        None => PayloadCaller::Refused,
    }
}

/// The lease owner a hosted call acts under: `hosted:<performer>:<owner>`.
/// Pure. `None` when either part is empty or the result exceeds the store's
/// owner limit — refused, never truncated (a truncated owner could collide).
pub fn hosted_owner(performer: &str, client_owner: &str) -> Option<String> {
    if performer.is_empty() || client_owner.is_empty() {
        return None;
    }
    let owner = format!("{HOSTED_OWNER_PREFIX}{performer}:{client_owner}");
    (owner.len() <= OWNER_LIMIT).then_some(owner)
}

/// May `performer` draw chunk `cid` as an OUTPUT of `task`? Pure.
///
/// The narrowest read-back rule: the runner is the task's requester, the
/// client-named owner is the task's CID, and the chunk is listed in the
/// provider's completion receipt (`completion.receipt.logs[].chunks[].cid`, the
/// descriptors `workspace.mjs` materializes). Nothing else is readable through
/// this widening — not another task's outputs, not an arbitrary blob.
pub fn task_output_admits(task: &Value, performer: &str, client_owner: &str, cid: &str) -> bool {
    if task["requester"].as_str() != Some(performer)
        || task["taskCid"].as_str() != Some(client_owner)
        || performer.is_empty()
        || client_owner.is_empty()
    {
        return false;
    }
    task["completion"]["receipt"]["logs"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|log| log["chunks"].as_array().into_iter().flatten())
        .any(|chunk| chunk["cid"].as_str() == Some(cid))
}

/// Why the hosted arm refused. Internals-free codes (commitment CIDs stay
/// server-side).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustodyRefusal {
    /// No active `hosted-cell` grant from THIS peer names the performer.
    NoActiveGrant,
    /// The projected grant's signed record could not be authenticated as this
    /// cell's own.
    RecordUnverified,
    /// The signed policy does not name this peer, the performer and the scope.
    PolicyMismatch,
    /// The shared bounds validator refused (window, rotation, reach, rate).
    Bounds(&'static str),
    /// A provider-authored withdrawal, or no provider-authored activation.
    Withdrawn,
    /// The caller declared a reach narrower than the one the hosting grant
    /// names (carried here). A runner's bytes travel at its sponsor's promised
    /// reach; anything more private needs a person to be private to, and a
    /// hosting grant names none.
    ReachNarrowerThanGrant(String),
}

impl CustodyRefusal {
    pub fn reason(&self) -> &'static str {
        match self {
            Self::NoActiveGrant => "no-active-hosted-cell-grant",
            Self::RecordUnverified => "hosted-cell-grant-unverified",
            Self::PolicyMismatch => "hosted-cell-grant-mismatch",
            Self::Bounds(code) => code,
            Self::Withdrawn => "hosted-cell-grant-withdrawn",
            Self::ReachNarrowerThanGrant(_) => "reach-narrower-than-hosting-grant",
        }
    }

    /// The reach the grant names, when the refusal is about reach.
    pub fn grant_reach(&self) -> Option<&str> {
        match self {
            Self::ReachNarrowerThanGrant(reach) => Some(reach),
            _ => None,
        }
    }
}

/// The native reads the hosted arm needs, behind a seam so the decision is
/// testable without a conductor.
#[async_trait::async_trait]
pub trait CustodyNative: Send + Sync {
    /// This peer's own cell actor (`uhCAk…`).
    fn own_key(&self) -> String;
    /// The grant at `action_hash`, authenticated as authored by this cell:
    /// `(entry-hash cid, signed policy)`.
    async fn own_grant(&self, action_hash: &str) -> Result<(String, Value), StorageError>;
    /// The commitment's lifecycle links.
    async fn authority_links(&self, cid: &str) -> Result<Vec<CommitmentStateLink>, StorageError>;
    /// The compute task at `request_action_hash`.
    async fn task(&self, request_action_hash: &str) -> Result<Value, StorageError>;
}

/// The live seam: this peer's conductor.
pub struct HcCustodyNative(pub Arc<HcClient>);

#[async_trait::async_trait]
impl CustodyNative for HcCustodyNative {
    fn own_key(&self) -> String {
        self.0.agent_key_uhcak()
    }
    async fn own_grant(&self, action_hash: &str) -> Result<(String, Value), StorageError> {
        use holochain_types::prelude::AgentPubKey;
        let grant = super::compute_tasks::fetch_authenticated_grant(
            &self.0,
            action_hash,
            AgentPubKey::from_raw_39(self.0.agent_pub_key()),
        )
        .await?;
        Ok((grant.cid, grant.policy))
    }
    async fn authority_links(&self, cid: &str) -> Result<Vec<CommitmentStateLink>, StorageError> {
        crate::services::conductor_writes::get_commitment_authority_links(&self.0, cid).await
    }
    async fn task(&self, request_action_hash: &str) -> Result<Value, StorageError> {
        super::compute_tasks::call(
            &self.0,
            &serde_json::json!({"operation":"get","requestActionHash":request_action_hash}),
        )
        .await
    }
}

/// Is `performer` the recipient of an ACTIVE `hosted-cell` grant THIS peer
/// issued, at `signed_at`? Returns the grant's CID (its entry hash — the
/// `bounded_by` join key) on admission.
///
/// Verification happens at the acting node, the way compute launch does it:
/// 1. projection lookup filtered to provider == own cell key (a grant another
///    peer issued never admits);
/// 2. the exact signed record, authenticated as authored by this cell, and its
///    policy naming this peer, the performer and `hosted-cell` — the
///    projection only SELECTS, it never authorizes (C5);
/// 3. the declared reach (`declared_reach`, default: the grant's own) is not
///    narrower than the grant's `reach_ceiling`, then the shared bounds
///    validator over the signed policy (window, rotation, reach ≤ ceiling, rate
///    counted from recorded uses);
/// 4. provider-authored lifecycle links: an activation, and no withdrawal.
pub async fn authorize_custody<N: CustodyNative + ?Sized>(
    pool: &DbPool,
    native: &N,
    performer: &str,
    signed_at: &str,
    declared_reach: Option<&str>,
) -> Result<String, CustodyRefusal> {
    let own = native.own_key();
    let anchor = {
        let mut conn = pool.get().map_err(|_| CustodyRefusal::NoActiveGrant)?;
        let active = crate::db::mishpat_commitments::find_active_delegates_compute_from_provider(
            &mut conn,
            performer,
            HOSTED_CELL_SCOPE,
            &own,
        )
        .map_err(|_| CustodyRefusal::NoActiveGrant)?
        .and_then(|row| row.dht_anchor_hash);
        match active {
            Some(anchor) => anchor,
            // No live promise: say whether this peer once made one and withdrew
            // it, so the runner learns its sponsor stopped hosting rather than
            // that it was never sponsored here.
            None => {
                let withdrawn =
                    crate::db::mishpat_commitments::list_delegates_compute_from_provider(
                        &mut conn,
                        &own,
                        Some(performer),
                        Some(HOSTED_CELL_SCOPE),
                    )
                    .map(|rows| rows.iter().any(|row| row.revoked_at.is_some()))
                    .unwrap_or(false);
                return Err(if withdrawn {
                    CustodyRefusal::Withdrawn
                } else {
                    CustodyRefusal::NoActiveGrant
                });
            }
        }
    };
    let (cid, policy) = native
        .own_grant(&anchor)
        .await
        .map_err(|_| CustodyRefusal::RecordUnverified)?;
    if policy["provider"].as_str() != Some(own.as_str())
        || policy["recipient"].as_str() != Some(performer)
        || policy["scope"].as_str() != Some(HOSTED_CELL_SCOPE)
        || policy["action"].as_str() != Some("delegates-compute")
    {
        return Err(CustodyRefusal::PolicyMismatch);
    }
    let grant_reach = policy["bounds"]["reach_ceiling"]
        .as_str()
        .unwrap_or("")
        .to_owned();
    let reach = declared_reach.unwrap_or(&grant_reach).to_owned();
    if let (Some(asked), Some(granted)) = (
        crate::services::bounds_validator::reach_rank(&reach),
        crate::services::bounds_validator::reach_rank(&grant_reach),
    ) {
        if asked < granted {
            return Err(CustodyRefusal::ReachNarrowerThanGrant(grant_reach));
        }
    }
    use crate::services::commitment_fetcher::CommitmentRecord;
    let fetcher = super::compute_tasks::VerifiedGrant(CommitmentRecord {
        cid: cid.clone(),
        action: "delegates-compute".into(),
        scope: HOSTED_CELL_SCOPE.into(),
        provider: own.clone(),
        recipient: performer.to_owned(),
        bounds: policy["bounds"].clone(),
        valid_from: policy["valid_from"].as_str().unwrap_or("").to_owned(),
        valid_until: policy["valid_until"].as_str().unwrap_or("").to_owned(),
        revoked_at: None,
    });
    crate::services::bounds_validator::validate(
        &crate::services::bounds_validator::EventForValidation {
            action: HOSTED_CELL_SCOPE.into(),
            performer: performer.to_owned(),
            bounded_by: cid.clone(),
            target_epr_id: "*".into(),
            reach,
            signed_at: signed_at.to_owned(),
        },
        &fetcher,
        &crate::services::rate_history::DieselRateHistory { pool: pool.clone() },
    )
    .await
    .map_err(|v| CustodyRefusal::Bounds(bounds_code(&v.kind)))?;
    let states = native
        .authority_links(&cid)
        .await
        .map_err(|_| CustodyRefusal::Withdrawn)?;
    if !super::compute_tasks::grant_links_allow(&states, &own) {
        return Err(CustodyRefusal::Withdrawn);
    }
    Ok(cid)
}

fn bounds_code(kind: &elohim_views::bounds::ViolationKind) -> &'static str {
    use elohim_views::bounds::ViolationKind as K;
    match kind {
        K::CommitmentNotFound => "commitment-not-found",
        K::CommitmentRevoked => "commitment-revoked",
        K::CommitmentInactive => "commitment-inactive",
        K::ScopeNotIncluded => "scope-not-included",
        K::ReachCeilingExceeded => "reach-ceiling-exceeded",
        K::RateLimitExceeded => "rate-limit-exceeded",
        K::KeyRotationStale => "key-rotation-stale",
        K::ConstitutionalRatioBreach => "constitutional-ratio-breach",
    }
}

/// Record one admitted custody use BEFORE acting (emit-then-act): an
/// `economic_events` row with `bounded_by` = the grant CID, which the bounds
/// validator's rate check counts on the next call. A use that cannot be
/// recorded must not be performed.
pub fn record_custody_use(
    pool: &DbPool,
    provider: &str,
    performer: &str,
    grant_cid: &str,
    at: &str,
) -> Result<(), StorageError> {
    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Database(e.to_string()))?;
    crate::db::economic_events::record_operator_verb_event(
        &mut conn,
        &AppContext::default(),
        &format!("compute-custody:{}", uuid::Uuid::new_v4()),
        HOSTED_CELL_SCOPE,
        provider,
        performer,
        grant_cid,
        at,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::mishpat_commitments;
    use crate::mishpat_projection::{parse_commitment_payload, CommitmentProjection};
    use serde_json::json;
    use std::collections::HashMap;

    const POOL: &str = "uhCAk-pool-peer";
    const OTHER_PEER: &str = "uhCAk-other-peer";
    const RUNNER: &str = "uhCAk-runner";
    const RUNNER_B: &str = "uhCAk-runner-b";
    const NOW: &str = "2026-10-19T12:00:00+00:00";

    fn policy(provider: &str, recipient: &str, rate: u64) -> Value {
        json!({"action":"delegates-compute","scope":HOSTED_CELL_SCOPE,
            "provider":provider,"recipient":recipient,
            "bounds":{"epr_scope":["*"],"reach_ceiling":"commons","rate_per_hour":rate,"rotation_ttl_days":365},
            "valid_from":"2026-10-01T00:00:00+00:00","valid_until":"2027-09-30T00:00:00+00:00"})
    }

    /// Project a grant the way `compute_grants::issue` does (active, anchored).
    fn project(pool: &DbPool, cid: &str, action_hash: &str, p: &Value) {
        let mut row =
            match parse_commitment_payload("delegates-compute", &p.to_string(), cid, action_hash)
                .expect("parses")
            {
                CommitmentProjection::Upsert(row) => row,
                _ => panic!("delegates-compute projects as an upsert"),
            };
        row.state = "active".into();
        let mut conn = pool.get().expect("conn");
        mishpat_commitments::upsert_with_anchor(&mut conn, row).expect("upsert");
    }

    fn active_link() -> CommitmentStateLink {
        CommitmentStateLink {
            author: POOL.into(),
            state: "active".into(),
            signed_at: "2026-10-01T00:00:00+00:00".into(),
            event_hash: "uhCkk-consent".into(),
        }
    }

    /// The native seam: signed records by action hash, links by cid.
    struct Native {
        grants: HashMap<String, (String, Value)>,
        links: Vec<CommitmentStateLink>,
        task: Value,
    }

    #[async_trait::async_trait]
    impl CustodyNative for Native {
        fn own_key(&self) -> String {
            POOL.into()
        }
        async fn own_grant(&self, action_hash: &str) -> Result<(String, Value), StorageError> {
            self.grants
                .get(action_hash)
                .cloned()
                .ok_or_else(|| StorageError::InvalidInput("signed grant unavailable".into()))
        }
        async fn authority_links(
            &self,
            _cid: &str,
        ) -> Result<Vec<CommitmentStateLink>, StorageError> {
            Ok(self.links.clone())
        }
        async fn task(&self, _r: &str) -> Result<Value, StorageError> {
            Ok(self.task.clone())
        }
    }

    /// A pool peer that has issued the runner an active hosted-cell grant.
    fn sponsored(rate: u64) -> (DbPool, Native) {
        let pool = crate::test_util::test_pool();
        let p = policy(POOL, RUNNER, rate);
        project(&pool, "uhCEk-grant", "uhCkk-grant", &p);
        let native = Native {
            grants: HashMap::from([("uhCkk-grant".to_string(), ("uhCEk-grant".to_string(), p))]),
            links: vec![active_link()],
            task: Value::Null,
        };
        (pool, native)
    }

    #[tokio::test]
    async fn an_active_hosted_cell_recipient_is_admitted() {
        let (pool, native) = sponsored(3600);
        assert_eq!(
            authorize_custody(&pool, &native, RUNNER, NOW, None).await,
            Ok("uhCEk-grant".to_string())
        );
    }

    /// A runner's bytes travel at the reach its sponsor promised: declaring
    /// that reach (or none) is admitted; anything more private is refused,
    /// naming the grant's reach.
    #[tokio::test]
    async fn a_reach_more_private_than_the_hosting_grant_is_refused() {
        let (pool, native) = sponsored(3600);
        assert_eq!(
            authorize_custody(&pool, &native, RUNNER, NOW, Some("commons")).await,
            Ok("uhCEk-grant".to_string())
        );
        for narrower in ["private", "intimate", "community"] {
            let refusal = authorize_custody(&pool, &native, RUNNER, NOW, Some(narrower))
                .await
                .unwrap_err();
            assert_eq!(
                refusal,
                CustodyRefusal::ReachNarrowerThanGrant("commons".into())
            );
            assert_eq!(refusal.grant_reach(), Some("commons"));
            assert_eq!(refusal.reason(), "reach-narrower-than-hosting-grant");
        }
    }

    /// Once the projection records the sponsor's withdrawal, the runner is told
    /// its promise was withdrawn — not that it was never sponsored here.
    #[tokio::test]
    async fn a_projected_withdrawal_names_the_withdrawn_promise() {
        let (pool, native) = sponsored(3600);
        {
            let mut conn = pool.get().expect("conn");
            mishpat_commitments::set_revoked_at(&mut conn, "uhCEk-grant", NOW).expect("revoke");
        }
        assert_eq!(
            authorize_custody(&pool, &native, RUNNER, NOW, None).await,
            Err(CustodyRefusal::Withdrawn)
        );
        assert_eq!(
            authorize_custody(&pool, &native, RUNNER_B, NOW, None).await,
            Err(CustodyRefusal::NoActiveGrant),
            "a runner this peer never sponsored is still simply unsponsored"
        );
    }

    #[tokio::test]
    async fn a_withdrawn_grant_is_refused() {
        let (pool, mut native) = sponsored(3600);
        native.links.push(CommitmentStateLink {
            state: "revoked".into(),
            signed_at: "2026-10-10T00:00:00+00:00".into(),
            ..active_link()
        });
        assert_eq!(
            authorize_custody(&pool, &native, RUNNER, NOW, None).await,
            Err(CustodyRefusal::Withdrawn)
        );
        // A withdrawal authored by anyone OTHER than the provider is not one —
        // but a missing provider activation also refuses.
        native.links = vec![CommitmentStateLink {
            author: OTHER_PEER.into(),
            ..active_link()
        }];
        assert_eq!(
            authorize_custody(&pool, &native, RUNNER, NOW, None).await,
            Err(CustodyRefusal::Withdrawn)
        );
    }

    #[tokio::test]
    async fn an_expired_grant_is_refused() {
        let (pool, native) = sponsored(3600);
        assert_eq!(
            authorize_custody(&pool, &native, RUNNER, "2027-10-19T12:00:00+00:00", None).await,
            Err(CustodyRefusal::Bounds("commitment-inactive"))
        );
    }

    #[tokio::test]
    async fn a_grant_issued_by_a_different_peer_is_refused() {
        let pool = crate::test_util::test_pool();
        let p = policy(OTHER_PEER, RUNNER, 3600);
        project(&pool, "uhCEk-foreign", "uhCkk-foreign", &p);
        let native = Native {
            grants: HashMap::from([(
                "uhCkk-foreign".to_string(),
                ("uhCEk-foreign".to_string(), p),
            )]),
            links: vec![active_link()],
            task: Value::Null,
        };
        assert_eq!(
            authorize_custody(&pool, &native, RUNNER, NOW, None).await,
            Err(CustodyRefusal::NoActiveGrant),
            "another peer's promise never authorizes custody on this node"
        );
    }

    /// The projection only selects (C5): a row whose signed record does not
    /// name this peer as provider is refused even if the row says it does.
    #[tokio::test]
    async fn a_projection_the_signed_record_contradicts_is_refused() {
        let (pool, mut native) = sponsored(3600);
        native.grants.insert(
            "uhCkk-grant".into(),
            ("uhCEk-grant".into(), policy(OTHER_PEER, RUNNER, 3600)),
        );
        assert_eq!(
            authorize_custody(&pool, &native, RUNNER, NOW, None).await,
            Err(CustodyRefusal::PolicyMismatch)
        );
        native.grants.clear();
        assert_eq!(
            authorize_custody(&pool, &native, RUNNER, NOW, None).await,
            Err(CustodyRefusal::RecordUnverified)
        );
    }

    #[tokio::test]
    async fn a_non_recipient_is_refused() {
        let (pool, native) = sponsored(3600);
        assert_eq!(
            authorize_custody(&pool, &native, RUNNER_B, NOW, None).await,
            Err(CustodyRefusal::NoActiveGrant)
        );
        assert_eq!(
            authorize_custody(&pool, &native, POOL, NOW, None).await,
            Err(CustodyRefusal::NoActiveGrant),
            "the provider is not its own recipient"
        );
    }

    #[test]
    fn no_performer_is_refused_and_a_performer_never_uses_the_token() {
        assert_eq!(classify_payload_caller(None, false), PayloadCaller::Refused);
        assert_eq!(
            classify_payload_caller(Some("  "), false),
            PayloadCaller::Refused
        );
        assert_eq!(classify_payload_caller(None, true), PayloadCaller::Local);
        assert_eq!(
            classify_payload_caller(Some(RUNNER), true),
            PayloadCaller::Hosted(RUNNER.into()),
            "a forwarded performer selects the hosted arm even if a valid token rode along"
        );
    }

    #[tokio::test]
    async fn the_rate_bound_counts_recorded_uses() {
        let (pool, native) = sponsored(2);
        for _ in 0..2 {
            let cid = authorize_custody(&pool, &native, RUNNER, NOW, None)
                .await
                .expect("within the rate");
            record_custody_use(&pool, POOL, RUNNER, &cid, NOW).expect("recorded");
        }
        assert_eq!(
            authorize_custody(&pool, &native, RUNNER, NOW, None).await,
            Err(CustodyRefusal::Bounds("rate-limit-exceeded"))
        );
        assert_eq!(
            authorize_custody(&pool, &native, RUNNER, "2026-10-19T13:30:00+00:00", None).await,
            Ok("uhCEk-grant".to_string()),
            "the window slides: an hour later the uses no longer count"
        );
    }

    #[test]
    fn two_runners_never_share_a_lease() {
        let a = hosted_owner(RUNNER, "bafkrei-task").expect("owner");
        let b = hosted_owner(RUNNER_B, "bafkrei-task").expect("owner");
        assert_ne!(a, b, "the same task CID names two different leases");
        assert!(a.starts_with("hosted:uhCAk-runner:"));
        assert_eq!(hosted_owner("", "t"), None);
        assert_eq!(hosted_owner(RUNNER, ""), None);
        assert_eq!(
            hosted_owner(RUNNER, &"x".repeat(300)),
            None,
            "refused, never truncated"
        );
    }

    #[test]
    fn hosted_custody_is_never_open_ended() {
        assert_eq!(
            hosted_retention(0),
            HOSTED_MAX_RETAIN_SECONDS,
            "0 never means forever here"
        );
        assert_eq!(hosted_retention(86400), 86400);
        assert_eq!(hosted_retention(u64::MAX), HOSTED_MAX_RETAIN_SECONDS);
    }

    fn completed_task(requester: &str) -> Value {
        json!({"requester":requester,"provider":"uhCAk-jessica","taskCid":"bafkrei-task",
            "completion":{"receipt":{"logs":[
                {"name":"stdout.log","chunks":[{"cid":"bafkrei-out-1"},{"cid":"bafkrei-out-2"}]},
                {"name":"witness.json","chunks":[{"cid":"bafkrei-witness"}]}]}}})
    }

    #[test]
    fn a_runner_reads_back_only_its_own_tasks_outputs() {
        let task = completed_task(RUNNER);
        assert!(task_output_admits(
            &task,
            RUNNER,
            "bafkrei-task",
            "bafkrei-out-2"
        ));
        assert!(task_output_admits(
            &task,
            RUNNER,
            "bafkrei-task",
            "bafkrei-witness"
        ));
        assert!(
            !task_output_admits(&task, RUNNER_B, "bafkrei-task", "bafkrei-out-2"),
            "another runner cannot read this task's outputs"
        );
        assert!(
            !task_output_admits(&task, RUNNER, "bafkrei-other-task", "bafkrei-out-2"),
            "the named owner must be this task"
        );
        assert!(
            !task_output_admits(&task, RUNNER, "bafkrei-task", "bafkrei-anything"),
            "an unlisted chunk is not an output of this task"
        );
        assert!(!task_output_admits(
            &json!({"requester":RUNNER,"taskCid":"bafkrei-task"}),
            RUNNER,
            "bafkrei-task",
            "bafkrei-out-1"
        ));
    }
}
