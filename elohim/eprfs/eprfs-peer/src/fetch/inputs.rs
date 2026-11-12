//! Input documents reuse the artifact's manifest, tier membership and disclosure gate.
//! These are private bytes named by the existing key, never another custody Produce.

use super::*;

/// Store the exact input document, refusing bytes that do not name the key's input CID.
///
/// # Errors
/// A channel error on a CID mismatch or a store failure.
pub async fn put_input_document<S: EprfsStorage + ?Sized>(
    store: &S,
    input: &BlobCid,
    bytes: Bytes,
) -> Result<BlobCid, PeerError> {
    let failed = |reason| PeerError::ChannelTree { reason };
    if !input.verifies(&bytes) {
        return Err(failed("input document CID mismatch".into()));
    }
    store
        .put_blob_verified(input, bytes)
        .await
        .map_err(|e| failed(e.to_string()))?;
    Ok(input.clone())
}

/// Fetch the input document for a recorded key, verifying its manifest and then its bytes.
/// No artifact chunks are fetched and no executable is installed. The caller supplies holders
/// resolved through the device roster, just as for artifact fetch.
///
/// # Errors
/// A typed fetch gap when unavailable, refused, oversized, timed out or corrupt.
pub async fn fetch_input_document(
    endpoint: &Endpoint,
    key: &DerivationKey,
    output: &BlobCid,
    holders: &[Holder],
    budget: &FetchBudget,
) -> Result<Bytes, FetchGap> {
    let progress = Progress::default();
    let bounds = Bounds {
        budget,
        deadline: Instant::now() + budget.total,
        progress: &progress,
    };
    let accepted = accept_manifest(
        endpoint,
        Some(key),
        output,
        holders,
        &bounds,
        &mut BTreeSet::new(),
    )
    .await?;
    let holder = &holders[accepted.holder];
    let input = key.input.cid();
    let request = TierRequest::Record {
        artifact: output.clone(),
        manifest: accepted.cid.clone(),
        key: Some(crate::key_cid(key).map_err(|e| FetchGap::ManifestRejected {
            reason: e.to_string(),
        })?),
        cid: Some(input.clone()),
    };
    let bound = request_bound(
        relay_only(&holder.addr),
        u64::from(budget.manifest_cap),
        budget,
        bounds.deadline,
    );
    let received = AtomicU64::new(0);
    let answer = tokio::time::timeout(
        bound.wait,
        tier_request_counted(
            &accepted.connection,
            &request,
            budget.manifest_cap,
            &received,
        ),
    )
    .await;
    accepted.connection.close(VarInt::from_u32(0), b"");
    match answer {
        Ok(Ok(TierAnswer::Blob { cid, bytes })) if cid == *input && input.verifies(&bytes) => {
            Ok(bytes)
        }
        Ok(Ok(TierAnswer::Blob { .. })) => Err(FetchGap::ManifestRejected {
            reason: "input document CID mismatch".into(),
        }),
        Ok(Ok(TierAnswer::Unanswered { why, .. })) => Err(FetchGap::Refused {
            device: holder.device.clone(),
            refusal: refusal_text(&why.why),
        }),
        Ok(Ok(TierAnswer::Failed { reason })) => Err(FetchGap::Refused {
            device: holder.device.clone(),
            refusal: reason,
        }),
        Ok(Err(PeerError::TierFrameTooLarge { len, cap })) => {
            Err(FetchGap::ManifestTooLarge { len, cap })
        }
        Err(_) => Err(FetchGap::DeadlineExceeded {
            fetched: 0,
            total: 0,
        }),
        Ok(other) => Err(FetchGap::Refused {
            device: holder.device.clone(),
            refusal: format!("input document unavailable: {other:?}"),
        }),
    }
}
