//! Bridge call utilities for cross-DNA communication
//!
//! # RNA Metaphor: Transfer RNA (tRNA)
//!
//! In biology, tRNA carries amino acids to the ribosome during protein synthesis.
//! It doesn't know what protein is being built - it just transfers building blocks.
//!
//! Similarly, `bridge_call` transfers data between DNA cells without knowing
//! the data's meaning. It's the generic transport mechanism for migration.
//!
//! # Usage
//!
//! ```rust,ignore
//! use hc_rna::bridge_call;
//!
//! // Call export function on previous DNA version
//! let old_data: Vec<Content> = bridge_call(
//!     "my-dna-v1",      // Role name in happ.yaml
//!     "coordinator",     // Zome name
//!     "export_all",      // Function name
//!     ()                 // Payload (empty tuple for no args)
//! )?;
//! ```

use hdk::prelude::*;

/// Call a function on another DNA role via bridge
///
/// This is the "tRNA" of the RNA module - it transfers data between cells
/// without knowing what the data represents.
///
/// # Arguments
///
/// * `role_name` - The role name from happ.yaml (e.g., "my-dna-v1")
/// * `zome_name` - The zome to call (e.g., "coordinator")
/// * `fn_name` - The function to call (e.g., "export_all")
/// * `payload` - The input payload (use `()` for no input)
///
/// # Returns
///
/// The deserialized response from the target zome function.
///
/// # Errors
///
/// Returns an error if:
/// - The target role doesn't exist in the hApp
/// - The zome or function doesn't exist
/// - Authorization fails
/// - Network errors occur
/// - Response deserialization fails
///
/// # Example
///
/// ```rust,ignore
/// // Export all content from v1 DNA
/// let content: Vec<ContentExport> = bridge_call(
///     "lamad-v1",
///     "content_store",
///     "export_all_content",
///     ()
/// )?;
///
/// // Get schema version
/// let version: String = bridge_call(
///     "lamad-v1",
///     "content_store",
///     "export_schema_version",
///     ()
/// )?;
/// ```
pub fn bridge_call<I, O>(
    role_name: &str,
    zome_name: &str,
    fn_name: &str,
    payload: I,
) -> ExternResult<O>
where
    I: Serialize + std::fmt::Debug,
    O: serde::de::DeserializeOwned + std::fmt::Debug,
{
    // Use CallTargetCell::OtherRole for cross-DNA calls within the same hApp
    // The role_name maps to the DNA role defined in happ.yaml
    let response = call(
        CallTargetCell::OtherRole(role_name.to_string()),
        ZomeName::from(zome_name),
        FunctionName::from(fn_name),
        None, // CapSecret - None for unrestricted calls
        payload,
    )?;

    match response {
        ZomeCallResponse::Ok(result) => {
            let output: O = result.decode().map_err(|e| {
                wasm_error!(WasmErrorInner::Guest(format!(
                    "Failed to decode bridge call response from {}::{}: {:?}",
                    zome_name, fn_name, e
                )))
            })?;
            Ok(output)
        }
        ZomeCallResponse::Unauthorized(cell_id, zome, func, _) => Err(wasm_error!(
            WasmErrorInner::Guest(format!(
                "Unauthorized bridge call to {:?}::{:?}::{:?} - check capability grants",
                cell_id, zome, func
            ))
        )),
        ZomeCallResponse::NetworkError(err) => Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Network error in bridge call to {}::{}: {}",
            zome_name, fn_name, err
        )))),
        ZomeCallResponse::CountersigningSession(err) => {
            Err(wasm_error!(WasmErrorInner::Guest(format!(
                "Countersigning error in bridge call: {}",
                err
            ))))
        }
        ZomeCallResponse::AuthenticationFailed(_, _) => Err(wasm_error!(WasmErrorInner::Guest(
            "Authentication failed for bridge call - invalid signature".to_string()
        ))),
    }
}

/// Result type for bridge calls with detailed error information
pub type BridgeResult<T> = Result<T, BridgeError>;

/// Detailed error type for bridge call failures
#[derive(Debug, Clone)]
pub enum BridgeError {
    /// Target role, zome, or function not found
    NotFound { role: String, zome: String, func: String },
    /// Capability authorization failed
    Unauthorized { role: String, zome: String, func: String },
    /// Network communication failed
    NetworkError(String),
    /// Response couldn't be deserialized
    DecodingError(String),
    /// Signature verification failed
    AuthenticationFailed,
    /// Other errors
    Other(String),
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BridgeError::NotFound { role, zome, func } => {
                write!(f, "Bridge target not found: {}::{}::{}", role, zome, func)
            }
            BridgeError::Unauthorized { role, zome, func } => {
                write!(f, "Unauthorized bridge call: {}::{}::{}", role, zome, func)
            }
            BridgeError::NetworkError(msg) => write!(f, "Bridge network error: {}", msg),
            BridgeError::DecodingError(msg) => write!(f, "Bridge decoding error: {}", msg),
            BridgeError::AuthenticationFailed => write!(f, "Bridge authentication failed"),
            BridgeError::Other(msg) => write!(f, "Bridge error: {}", msg),
        }
    }
}

impl From<BridgeError> for WasmError {
    fn from(e: BridgeError) -> Self {
        wasm_error!(WasmErrorInner::Guest(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_error_display() {
        let err = BridgeError::NotFound {
            role: "my-dna-v1".to_string(),
            zome: "coordinator".to_string(),
            func: "export_all".to_string(),
        };
        assert!(err.to_string().contains("my-dna-v1"));
    }
}
