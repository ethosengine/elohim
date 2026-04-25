//! DefenderManifest — declares the specialist's role/inputs/outputs.
//! Schema: elohim/sdk/schemas/v1/agent/defender-manifest.schema.json

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefenderManifest {
    pub specialist_kind: String,        // const "defender"
    pub for_humans: Vec<String>,        // Human ActionHashes (b64url)
    pub disclosure_tier: String,
    pub outputs: Vec<String>,
    pub system_prompt_template: String,
}

impl DefenderManifest {
    pub fn from_path<P: AsRef<std::path::Path>>(path: P) -> Result<Self, ManifestError> {
        let content = std::fs::read_to_string(path).map_err(ManifestError::Io)?;
        let manifest: DefenderManifest =
            serde_json::from_str(&content).map_err(ManifestError::Parse)?;
        if manifest.specialist_kind != "defender" {
            return Err(ManifestError::WrongKind(manifest.specialist_kind));
        }
        Ok(manifest)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("wrong specialist kind: {0}")]
    WrongKind(String),
}
