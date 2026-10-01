//! Identity signing is an explicit exact-payload ceremony, not a function grant.
use hdk::prelude::*;
use qahal_types::invocation_mandate::{
    InvocationContext, InvocationMandate, MANDATE_TAG, MAX_TAG_BYTES,
};

pub(crate) fn authorize<T: serde::Serialize>(operation: &str, payload: &T) -> ExternResult<()> {
    let refuse = |reason: &str| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "identity invocation: {reason}"
        )))
    };
    let me = agent_info()?.agent_initial_pubkey;
    let info = call_info()?;
    match info.cap_grant {
        CapGrant::ChainAuthor(author) if author == me && info.provenance == me => Ok(()),
        CapGrant::RemoteAgent(grant) => {
            if !matches!(grant.access, CapAccess::Assigned { ref assignees, .. } if assignees.contains(&info.provenance))
                || !matches!(grant.functions, GrantedFunctions::Listed(_))
                || grant.tag.len() > MAX_TAG_BYTES
            {
                return Err(refuse("assigned, function-listed ceremony required"));
            }
            let encoded = grant
                .tag
                .strip_prefix(MANDATE_TAG)
                .ok_or_else(|| refuse("explicit scoped ceremony required"))?;
            let mandate: InvocationMandate =
                serde_json::from_str(encoded).map_err(|_| refuse("malformed native mandate"))?;
            let json = serde_json::to_string(payload).map_err(|_| refuse("ceremony encoding"))?;
            mandate
                .check(&InvocationContext {
                    issuer: &me.to_string(),
                    requester: &info.provenance.to_string(),
                    dna: &dna_info()?.hash.to_string(),
                    operation,
                    now: sys_time()?.as_micros(),
                    subject: None,
                    delegate: None,
                    grant_expiry: None,
                    payload_json: Some(&json),
                })
                .map_err(refuse)
        }
        _ => Err(refuse("authenticated provenance does not own this cell")),
    }
}
