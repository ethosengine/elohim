//! Placement and local resolution inputs for a runtime manifest.

use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
    str::FromStr,
};

use serde::{Deserialize, Serialize};

/// Verbosity of the ark's own operator-facing diagnostics.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    /// Print failures only.
    Error,
    /// Print warnings and failures.
    Warn,
    /// Print lifecycle state changes.
    #[default]
    Info,
    /// Print per-poll, ladder, and reap detail.
    Debug,
    /// Print every syscall result.
    Trace,
}

impl LogLevel {
    /// Returns the lowercase configuration name of this level.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for LogLevel {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "error" => Ok(Self::Error),
            "warn" => Ok(Self::Warn),
            "info" => Ok(Self::Info),
            "debug" => Ok(Self::Debug),
            "trace" => Ok(Self::Trace),
            _ => Err(format!(
                "unknown log level {value:?}; expected error, warn, info, debug, or trace"
            )),
        }
    }
}

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
    /// Verbosity of this ark instance's own diagnostics.
    #[serde(default)]
    pub log_level: LogLevel,
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
    /// A template opened a brace it never closed.
    #[error("malformed template: {0}")]
    Malformed(String),
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
                return Err(BerthError::Malformed(format!(
                    "unterminated template near '{{{after_open}'"
                )));
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
            "log_level" => Ok(self.log_level.to_string()),
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
    use std::str::FromStr;

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

        assert_eq!(berth.log_level, LogLevel::Info);

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
        assert!(matches!(
            berth.resolve_template("conductor", artifact, "{data_root"),
            Err(BerthError::Malformed(_))
        ));
    }

    #[test]
    fn log_level_parses_case_insensitively() {
        assert_eq!(LogLevel::from_str("error").unwrap(), LogLevel::Error);
        assert_eq!(LogLevel::from_str("WARN").unwrap(), LogLevel::Warn);
        assert_eq!(LogLevel::from_str("Info").unwrap(), LogLevel::Info);
        assert_eq!(LogLevel::from_str("dEbUg").unwrap(), LogLevel::Debug);
        assert_eq!(LogLevel::from_str("TRACE").unwrap(), LogLevel::Trace);
        assert!(LogLevel::from_str("verbose").is_err());
    }

    #[test]
    fn berth_resolves_log_level_template() {
        let berth = Berth {
            log_level: LogLevel::Debug,
            ..Berth::default()
        };

        assert_eq!(
            berth
                .resolve_template(
                    "conductor",
                    Path::new("/opt/elohim/conductor"),
                    "RUST_LOG={log_level}",
                )
                .unwrap(),
            "RUST_LOG=debug"
        );
    }
}
