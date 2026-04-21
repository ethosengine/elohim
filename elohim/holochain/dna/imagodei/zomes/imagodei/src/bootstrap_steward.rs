//! Bootstrap Steward Pattern — reference implementation (imagodei).
//!
//! "Bootstrap steward" is our stewardship-aligned name for what Holochain
//! primitives call `progenitor_pubkey`: the pubkey that first seeds a network
//! and bootstraps authority. Authority is then **graduated** to later-joining
//! stewards per `project_stewardship_philosophy.md` — it is not retained as
//! sovereignty.
//!
//! The Holochain schema field name `progenitor_pubkey` stays as-is (we cannot
//! rename their primitive). Everywhere in our surface — function names, logs,
//! errors, docs, UI — we say "bootstrap steward" or "founding steward".
//!
//! Source of truth: `happ.yaml` role.dna.modifiers.properties.progenitor_pubkey.
//! Each DNA has its own `DnaProperties` struct that reads this modifier.
//!
//! # Usage
//!
//! ```ignore
//! use crate::bootstrap_steward::{bootstrap_steward, am_i_bootstrap_steward};
//!
//! let steward = bootstrap_steward()?;
//! let is_me = am_i_bootstrap_steward()?;
//! ```
//!
//! # Graduated authority
//!
//! This module only *identifies* the bootstrap steward. It does not enforce
//! exclusive authority. Per stewardship philosophy, authority flows from the
//! bootstrap steward to later stewards via explicit grants (attestations,
//! relationships) — not by keeping the bootstrap pubkey as the only admin.
//!
//! Integrity zome validators may reject bootstrap-only actions taken by
//! non-bootstrap-steward agents; coordinator functions use these helpers to
//! short-circuit before requesting chain authorship. Either way, this is the
//! identity anchor, not a capability gate.

use hdk::prelude::*;

/// Errors raised by the bootstrap-steward helpers.
///
/// These are surfaced with stewardship vocabulary. Holochain's `progenitor_*`
/// primitive names do not leak into caller-facing error messages.
#[derive(Debug)]
pub enum BootstrapStewardError {
    /// The DNA was installed without `modifiers.properties.progenitor_pubkey`
    /// (happ.yaml left it as `~`). Publishing an alpha network requires this.
    NotConfigured,
    /// The configured bootstrap-steward pubkey failed to parse as an AgentPubKey.
    Malformed(String),
    /// dna_info() call failed at the HDK boundary.
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
                write!(f, "bootstrap steward pubkey in DNA modifiers is malformed: {e}")
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

/// DNA modifier-properties shape expected by imagodei.
///
/// The field is named `progenitor_pubkey` to match the Holochain primitive and
/// the `happ.yaml` modifiers.properties schema. In surface language, it is the
/// **bootstrap steward pubkey**.
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]
pub struct DnaProperties {
    /// Holochain-primitive name. Surface language: "bootstrap steward pubkey".
    pub progenitor_pubkey: String,
}

impl DnaProperties {
    /// Read the DNA's modifier properties from the current conductor context.
    ///
    /// Returns `NotConfigured` if no properties were provided at install time
    /// (e.g., `happ.yaml` left `modifiers.properties: ~`).
    pub fn read() -> ExternResult<Self> {
        let info = dna_info().map_err(|e| BootstrapStewardError::DnaInfo(e.to_string()))?;
        let bytes = info.modifiers.properties;
        // `properties` is `SerializedBytes`. An empty/nil properties block
        // deserializes as unit `()` or empty bytes — treat as NotConfigured.
        if bytes.bytes().is_empty() {
            return Err(BootstrapStewardError::NotConfigured.into());
        }
        bytes
            .try_into()
            .map_err(|e: SerializedBytesError| {
                BootstrapStewardError::Malformed(e.to_string()).into()
            })
    }
}

/// Returns the pubkey of the bootstrap steward for this DNA.
///
/// Errors if the DNA was installed without a bootstrap steward configured
/// (`modifiers.properties.progenitor_pubkey` left as `~`). Callers that treat
/// "no bootstrap steward" as a legitimate state should use
/// [`maybe_bootstrap_steward`] instead.
pub fn bootstrap_steward() -> ExternResult<AgentPubKey> {
    let props = DnaProperties::read()?;
    AgentPubKey::try_from(props.progenitor_pubkey.clone())
        .map_err(|e| BootstrapStewardError::Malformed(e.to_string()).into())
}

/// Like [`bootstrap_steward`] but returns `None` when the DNA has no bootstrap
/// steward configured. Malformed pubkeys still error.
pub fn maybe_bootstrap_steward() -> ExternResult<Option<AgentPubKey>> {
    match DnaProperties::read() {
        Ok(props) => AgentPubKey::try_from(props.progenitor_pubkey.clone())
            .map(Some)
            .map_err(|e| BootstrapStewardError::Malformed(e.to_string()).into()),
        Err(e) => {
            // Match the typed error (which was converted to WasmError::Guest).
            // If it's NotConfigured, surface as None; otherwise propagate.
            let msg = e.to_string();
            if msg.contains("bootstrap steward is not configured") {
                Ok(None)
            } else {
                Err(e)
            }
        }
    }
}

/// Returns true when the calling agent is the bootstrap steward for this DNA.
///
/// This is identity, not authority: callers that want "only the bootstrap
/// steward may perform X" should gate on this AND on additional stewardship
/// checks (e.g., a later attestation from the bootstrap steward delegating
/// the capability). Per stewardship philosophy, authority is graduated.
pub fn am_i_bootstrap_steward() -> ExternResult<bool> {
    let steward = bootstrap_steward()?;
    let me = agent_info()?.agent_initial_pubkey;
    Ok(steward == me)
}

// =============================================================================
// Coordinator externs
// =============================================================================

/// Expose the bootstrap steward pubkey (if any) to callers.
#[hdk_extern]
pub fn get_bootstrap_steward(_: ()) -> ExternResult<Option<AgentPubKey>> {
    maybe_bootstrap_steward()
}

/// Expose the "am I the bootstrap steward?" check.
#[hdk_extern]
pub fn is_bootstrap_steward(_: ()) -> ExternResult<bool> {
    // When no steward is configured, no agent is the bootstrap steward.
    let maybe_steward = maybe_bootstrap_steward()?;
    let Some(steward) = maybe_steward else {
        return Ok(false);
    };
    let my_pubkey = agent_info()?.agent_initial_pubkey;
    Ok(steward == my_pubkey)
}
