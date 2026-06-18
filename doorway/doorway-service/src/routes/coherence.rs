//! `GET /api/v1/federation/coherence` — per-edge EPR-head self-fingerprint +
//! (later) cross-edge divergence detection (DETECT-ONLY).
//!
//! Cat-C node-local read-model (`doorway/CLAUDE.md` "views served THROUGH not
//! BY"): a fresh projection composed from THIS doorway's `EprRouter` table — any
//! doorway projecting the same substrate emits the same self-fingerprint
//! (swap-test clean for the self leg). The cross-edge view (Task 3) is
//! observability-only and never reconciles — reconciliation would author
//! cross-edge truth, which is forbidden. The real "never partition" fix is
//! F-BOOTSTRAP + F-DEPLOY; this is the detector the diagnostic asked for.
//!
//! **Addressing (CID-canonical).** The `digest` is a **CIDv1 dag-cbor**
//! (`bafyrei…`) over the canonically dag-cbor-serialized *sorted* `(url_path,
//! epr_id)` head set — NOT a bare `sha256-<hex>`. A CID is the same sha2-256 a
//! bare hash would expose, wrapped in a self-describing multihash + codec; here
//! the `0x71` codec honestly describes the dag-cbor preimage. The per-head
//! `epr_id`s are themselves already CIDs. See `.claude/skills/p2p-design-gate`
//! Step 2 "Canonical address forms" + `doorway/CLAUDE.md` "Addressing canon."

use std::sync::Arc;

use cid::Cid;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Response, StatusCode};
use multihash_codetable::{Code, MultihashDigest};
use serde::{Deserialize, Serialize};

use crate::projection::epr_router::EprRouter;
use crate::server::AppState;

/// dag-cbor multicodec (0x71). The digest wraps the SAME sha2-256 a bare hash
/// would, in a self-describing CID — never a standalone `sha256-<hex>`.
const DAG_CBOR_CODEC: u64 = 0x71;

/// One pillar's projected EPR head on this doorway. `epr_id` is the projected
/// EPR atom CID (`EprProjectionView.epr_id`) — itself already a CID, NOT a blob
/// hash and NOT a deploy build SHA (those are distinct — see
/// `CoherenceManifest.build_id`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EprHeadFingerprint {
    pub url_path: String,
    pub epr_id: String,
}

/// This doorway's full routing-table fingerprint. `digest` is a content-stable
/// **CIDv1 dag-cbor** (`bafyrei…`) over the sorted `(url_path, epr_id)` set, so
/// two edges agree iff their digests match. `build_id` is the deploy git SHA
/// (the operator's "two EPR heads" symptom was actually build_id skew, not
/// content divergence) — carried alongside so deploy-skew is reported, never
/// confused with content skew.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoherenceManifest {
    pub doorway_id: String,
    pub generation: u64,
    pub heads: Vec<EprHeadFingerprint>,
    pub digest: String,
    pub build_id: Option<String>,
}

/// PURE read over the live router table. No fetch, no lock held across an await.
/// The digest is a CIDv1 dag-cbor address of the sorted head set.
pub fn router_fingerprint(
    router: &EprRouter,
    doorway_id: &str,
    build_id: Option<&str>,
) -> CoherenceManifest {
    let mut heads: Vec<EprHeadFingerprint> = router
        .head_fingerprints()
        .into_iter()
        .map(|(url_path, epr_id)| EprHeadFingerprint { url_path, epr_id })
        .collect();
    heads.sort_by(|a, b| a.url_path.cmp(&b.url_path).then(a.epr_id.cmp(&b.epr_id)));

    // Canonical preimage = dag-cbor of the sorted head set; wrap its sha2-256 in
    // a self-describing CIDv1 (dag-cbor codec) → `bafyrei…`. dag-cbor of a Vec of
    // `{String, String}` structs is infallible.
    let preimage = serde_ipld_dagcbor::to_vec(&heads)
        .expect("dag-cbor encode of EprHeadFingerprint set is infallible");
    let mh = Code::Sha2_256.digest(&preimage);
    let digest = Cid::new_v1(DAG_CBOR_CODEC, mh).to_string();

    CoherenceManifest {
        doorway_id: doorway_id.to_string(),
        generation: router.generation(),
        heads,
        digest,
        build_id: build_id.map(str::to_string),
    }
}

/// Per-peer divergence verdict (Cat-C, observability-only). `agrees` = the peer
/// was reachable AND its `digest` matches ours. `divergent_paths` names the
/// pillars whose heads differ (operator-actionable). This is NEVER reconciled —
/// the doorway surfaces divergence, it never authors cross-edge truth.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerCoherence {
    pub doorway_id: String,
    pub reachable: bool,
    pub digest: Option<String>,
    pub build_id: Option<String>,
    pub agrees: bool,
    pub divergent_paths: Vec<String>,
}

/// Cross-edge coherence view (Cat-C, observability-only — NEVER reconciles).
/// `in_agreement` is true iff every compared peer agrees.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoherenceView {
    pub self_digest: String,
    pub self_build_id: Option<String>,
    pub peers: Vec<PeerCoherence>,
    pub in_agreement: bool,
}

/// Compare this edge's manifest against one peer's. PURE — no I/O.
///
/// `reachable` carries a precise meaning: the coherence probe returned a
/// manifest. A peer that is up but pre-F-COHERENCE (404 on the coherence route)
/// reads as NOT reachable, so it can never fire a false divergence alarm while
/// the sibling edge is still deploying.
pub fn compare_to_peer(
    me: &CoherenceManifest,
    peer_id: &str,
    reachable: bool,
    peer: Option<&CoherenceManifest>,
) -> PeerCoherence {
    match (reachable, peer) {
        (true, Some(p)) => {
            let agrees = p.digest == me.digest;
            let mut divergent: Vec<String> = Vec::new();
            if !agrees {
                let mine: std::collections::HashMap<&str, &str> = me
                    .heads
                    .iter()
                    .map(|h| (h.url_path.as_str(), h.epr_id.as_str()))
                    .collect();
                // Pillars present on the peer with a different (or our-missing) head.
                for h in &p.heads {
                    match mine.get(h.url_path.as_str()) {
                        Some(my_epr) if *my_epr == h.epr_id => {}
                        _ => divergent.push(h.url_path.clone()),
                    }
                }
                // Pillars we have that the peer is missing entirely.
                for path in mine.keys() {
                    if !p.heads.iter().any(|h| h.url_path == *path) {
                        divergent.push((*path).to_string());
                    }
                }
                divergent.sort();
                divergent.dedup();
            }
            PeerCoherence {
                doorway_id: peer_id.to_string(),
                reachable: true,
                digest: Some(p.digest.clone()),
                build_id: p.build_id.clone(),
                agrees,
                divergent_paths: divergent,
            }
        }
        _ => PeerCoherence {
            doorway_id: peer_id.to_string(),
            reachable,
            digest: None,
            build_id: None,
            agrees: false,
            divergent_paths: Vec::new(),
        },
    }
}

/// Compose the cross-edge view from this edge's manifest + the per-peer verdicts.
/// `in_agreement` is true iff every peer agrees (vacuously true with no peers).
pub fn build_coherence_view(me: &CoherenceManifest, peers: Vec<PeerCoherence>) -> CoherenceView {
    let in_agreement = peers.iter().all(|p| p.agrees);
    CoherenceView {
        self_digest: me.digest.clone(),
        self_build_id: me.build_id.clone(),
        peers,
        in_agreement,
    }
}

/// `GET /api/v1/federation/coherence` — this edge's self-fingerprint. The single
/// missing primitive the diagnostic asked for: per-edge head state externally
/// inspectable. The `digest` is a CIDv1 dag-cbor (see `router_fingerprint`);
/// `build_id` carries the deploy SHA so deploy-skew is never confused with
/// content-head skew. Cat-C doorway-local Operational state — no auth (operator
/// scope is an ingress property, matching `/admin/bootstrap-coherence`).
pub async fn handle_federation_coherence(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let doorway_id = state
        .args
        .doorway_id
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    let build = elohim_compute::BuildInfo::new("elohim-doorway");
    let manifest = router_fingerprint(state.epr_router.as_ref(), &doorway_id, Some(&build.commit));
    match serde_json::to_string(&manifest) {
        Ok(json) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "no-store")
            .body(Full::new(Bytes::from(json)))
            .unwrap(),
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Full::new(Bytes::from(
                "Failed to serialize coherence manifest",
            )))
            .unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::epr_router::EprRouter;
    use elohim_views::projection::{EprProjectionView, ProjectionMode};

    fn sample_projection(url_path: &str, epr_id: &str) -> EprProjectionView {
        EprProjectionView {
            commitment_id: format!("test-{epr_id}"),
            epr_id: epr_id.into(),
            doorway_id: "doorway:test".into(),
            url_path: url_path.into(),
            mode: ProjectionMode::Cached,
            reach: "commons".into(),
            base_href: if url_path == "/" {
                "/".into()
            } else {
                format!("{url_path}/")
            },
            entry_file: "index.html".into(),
            spa_fallback: true,
            redirects_from: vec![],
            redirect_templates: vec![],
            route_claims: None,
            preview_epr_ref: None,
            gate_hints: vec![],
            dead_end: false,
            steward_direct_endpoint: None,
            seeded_at: "2026-05-25T00:00:00Z".into(),
            seeded_by: "test".into(),
        }
    }

    #[test]
    fn router_fingerprint_is_deterministic_cid_and_camel_case() {
        let router = EprRouter::new();
        router.replace_all(vec![
            sample_projection("/lamad", "epr-lamad-aaa"),
            sample_projection("/qahal", "epr-qahal-bbb"),
        ]);
        let m = router_fingerprint(&router, "alpha-elohim-host", Some("e0352a7"));
        assert_eq!(m.doorway_id, "alpha-elohim-host");
        assert_eq!(m.heads.len(), 2);
        // The digest is a CIDv1 dag-cbor address (`bafy…`), NOT a bare `sha256-<hex>`.
        assert!(
            m.digest.starts_with("bafy"),
            "expected CIDv1 dag-cbor digest, got {}",
            m.digest
        );
        // Stable regardless of HashMap iteration order.
        let m2 = router_fingerprint(&router, "alpha-elohim-host", Some("e0352a7"));
        assert_eq!(m.digest, m2.digest);
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("\"doorwayId\""), "{json}");
        assert!(json.contains("\"buildId\":\"e0352a7\""), "{json}");
    }

    #[test]
    fn different_heads_produce_different_digest() {
        let a = EprRouter::new();
        a.replace_all(vec![sample_projection("/lamad", "epr-AAA")]);
        let b = EprRouter::new();
        b.replace_all(vec![sample_projection("/lamad", "epr-BBB")]);
        assert_ne!(
            router_fingerprint(&a, "x", None).digest,
            router_fingerprint(&b, "x", None).digest
        );
    }

    #[test]
    fn empty_router_still_mints_a_cid_digest() {
        let router = EprRouter::new();
        let m = router_fingerprint(&router, "x", None);
        assert!(m.heads.is_empty());
        assert!(m.digest.starts_with("bafy"), "got {}", m.digest);
    }

    /// Build a `CoherenceManifest` directly from `(url_path, epr_id)` pairs,
    /// minting the same CIDv1 dag-cbor digest `router_fingerprint` would so the
    /// comparison tests exercise the real digest-equality path.
    fn sample_manifest(id: &str, heads: &[(&str, &str)]) -> CoherenceManifest {
        let mut heads: Vec<EprHeadFingerprint> = heads
            .iter()
            .map(|(url_path, epr_id)| EprHeadFingerprint {
                url_path: (*url_path).to_string(),
                epr_id: (*epr_id).to_string(),
            })
            .collect();
        heads.sort_by(|a, b| a.url_path.cmp(&b.url_path).then(a.epr_id.cmp(&b.epr_id)));
        let preimage = serde_ipld_dagcbor::to_vec(&heads).expect("dag-cbor encode");
        let mh = Code::Sha2_256.digest(&preimage);
        let digest = Cid::new_v1(DAG_CBOR_CODEC, mh).to_string();
        CoherenceManifest {
            doorway_id: id.to_string(),
            generation: 1,
            heads,
            digest,
            build_id: None,
        }
    }

    #[test]
    fn compare_marks_agreement_and_divergent_paths() {
        let me = sample_manifest("alpha", &[("/lamad", "A"), ("/qahal", "B")]);
        let peer = sample_manifest("apex", &[("/lamad", "A"), ("/qahal", "C")]);
        let pc = compare_to_peer(&me, "apex", true, Some(&peer));
        assert_eq!(pc.doorway_id, "apex");
        assert!(pc.reachable);
        assert!(!pc.agrees);
        assert_eq!(pc.divergent_paths, vec!["/qahal".to_string()]);
    }

    #[test]
    fn compare_unreachable_peer_does_not_agree() {
        let me = sample_manifest("alpha", &[("/lamad", "A")]);
        let pc = compare_to_peer(&me, "apex", false, None);
        assert!(!pc.reachable);
        assert!(!pc.agrees);
    }

    #[test]
    fn coherence_view_in_agreement_when_all_peers_match() {
        let me = sample_manifest("alpha", &[("/lamad", "A")]);
        let peers = vec![compare_to_peer(
            &me,
            "apex",
            true,
            Some(&sample_manifest("apex", &[("/lamad", "A")])),
        )];
        let view = build_coherence_view(&me, peers);
        assert!(view.in_agreement);
    }
}
