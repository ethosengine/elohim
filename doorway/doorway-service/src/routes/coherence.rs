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

use std::sync::{Arc, LazyLock};

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

/// Process-constant deploy commit SHA. `BuildInfo::new` does work (env reads,
/// string formatting) for a value that never changes within a process, so
/// compute it ONCE and reuse it for every `/api/v1/federation/coherence` request
/// (and the per-tick coherence probe constructs its own once, in the loop setup).
static SELF_BUILD_COMMIT: LazyLock<String> =
    LazyLock::new(|| elohim_compute::BuildInfo::new("elohim-doorway").commit);

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

/// Mint the canonical content-stable digest over a head set: sort (url_path,
/// epr_id) for order-independence, dag-cbor-serialize the sorted set, sha2-256
/// it, and wrap that multihash in a self-describing CIDv1 (dag-cbor codec) →
/// `bafyrei…`. The input slice is sorted in place so the caller can store the
/// canonical (sorted) head order alongside the digest it produced.
///
/// This is the ONE place the digest formula lives. Both `router_fingerprint`
/// (production) and the test helper `sample_manifest` call it, so a test can
/// never stay green while production minting drifts (the prior hand-copied
/// formula was that drift hazard).
pub fn mint_head_set_digest(heads: &mut [EprHeadFingerprint]) -> String {
    heads.sort_by(|a, b| a.url_path.cmp(&b.url_path).then(a.epr_id.cmp(&b.epr_id)));
    // dag-cbor of a Vec of `{String, String}` structs is infallible.
    let preimage = serde_ipld_dagcbor::to_vec(&*heads)
        .expect("dag-cbor encode of EprHeadFingerprint set is infallible");
    let mh = Code::Sha2_256.digest(&preimage);
    Cid::new_v1(DAG_CBOR_CODEC, mh).to_string()
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
    // Single shared mint (sorts `heads` in place + returns the CIDv1 digest).
    let digest = mint_head_set_digest(&mut heads);

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
///
/// **`divergent_paths` is a flat `Vec<String>` BY DESIGN.** Per-path
/// directionality/class (head-conflict vs peer-missing-it vs peer-has-extra) is
/// a deferred refinement; every path listed here genuinely diverges (it is in
/// exactly one of those three buckets), so the flat list is correct and
/// actionable as-is — the operator knows *which* pillars to look at, the
/// classification of *how* is the only thing deferred.
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
/// `reachable` + `peer` are a tri-state from the probe (see
/// `federation::fetch_peer_coherence`):
/// - `(false, None)` — transport error / 404 / non-200 (down or not-yet-deployed):
///   never alarms while a sibling is still deploying.
/// - `(true, None)` — HTTP 200 but the body did not deserialize: a peer serving
///   a 200 garbage body IS a divergence. The `_` arm yields `agrees=false`, so
///   the caller's WARN fires (correct).
/// - `(true, Some(p))` — a usable manifest; we compare digests.
///
/// **Alarm semantics (DELIBERATE — do not change):** `agrees = (peer.digest ==
/// me.digest)` is a pure *content-coherence* check. A peer with coherent content
/// but a different `build_id` (a rolling-deploy version skew) does NOT alarm —
/// build-skew is a deploy *state*, not a content-coherence failure, and alarming
/// on it would manufacture rolling-deploy noise. `build_id` is carried as TRIAGE
/// CONTEXT (in the verdict, the WARN fields, and the endpoint) so an operator can
/// distinguish content-skew (digests differ, builds equal) from deploy-skew
/// (builds differ) — but only digest mismatch is the alarm condition.
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
                // Label from the manifest's authoritative self-report when present
                // (the discovery-advertised `peer_id` can lag or differ); fall back
                // to `peer_id` only in the `_` arm where no manifest exists.
                doorway_id: if p.doorway_id.is_empty() {
                    peer_id.to_string()
                } else {
                    p.doorway_id.clone()
                },
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
///
/// `CoherenceView` / `build_coherence_view` are TEST-ONLY by design (Task-5
/// deferred): the X-COH-DIAG / SelfHealingView wiring that would surface this
/// composed view on an endpoint is Wave-F3, behind P-DIAGNOSTIC. The per-peer
/// verdicts ARE live — they are stored in `PeerCoherenceCache` and read by the
/// edge-trigger (`divergences_to_warn`); only the *composed* cross-edge view is
/// deferred to F3.
pub fn build_coherence_view(me: &CoherenceManifest, peers: Vec<PeerCoherence>) -> CoherenceView {
    let in_agreement = peers.iter().all(|p| p.agrees);
    CoherenceView {
        self_digest: me.digest.clone(),
        self_build_id: me.build_id.clone(),
        peers,
        in_agreement,
    }
}

/// EDGE-TRIGGER decision (PURE — no I/O, no logging). Given the PRIOR tick's
/// verdicts and the CURRENT tick's, return the subset of CURRENT verdicts whose
/// divergence is NEW or CHANGED — the set the caller should WARN about.
///
/// A current verdict warns iff it is divergent (`reachable && !agrees`) AND
/// either:
///   - there was no PRIOR divergent verdict for that `doorway_id` (newly
///     divergent — a freshly-detected split), OR
///   - the peer's `digest` CHANGED since the prior divergent verdict (the
///     divergence moved to a new head — re-surface it).
///
/// A SUSTAINED divergence (same peer, same digest, still diverging) does NOT
/// re-warn — this is what kills the 60s-per-tick log-spam and turns the
/// `coherence_cache` into a real READER (resolving the write-only-dead-store
/// finding).
///
/// Caveats (documented so they are not later read as bugs):
///   - Keying is by `doorway_id`. After the manifest-id labeling (fix 7), a peer
///     is keyed by its manifest's self-reported id when a manifest is present and
///     by the discovery id when not (200-garbage / unreachable). A
///     manifest-presence TOGGLE for one physical peer can therefore re-key it and
///     double-WARN once across the toggle. Acceptable for a detect-only alarm.
///   - If a peer's digest is UNCHANGED but OUR self-digest changed (so we still
///     diverge), this suppresses the re-warn (digest-unchanged wins). That is the
///     defined behavior: edge-trigger keys on the PEER's digest, not on our own
///     churn.
pub fn divergences_to_warn<'a>(
    prior: &[PeerCoherence],
    current: &'a [PeerCoherence],
) -> Vec<&'a PeerCoherence> {
    current
        .iter()
        .filter(|cur| cur.reachable && !cur.agrees)
        .filter(|cur| {
            match prior
                .iter()
                .find(|p| p.doorway_id == cur.doorway_id && p.reachable && !p.agrees)
            {
                // No prior divergence for this peer → newly divergent → warn.
                None => true,
                // Prior divergence existed → warn only if the digest moved.
                Some(prev) => prev.digest != cur.digest,
            }
        })
        .collect()
}

/// GENERATION-GATE decision (PURE) for the per-tick self-manifest recompute.
/// `true` iff there is no cached manifest yet OR the router generation changed
/// since the cached manifest was minted. When `false`, the caller reuses the
/// cached `CoherenceManifest` instead of re-running the clone+sort+dag-cbor+
/// sha2-256+CID mint every tick (the sitemap materializer uses the same gate).
pub fn should_recompute_self_manifest(cached: Option<&(u64, CoherenceManifest)>, gen: u64) -> bool {
    match cached {
        None => true,
        Some((cached_gen, _)) => *cached_gen != gen,
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
    // Process-constant build commit — hoisted into a LazyLock so it is computed
    // once, not per request (it is the same value every time).
    let manifest = router_fingerprint(
        state.epr_router.as_ref(),
        &doorway_id,
        Some(SELF_BUILD_COMMIT.as_str()),
    );
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

/// Test-support: build a minimal `EprProjectionView` for a `(url_path, epr_id)`
/// pair. `pub(crate)` so sibling test modules (e.g. `services::federation`) can
/// populate an `EprRouter` to mint a real self-manifest without duplicating the
/// 19-field struct.
#[cfg(test)]
pub(crate) fn sample_projection(
    url_path: &str,
    epr_id: &str,
) -> elohim_views::projection::EprProjectionView {
    use elohim_views::projection::{EprProjectionView, ProjectionMode};
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::epr_router::EprRouter;

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
    /// minting the digest via the SHARED `mint_head_set_digest` so production and
    /// test minting can never drift apart (fix 9 — the prior hand-copied formula
    /// could stay green while `router_fingerprint` changed).
    fn sample_manifest(id: &str, heads: &[(&str, &str)]) -> CoherenceManifest {
        sample_manifest_gen(id, heads, 1)
    }

    /// `sample_manifest` with an explicit generation (for the gen-gate tests).
    fn sample_manifest_gen(id: &str, heads: &[(&str, &str)], generation: u64) -> CoherenceManifest {
        let mut heads: Vec<EprHeadFingerprint> = heads
            .iter()
            .map(|(url_path, epr_id)| EprHeadFingerprint {
                url_path: (*url_path).to_string(),
                epr_id: (*epr_id).to_string(),
            })
            .collect();
        // Shared mint sorts `heads` in place and returns the CIDv1 digest.
        let digest = mint_head_set_digest(&mut heads);
        CoherenceManifest {
            doorway_id: id.to_string(),
            generation,
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

    // ── Fix 7: label from the manifest's authoritative self-report ───────────
    #[test]
    fn compare_labels_from_manifest_doorway_id_not_discovery_id() {
        // The peer manifest self-reports "apex-canonical"; discovery advertised a
        // different/stale id "apex-stale". The verdict must carry the manifest's id.
        let me = sample_manifest("alpha", &[("/lamad", "A")]);
        let peer = sample_manifest("apex-canonical", &[("/lamad", "B")]);
        let pc = compare_to_peer(&me, "apex-stale", true, Some(&peer));
        assert_eq!(
            pc.doorway_id, "apex-canonical",
            "verdict must label from the manifest's self-reported doorway_id"
        );
    }

    #[test]
    fn compare_unreachable_falls_back_to_discovery_id() {
        // No manifest → fall back to the discovery-advertised peer_id.
        let me = sample_manifest("alpha", &[("/lamad", "A")]);
        let pc = compare_to_peer(&me, "apex-discovery", false, None);
        assert_eq!(pc.doorway_id, "apex-discovery");
        assert!(!pc.reachable);
        assert!(!pc.agrees);
    }

    #[test]
    fn compare_reachable_200_garbage_is_divergence() {
        // Fix 2 wiring: a (reachable=true, manifest=None) probe result — a peer
        // that answered 200 with an undeserializable body — must yield a
        // divergence verdict (reachable, !agrees) so the caller WARNs.
        let me = sample_manifest("alpha", &[("/lamad", "A")]);
        let pc = compare_to_peer(&me, "apex", true, None);
        assert!(pc.reachable, "200-garbage peer is reachable");
        assert!(!pc.agrees, "200-garbage peer does not agree → divergence");
    }

    // ── Fix 5: edge-trigger divergence WARN ──────────────────────────────────
    fn diverging_verdict(id: &str, digest: &str) -> PeerCoherence {
        PeerCoherence {
            doorway_id: id.to_string(),
            reachable: true,
            digest: Some(digest.to_string()),
            build_id: None,
            agrees: false,
            divergent_paths: vec!["/lamad".to_string()],
        }
    }
    fn agreeing_verdict(id: &str) -> PeerCoherence {
        PeerCoherence {
            doorway_id: id.to_string(),
            reachable: true,
            digest: Some("bafy-same".to_string()),
            build_id: None,
            agrees: true,
            divergent_paths: vec![],
        }
    }

    #[test]
    fn edge_trigger_warns_newly_divergent_peer() {
        // Prior tick: peer agreed (no divergence). Current: it diverges → WARN.
        let prior = vec![agreeing_verdict("apex")];
        let current = vec![diverging_verdict("apex", "bafy-X")];
        let warns = divergences_to_warn(&prior, &current);
        assert_eq!(warns.len(), 1);
        assert_eq!(warns[0].doorway_id, "apex");
    }

    #[test]
    fn edge_trigger_warns_on_first_ever_tick() {
        // No prior verdict at all (empty cache) → a divergence is new → WARN.
        let prior: Vec<PeerCoherence> = vec![];
        let current = vec![diverging_verdict("apex", "bafy-X")];
        assert_eq!(divergences_to_warn(&prior, &current).len(), 1);
    }

    #[test]
    fn edge_trigger_suppresses_sustained_divergence() {
        // Same peer, same digest, still diverging across ticks → NO re-warn.
        let prior = vec![diverging_verdict("apex", "bafy-X")];
        let current = vec![diverging_verdict("apex", "bafy-X")];
        assert!(
            divergences_to_warn(&prior, &current).is_empty(),
            "a sustained, unchanged divergence must not re-warn (kills 60s log-spam)"
        );
    }

    #[test]
    fn edge_trigger_rewarns_when_peer_digest_changed() {
        // Still diverging but the peer moved to a NEW head (digest changed) → WARN.
        let prior = vec![diverging_verdict("apex", "bafy-X")];
        let current = vec![diverging_verdict("apex", "bafy-Y")];
        let warns = divergences_to_warn(&prior, &current);
        assert_eq!(warns.len(), 1);
        assert_eq!(warns[0].digest.as_deref(), Some("bafy-Y"));
    }

    #[test]
    fn edge_trigger_does_not_warn_on_agreement() {
        let prior = vec![diverging_verdict("apex", "bafy-X")];
        let current = vec![agreeing_verdict("apex")];
        assert!(
            divergences_to_warn(&prior, &current).is_empty(),
            "a peer that came back into agreement must not warn"
        );
    }

    // ── Fix 8: generation-gate self-manifest recompute ───────────────────────
    #[test]
    fn gen_gate_recomputes_when_no_cache() {
        assert!(should_recompute_self_manifest(None, 7));
    }

    #[test]
    fn gen_gate_recomputes_when_generation_changed() {
        let cached = (3u64, sample_manifest_gen("alpha", &[("/lamad", "A")], 3));
        assert!(should_recompute_self_manifest(Some(&cached), 4));
    }

    #[test]
    fn gen_gate_reuses_when_generation_unchanged() {
        let cached = (3u64, sample_manifest_gen("alpha", &[("/lamad", "A")], 3));
        assert!(
            !should_recompute_self_manifest(Some(&cached), 3),
            "unchanged generation must reuse the cached manifest (no re-mint)"
        );
    }

    // ── Fix 9: shared digest mint between production and the helper ───────────
    #[test]
    fn shared_mint_matches_router_fingerprint() {
        // The helper-minted digest must equal the production-minted digest for
        // the same head set — the single shared formula guarantees it.
        let router = EprRouter::new();
        router.replace_all(vec![
            sample_projection("/lamad", "epr-L"),
            sample_projection("/qahal", "epr-Q"),
        ]);
        let prod = router_fingerprint(&router, "alpha", None);
        let helper = sample_manifest("alpha", &[("/lamad", "epr-L"), ("/qahal", "epr-Q")]);
        assert_eq!(
            prod.digest, helper.digest,
            "shared mint: helper and router_fingerprint must agree on the digest"
        );
    }
}
