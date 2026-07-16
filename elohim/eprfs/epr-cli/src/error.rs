use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not find a git worktree from {0}")]
    RepositoryNotFound(PathBuf),

    #[error("invalid arguments: {0}")]
    InvalidArguments(String),

    #[error("failed to read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to run `{program}`: {source}")]
    Spawn {
        program: String,
        #[source]
        source: std::io::Error,
    },

    #[error("`{program}` failed: {stderr}")]
    Command { program: String, stderr: String },

    #[error("invalid package JSON at {path}: {source}")]
    PackageJson {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error(transparent)]
    Meta(#[from] eprfs_meta::EprMetaError),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
