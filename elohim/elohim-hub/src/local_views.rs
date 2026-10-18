//! Private operational history around the existing native projection reader.
//! Browser identity belongs to the access adapter; this loopback service requires its token.
use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, StatusCode},
    response::Response,
    Router,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use elohim_epr_cli::view;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use ts_rs::TS;

type Result<T> = std::result::Result<T, String>;
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Config {
    pub root: PathBuf,
    pub store: PathBuf,
    pub views: BTreeMap<String, String>,
    pub listen: std::net::SocketAddr,
}
#[derive(Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Observation {
    pub sequence: u32,
    pub observed_at: String,
    pub snapshot_cid: String,
    pub declaration_cid: String,
}
#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotBundle {
    pub snapshot_cid: String,
    /// Original immutable envelope, not reserialized by the HTTP adapter.
    pub envelope: String,
    /// Raw source CID -> base64 bytes. No checkout paths accepted by this endpoint.
    pub objects: BTreeMap<String, String>,
}
pub struct LocalViews {
    config: Config,
    token: String,
    // Held for the process lifetime: two readers cannot race the history commit.
    _lease: fs::File,
}
impl LocalViews {
    pub fn new(mut config: Config, token: String) -> Result<Self> {
        if token.len() < 32
            || !config.listen.ip().is_loopback()
            || config.views.is_empty()
            || config.views.len() > 32
        {
            return Err(
                "local views require loopback, a private token and 1..32 configured views".into(),
            );
        }
        config.root = config.root.canonicalize().map_err(err)?;
        for id in config.views.keys() {
            if id.is_empty()
                || id.len() > 80
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            {
                return Err("invalid configured view id".into());
            }
        }
        use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&config.store)
            .map_err(err)?;
        let meta = fs::symlink_metadata(&config.store).map_err(err)?;
        if !meta.is_dir() || meta.file_type().is_symlink() || meta.permissions().mode() & 0o077 != 0
        {
            return Err("view store must be an owner-only directory".into());
        }
        let lease = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
            .open(config.store.join(".history-lock"))
            .map_err(err)?;
        rustix::fs::flock(&lease, rustix::fs::FlockOperation::NonBlockingLockExclusive)
            .map_err(err)?;
        Ok(Self {
            config,
            token,
            _lease: lease,
        })
    }
    fn declaration(&self, id: &str) -> Result<&str> {
        self.config
            .views
            .get(id)
            .map(String::as_str)
            .ok_or_else(|| "unknown view".into())
    }
    fn history_path(&self, id: &str) -> Result<PathBuf> {
        let declaration = self.declaration(id)?;
        let binding = format!("{}\n{declaration}", self.config.root.display());
        Ok(self.config.store.join(format!(
            "{}.history.json",
            eprfs_core::BlobCid::compute_raw(binding.as_bytes())
        )))
    }
    pub fn history(&self, id: &str) -> Result<Vec<Observation>> {
        let path = self.history_path(id)?;
        if !path.exists() {
            return Ok(Vec::new());
        }
        let metadata = fs::symlink_metadata(&path).map_err(err)?;
        if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 1024 * 1024
        {
            return Err("invalid history file".into());
        }
        let rows: Vec<Observation> =
            serde_json::from_slice(&fs::read(path).map_err(err)?).map_err(err)?;
        if rows.len() > 1000
            || rows
                .iter()
                .enumerate()
                .any(|(i, o)| o.sequence as usize != i + 1)
        {
            return Err("invalid observation sequence".into());
        }
        Ok(rows)
    }
    pub fn capture(&self, id: &str) -> Result<Observation> {
        let mut history = self.history(id)?;
        if history.len() >= 1000 {
            return Err("history bound reached (1000 observations)".into());
        }
        let captured = view::project(&self.config.root, self.declaration(id)?)?;
        let cid = view::save(&self.config.store.join("objects"), &captured)?;
        let observation = Observation {
            sequence: history.len() as u32 + 1,
            observed_at: chrono::Utc::now().to_rfc3339(),
            snapshot_cid: cid,
            declaration_cid: captured.projection.declaration_cid,
        };
        history.push(observation.clone());
        let path = self.history_path(id)?;
        let scratch = path.with_extension("pending");
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
            .open(&scratch)
            .map_err(err)?;
        file.write_all(&serde_json::to_vec(&history).map_err(err)?)
            .map_err(err)?;
        file.sync_all().map_err(err)?;
        fs::rename(scratch, path).map_err(err)?;
        fs::File::open(&self.config.store)
            .map_err(err)?
            .sync_all()
            .map_err(err)?;
        Ok(observation)
    }
    pub fn snapshot(&self, id: &str, cid: &str) -> Result<SnapshotBundle> {
        if !self.history(id)?.iter().any(|o| o.snapshot_cid == cid) {
            return Err("snapshot outside view history".into());
        }
        let store = self.config.store.join("objects");
        let captured = view::reopen(&store, cid)?;
        let objects = captured
            .bytes
            .into_iter()
            .map(|(path, bytes)| {
                (
                    captured.projection.sources[&path].clone(),
                    STANDARD.encode(bytes),
                )
            })
            .collect();
        Ok(SnapshotBundle {
            snapshot_cid: cid.into(),
            envelope: String::from_utf8(view::snapshot_bytes(&store, cid)?).map_err(err)?,
            objects,
        })
    }
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn response(status: StatusCode, bytes: Vec<u8>) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CACHE_CONTROL, "private, no-store")
        .header("X-Content-Type-Options", "nosniff")
        .body(Body::from(bytes))
        .expect("static response headers")
}
async fn handle(State(state): State<Arc<Mutex<LocalViews>>>, req: Request) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let authorization = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    if req.uri().query().is_some() {
        return response(StatusCode::BAD_REQUEST, b"{}".to_vec());
    }
    let result = tokio::task::spawn_blocking(move || {
        let reader = state.lock().map_err(|_| "reader unavailable".to_string())?;
        if authorization != format!("Bearer {}", reader.token) {
            return Err("unauthorized".into());
        }
        let parts: Vec<_> = path.trim_matches('/').split('/').collect();
        let value = match (method.as_str(), parts.as_slice()) {
            ("GET", ["api", "local-views"]) => {
                serde_json::to_value(reader.config.views.keys().collect::<Vec<_>>()).map_err(err)?
            }
            ("GET", ["api", "local-views", id, "history"]) => {
                serde_json::to_value(reader.history(id)?).map_err(err)?
            }
            ("POST", ["api", "local-views", id, "capture"]) => {
                serde_json::to_value(reader.capture(id)?).map_err(err)?
            }
            ("GET", ["api", "local-views", id, "snapshots", cid]) => {
                serde_json::to_value(reader.snapshot(id, cid)?).map_err(err)?
            }
            _ => return Err("unknown local-view request".into()),
        };
        serde_json::to_vec(&value).map_err(err)
    })
    .await;
    match result {
        Ok(Ok(bytes)) => response(StatusCode::OK, bytes),
        Ok(Err(e)) => response(
            if e == "unauthorized" {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::BAD_REQUEST
            },
            serde_json::to_vec(&serde_json::json!({"error":e})).unwrap(),
        ),
        Err(_) => response(StatusCode::INTERNAL_SERVER_ERROR, b"{}".to_vec()),
    }
}
pub fn router(reader: LocalViews) -> Router {
    Router::new()
        .fallback(handle)
        .with_state(Arc::new(Mutex::new(reader)))
}
/// Local-view composition: returns before any node services are constructed.
pub async fn serve(config_path: &std::path::Path) -> Result<()> {
    let config: Config =
        serde_json::from_slice(&fs::read(config_path).map_err(err)?).map_err(err)?;
    let listener = tokio::net::TcpListener::bind(config.listen)
        .await
        .map_err(err)?;
    let token = std::env::var("LOCAL_VIEW_READER_TOKEN").map_err(err)?;
    axum::serve(listener, router(LocalViews::new(config, token)?))
        .await
        .map_err(err)
}
pub fn typescript_contract() -> String {
    use view::{
        composition::{CompositionReading, GroupReading, WorkLink},
        declaration::{Column, Presentation},
        Projection, Subject,
    };
    format!("// Generated by elohim-hub. Do not edit.\n/* eslint-disable @typescript-eslint/array-type, @typescript-eslint/consistent-type-definitions, @typescript-eslint/consistent-indexed-object-style */\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n", serde_json::Value::decl(), Observation::decl(), SnapshotBundle::decl(), Projection::decl(), Subject::decl(), Presentation::decl(), Column::decl(), CompositionReading::decl(), GroupReading::decl(), WorkLink::decl()).replace("type ", "export type ")
}
