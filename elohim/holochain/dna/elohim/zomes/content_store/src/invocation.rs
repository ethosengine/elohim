//! Native boundary for invocation conveniences. Never trust an input-side tag.
use hdk::prelude::*;
use qahal_types::invocation_mandate::{
    InvocationContext, InvocationMandate, MANDATE_TAG, MAX_TAG_BYTES,
};

fn refuse(reason: &str) -> WasmError {
    wasm_error!(WasmErrorInner::Guest(format!(
        "invocation authority: {reason}"
    )))
}

pub(crate) fn authorize(
    operation: &str,
    id: &str,
    root: &ActionHash,
    delegate: &AgentPubKey,
    expiry: Option<Timestamp>,
) -> ExternResult<Option<InvocationMandate>> {
    authorize_at(operation, id, root, delegate, expiry, None)
}

/// `witnessed_at` is supplied only after exact native historical verification.
pub(crate) fn authorize_at(
    operation: &str,
    id: &str,
    root: &ActionHash,
    delegate: &AgentPubKey,
    expiry: Option<Timestamp>,
    witnessed_at: Option<Timestamp>,
) -> ExternResult<Option<InvocationMandate>> {
    let me = agent_info()?.agent_initial_pubkey;
    let info = call_info()?;
    match info.cap_grant {
        CapGrant::ChainAuthor(author) if author == me && info.provenance == me => Ok(None),
        CapGrant::RemoteAgent(grant) => {
            if !matches!(grant.access, CapAccess::Assigned { ref assignees, .. } if assignees.contains(&info.provenance))
                || !matches!(grant.functions, GrantedFunctions::Listed(_))
                || grant.tag.len() > MAX_TAG_BYTES
            {
                return Err(refuse("assigned, function-listed mandate required"));
            }
            let encoded = grant.tag.strip_prefix(MANDATE_TAG).ok_or_else(|| {
                refuse("explicit scoped ceremony required; function access is insufficient")
            })?;
            let mandate: InvocationMandate =
                serde_json::from_str(encoded).map_err(|_| refuse("malformed native mandate"))?;
            mandate
                .check(&InvocationContext {
                    issuer: &me.to_string(),
                    requester: &info.provenance.to_string(),
                    dna: &dna_info()?.hash.to_string(),
                    operation,
                    now: witnessed_at.unwrap_or(sys_time()?).as_micros(),
                    subject: Some((id, &root.to_string())),
                    delegate: Some(&delegate.to_string()),
                    grant_expiry: expiry.map(|t| t.as_micros()),
                    payload_json: None,
                })
                .map_err(refuse)?;
            Ok(Some(mandate))
        }
        _ => Err(refuse("authenticated provenance does not own this cell")),
    }
}

pub(crate) fn verify_device(
    binding: ActionHash,
    device: AgentPubKey,
) -> ExternResult<qahal_types::VerifiedDevice> {
    let input = qahal_types::VerifyDeviceInput {
        binding,
        expected_device: device,
        expected_content_dna: dna_info()?.hash,
    };
    match call(
        CallTargetCell::OtherRole("mishpat".into()),
        ZomeName::from("mishpat"),
        "verify_device_binding".into(),
        None,
        input,
    )? {
        ZomeCallResponse::Ok(bytes) => bytes
            .decode()
            .map_err(|_| refuse("device verification response malformed")),
        _ => Err(refuse(
            "device relationship verification unavailable — PENDING",
        )),
    }
}

pub(crate) fn verify_device_publication(
    binding: ActionHash,
    device: AgentPubKey,
    root: ActionHash,
    head: ActionHash,
    witness: ActionHash,
    root_acceptance: ActionHash,
) -> ExternResult<qahal_types::VerifiedDevicePublication> {
    let input = qahal_types::VerifyDevicePublicationInput {
        publication: qahal_types::DevicePublicationInput {
            device: qahal_types::VerifyDeviceInput {
                binding,
                expected_device: device,
                expected_content_dna: dna_info()?.hash,
            },
            content_root: root,
            content_head: head,
            root_acceptance,
        },
        witness,
    };
    match call(
        CallTargetCell::OtherRole("mishpat".into()),
        ZomeName::from("mishpat"),
        "verify_device_publication".into(),
        None,
        input,
    )? {
        ZomeCallResponse::Ok(bytes) => bytes
            .decode()
            .map_err(|_| refuse("controller publication verification response malformed")),
        _ => Err(refuse(
            "controller publication history unavailable — PENDING",
        )),
    }
}
