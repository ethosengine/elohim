//! `POST /admin/test/feedback-notify` — a SECOND INGRESS to the real
//! notification receiver (accountable-correction contract §4).
//!
//! **What this is.** The `feedback-signal` notification receiver lives in
//! [`crate::epr_atom_service::EprAtomService::handle`] behind the
//! `IntegrityNotify { kind: "feedback-signal" }` request variant. Both
//! transports reach it the same way: libp2p through
//! `P2PNode::handle_epr_atom_request`, iroh through `EprAtomServiceBackend`.
//! This route is a third caller of that SAME function with the SAME argument
//! shape — MessagePack payload bytes produced by `rmp_serde::to_vec_named`,
//! exactly as a sending peer produces them — and it returns the receiver's own
//! verdict verbatim.
//!
//! **Why it exists.** Station 2 of the accountable-correction feature has to
//! prove that a notification naming a FOREIGN origin DNA hash is REFUSED rather
//! than trusted. A household mesh cannot manufacture a peer in a different
//! content space, and asserting "nothing was applied" after sending nothing
//! proves nothing. This route delivers a real envelope to the real receiver so
//! the refusal is observed rather than argued.
//!
//! **What it is NOT.** It is not a product read/write surface, it is not in
//! `build_manifest()`, and a doorway must never proxy it. It performs no
//! verification of its own and grants no privilege the receiver does not
//! already grant an anonymous peer: every decision — decode, act-reference
//! presence, DNA scoping, dedup, subscription admission — is made by
//! `EprAtomService`, not here.
//!
//! **Gate.** Off unless `ELOHIM_TEST_FEEDBACK_INGRESS=1` is in the process
//! environment at boot. Read ONCE into a `OnceLock`: a per-request
//! `std::env::var` on a route is the shape that makes parallel tests flake.

use std::sync::{Arc, OnceLock};

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{body::Incoming, Request, Response, StatusCode};
use serde::Deserialize;

use crate::db::DbPool;
use crate::epr_atom_service::EprAtomService;
use crate::error::StorageError;
use crate::hc_client_registry::HcClientRegistry;
use crate::p2p::dedup::DedupLru;
use crate::p2p::epr_atom_protocol::EprAtomRequest;
use crate::p2p::identity_map::CallerIdentity;
use crate::services::response;

/// The `IntegrityNotify` kind this route delivers. Fixed: this ingress exists
/// for the feedback plane, and a free-form kind would make it a generic
/// integrity-injection surface.
const NOTIFY_KIND: &str = "feedback-signal";

static ENABLED: OnceLock<bool> = OnceLock::new();

/// Is the test ingress armed on this process?
pub fn enabled() -> bool {
    *ENABLED.get_or_init(|| {
        std::env::var("ELOHIM_TEST_FEEDBACK_INGRESS")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IngressBody {
    /// The SEMANTIC signal, in the same camelCase shape the wire carries.
    /// Serialized here with `rmp_serde::to_vec_named` — the encoder the real
    /// sender uses — so the receiver sees bytes it cannot distinguish from a
    /// peer's.
    signal: crate::p2p::feedback_signal::FeedbackSignal,
    /// Label the receiver logs as the sending peer. Defaults to a name that is
    /// obviously not a peer id, so a mesh log never reads as a real peer.
    #[serde(default)]
    from: Option<String>,
}

/// Handle `GET /admin/test/feedback-notify` — report the binding only.
///
/// A caller that wants to deliver a reference in THIS space needs to know which
/// space that is, and a caller asserting that a refusal left the binding alone
/// needs to read it before and after. Neither should have to deliver an envelope
/// to find out.
pub fn report(
    hc_registry: Option<&Arc<HcClientRegistry>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    if !enabled() {
        return Ok(response::not_found(
            "no such route (test ingress is not armed on this node)",
        ));
    }
    let origin_dna_hash = hc_registry
        .and_then(|r| r.lamad_client())
        .map(|c| c.cell_id().dna_hash().to_string());
    Ok(response::json_response(
        StatusCode::OK,
        &serde_json::json!({
            "armed": true,
            "kind": NOTIFY_KIND,
            "originDnaHash": origin_dna_hash,
        }),
    ))
}

/// Handle `POST /admin/test/feedback-notify`.
pub async fn handle(
    req: Request<Incoming>,
    pool: &DbPool,
    hc_registry: Option<&Arc<HcClientRegistry>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    if !enabled() {
        // 404, not 403: an unarmed node does not advertise that the route
        // exists at all.
        return Ok(response::not_found(
            "no such route (test ingress is not armed on this node)",
        ));
    }

    let body = req
        .into_body()
        .collect()
        .await
        .map_err(|e| StorageError::InvalidInput(format!("read body: {e}")))?
        .to_bytes();
    let parsed: IngressBody = serde_json::from_slice(&body)
        .map_err(|e| StorageError::InvalidInput(format!("parse body: {e}")))?;

    let payload_bytes = rmp_serde::to_vec_named(&parsed.signal)
        .map_err(|e| StorageError::InvalidInput(format!("encode signal: {e}")))?;

    // The SAME service both transports build, with the same origin-DNA binding.
    // A fresh dedup window per call is deliberate: dedup is a transport
    // concern, and a test that wants to deliver the same act twice must be able
    // to.
    let service = EprAtomService::new(Some(pool.clone()), Arc::new(DedupLru::new()));
    let origin_dna_hash = hc_registry
        .and_then(|r| r.lamad_client())
        .map(|c| c.cell_id().dna_hash().to_string());
    let service = match origin_dna_hash.clone() {
        Some(dna) => service.with_origin_dna_hash(dna),
        None => service,
    };

    let peer_label = parsed.from.as_deref().unwrap_or("test-ingress");
    let verdict = service.handle(
        peer_label,
        CallerIdentity::Anonymous,
        EprAtomRequest::IntegrityNotify {
            kind: NOTIFY_KIND.to_string(),
            payload_bytes,
        },
    );

    // The receiver's own verdict, verbatim. `IntegrityAck { received: false,
    // reason: "foreign origin DNA hash" }` is the refusal station 2 asserts on.
    Ok(response::json_response(
        StatusCode::OK,
        &serde_json::json!({
            "kind": NOTIFY_KIND,
            "from": peer_label,
            // This node's OWN content space, reported so a caller can compare it
            // against the reference it delivered — a refusal is only meaningful
            // against the hash it was measured against, and this is the same
            // value the receiver scoped by.
            "originDnaHash": origin_dna_hash,
            "verdict": verdict,
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The gate is read once. A test that mutated the environment to flip it
    /// would leak into every other test in the binary — the documented
    /// parallel-test flake shape — so this asserts the DEFAULT only.
    #[test]
    fn ingress_is_off_unless_armed_at_boot() {
        // No environment mutation: whatever the harness booted with is the
        // answer, and in a plain `cargo test` run that is "off".
        assert!(
            !enabled() || std::env::var("ELOHIM_TEST_FEEDBACK_INGRESS").is_ok(),
            "the ingress may only be on when the env var armed it"
        );
    }

    #[test]
    fn body_parses_the_semantic_signal_and_its_act_reference() {
        let raw = serde_json::json!({
            "signal": {
                "targetCid": "uhCkkTARGET",
                "signalKind": "correction",
                "evidenceCid": "uhCkkEVIDENCE",
                "standingImpact": "debit-soft",
                "signedBy": "key",
                "signature": "sig",
                "actRef": {
                    "originDnaHash": "uhC0kFOREIGN",
                    "actionHash": "uhCkkACT",
                    "routingKey": "uhCkkTARGET"
                }
            },
            "from": "peer-foreign"
        });
        let parsed: IngressBody = serde_json::from_value(raw).expect("parses");
        let act_ref = parsed.signal.act_ref.expect("carries the reference");
        assert_eq!(act_ref.origin_dna_hash, "uhC0kFOREIGN");
        assert_eq!(parsed.from.as_deref(), Some("peer-foreign"));
    }
}
