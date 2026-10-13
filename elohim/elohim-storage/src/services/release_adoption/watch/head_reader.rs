//! The head read, behind a seam.
//!
//! [`AdoptionController::resolve_head`] is the one call every sweep spends, so
//! it is the one a test has to be able to count and to answer. Production is
//! [`ConductorHeads`], which makes exactly the call `resolve_head` made inline
//! before the seam existed; the contract (I1, `GetStrategy::Local`,
//! `Background` class) is documented there and unchanged here.

use super::*;

/// Where a channel's canonical head is read from. One method, because one
/// call is all the controller ever asks of it.
#[async_trait::async_trait]
pub(super) trait HeadReader: Send + Sync {
    async fn resolve(&self, channel_id: &str) -> Answer<ContentHeadWire>;
}

/// The production [`HeadReader`]: this node's own conductor.
pub(super) struct ConductorHeads(pub(super) Arc<HcClient>);

#[async_trait::async_trait]
impl HeadReader for ConductorHeads {
    async fn resolve(&self, channel_id: &str) -> Answer<ContentHeadWire> {
        let payload = match rmp_serde::to_vec_named(&channel_id.to_string()) {
            Ok(p) => p,
            Err(_) => return Answer::Unreachable,
        };
        let bytes = match self
            .0
            .call_zome_timed(
                "content_store",
                "resolve_content_head_local",
                payload,
                AdmissionClass::Background,
            )
            .await
        {
            Ok((bytes, _timing)) => bytes,
            Err(e) => {
                tracing::debug!(
                    channel = %channel_id,
                    error = %e,
                    "release-adoption: head resolve failed — unreachable, never absence"
                );
                return Answer::Unreachable;
            }
        };
        match rmp_serde::from_slice::<Option<ContentHeadWire>>(&bytes) {
            // The conductor ANSWERED and reports no head. That is an observed
            // absence, and it is the C4-correct input to an idle verdict.
            Ok(head) => Answer::observed_absence(head),
            Err(e) => {
                tracing::warn!(
                    channel = %channel_id,
                    error = %e,
                    "release-adoption: could not decode ContentHeadWire"
                );
                Answer::Unreachable
            }
        }
    }
}
