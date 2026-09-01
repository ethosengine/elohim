//! Bootstrap Steward Pattern (mishpat) — ported from imagodei reference.
//!
//! See `elohim/holochain/dna/imagodei/zomes/imagodei/src/bootstrap_steward.rs`
//! for the full pattern documentation. This is the mishpat copy (governance
//! DNA) — each DNA has its own because each Cargo workspace is independent
//! and `DnaProperties` must be a concrete type in each integrity+coordinator
//! pair.
//!
//! In mishpat, the bootstrap steward is the initial `constitutional`-tier
//! steward at DNA install time. Authority is **not** exclusive to this
//! pubkey at any point; this module exposes only **identity**. Authority
//! checks must flow through the stewardship-grant resolution layer, not
//! through `is_bootstrap_steward`. Frame rationale:
//! `genesis/docs/superpowers/specs/2026-04-21-bootstrap-steward-authority-frame-design.md`

use hdk::prelude::*;

#[derive(Debug)]
pub enum BootstrapStewardError {
    NotConfigured,
    Malformed(String),
    DnaInfo(String),
}

impl core::fmt::Display for BootstrapStewardError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotConfigured => write!(
                f,
                "bootstrap steward is not configured in this network's DNA modifiers; \
                 set modifiers.properties.progenitor_pubkey in happ.yaml before publishing"
            ),
            Self::Malformed(e) => {
                write!(
                    f,
                    "bootstrap steward pubkey in DNA modifiers is malformed: {e}"
                )
            }
            Self::DnaInfo(e) => write!(f, "failed to read DNA info: {e}"),
        }
    }
}

impl From<BootstrapStewardError> for WasmError {
    fn from(err: BootstrapStewardError) -> Self {
        wasm_error!(WasmErrorInner::Guest(err.to_string()))
    }
}

#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]
pub struct DnaProperties {
    /// Holochain-primitive name. Surface language: "bootstrap steward pubkey".
    #[serde(default)]
    pub progenitor_pubkey: Option<String>,
}

impl DnaProperties {
    pub fn read() -> ExternResult<Option<Self>> {
        let info = dna_info().map_err(|e| BootstrapStewardError::DnaInfo(e.to_string()))?;
        let bytes = info.modifiers.properties;
        if bytes.bytes().is_empty() {
            return Ok(None);
        }
        holochain_serialized_bytes::decode(bytes.bytes()).map_err(|e: SerializedBytesError| {
            BootstrapStewardError::Malformed(e.to_string()).into()
        })
    }
}

pub fn bootstrap_steward() -> ExternResult<AgentPubKey> {
    let props = DnaProperties::read()?.ok_or(BootstrapStewardError::NotConfigured)?;
    let pubkey = props
        .progenitor_pubkey
        .ok_or(BootstrapStewardError::NotConfigured)?;
    AgentPubKey::try_from(pubkey)
        .map_err(|e| BootstrapStewardError::Malformed(e.to_string()).into())
}

pub fn maybe_bootstrap_steward() -> ExternResult<Option<AgentPubKey>> {
    let Some(props) = DnaProperties::read()? else {
        return Ok(None);
    };
    let Some(pubkey) = props.progenitor_pubkey else {
        return Ok(None);
    };
    AgentPubKey::try_from(pubkey)
        .map(Some)
        .map_err(|e| BootstrapStewardError::Malformed(e.to_string()).into())
}

pub fn am_i_bootstrap_steward() -> ExternResult<bool> {
    let Some(steward) = maybe_bootstrap_steward()? else {
        return Ok(false);
    };
    let me = agent_info()?.agent_initial_pubkey;
    Ok(steward == me)
}

#[hdk_extern]
pub fn get_bootstrap_steward(_: ()) -> ExternResult<Option<AgentPubKey>> {
    maybe_bootstrap_steward()
}

#[hdk_extern]
pub fn is_bootstrap_steward(_: ()) -> ExternResult<bool> {
    am_i_bootstrap_steward()
}
