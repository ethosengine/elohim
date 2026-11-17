//! Where this device keeps its config, executables, state, build pool and checkout: one resolver.
//!
//! Every path a device-side command needs that is not inside the checkout it runs from comes
//! from [`resolve`]. A Che workspace keeps its durable state under `/projects`; a physical
//! device has no `/projects` and keeps it under the OS user's XDG directories. The order for
//! each field is: an explicit environment variable, then (for the checkout, the one the caller
//! stands in) the device's saved layout, then a host default that names `/projects` only when
//! that layout exists on this host.
//!
//! The saved layout is `<config_dir>/layout.json`, written only by an explicit
//! [`save`] (`epr device layout save`); nothing here migrates a store. When an environment
//! variable and the saved value disagree the variable wins and the difference is reported in
//! [`DeviceLayout::differs`], never refused: gates and tests set `BIN_STORE` and
//! `CARGO_TARGET_POOL_ROOT` routinely.
use crate::bin_store::{files, Refusal, Result};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

/// The saved layout's file name inside the device config directory.
pub const SAVED_FILE: &str = "layout.json";
/// The Che workspace's persistent config tree; its presence selects the `/projects` defaults.
pub const CHE_CONFIG: &str = "/projects/.claude-config";
/// The Che workspace's bin-store, under [`CHE_CONFIG`].
pub const CHE_BIN_STORE: &str = "/projects/.claude-config/tools/bin-store";
/// The Che workspace's state directory (berth store, session state): [`CHE_CONFIG`] itself,
/// where those stores have always lived there.
pub const CHE_STATE_DIR: &str = CHE_CONFIG;
/// The Che workspace's cargo target pool.
pub const CHE_CARGO_POOL: &str = "/projects/.cargo-target-pool";
/// The Che workspace's main checkout.
pub const CHE_PROJECT_ROOT: &str = "/projects/elohim";
/// The bin-store subdirectory a peer daemon's queued fetches may install into.
pub const INCOMING_DIR: &str = "incoming";

/// One resolved location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Field {
    /// The device config directory: key, rosters, manifest chain, sessions, saved layout.
    ConfigDir,
    /// The content-addressed executable store (`<bin_store>/<tool>`).
    BinStore,
    /// The device's mutable coordination state: the berth store and its session state.
    StateDir,
    /// The cargo target pool, when this device has one.
    CargoPool,
    /// The checkout device commands act on, when one is known.
    ProjectRoot,
}

impl Field {
    /// Every field, in report order.
    pub const ALL: [Field; 5] = [
        Field::ConfigDir,
        Field::BinStore,
        Field::StateDir,
        Field::CargoPool,
        Field::ProjectRoot,
    ];
    /// The field's name as the saved file and `layout show --json` spell it.
    pub fn name(self) -> &'static str {
        match self {
            Field::ConfigDir => "configDir",
            Field::BinStore => "binStore",
            Field::StateDir => "stateDir",
            Field::CargoPool => "cargoPool",
            Field::ProjectRoot => "projectRoot",
        }
    }
    /// The environment variable that overrides the field.
    pub fn env(self) -> &'static str {
        match self {
            Field::ConfigDir => "ELOHIM_DEVICE_KEY_FILE",
            Field::BinStore => "BIN_STORE",
            Field::StateDir => "ELOHIM_STATE_DIR",
            Field::CargoPool => "CARGO_TARGET_POOL_ROOT",
            Field::ProjectRoot => "EPR_PROJECT_ROOT",
        }
    }
}

/// Where a resolved location came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "name")]
pub enum Origin {
    /// An environment variable (named).
    Env(String),
    /// `<config_dir>/layout.json`.
    Saved,
    /// The git checkout enclosing the working directory.
    Discovered,
    /// This host's default (the `/projects` layout when present, else the XDG one).
    HostDefault,
    /// Nothing names it on this device.
    Absent,
}

/// One field's resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    /// The path, unless [`Origin::Absent`].
    pub path: Option<PathBuf>,
    /// Where it came from.
    pub origin: Origin,
}

impl Location {
    /// `<path> (<origin>)`, or `absent (nothing names it)`: how every report names a location.
    pub fn describe(&self) -> String {
        let path = self
            .path
            .as_ref()
            .map_or_else(|| "absent".into(), |p| p.display().to_string());
        let origin = match &self.origin {
            Origin::Env(name) => format!("env {name}"),
            Origin::Saved => "saved".into(),
            Origin::Discovered => "discovered".into(),
            Origin::HostDefault => "host default".into(),
            Origin::Absent => "nothing names it".into(),
        };
        format!("{path} ({origin})")
    }
}

/// An environment variable and the saved layout naming different paths; the variable won.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Differs {
    /// The field both name.
    pub field: Field,
    /// The variable's path, which is used.
    pub env: PathBuf,
    /// The saved path, which is not.
    pub saved: PathBuf,
}

impl Differs {
    /// The one line `peer serve --check` and `device status` print.
    pub fn line(&self) -> String {
        format!(
            "layout-override-differs {} env {} saved {}",
            self.field.name(),
            self.env.display(),
            self.saved.display()
        )
    }
}

/// Every location a device-side command resolves outside its own checkout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceLayout {
    /// The device config directory; absent only when no key file, XDG config or home is set.
    pub config_dir: Location,
    /// The bin-store root; absent only on a host with no `/projects`, `XDG_DATA_HOME` or `HOME`.
    pub bin_store: Location,
    /// The state directory; absent only on a host with no `/projects`, `XDG_STATE_HOME` or `HOME`.
    pub state_dir: Location,
    /// The cargo target pool.
    pub cargo_pool: Location,
    /// The checkout.
    pub project_root: Location,
    /// Environment overrides that disagree with the saved layout.
    pub differs: Vec<Differs>,
}

impl DeviceLayout {
    /// The location of one field.
    pub fn get(&self, field: Field) -> &Location {
        match field {
            Field::ConfigDir => &self.config_dir,
            Field::BinStore => &self.bin_store,
            Field::StateDir => &self.state_dir,
            Field::CargoPool => &self.cargo_pool,
            Field::ProjectRoot => &self.project_root,
        }
    }
    /// The device config directory.
    pub fn config_dir(&self) -> Result<&Path> {
        self.config_dir.path.as_deref().ok_or_else(|| {
            Refusal::new(
                "layout-no-config-home",
                "no device config directory: ELOHIM_DEVICE_KEY_FILE, XDG_CONFIG_HOME and HOME \
                 are all unset",
            )
        })
    }
    /// The bin-store root, or the refusal a command that needs one reports.
    pub fn bin_store(&self) -> Result<&Path> {
        self.bin_store.path.as_deref().ok_or_else(|| {
            Refusal::new(
                "layout-no-data-home",
                "no bin-store: set BIN_STORE, XDG_DATA_HOME or HOME (this host has no /projects \
                 layout)",
            )
        })
    }
    /// The state directory, or the refusal a command that needs one reports.
    pub fn state_dir(&self) -> Result<&Path> {
        self.state_dir.path.as_deref().ok_or_else(|| {
            Refusal::new(
                "layout-no-state-home",
                "no state directory: set ELOHIM_STATE_DIR, XDG_STATE_HOME or HOME (this host has \
                 no /projects layout)",
            )
        })
    }
    /// The cargo target pool, when this device has one.
    pub fn cargo_pool(&self) -> Option<&Path> {
        self.cargo_pool.path.as_deref()
    }
    /// The checkout, when one is known.
    pub fn project_root(&self) -> Option<&Path> {
        self.project_root.path.as_deref()
    }
    /// Where a peer daemon's queued fetches install on this device: `<bin_store>/incoming`.
    pub fn incoming(&self) -> Result<PathBuf> {
        Ok(self.bin_store()?.join(INCOMING_DIR))
    }
    /// `field = path (origin)` lines, then one line per [`Differs`].
    pub fn lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = Field::ALL
            .iter()
            .map(|&field| format!("layout {} = {}", field.name(), self.get(field).describe()))
            .collect();
        lines.extend(self.differs.iter().map(Differs::line));
        lines
    }
}

/// The saved layout file: every field optional, absolute paths only.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Saved {
    /// Schema version, 1.
    pub version: u32,
    /// The saved bin-store root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bin_store: Option<PathBuf>,
    /// The saved state directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_dir: Option<PathBuf>,
    /// The saved cargo pool.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo_pool: Option<PathBuf>,
    /// The saved checkout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_root: Option<PathBuf>,
}

impl Saved {
    fn get(&self, field: Field) -> Option<&PathBuf> {
        match field {
            Field::ConfigDir => None,
            Field::BinStore => self.bin_store.as_ref(),
            Field::StateDir => self.state_dir.as_ref(),
            Field::CargoPool => self.cargo_pool.as_ref(),
            Field::ProjectRoot => self.project_root.as_ref(),
        }
    }
    fn set(&mut self, field: Field, path: PathBuf) {
        match field {
            Field::ConfigDir => {}
            Field::BinStore => self.bin_store = Some(path),
            Field::StateDir => self.state_dir = Some(path),
            Field::CargoPool => self.cargo_pool = Some(path),
            Field::ProjectRoot => self.project_root = Some(path),
        }
    }
}

/// What the resolver may observe of its host: an environment lookup, an existence probe and
/// the working directory. [`resolve`] uses the real ones; fixtures inject their own.
pub struct Host<'a> {
    /// Environment lookup.
    pub env: &'a dyn Fn(&str) -> Option<OsString>,
    /// Whether a path exists (a directory or a file).
    pub exists: &'a dyn Fn(&Path) -> bool,
    /// The working directory, for checkout discovery.
    pub cwd: Option<PathBuf>,
}

/// Resolve this device's layout from the process environment and filesystem.
pub fn resolve() -> Result<DeviceLayout> {
    let env = |name: &str| std::env::var_os(name);
    let exists = |path: &Path| path.exists();
    resolve_with(&Host {
        env: &env,
        exists: &exists,
        cwd: std::env::current_dir().ok(),
    })
}

/// Resolve against an explicit host. The saved layout is read from the resolved config dir.
pub fn resolve_with(host: &Host<'_>) -> Result<DeviceLayout> {
    let set = |name: &str| (host.env)(name).filter(|value| !value.is_empty());
    let absolute = |field: Field, value: OsString| -> Result<PathBuf> {
        let path = PathBuf::from(value);
        if path.is_absolute() {
            Ok(path)
        } else {
            Err(Refusal::new(
                "layout-relative-path",
                format!(
                    "{}={} is relative; a device location is never joined to the working \
                     directory",
                    field.env(),
                    path.display()
                ),
            ))
        }
    };
    let xdg = |name: &str| set(name).map(PathBuf::from).filter(|p| p.is_absolute());
    let home = set("HOME").map(PathBuf::from);

    let config_dir = config_dir(host)?;
    let saved = match &config_dir {
        Some(dir) => read_saved(dir)?,
        None => Saved::default(),
    };
    let mut differs = Vec::new();
    let mut explicit = |field: Field| -> Result<Option<Location>> {
        let Some(value) = set(field.env()) else {
            return Ok(None);
        };
        let path = absolute(field, value)?;
        if let Some(saved) = saved.get(field).filter(|saved| **saved != path) {
            differs.push(Differs {
                field,
                env: path.clone(),
                saved: saved.clone(),
            });
        }
        Ok(Some(Location {
            path: Some(path),
            origin: Origin::Env(field.env().into()),
        }))
    };
    let from_saved = |field: Field| {
        saved.get(field).map(|path| Location {
            path: Some(path.clone()),
            origin: Origin::Saved,
        })
    };
    let default = |path: PathBuf| Location {
        path: Some(path),
        origin: Origin::HostDefault,
    };
    let absent = || Location {
        path: None,
        origin: Origin::Absent,
    };
    let che = (host.exists)(Path::new(CHE_CONFIG));

    let bin_store = match explicit(Field::BinStore)? {
        Some(location) => location,
        None => match from_saved(Field::BinStore) {
            Some(location) => location,
            None if che => default(CHE_BIN_STORE.into()),
            None => xdg("XDG_DATA_HOME")
                .or_else(|| home.as_ref().map(|h| h.join(".local/share")))
                .map_or_else(absent, |data| {
                    default(data.join("elohim").join("bin-store"))
                }),
        },
    };
    // A Che workspace's berth store and session state already live in its config tree; they
    // never move, so its presence keeps them there.
    let state_dir = match explicit(Field::StateDir)? {
        Some(location) => location,
        None => match from_saved(Field::StateDir) {
            Some(location) => location,
            None if che => default(CHE_STATE_DIR.into()),
            None => xdg("XDG_STATE_HOME")
                .or_else(|| home.as_ref().map(|h| h.join(".local/state")))
                .map_or_else(absent, |state| default(state.join("elohim"))),
        },
    };
    let cargo_pool = match explicit(Field::CargoPool)? {
        Some(location) => location,
        None => from_saved(Field::CargoPool).unwrap_or_else(|| {
            if (host.exists)(Path::new(CHE_CARGO_POOL)) {
                default(CHE_CARGO_POOL.into())
            } else {
                absent()
            }
        }),
    };
    // The checkout the caller stands in beats a saved one: a saved root never redirects a
    // command run inside another checkout or worktree.
    let project_root = match explicit(Field::ProjectRoot)? {
        Some(location) => location,
        None => match host
            .cwd
            .as_deref()
            .and_then(|cwd| enclosing_checkout(cwd, host))
        {
            Some(root) => Location {
                path: Some(root),
                origin: Origin::Discovered,
            },
            None => from_saved(Field::ProjectRoot).unwrap_or_else(|| {
                if (host.exists)(Path::new(CHE_PROJECT_ROOT)) {
                    default(CHE_PROJECT_ROOT.into())
                } else {
                    absent()
                }
            }),
        },
    };
    Ok(DeviceLayout {
        config_dir: match config_dir {
            Some(dir) => Location {
                path: Some(dir),
                origin: config_origin(host),
            },
            None => absent(),
        },
        bin_store,
        state_dir,
        cargo_pool,
        project_root,
        differs,
    })
}

/// The device config directory, by the device key's rule: `ELOHIM_DEVICE_KEY_FILE`'s parent,
/// else `$XDG_CONFIG_HOME/elohim/device` (absolute only), else `$HOME/.config/elohim/device`.
/// The key file's own admission checks stay with the device key.
fn config_dir(host: &Host<'_>) -> Result<Option<PathBuf>> {
    let set = |name: &str| (host.env)(name).filter(|value| !value.is_empty());
    if let Some(key) = set(Field::ConfigDir.env()) {
        let key = PathBuf::from(key);
        return match key.parent().filter(|_| key.is_absolute()) {
            Some(parent) => Ok(Some(parent.to_owned())),
            None => Err(Refusal::new(
                "layout-relative-path",
                format!(
                    "{}={} must be an absolute file path",
                    Field::ConfigDir.env(),
                    key.display()
                ),
            )),
        };
    }
    Ok(default_config_dir_with(host.env))
}

/// The device config directory when no key file names it: `$XDG_CONFIG_HOME/elohim/device`
/// (absolute only, per the base-directory spec), else `$HOME/.config/elohim/device`, else none.
/// The device key derives its default seed path from this, so the two never disagree.
pub fn default_config_dir_with(env: &dyn Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let set = |name: &str| env(name).filter(|value| !value.is_empty());
    let config_home = match set("XDG_CONFIG_HOME").map(PathBuf::from) {
        Some(xdg) if xdg.is_absolute() => xdg,
        _ => PathBuf::from(set("HOME")?).join(".config"),
    };
    Some(config_home.join("elohim").join("device"))
}

fn config_origin(host: &Host<'_>) -> Origin {
    let set = |name: &str| (host.env)(name).is_some_and(|value| !value.is_empty());
    if set(Field::ConfigDir.env()) {
        Origin::Env(Field::ConfigDir.env().into())
    } else if (host.env)("XDG_CONFIG_HOME").is_some_and(|v| Path::new(&v).is_absolute()) {
        Origin::Env("XDG_CONFIG_HOME".into())
    } else {
        Origin::HostDefault
    }
}

/// The nearest ancestor of `start` holding a `.git` entry (a directory in a clone, a file in
/// a worktree or submodule).
fn enclosing_checkout(start: &Path, host: &Host<'_>) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|dir| (host.exists)(&dir.join(".git")))
        .map(Path::to_owned)
}

/// Read `<config_dir>/layout.json`; absent is an empty layout, malformed is a refusal.
pub fn read_saved(config_dir: &Path) -> Result<Saved> {
    let path = config_dir.join(SAVED_FILE);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Saved::default()),
        Err(e) => return Err(e.into()),
    };
    let saved: Saved = serde_json::from_slice(&bytes)
        .map_err(|e| Refusal::new("layout-saved-invalid", format!("{}: {e}", path.display())))?;
    if saved.version != 1 {
        return Err(Refusal::new(
            "layout-saved-invalid",
            format!("{}: version {} (expected 1)", path.display(), saved.version),
        ));
    }
    for field in Field::ALL {
        if let Some(value) = saved.get(field).filter(|p| !p.is_absolute()) {
            return Err(Refusal::new(
                "layout-saved-invalid",
                format!(
                    "{}: {} = {} is not absolute",
                    path.display(),
                    field.name(),
                    value.display()
                ),
            ));
        }
    }
    Ok(saved)
}

/// Record the current resolution of `fields` (every savable field when empty) in the saved
/// layout, keeping fields not named. Absent fields are not saved. Returns the saved layout.
pub fn save(layout: &DeviceLayout, fields: &[Field]) -> Result<Saved> {
    let fields: Vec<Field> = if fields.is_empty() {
        vec![
            Field::BinStore,
            Field::StateDir,
            Field::CargoPool,
            Field::ProjectRoot,
        ]
    } else {
        fields.to_vec()
    };
    if fields.contains(&Field::ConfigDir) {
        return Err(Refusal::new(
            "layout-field-unsavable",
            "configDir holds the saved layout; it is named by the environment only",
        ));
    }
    let mut saved = read_saved(layout.config_dir()?)?;
    saved.version = 1;
    for field in fields {
        if let Some(path) = layout.get(field).path.clone() {
            saved.set(field, path);
        }
    }
    let path = layout.config_dir()?.join(SAVED_FILE);
    files::atomic_bytes(&path, &serde_json::to_vec_pretty(&saved)?, 0o600)?;
    Ok(saved)
}

/// Refuse a destination a launch could not create: its nearest existing ancestor must be a
/// writable directory. Nothing is created.
pub fn check_creatable(field: &str, path: &Path) -> Result<()> {
    let refuse = |why: String| {
        Refusal::new(
            "layout-destination-unwritable",
            format!("{field} {}: {why}", path.display()),
        )
    };
    let existing = path
        .ancestors()
        .find(|dir| dir.exists())
        .ok_or_else(|| refuse("no ancestor exists".into()))?;
    if !existing.is_dir() {
        return Err(refuse(format!("{} is not a directory", existing.display())));
    }
    rustix::fs::access(
        existing,
        rustix::fs::Access::WRITE_OK | rustix::fs::Access::EXEC_OK,
    )
    .map_err(|e| refuse(format!("{} is not writable ({e})", existing.display())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    struct Fixture {
        env: BTreeMap<String, OsString>,
        present: BTreeSet<PathBuf>,
        cwd: Option<PathBuf>,
    }
    impl Fixture {
        fn new(env: &[(&str, &str)]) -> Self {
            Self {
                env: env
                    .iter()
                    .map(|(k, v)| ((*k).to_owned(), OsString::from(v)))
                    .collect(),
                present: BTreeSet::new(),
                cwd: None,
            }
        }
        fn with(mut self, path: &str) -> Self {
            self.present.insert(path.into());
            self
        }
        fn at(mut self, cwd: &str) -> Self {
            self.cwd = Some(cwd.into());
            self
        }
        fn resolve(&self) -> Result<DeviceLayout> {
            let env = |name: &str| self.env.get(name).cloned();
            let exists = |path: &Path| self.present.contains(path);
            resolve_with(&Host {
                env: &env,
                exists: &exists,
                cwd: self.cwd.clone(),
            })
        }
    }
    fn path(location: &Location) -> Option<&str> {
        location.path.as_deref().and_then(Path::to_str)
    }

    #[test]
    fn che_workspace_resolves_byte_identical_to_the_paths_it_used_before() {
        // This workspace: XDG_CONFIG_HOME and XDG_STATE_HOME on the persistent /nix/xdg volume,
        // /projects present. The berth store and session state stay in the config tree.
        let layout = Fixture::new(&[
            ("XDG_CONFIG_HOME", "/nix/xdg/config"),
            ("XDG_STATE_HOME", "/nix/xdg/state"),
            ("HOME", "/home/user"),
        ])
        .with(CHE_CONFIG)
        .with(CHE_CARGO_POOL)
        .with(CHE_PROJECT_ROOT)
        .resolve()
        .unwrap();
        assert_eq!(
            path(&layout.config_dir),
            Some("/nix/xdg/config/elohim/device")
        );
        assert_eq!(
            path(&layout.bin_store),
            Some("/projects/.claude-config/tools/bin-store")
        );
        assert_eq!(
            path(&layout.cargo_pool),
            Some("/projects/.cargo-target-pool")
        );
        assert_eq!(path(&layout.state_dir), Some("/projects/.claude-config"));
        assert_eq!(path(&layout.project_root), Some("/projects/elohim"));
        assert_eq!(layout.bin_store.origin, Origin::HostDefault);
        assert_eq!(layout.state_dir.origin, Origin::HostDefault);
        assert!(layout.differs.is_empty());
    }

    #[test]
    fn a_host_without_projects_resolves_under_the_users_xdg_directories() {
        let layout = Fixture::new(&[("HOME", "/home/m")]).resolve().unwrap();
        assert_eq!(
            path(&layout.config_dir),
            Some("/home/m/.config/elohim/device")
        );
        assert_eq!(
            path(&layout.bin_store),
            Some("/home/m/.local/share/elohim/bin-store")
        );
        assert_eq!(path(&layout.state_dir), Some("/home/m/.local/state/elohim"));
        assert_eq!(layout.cargo_pool.origin, Origin::Absent);
        assert_eq!(layout.project_root.origin, Origin::Absent);
        assert_eq!(
            layout.incoming().unwrap(),
            PathBuf::from("/home/m/.local/share/elohim/bin-store/incoming")
        );
        let data = Fixture::new(&[("HOME", "/home/m"), ("XDG_DATA_HOME", "/data")])
            .resolve()
            .unwrap();
        assert_eq!(path(&data.bin_store), Some("/data/elohim/bin-store"));
        let state = Fixture::new(&[("HOME", "/home/m"), ("XDG_STATE_HOME", "/state")])
            .resolve()
            .unwrap();
        assert_eq!(path(&state.state_dir), Some("/state/elohim"));
        let relative_state = Fixture::new(&[("HOME", "/home/m"), ("XDG_STATE_HOME", "rel")])
            .resolve()
            .unwrap();
        assert_eq!(
            path(&relative_state.state_dir),
            Some("/home/m/.local/state/elohim")
        );
        let none = Fixture::new(&[("BIN_STORE", "/gate/bins")])
            .resolve()
            .unwrap();
        assert_eq!(none.state_dir.origin, Origin::Absent);
        assert_eq!(none.state_dir().unwrap_err().kind, "layout-no-state-home");
        let relative_xdg = Fixture::new(&[("HOME", "/home/m"), ("XDG_DATA_HOME", "rel")])
            .resolve()
            .unwrap();
        assert_eq!(
            path(&relative_xdg.bin_store),
            Some("/home/m/.local/share/elohim/bin-store"),
            "a relative XDG value is ignored, per the base-directory spec"
        );
    }

    #[test]
    fn config_dir_follows_the_device_key_rule() {
        let key = Fixture::new(&[
            ("ELOHIM_DEVICE_KEY_FILE", "/keys/dev/ed25519.seed"),
            ("XDG_CONFIG_HOME", "/xdg"),
        ])
        .resolve()
        .unwrap();
        assert_eq!(path(&key.config_dir), Some("/keys/dev"));
        assert_eq!(
            key.bin_store.origin,
            Origin::Absent,
            "a key alone names no data home"
        );
        assert_eq!(key.bin_store().unwrap_err().kind, "layout-no-data-home");
        assert_eq!(
            key.config_dir.origin,
            Origin::Env("ELOHIM_DEVICE_KEY_FILE".into())
        );
        let relative = Fixture::new(&[("XDG_CONFIG_HOME", "xdg"), ("HOME", "/h")])
            .resolve()
            .unwrap();
        assert_eq!(path(&relative.config_dir), Some("/h/.config/elohim/device"));
        let none = Fixture::new(&[("BIN_STORE", "/gate/bins")])
            .resolve()
            .unwrap();
        assert_eq!(none.config_dir.origin, Origin::Absent);
        assert_eq!(none.config_dir().unwrap_err().kind, "layout-no-config-home");
        assert_eq!(
            none.bin_store().unwrap(),
            Path::new("/gate/bins"),
            "BIN_STORE alone suffices"
        );
        let bad = Fixture::new(&[("ELOHIM_DEVICE_KEY_FILE", "seed")])
            .resolve()
            .unwrap_err();
        assert_eq!(bad.kind, "layout-relative-path");
    }

    #[test]
    fn env_wins_over_saved_and_reports_the_difference_without_refusing() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("elohim/device");
        fs::create_dir_all(&config).unwrap();
        fs::write(
            config.join(SAVED_FILE),
            r#"{"version":1,"binStore":"/saved/bins","stateDir":"/saved/state","cargoPool":"/saved/pool","projectRoot":"/saved/repo"}"#,
        )
        .unwrap();
        let xdg = dir.path().to_str().unwrap();
        let saved = Fixture::new(&[("XDG_CONFIG_HOME", xdg)])
            .with(CHE_CONFIG)
            .resolve()
            .unwrap();
        assert_eq!(
            path(&saved.bin_store),
            Some("/saved/bins"),
            "saved beats the host default"
        );
        assert_eq!(saved.bin_store.origin, Origin::Saved);
        assert_eq!(
            path(&saved.state_dir),
            Some("/saved/state"),
            "a saved state directory beats the Che default too"
        );
        assert_eq!(path(&saved.project_root), Some("/saved/repo"));
        let overridden = Fixture::new(&[
            ("XDG_CONFIG_HOME", xdg),
            ("BIN_STORE", "/gate/bins"),
            ("ELOHIM_STATE_DIR", "/gate/state"),
            ("CARGO_TARGET_POOL_ROOT", "/saved/pool"),
        ])
        .resolve()
        .unwrap();
        assert_eq!(path(&overridden.bin_store), Some("/gate/bins"));
        assert_eq!(overridden.bin_store.origin, Origin::Env("BIN_STORE".into()));
        assert_eq!(
            overridden.differs,
            vec![
                Differs {
                    field: Field::BinStore,
                    env: "/gate/bins".into(),
                    saved: "/saved/bins".into()
                },
                Differs {
                    field: Field::StateDir,
                    env: "/gate/state".into(),
                    saved: "/saved/state".into()
                }
            ],
            "an equal override is not a difference"
        );
        assert_eq!(
            overridden.differs[0].line(),
            "layout-override-differs binStore env /gate/bins saved /saved/bins"
        );
        let relative = Fixture::new(&[("XDG_CONFIG_HOME", xdg), ("BIN_STORE", "bins")])
            .resolve()
            .unwrap_err();
        assert_eq!(relative.kind, "layout-relative-path");
    }

    #[test]
    fn the_checkout_the_caller_stands_in_beats_a_saved_root() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("elohim/device");
        fs::create_dir_all(&config).unwrap();
        fs::write(
            config.join(SAVED_FILE),
            r#"{"version":1,"projectRoot":"/saved/repo"}"#,
        )
        .unwrap();
        let xdg = dir.path().to_str().unwrap();
        let inside = Fixture::new(&[("XDG_CONFIG_HOME", xdg)])
            .with("/work/tree/.git")
            .at("/work/tree/elohim/eprfs")
            .resolve()
            .unwrap();
        assert_eq!(path(&inside.project_root), Some("/work/tree"));
        assert_eq!(inside.project_root.origin, Origin::Discovered);
        let outside = Fixture::new(&[("XDG_CONFIG_HOME", xdg)])
            .with(CHE_PROJECT_ROOT)
            .at("/somewhere")
            .resolve()
            .unwrap();
        assert_eq!(path(&outside.project_root), Some("/saved/repo"));
        let env = Fixture::new(&[("XDG_CONFIG_HOME", xdg), ("EPR_PROJECT_ROOT", "/explicit")])
            .with("/work/tree/.git")
            .at("/work/tree")
            .resolve()
            .unwrap();
        assert_eq!(path(&env.project_root), Some("/explicit"));
    }

    #[test]
    fn save_records_the_current_resolution_and_never_the_config_dir() {
        let dir = tempfile::tempdir().unwrap();
        let xdg = dir.path().to_str().unwrap();
        let fixture = Fixture::new(&[("XDG_CONFIG_HOME", xdg), ("HOME", "/home/m")]);
        let layout = fixture.resolve().unwrap();
        let saved = save(&layout, &[]).unwrap();
        assert_eq!(
            saved.bin_store.as_deref(),
            Some(Path::new("/home/m/.local/share/elohim/bin-store"))
        );
        assert_eq!(
            saved.state_dir.as_deref(),
            Some(Path::new("/home/m/.local/state/elohim"))
        );
        assert_eq!(saved.cargo_pool, None, "an absent field is not saved");
        let again = fixture.resolve().unwrap();
        assert_eq!(again.bin_store.origin, Origin::Saved);
        assert_eq!(
            save(&layout, &[Field::ConfigDir]).unwrap_err().kind,
            "layout-field-unsavable"
        );
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(dir.path().join("elohim/device").join(SAVED_FILE))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
        fs::write(
            dir.path().join("elohim/device").join(SAVED_FILE),
            r#"{"version":1,"binStore":"relative"}"#,
        )
        .unwrap();
        assert_eq!(fixture.resolve().unwrap_err().kind, "layout-saved-invalid");
    }

    #[test]
    fn check_creatable_names_the_destination_and_creates_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let fresh = dir.path().join("a/b/bin-store");
        check_creatable("binStore", &fresh).unwrap();
        assert!(!dir.path().join("a").exists());
        let file = dir.path().join("plain");
        fs::write(&file, "").unwrap();
        let blocked = check_creatable("binStore", &file.join("bin-store/epr")).unwrap_err();
        assert_eq!(blocked.kind, "layout-destination-unwritable");
        assert!(blocked.detail.contains("binStore"));
        assert!(blocked
            .detail
            .contains(&file.join("bin-store/epr").display().to_string()));
    }
}
