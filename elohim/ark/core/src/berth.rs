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
    /// Captured environment values available to manifest templates.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
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
        for piece in template_pieces(template)? {
            match piece {
                TemplatePiece::Literal(text) => resolved.push_str(text),
                TemplatePiece::Key(key) => {
                    resolved.push_str(&self.template_value(process, artifact_path, key)?)
                }
            }
        }
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
                if let Some(env_key) = key.strip_prefix("env.") {
                    if let Some(value) = self.env.get(env_key) {
                        return Ok(value.clone());
                    }
                }
                Err(BerthError::UnknownTemplate(key.to_string()))
            }
        }
    }
}

/// One piece of a manifest template: literal text or a `{key}` placeholder.
enum TemplatePiece<'a> {
    Literal(&'a str),
    Key(&'a str),
}

/// Splits a template into literal text and placeholder keys; an unclosed `{` is malformed.
fn template_pieces(template: &str) -> Result<Vec<TemplatePiece<'_>>, BerthError> {
    let mut pieces = Vec::new();
    let mut remainder = template;
    while let Some(open) = remainder.find('{') {
        pieces.push(TemplatePiece::Literal(&remainder[..open]));
        let after_open = &remainder[open + 1..];
        let Some(close) = after_open.find('}') else {
            return Err(BerthError::Malformed(format!(
                "unterminated template near '{{{after_open}'"
            )));
        };
        pieces.push(TemplatePiece::Key(&after_open[..close]));
        remainder = &after_open[close + 1..];
    }
    pieces.push(TemplatePiece::Literal(remainder));
    Ok(pieces)
}

/// Checks a template before any berth exists: every placeholder must be well formed and name
/// a key some berth can resolve (`data_root`, `name`, `artifact`, `log_level`, `port.<k>` or
/// `env.<k>`). Whether a given berth declares that port or env entry is known only at spawn.
pub fn check_template(template: &str) -> Result<(), BerthError> {
    for piece in template_pieces(template)? {
        if let TemplatePiece::Key(key) = piece {
            let resolvable = matches!(key, "data_root" | "name" | "artifact" | "log_level")
                || key
                    .strip_prefix("port.")
                    .or_else(|| key.strip_prefix("env."))
                    .is_some_and(|rest| !rest.is_empty());
            if !resolvable {
                return Err(BerthError::UnknownTemplate(key.to_string()));
            }
        }
    }
    Ok(())
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
    fn berth_resolves_env_templates_and_names_absent_key() {
        let berth = Berth {
            env: BTreeMap::from([("XDG_CONFIG_HOME".to_string(), "/nix/xdg/config".to_string())]),
            ..Berth::default()
        };
        let artifact = Path::new("/opt/elohim/epr");

        assert_eq!(
            berth
                .resolve_template("epr", artifact, "config={env.XDG_CONFIG_HOME}/elohim",)
                .unwrap(),
            "config=/nix/xdg/config/elohim"
        );
        assert_eq!(
            berth.resolve_template("epr", artifact, "{env.EPR_CHECKOUT}"),
            Err(BerthError::UnknownTemplate("env.EPR_CHECKOUT".to_string()))
        );
    }

    #[test]
    fn historic_berth_bytes_unchanged() {
        let historic = br#"{"manifest":"bafy-manifest","node":"uhCAk-node","data_root":"/srv/elohim/alpha","log_level":"debug","passphrase":"empty","ports":{"admin_ws":4444},"artifacts":{"conductor":"/opt/elohim/conductor"},"incarnation":7}"#;
        let berth: Berth = serde_json::from_slice(historic).unwrap();

        assert!(berth.env.is_empty());
        assert_eq!(serde_json::to_vec(&berth).unwrap(), historic);
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
