//! Daemon-independent local custody for chunked derivation artifacts.
//!
//! This is the one-device floor: storage is direct library I/O and observations append to the
//! existing flow store. No endpoint, daemon, peer, credential, or publication head participates.

use bytes::{Bytes, BytesMut};
use elohim_epr_rea::{AgentRef, FlowEvent, FlowRecord, FlowStore, Magnitude, ReaVerb};
use eprfs_core::{BlobCid, DerivationRecord, EprfsStorage};
use eprfs_storage::{
    put_tier_blob, stream_tier_blob, TierBlobChunk, TierBlobDraw, TierBlobPut, TierBlobSink,
    TierBlobStreamDraw,
};

use crate::PeerError;

/// Attribution and scope supplied by the local caller. The storage path has no clock or identity
/// resolver of its own, and this context does not claim signer standing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalBlobContext {
    pub custodian: AgentRef,
    pub in_scope_of: cid::Cid,
    /// RFC 3339 timestamp supplied by the caller.
    pub occurred_at: String,
}

/// Put a chunked derivation artifact and record its Produce event without a daemon.
///
/// # Errors
///
/// Returns [`PeerError::LocalBlob`] when custody fails. If custody succeeds but the flow append
/// fails, returns [`PeerError::LocalBlobUnrecorded`] carrying the persisted manifest CID; success
/// is never reported without the event.
pub async fn put_local_blob<S, F>(
    store: &S,
    flow: &mut F,
    record: DerivationRecord,
    bytes: Bytes,
    context: &LocalBlobContext,
) -> Result<TierBlobPut, PeerError>
where
    S: EprfsStorage + ?Sized,
    F: FlowStore + ?Sized,
{
    let put = put_tier_blob(store, record, bytes)
        .await
        .map_err(PeerError::LocalBlob)?;
    let event = FlowEvent {
        action: ReaVerb::Produce,
        provider: AgentRef(put.manifest.record.computed_by.clone()),
        receiver: context.custodian.clone(),
        resource: *put.manifest.record.output.as_cid(),
        quantity: Magnitude::Count {
            value: put.manifest.byte_len as f64,
            unit: "bytes".into(),
        },
        process: None,
        in_scope_of: context.in_scope_of,
        fulfills: Vec::new(),
        satisfies: Vec::new(),
        occurred_at: context.occurred_at.clone(),
        classified_as: vec![
            "custody-blob".into(),
            put.manifest.record.output.to_string(),
            format!("manifest:{}", put.manifest_cid),
            "copy:self".into(),
        ],
    };
    flow.append(FlowRecord::Event(event))
        .map_err(|source| PeerError::LocalBlobUnrecorded {
            manifest: Box::new(put.manifest_cid.clone()),
            source,
        })?;
    Ok(put)
}

/// Stream a local chunked artifact and record one Use event for its typed outcome.
///
/// # Errors
///
/// Returns [`PeerError::LocalBlobDrawUnrecorded`] when the one completion/stoppage Use event cannot
/// be appended. A sink may then hold a verified prefix, but the caller receives no completed draw.
pub async fn stream_local_blob<S, F, T>(
    store: &S,
    flow: &mut F,
    manifest_cid: &BlobCid,
    context: &LocalBlobContext,
    sink: &mut T,
) -> Result<TierBlobStreamDraw, PeerError>
where
    S: EprfsStorage + ?Sized,
    F: FlowStore + ?Sized,
    T: TierBlobSink + ?Sized,
{
    let draw = stream_tier_blob(store, manifest_cid, sink).await;
    let (resource, byte_len, outcome) = match &draw {
        TierBlobStreamDraw::Completed { manifest } => (
            *manifest.record.output.as_cid(),
            manifest.byte_len as f64,
            "draw:hit".to_string(),
        ),
        TierBlobStreamDraw::Miss { .. } => (*manifest_cid.as_cid(), 0.0, "draw:miss".to_string()),
        TierBlobStreamDraw::Unavailable { .. } => {
            (*manifest_cid.as_cid(), 0.0, "draw:unavailable".to_string())
        }
    };
    let mut classified_as = vec![
        "custody-blob".into(),
        manifest_cid.to_string(),
        outcome,
        "copy:self".into(),
    ];
    if let Some(reason) = draw.reason() {
        classified_as.push(format!("reason:{reason}"));
    }
    let event = FlowEvent {
        action: ReaVerb::Use,
        provider: context.custodian.clone(),
        receiver: context.custodian.clone(),
        resource,
        quantity: Magnitude::Count {
            value: byte_len,
            unit: "bytes".into(),
        },
        process: None,
        in_scope_of: context.in_scope_of,
        fulfills: Vec::new(),
        satisfies: Vec::new(),
        occurred_at: context.occurred_at.clone(),
        classified_as,
    };
    flow.append(FlowRecord::Event(event))
        .map_err(|source| PeerError::LocalBlobDrawUnrecorded {
            manifest: Box::new(manifest_cid.clone()),
            source,
        })?;
    Ok(draw)
}

/// Read a complete local chunked artifact into memory and record exactly one Use event.
///
/// This convenience wrapper is intended for small artifacts. It withholds collected bytes if the
/// Use event cannot be recorded; large-artifact callers should use [`stream_local_blob`].
///
/// # Errors
///
/// Returns [`PeerError::LocalBlobDrawUnrecorded`] when the Use event cannot be appended.
pub async fn read_local_blob<S, F>(
    store: &S,
    flow: &mut F,
    manifest_cid: &BlobCid,
    context: &LocalBlobContext,
) -> Result<TierBlobDraw, PeerError>
where
    S: EprfsStorage + ?Sized,
    F: FlowStore + ?Sized,
{
    let mut bytes = BytesMut::new();
    let mut collect = |_: &TierBlobChunk, chunk: Bytes| {
        bytes.extend_from_slice(&chunk);
        Ok(())
    };
    match stream_local_blob(store, flow, manifest_cid, context, &mut collect).await? {
        TierBlobStreamDraw::Completed { manifest } => Ok(TierBlobDraw::Hit {
            manifest,
            bytes: bytes.freeze(),
        }),
        TierBlobStreamDraw::Miss { manifest_cid, why } => {
            Ok(TierBlobDraw::Miss { manifest_cid, why })
        }
        TierBlobStreamDraw::Unavailable { manifest_cid, why } => {
            Ok(TierBlobDraw::Unavailable { manifest_cid, why })
        }
    }
}
