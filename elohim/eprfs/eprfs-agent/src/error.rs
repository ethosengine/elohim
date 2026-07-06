//! Errors for the elohim-agent -> eprfs projection adapter.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentProjectionError {
    #[error("frontmatter delimiter (---) missing or malformed in capability source")]
    MissingFrontmatter,
    #[error("frontmatter is not valid YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("capability is missing required frontmatter field: {0}")]
    MissingField(&'static str),
    #[error("unknown projection runtime: {0}")]
    UnknownRuntime(String),
    #[error(transparent)]
    Eprfs(#[from] eprfs_core::EprfsError),
}

pub type Result<T> = std::result::Result<T, AgentProjectionError>;
