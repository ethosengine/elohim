//! Which checkouts this device declared trusted, read for the git and brit-build-ref children
//! ark's brit index spawns.
//!
//! `epr device trust <path>` writes the declaration; epr's own reader is authoritative, and the
//! on-disk format and every rule a reader holds to (location beside the device key, custody,
//! matching, both renderings, the reason classes) are stated once, in the module doc of
//! `elohim/eprfs/epr-cli/src/git.rs`. This is a second reader of that one format, kept here so
//! ark need not link the CLI or a crate the node's storage runtime builds. It cannot drift: the
//! test at the bottom runs it against `elohim/eprfs/epr-cli/tests/fixtures/declared-trust-vectors.json`,
//! which epr-cli mints from its own reader.
//!
//! **Nothing is granted that the device did not declare**: only the exact checkout root holding
//! the working directory, only when it is a line of a declaration whose directory and file pass
//! the custody rule; `safe.directory=*` is never produced. When a declaration exists but cannot
//! be used, the reason is kept ([`DeclaredTrust::unapplied`]) for a failed child's message.

use std::ffi::{OsStr, OsString};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

/// The device key path override, as epr names it.
const DEVICE_KEY_ENV: &str = "ELOHIM_DEVICE_KEY_FILE";
/// The seed's file name inside the device config directory.
const KEY_FILE_NAME: &str = "ed25519.seed";
/// The declaration's file name, beside the device key.
const TRUST_FILE_NAME: &str = "trusted-checkouts";

/// The device key path from an environment lookup, by epr's rule: an absolute
/// `ELOHIM_DEVICE_KEY_FILE` outside every repository, else `$XDG_CONFIG_HOME/elohim/device`
/// (absolute only), else `$HOME/.config/elohim/device`, with the seed's name.
fn resolve_key_with(lookup: &dyn Fn(&str) -> Option<OsString>) -> io::Result<PathBuf> {
    let set = |name: &str| lookup(name).filter(|value| !value.is_empty());
    if let Some(explicit) = set(DEVICE_KEY_ENV) {
        let explicit = PathBuf::from(explicit);
        check_override_path(&explicit)?;
        return Ok(explicit);
    }
    let config_home = match set("XDG_CONFIG_HOME").map(PathBuf::from) {
        Some(xdg) if xdg.is_absolute() => xdg,
        _ => match set("HOME") {
            Some(home) => PathBuf::from(home).join(".config"),
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "no device key path: {DEVICE_KEY_ENV}, XDG_CONFIG_HOME and HOME are all \
                         unset — the device key lives in the OS user's config home, never in a \
                         checkout"
                    ),
                ))
            }
        },
    };
    Ok(config_home
        .join("elohim")
        .join("device")
        .join(KEY_FILE_NAME))
}

/// Refuse a key override that is relative or lies inside a git repository, as epr does.
fn check_override_path(path: &Path) -> io::Result<()> {
    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{DEVICE_KEY_ENV}=`{}` is relative — a relative key path names a different key in \
                 every checkout; give an absolute path outside any repository",
                path.display()
            ),
        ));
    }
    if let Some(repo) = path
        .ancestors()
        .skip(1)
        .find(|dir| dir.join(".git").exists())
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{DEVICE_KEY_ENV}=`{}` lies inside the git repository at `{}` — a seed in a \
                 repository is one commit from being published; keep it outside every checkout",
                path.display(),
                repo.display()
            ),
        ));
    }
    Ok(())
}

/// The declaration beside a device key at `key`.
fn trust_file_beside(key: &Path) -> io::Result<PathBuf> {
    key.parent()
        .map(|p| p.join(TRUST_FILE_NAME))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "device key has no directory"))
}

/// Locate a checkout without asking git to trust it first. Worktree `.git` files are boundaries.
fn checkout(start: &Path) -> Option<PathBuf> {
    let path = start.canonicalize().ok()?;
    path.ancestors()
        .find(|p| p.join(".git").exists())
        .map(Path::to_owned)
}

/// Whether `text` lists `root` as a whole line (`str::lines` strips a trailing `\r`; `Path`
/// equality ignores a trailing `/`).
fn lists(text: &str, root: &Path) -> bool {
    text.lines().any(|line| Path::new(line) == root)
}

/// `-c safe.directory=<root>` for a declared root.
fn args_for(root: &Path) -> Vec<String> {
    vec![
        "-c".to_owned(),
        format!("safe.directory={}", root.display()),
    ]
}

/// One `safe.directory` entry appended after the caller's `existing` count; a count that is not
/// a UTF-8 decimal, or would overflow, adds nothing.
fn config_env_os(existing: Option<&OsStr>, root: &Path) -> Vec<(String, String)> {
    let index = match existing.map(|count| count.to_str().map(|c| c.trim().parse::<usize>())) {
        None => 0,
        Some(Some(Ok(count))) => count,
        Some(_) => return Vec::new(),
    };
    let Some(count) = index.checked_add(1) else {
        return Vec::new();
    };
    vec![
        (
            format!("GIT_CONFIG_KEY_{index}"),
            "safe.directory".to_owned(),
        ),
        (
            format!("GIT_CONFIG_VALUE_{index}"),
            root.display().to_string(),
        ),
        ("GIT_CONFIG_COUNT".to_owned(), count.to_string()),
    ]
}

/// This process's real, effective and supplementary group ids, read from `/proc/self/status` —
/// the same source epr's `device_key::own_groups` reads, so the two readers judge one set.
/// Empty where that file does not exist, so a group-writable declaration refuses there.
fn own_groups() -> Vec<u32> {
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
        return Vec::new();
    };
    status
        .lines()
        .filter_map(|line| {
            line.strip_prefix("Gid:")
                .or_else(|| line.strip_prefix("Groups:"))
        })
        .flat_map(|ids| ids.split_whitespace().filter_map(|id| id.parse().ok()))
        .collect()
}

/// The custody refusal `what` at `path` earns: writable by everyone, or by a group this process
/// does not hold (`own`). Worded as epr's `device_key::check_key_dir` words it.
fn custody_refusal(
    what: &str,
    path: &Path,
    meta: &std::fs::Metadata,
    consequence: &str,
    own: &[u32],
) -> Option<String> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let mode = meta.permissions().mode();
    let world = mode & 0o002 != 0;
    if !world && (mode & 0o020 == 0 || own.contains(&meta.gid())) {
        return None;
    }
    Some(format!(
        "{what} `{}` is writable by {} (mode {:o}, group {}) — anyone who can write there \
         {consequence}; `chmod {}` it first",
        path.display(),
        if world {
            "everyone"
        } else {
            "a group this process is not in"
        },
        mode & 0o7777,
        meta.gid(),
        if world { "o-w" } else { "g-w" }
    ))
}

/// Why a declaration could not be applied: the format contract's reason classes.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Unapplied {
    Unresolved(String),
    Unreadable(String),
    Custody(String),
}

impl Unapplied {
    #[cfg(test)]
    fn class(&self) -> &'static str {
        match self {
            Self::Unresolved(_) => "unresolved",
            Self::Unreadable(_) => "unreadable",
            Self::Custody(_) => "custody",
        }
    }

    fn reason(&self) -> &str {
        match self {
            Self::Unresolved(why) | Self::Unreadable(why) | Self::Custody(why) => why,
        }
    }
}

/// How one look-up came out.
enum Reading {
    Trusted(PathBuf),
    /// No file yet, another checkout declared, or not a checkout: the ordinary states.
    Undeclared,
    Unapplied(Unapplied),
}

/// The declaration's text, read only when its directory and the file (stat'ed through the
/// descriptor it is read from) pass the custody rule. `Ok(None)`: no file yet.
fn read_in_custody(file: &Path) -> Result<Option<String>, Unapplied> {
    let unreadable =
        |error: io::Error| Unapplied::Unreadable(format!("{}: {error}", file.display()));
    let own = own_groups();
    if let Some(dir) = file.parent().filter(|p| !p.as_os_str().is_empty()) {
        match std::fs::metadata(dir) {
            Ok(meta) => {
                if let Some(refused) = custody_refusal(
                    "the device key directory",
                    dir,
                    &meta,
                    "can swap the key",
                    &own,
                ) {
                    return Err(Unapplied::Custody(refused));
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(unreadable(error)),
        }
    }
    let mut handle = match std::fs::File::open(file) {
        Ok(handle) => handle,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(unreadable(error)),
    };
    if let Some(refused) = custody_refusal(
        "the checkout-trust declaration",
        file,
        &handle.metadata().map_err(unreadable)?,
        "can grant git trust to a checkout they plant",
        &own,
    ) {
        return Err(Unapplied::Custody(refused));
    }
    let mut text = String::new();
    handle.read_to_string(&mut text).map_err(unreadable)?;
    Ok(Some(text))
}

/// Where the declarations come from.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Source {
    File(PathBuf),
    Nothing,
    Unresolved(String),
}

/// Where this process reads checkout-trust declarations from, and the `GIT_CONFIG_COUNT` it
/// passes on to its children (captured when the value is built).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredTrust {
    source: Source,
    inherited_count: Option<OsString>,
}

impl DeclaredTrust {
    /// This device's declaration, from the process environment.
    pub fn device() -> Self {
        Self::device_with(&|name| std::env::var_os(name))
    }

    /// This device's declaration with the key path and inherited count read from `lookup`.
    pub fn device_with(lookup: &dyn Fn(&str) -> Option<OsString>) -> Self {
        let source = match resolve_key_with(lookup).and_then(|key| trust_file_beside(&key)) {
            Ok(file) => Source::File(file),
            Err(error) => Source::Unresolved(format!("device trust file not located: {error}")),
        };
        Self {
            source,
            inherited_count: lookup("GIT_CONFIG_COUNT"),
        }
    }

    /// The declarations in `file`.
    #[cfg(test)]
    pub fn at(file: impl Into<PathBuf>) -> Self {
        Self {
            source: Source::File(file.into()),
            inherited_count: std::env::var_os("GIT_CONFIG_COUNT"),
        }
    }

    /// No declarations at all: every rendering is empty and nothing is reported as unapplied.
    #[cfg(test)]
    pub fn none() -> Self {
        Self {
            source: Source::Nothing,
            inherited_count: std::env::var_os("GIT_CONFIG_COUNT"),
        }
    }

    /// The same declarations, with `count` as the `GIT_CONFIG_COUNT` this process passes on.
    #[cfg(test)]
    pub fn inheriting(mut self, count: Option<OsString>) -> Self {
        self.inherited_count = count;
        self
    }

    fn read(&self, cwd: &Path) -> Reading {
        let file = match &self.source {
            Source::File(file) => file,
            Source::Nothing => return Reading::Undeclared,
            Source::Unresolved(why) => {
                return Reading::Unapplied(Unapplied::Unresolved(why.clone()))
            }
        };
        let text = match read_in_custody(file) {
            Ok(Some(text)) => text,
            Ok(None) => return Reading::Undeclared,
            Err(why) => return Reading::Unapplied(why),
        };
        match checkout(cwd) {
            Some(root) if lists(&text, &root) => Reading::Trusted(root),
            _ => Reading::Undeclared,
        }
    }

    /// The checkout holding `cwd`, when this device declared that exact checkout trusted.
    pub fn root(&self, cwd: &Path) -> Option<PathBuf> {
        match self.read(cwd) {
            Reading::Trusted(root) => Some(root),
            Reading::Undeclared | Reading::Unapplied(_) => None,
        }
    }

    /// Why a declaration could not be applied for `cwd` (key path unresolved, file unreadable,
    /// or refused its custody); `None` when trust applies and in the ordinary undeclared states.
    pub fn unapplied(&self, cwd: &Path) -> Option<String> {
        match self.read(cwd) {
            Reading::Unapplied(why) => Some(why.reason().to_owned()),
            Reading::Trusted(_) | Reading::Undeclared => None,
        }
    }

    /// The `-c safe.directory=<root>` pair for `cwd`'s checkout, or nothing when undeclared.
    pub fn git_args(&self, cwd: &Path) -> Vec<String> {
        self.root(cwd)
            .map(|root| args_for(&root))
            .unwrap_or_default()
    }

    /// The same trust as `GIT_CONFIG_*` entries appended after the inherited count.
    pub fn git_config_env_inherited(&self, cwd: &Path) -> Vec<(String, String)> {
        self.root(cwd)
            .map(|root| config_env_os(self.inherited_count.as_deref(), &root))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::ffi::OsStringExt;
    use std::os::unix::fs::PermissionsExt;

    use serde_json::Value;

    // gate-reads: elohim/eprfs/epr-cli/tests/fixtures/declared-trust-vectors.json
    const VECTORS: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../eprfs/epr-cli/tests/fixtures/declared-trust-vectors.json"
    );

    fn mode(value: &Value) -> u32 {
        u32::from_str_radix(value.as_str().unwrap(), 8).unwrap()
    }

    /// The tree the vector file describes (its `tree` field), with one case's device directory.
    fn materialize(base: &Path, input: &Value) {
        let sub = |s: &str| PathBuf::from(s.replace("{base}", &base.display().to_string()));
        for dir in [
            "repo/.git",
            "repo/src/deep",
            "repo/vendor/sub",
            "other/.git",
        ] {
            std::fs::create_dir_all(base.join(dir)).unwrap();
        }
        std::fs::write(
            base.join("repo/vendor/sub/.git"),
            "gitdir: ../../.git/modules/sub\n",
        )
        .unwrap();
        std::os::unix::fs::symlink(base.join("repo"), base.join("link")).unwrap();
        let Some(device) = input["deviceDir"].as_object() else {
            return;
        };
        let dir = sub(device["path"].as_str().unwrap());
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join(TRUST_FILE_NAME);
        let trust = &input["trustFile"];
        if let Some(text) = trust["text"].as_str() {
            std::fs::write(&file, text.replace("{base}", &base.display().to_string())).unwrap();
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(mode(&trust["mode"])))
                .unwrap();
        } else if trust["directory"].as_bool() == Some(true) {
            std::fs::create_dir(&file).unwrap();
        }
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(mode(&device["mode"])))
            .unwrap();
    }

    #[test]
    fn this_reader_answers_every_vector_epr_minted() {
        let document: Value =
            serde_json::from_str(&std::fs::read_to_string(VECTORS).unwrap()).unwrap();
        assert_eq!(document["format"], "elohim-declared-trust-vectors@1");
        let cases = document["cases"].as_array().unwrap();
        assert!(cases.len() >= 20, "the vector file carries its cases");
        for case in cases {
            let name = case["name"].as_str().unwrap();
            let input = &case["input"];
            let dir = tempfile::tempdir().unwrap();
            let base = dir.path().canonicalize().unwrap();
            materialize(&base, input);
            let b = base.display().to_string();
            let unsub = |s: String| s.replace(&b, "{base}");
            let env: Vec<(String, OsString)> = input["env"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap().replace("{base}", &b).into()))
                .collect();
            let count = match &input["inheritedCount"] {
                Value::Null => None,
                count if count["utf8"].is_string() => {
                    Some(OsString::from(count["utf8"].as_str().unwrap()))
                }
                count => Some(OsString::from_vec(
                    count["bytes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|b| u8::try_from(b.as_u64().unwrap()).unwrap())
                        .collect(),
                )),
            };
            let lookup = |name: &str| {
                if name == "GIT_CONFIG_COUNT" {
                    return count.clone();
                }
                env.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone())
            };
            let cwd = PathBuf::from(input["cwd"].as_str().unwrap().replace("{base}", &b));
            let trust = DeclaredTrust::device_with(&lookup);
            let expect = &case["expect"];

            let trust_file = match &trust.source {
                Source::File(file) => Value::String(unsub(file.display().to_string())),
                Source::Nothing | Source::Unresolved(_) => Value::Null,
            };
            assert_eq!(trust_file, expect["trustFile"], "{name}: trust file");
            let (root, unapplied) = match trust.read(&cwd) {
                Reading::Trusted(root) => (
                    Value::String(unsub(root.display().to_string())),
                    Value::Null,
                ),
                Reading::Undeclared => (Value::Null, Value::Null),
                Reading::Unapplied(why) => (Value::Null, Value::String(why.class().into())),
            };
            assert_eq!(root, expect["root"], "{name}: root");
            assert_eq!(unapplied, expect["unapplied"], "{name}: unapplied class");
            let args: Vec<Value> = trust
                .git_args(&cwd)
                .into_iter()
                .map(|a| Value::String(unsub(a)))
                .collect();
            assert_eq!(Value::Array(args), expect["gitArgs"], "{name}: git args");
            let env: Vec<Value> = trust
                .git_config_env_inherited(&cwd)
                .into_iter()
                .map(|(k, v)| Value::Array(vec![Value::String(k), Value::String(unsub(v))]))
                .collect();
            assert_eq!(Value::Array(env), expect["configEnv"], "{name}: config env");
        }
    }

    /// The ordinary states say nothing, so a failed child's message gains no clause from them.
    #[test]
    fn only_an_unusable_declaration_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().canonicalize().unwrap();
        std::fs::create_dir_all(base.join("repo/.git")).unwrap();
        let file = base.join(TRUST_FILE_NAME);
        assert_eq!(DeclaredTrust::at(&file).unapplied(&base.join("repo")), None);
        assert_eq!(DeclaredTrust::none().unapplied(&base.join("repo")), None);
        let unresolved = DeclaredTrust::device_with(&|_| None);
        assert!(unresolved
            .unapplied(&base.join("repo"))
            .unwrap()
            .starts_with("device trust file not located"));
    }
}
