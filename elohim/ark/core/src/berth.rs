//! Placement and local resolution inputs for a runtime manifest.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

/// The local placement and resolution inputs for one runtime-manifest instance.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default, rename_all = "snake_case")]
pub struct Berth {
    /// CID string of the manifest placed in this berth.
    #[serde(default)]
    pub manifest: String,
    /// Agent CID string bound to the berth, once identity is available.
    #[serde(default)]
    pub node: Option<String>,
    /// Root of the process instance's durable local data.
    #[serde(default)]
    pub data_root: PathBuf,
    /// Passphrase source supplied to processes that request one.
    #[serde(default)]
    pub passphrase: PassphraseSource,
    /// Named local ports available to manifest templates.
    #[serde(default)]
    pub ports: BTreeMap<String, u16>,
    /// Process name to resolved local artifact path.
    #[serde(default)]
    pub artifacts: BTreeMap<String, PathBuf>,
    /// Monotonically increasing placement incarnation.
    #[serde(default)]
    pub incarnation: u64,
}

/// Source of a passphrase supplied to a child process.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PassphraseSource {
    /// Supply an empty passphrase.
    #[default]
    Empty,
    /// Supply the contained literal.
    Literal(String),
    /// Read the passphrase from this local path.
    File(PathBuf),
}

/// Berth decoding or template-resolution failure.
#[derive(thiserror::Error, Clone, Debug, PartialEq)]
pub enum BerthError {
    /// Input was not valid berth JSON.
    #[error("berth JSON: {0}")]
    Json(String),
    /// A template key has no value in this berth.
    #[error("unknown berth template: {0}")]
    UnknownTemplate(String),
}

impl Berth {
    /// Decodes a hand-authored JSON berth.
    pub fn from_json(s: &str) -> Result<Self, BerthError> {
        serde_json::from_str(s).map_err(|error| BerthError::Json(error.to_string()))
    }

    /// Resolves manifest placeholders using this berth and process placement.
    pub fn resolve_template(
        &self,
        process: &str,
        artifact_path: &Path,
        template: &str,
    ) -> Result<String, BerthError> {
        let mut resolved = String::with_capacity(template.len());
        let mut remainder = template;

        while let Some(open) = remainder.find('{') {
            resolved.push_str(&remainder[..open]);
            let after_open = &remainder[open + 1..];
            let Some(close) = after_open.find('}') else {
                return Err(BerthError::UnknownTemplate(after_open.to_string()));
            };
            let key = &after_open[..close];
            resolved.push_str(&self.template_value(process, artifact_path, key)?);
            remainder = &after_open[close + 1..];
        }

        resolved.push_str(remainder);
        Ok(resolved)
    }

    fn template_value(
        &self,
        process: &str,
        artifact_path: &Path,
        key: &str,
    ) -> Result<String, BerthError> {
        match key {
            "data_root" => Ok(self.data_root.display().to_string()),
            "name" => Ok(process.to_string()),
            "artifact" => Ok(artifact_path.display().to_string()),
            _ => {
                if let Some(port_key) = key.strip_prefix("port.") {
                    if let Some(port) = self.ports.get(port_key) {
                        return Ok(port.to_string());
                    }
                }
                Err(BerthError::UnknownTemplate(key.to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn berth_resolves_templates_and_names_unknown_keys() {
        let berth = Berth::from_json(
            r#"{
                "manifest": "bafy-manifest",
                "data_root": "/srv/elohim/alpha",
                "ports": {"admin_ws": 4444}
            }"#,
        )
        .unwrap();
        let artifact = Path::new("/opt/elohim/conductor");

        assert_eq!(
            berth
                .resolve_template(
                    "conductor",
                    artifact,
                    "{data_root}/{name}/conductor-config.yaml",
                )
                .unwrap(),
            "/srv/elohim/alpha/conductor/conductor-config.yaml"
        );
        assert_eq!(
            berth
                .resolve_template("conductor", artifact, "{port.admin_ws}")
                .unwrap(),
            "4444"
        );
        assert_eq!(
            berth.resolve_template("conductor", artifact, "{port.nope}"),
            Err(BerthError::UnknownTemplate("port.nope".to_string()))
        );
    }
}
