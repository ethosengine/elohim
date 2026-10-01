//! The node's half of the adoption trigger's courier path
//! ([`crate::services::courier_obey`]): an owned head-record fetcher over the
//! reconcile peers, and byte presence over this node's blob store.

use std::sync::Arc;

use seam_contracts::Answer;
use tokio::sync::mpsc;

use crate::blob_store::BlobStore;
use crate::db::DbPool;
use crate::p2p::head_record_client::PeerHeadRecordFetcher;
use crate::p2p::reconcile_peers::ReconcilePeers;
use crate::p2p::P2PCommand;
use crate::services::courier_obey::BytePresence;
use crate::services::head_adoption::{CarriedHeadRecord, HeadRecordFetcher};

/// [`PeerHeadRecordFetcher`] that owns its peer source, so a long-lived worker
/// can hold it.
pub struct OwnedPeerHeadRecordFetcher(pub Arc<dyn ReconcilePeers>);

#[async_trait::async_trait]
impl HeadRecordFetcher for OwnedPeerHeadRecordFetcher {
    async fn fetch(&self, peer_id: &str, content_id: &str) -> Answer<CarriedHeadRecord> {
        PeerHeadRecordFetcher::new(self.0.as_ref())
            .fetch(peer_id, content_id)
            .await
    }
}

/// Byte presence over this node's blob store and shard manifests. Requests go
/// to the libp2p event loop as [`P2PCommand::HealBlobBytes`] when that plane
/// exists, else over the iroh fetch leg to the courier directly.
pub struct NodeBytePresence {
    pub blob_store: Arc<BlobStore>,
    pub pool: DbPool,
    /// The libp2p plane's command queue; `None` on a pure-iroh node.
    pub command_tx: Option<mpsc::Sender<P2PCommand>>,
    /// This node's steward identity, which books an iroh-fetched blob the way
    /// every other fetch is booked (`finalize_fetch_success`).
    pub self_cid: String,
}

/// Iroh byte requests in flight at once. A request past the bound is dropped:
/// the trigger re-checks presence on its next rung and asks again.
#[cfg(feature = "p2p-iroh")]
static IROH_BYTE_REQUESTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);

/// One dial, one answer: the same per-peer bound the heal-on-read race uses.
#[cfg(feature = "p2p-iroh")]
const IROH_BYTE_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

impl NodeBytePresence {
    /// Fetch `hash` from `holder` over the iroh leg and book it. Pure-iroh
    /// nodes have no libp2p event loop to send `HealBlobBytes` to; without
    /// this, the bytes gate would hold a head move until the sweep.
    #[cfg(feature = "p2p-iroh")]
    fn request_over_iroh(&self, origin: &str, hash: String, holder: &str) {
        let Ok(permit) = IROH_BYTE_REQUESTS.try_acquire() else {
            return;
        };
        let (blob_store, pool, self_cid) = (
            self.blob_store.clone(),
            self.pool.clone(),
            self.self_cid.clone(),
        );
        let (origin, holder) = (origin.to_string(), holder.to_string());
        tokio::spawn(async move {
            let _permit = permit;
            // No libp2p plane: an inert sender, and no peer counts as connected,
            // so only the iroh leg is planned.
            let (inert, _rx) = mpsc::channel(1);
            let outcome = crate::p2p::blob_swarm::race_fetch_dual(
                &hash,
                vec![holder],
                &inert,
                |_| false,
                1,
                IROH_BYTE_REQUEST_TIMEOUT,
            )
            .await;
            let crate::p2p::blob_fetch::FetchOutcome::Hit { bytes, source_peer } = outcome else {
                tracing::debug!(origin = %origin, hash = %hash, "iroh byte request: no bytes");
                return;
            };
            let Ok(mut conn) = pool.get() else {
                return;
            };
            if let Err(error) = crate::p2p::blob_fetch::finalize_fetch_success(
                &mut conn,
                &hash,
                &source_peer,
                &bytes,
                &self_cid,
                &blob_store,
            )
            .await
            {
                tracing::warn!(origin = %origin, hash = %hash, error = %error, "iroh byte request: store failed");
            }
        });
    }
}

/// `sha256-<hex>` for any address form the blob plane accepts, or `None`.
fn legacy_hash(address: &str) -> Option<String> {
    BlobStore::parse_content_address(address)
        .ok()
        .map(|hex| format!("sha256-{hex}"))
}

#[async_trait::async_trait]
impl BytePresence for NodeBytePresence {
    async fn held(&self, address: &str) -> bool {
        let Some(hash) = legacy_hash(address) else {
            return false;
        };
        if self.blob_store.exists(&hash).await {
            return true;
        }
        let manifest = self.pool.get().ok().and_then(|mut conn| {
            crate::db::shard_manifests::get_manifest_by_blob_hash(&mut conn, &hash)
                .ok()
                .flatten()
                .and_then(|row| crate::db::shard_manifests::hydrate_manifest(&row).ok())
        });
        match manifest {
            Some(m) => self.blob_store.holds_as_shards(&m).await,
            None => false,
        }
    }

    fn request(&self, origin: &str, address: &str, holder: &str) {
        let Some(hash) = legacy_hash(address) else {
            return;
        };
        let Some(command_tx) = self.command_tx.as_ref() else {
            #[cfg(feature = "p2p-iroh")]
            self.request_over_iroh(origin, hash, holder);
            return;
        };
        // try_send: a full command queue drops the request; the trigger's ladder
        // re-checks presence and asks again on its next rung.
        let _ = command_tx.try_send(P2PCommand::HealBlobBytes {
            origin: origin.to_string(),
            hash,
            holder_hint: Some(holder.to_string()),
        });
    }
}

/// Reconstruct only hosting for an locally projected public browser bundle. The
/// inventory is discovery: every selected ID is read from the own notary.
/// bounded-work: 3 peer requests, at most 64 entries per reply, 4 ID reads and
/// 64 shared exact-record reads. Pending work stays on the existing trigger
/// retry ladder and ordinary projection sweep; no anonymous hot-path work.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn reconstruct_bundle_hosting(
    peers: &dyn ReconcilePeers,
    hc: &Arc<crate::hc_client::HcClient>,
    pool: &DbPool,
    ctx: &crate::db::AppContext,
    events: Option<&crate::services::events::EventBus>,
    epr_id: &str,
    source_peer: &str,
    cursor: &mut crate::services::head_adoption_trigger::HostingCursor,
) -> bool {
    use crate::views::{
        ProjectionInventoryFilter, ProjectionInventoryPayload, ViewFederationRequest, ViewKind,
    };
    let is_public_bundle = pool
        .get()
        .ok()
        .and_then(|mut conn| {
            crate::db::content_diesel::get_content(
                &mut conn,
                ctx,
                epr_id,
                crate::db::content_diesel::MinTrust::Invisible,
            )
            .ok()
            .flatten()
        })
        .is_some_and(|c| {
            matches!(c.content_format.as_str(), "html5-app" | "spa-bundle")
                && matches!(c.reach.as_str(), "commons" | "public")
        });
    if !is_public_bundle {
        return true;
    }
    let kind = ViewKind::ProjectionInventory {
        table: crate::p2p::view_federation::PROJECTION_INVENTORY_TABLE_REA_COMMITMENTS.into(),
        filter: Some(ProjectionInventoryFilter::ProjectEpr {
            epr_id: epr_id.into(),
        }),
    };
    let mut targets = vec![source_peer.to_string()];
    for peer in peers.list_peers().await {
        if targets.len() >= 3 {
            break;
        }
        if !targets.contains(&peer.peer_id) {
            targets.push(peer.peer_id);
        }
    }
    let mut candidates = std::collections::BTreeMap::new();
    let mut supported = false;
    let mut pending = false;
    let mut partial_page = false;
    for target in targets {
        let request = ViewFederationRequest {
            view_kind: kind.clone(),
            agent_cid: peers.agent_pubkey().into(),
            request_id: uuid::Uuid::new_v4().to_string(),
            inventory_offset: Some(cursor.page_offset),
            head_corpus_digest: None,
        };
        let Ok(reply) = peers
            .view_federate(&target, request, std::time::Duration::from_secs(2))
            .await
        else {
            pending = true;
            continue;
        };
        // A legacy peer may ignore the request's optional filter. Its reply
        // omits the filter, and MUST NOT be read as the requested undertaking.
        if !hosting_filter_supported(&kind, &reply) {
            pending = true;
            continue;
        }
        let Ok(payload) =
            serde_json::from_value::<ProjectionInventoryPayload>(reply.slice.payload.0)
        else {
            pending = true;
            continue;
        };
        if payload.table != crate::p2p::view_federation::PROJECTION_INVENTORY_TABLE_REA_COMMITMENTS
            || payload.entries.len() > 64
        {
            pending = true;
            continue;
        }
        supported = true;
        partial_page |= payload.total > payload.entries.len();
        pending |= payload.total > payload.entries.len();
        for entry in payload.entries {
            if entry.id.is_empty() || entry.id.len() > 512 || entry.id.chars().any(char::is_control)
            {
                pending = true;
                continue;
            }
            candidates.entry(entry.id.clone()).or_insert(entry);
        }
    }
    // No hosting candidate is not a global absence proof. Recheck within the
    // already-bounded claim; gossip and the ordinary sweep remain the floor.
    if !supported {
        return false;
    }
    // Other windows may have failed; this attempt cannot certify all of them.
    pending |= candidates.len() > 4;
    let selected = hosting_candidate_window(candidates.into_keys().collect(), cursor, partial_page);
    if selected.is_empty() {
        return false;
    }
    let reads = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut native_ids = 0;
    pending |= cursor.candidate_offset != 0 || partial_page;
    for id in selected {
        // Projection/hint equality is not provenance: a seed or a stale peer
        // may agree with this row while the own notary observes a withdrawal.
        // Every selected candidate therefore costs its own current observation.
        native_ids += 1;
        match crate::services::rea_commitment_projection::refresh_hosting_by_id(
            hc, pool, ctx, &id, epr_id, &reads, events,
        )
        .await
        {
            Ok(true) => {}
            Ok(false) | Err(_) => {
                pending = true;
            }
        }
    }
    tracing::debug!(
        epr_id,
        native_ids,
        record_reads = reads.load(std::sync::atomic::Ordering::Relaxed).min(64),
        pending,
        "hosting reconstruction: own-native bounded attempt finished"
    );
    !pending
}

fn hosting_filter_supported(
    kind: &crate::views::ViewKind,
    reply: &crate::views::ViewFederationResponse,
) -> bool {
    reply.view_kind == *kind
        && reply.slice.view_kind == *kind
        && matches!(
            reply.slice.freshness.state,
            crate::views::FreshnessState::Live
        )
}

// Advance only this claim's private discovery window. An empty, full SQL
// page of denied near-matches progresses immediately; a populated page spends
// no more than four IDs before the next existing retry continues it.
fn hosting_candidate_window(
    ids: Vec<String>,
    cursor: &mut crate::services::head_adoption_trigger::HostingCursor,
    partial_page: bool,
) -> Vec<String> {
    if ids.is_empty() {
        if partial_page {
            cursor.page_offset = cursor.page_offset.saturating_add(64);
            cursor.candidate_offset = 0;
        }
        return Vec::new();
    }
    let offset = cursor.candidate_offset % ids.len();
    let selected = ids.iter().skip(offset).take(4).cloned().collect::<Vec<_>>();
    cursor.candidate_offset = offset + selected.len();
    if cursor.candidate_offset >= ids.len() {
        cursor.candidate_offset = 0;
        if partial_page {
            cursor.page_offset = cursor.page_offset.saturating_add(64);
        }
    }
    selected
}

#[cfg(test)]
mod hosting_work_window_tests {
    use super::*;

    #[test]
    fn pending_retry_reaches_fifth_candidate_with_same_fixed_work_bound() {
        use crate::services::head_adoption_trigger::HostingCursor;
        let ids = (0..5).map(|i| format!("candidate-{i}")).collect::<Vec<_>>();
        let mut cursor = HostingCursor::default();
        // Even if head adoption spent three rungs, hosting starts at its own 0.
        let first = hosting_candidate_window(ids.clone(), &mut cursor, false);
        let retry = hosting_candidate_window(ids, &mut cursor, false);
        assert_eq!(first.len(), 4);
        assert_eq!(retry, vec!["candidate-4"]);
        assert_eq!(cursor.candidate_offset, 0);
        assert_eq!(cursor.page_offset, 0);
        // A full denied SQL page must not starve the next exact candidate.
        let mut cursor = HostingCursor::default();
        assert!(hosting_candidate_window(vec![], &mut cursor, true).is_empty());
        assert_eq!(cursor.page_offset, 64);
        let next = hosting_candidate_window(vec!["own-native-exact".into()], &mut cursor, false);
        assert_eq!(next, vec!["own-native-exact"]);
    }

    #[test]
    fn malformed_full_inventory_page_advances_to_exact_native_candidate() {
        use crate::db::rea_commitments::{
            create_commitment, hosting_inventory, CreateReaCommitmentInput,
        };
        use crate::services::head_adoption_trigger::HostingCursor;
        use diesel::prelude::*;
        let pool = crate::test_util::test_pool();
        let app = crate::db::AppContext::default_lamad();
        let mut conn = pool.get().unwrap();
        for index in 0..65 {
            let exact = index == 64;
            let id = if exact {
                "own-native-exact".into()
            } else {
                format!("malformed-{index:02}")
            };
            let scope = if exact {
                "doorway:b|epr:garden".to_string()
            } else {
                format!("doorway:b|epr:garden|invalid:{index}")
            };
            create_commitment(
                &mut conn,
                &app,
                CreateReaCommitmentInput {
                    id: Some(id.clone()),
                    action: "project-epr".into(),
                    provider: "author".into(),
                    receiver: "operator".into(),
                    in_scope_of: Some(scope),
                    metadata_json: Some(serde_json::json!({"reach":"commons"}).to_string()),
                    ..Default::default()
                },
            )
            .unwrap();
            use crate::db::diesel_schema::rea_commitments as c;
            diesel::update(c::table.filter(c::id.eq(id)))
                .set(c::created_at.eq(if exact { "2026-10-01" } else { "2026-10-02" }))
                .execute(&mut conn)
                .unwrap();
        }
        let mut cursor = HostingCursor::default();
        let (rows, total) =
            hosting_inventory(&mut conn, &app, "garden", cursor.page_offset.into()).unwrap();
        assert!(
            rows.is_empty(),
            "exact parser denies every near-match on first bounded page"
        );
        assert!(hosting_candidate_window(
            rows.into_iter().map(|r| r.0).collect(),
            &mut cursor,
            total > 0
        )
        .is_empty());
        assert_eq!(cursor.page_offset, 64);
        let (rows, total) =
            hosting_inventory(&mut conn, &app, "garden", cursor.page_offset.into()).unwrap();
        let count = rows.len();
        let native_attention = hosting_candidate_window(
            rows.into_iter().map(|r| r.0).collect(),
            &mut cursor,
            total > count,
        );
        assert_eq!(
            native_attention,
            vec!["own-native-exact"],
            "second existing retry selects the ID for own-native refresh"
        );
    }

    #[test]
    fn hosting_discovery_requires_exact_filter_in_both_echoes() {
        use crate::views::*;
        let kind = ViewKind::ProjectionInventory {
            table: "rea_commitments".into(),
            filter: Some(ProjectionInventoryFilter::ProjectEpr {
                epr_id: "garden".into(),
            }),
        };
        let legacy = ViewKind::ProjectionInventory {
            table: "rea_commitments".into(),
            filter: None,
        };
        let mut reply = ViewFederationResponse {
            view_kind: kind.clone(),
            agent_cid: "peer".into(),
            request_id: "bounded".into(),
            slice: ViewSlice {
                peer_id: "peer".into(),
                view_kind: kind.clone(),
                freshness: Freshness {
                    state: FreshnessState::Live,
                    stale_since_ms: None,
                },
                payload: JsonVal(serde_json::json!({})),
                signature: String::new(),
            },
        };
        assert!(hosting_filter_supported(&kind, &reply));
        reply.view_kind = legacy.clone();
        assert!(!hosting_filter_supported(&kind, &reply));
        reply.view_kind = kind.clone();
        reply.slice.view_kind = legacy;
        assert!(!hosting_filter_supported(&kind, &reply));
        reply.slice.view_kind = kind.clone();
        reply.slice.freshness.state = FreshnessState::Offline;
        assert!(!hosting_filter_supported(&kind, &reply));
    }
}
