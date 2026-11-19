//! Which checkouts this device declared trusted, read for a child that runs git.
//!
//! `epr device trust <path>` appends one canonical checkout root to `trusted-checkouts` beside
//! the device key; it never touches global git configuration. After a workspace restart a
//! different uid can own the checkout, and until global `safe.directory` is restored git refuses
//! it for "dubious ownership". Every process that spawns git (or a tool that runs git itself,
//! such as `brit-build-ref`) reads that declaration here and passes exactly it on, two ways:
//!
//! - as arguments, the `-c safe.directory=<root>` pair for a `git` command line
//!   ([`DeclaredTrust::git_args`]);
//! - as environment, `GIT_CONFIG_KEY_n`/`GIT_CONFIG_VALUE_n`/`GIT_CONFIG_COUNT` appended after
//!   any entries the caller already passes, for a child that runs git (or gitoxide) itself
//!   ([`DeclaredTrust::git_config_env`]).
//!
//! **Nothing is granted that the device did not declare.** The root passed on is the checkout
//! holding the working directory, found without asking git (a worktree's or submodule's `.git`
//! file is a boundary, so trusting a parent never trusts a nested repository), and only when that
//! exact root is a line of the declaration file. No declaration, an unreadable file, or a
//! declaration for another checkout adds nothing; `safe.directory=*` is never produced.
//!
//! **Whoever can write the declaration can grant trust**, so a reader holds it to the custody
//! rule the device key's directory is held to ([`device_key_path::check_key_dir`]): neither its
//! directory nor the file itself may be writable by everyone or by a group this process does not
//! hold. Ownership is not checked, because the uid owning a container's home volume shifts across
//! restarts — the case this module exists for. A refused declaration grants nothing, and the
//! reason is kept ([`DeclaredTrust::unapplied`]) for a caller to name when its child fails.

use std::ffi::{OsStr, OsString};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use crate::device_key_path;

/// The declaration file's name, beside the device key.
pub const TRUST_FILE_NAME: &str = "trusted-checkouts";

/// The declaration file for a device key at `key`: `<key dir>/trusted-checkouts`.
pub fn trust_file_beside(key: &Path) -> io::Result<PathBuf> {
    key.parent()
        .map(|p| p.join(TRUST_FILE_NAME))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "device key has no directory"))
}

/// This device's declaration file, located from the device key path ([`device_key_path`]).
pub fn trust_file() -> io::Result<PathBuf> {
    trust_file_beside(&device_key_path::resolve_path()?)
}

/// Locate a checkout without asking git to trust it first. Worktree `.git` files are boundaries.
pub fn checkout(start: &Path) -> Option<PathBuf> {
    let path = start.canonicalize().ok()?;
    path.ancestors()
        .find(|p| p.join(".git").exists())
        .map(Path::to_owned)
}

/// Whether `root` is one line of the declaration file at `file`. Unreadable reads as undeclared.
///
/// This is the writer's duplicate check (`epr device trust` holds the file's lock); a reader
/// deciding trust goes through [`DeclaredTrust`], which also applies the custody rule.
pub fn declared_at(root: &Path, file: &Path) -> bool {
    std::fs::read_to_string(file)
        .ok()
        .is_some_and(|text| lists(&text, root))
}

/// Whether `text` has `root` as a whole line. `str::lines` also strips a trailing `\r`, and
/// `Path` equality ignores a trailing `/`, so `/r\r` and `/r/` both name `/r`, never another path.
fn lists(text: &str, root: &Path) -> bool {
    text.lines().any(|line| Path::new(line) == root)
}

/// `-c safe.directory=<root>` for a declared root: the pair a `git` command line takes.
pub fn args_for(root: &Path) -> Vec<String> {
    vec![
        "-c".to_owned(),
        format!("safe.directory={}", root.display()),
    ]
}

/// Append one `safe.directory` entry after any the caller already passes (`existing` is the
/// caller's `GIT_CONFIG_COUNT`). A malformed or overflowing count is left for git to refuse,
/// never repaired over the caller's own entries: nothing is added.
pub fn config_env(existing: Option<&str>, root: &Path) -> Vec<(String, String)> {
    let index = match existing.map(|count| count.trim().parse::<usize>()) {
        None => 0,
        Some(Ok(count)) => count,
        Some(Err(_)) => return Vec::new(),
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

/// [`config_env`] after a count read as bytes: a count that is not UTF-8 is malformed, so
/// nothing is added rather than entry 0 written over the caller's own.
pub fn config_env_os(existing: Option<&OsStr>, root: &Path) -> Vec<(String, String)> {
    match existing.map(OsStr::to_str) {
        None => config_env(None, root),
        Some(Some(count)) => config_env(Some(count), root),
        Some(None) => Vec::new(),
    }
}

/// Where the declarations come from.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Source {
    /// A declaration file, read (and custody-checked) on every look-up.
    File(PathBuf),
    /// No declarations by construction.
    Nothing,
    /// The device's declaration file could not be located; why.
    Unresolved(String),
}

/// How a look-up for one working directory came out.
enum Reading {
    Trusted(PathBuf),
    /// No declaration applies: none written yet, not a checkout, or another checkout declared.
    Undeclared,
    /// The declaration exists but could not be used; why.
    Unusable(String),
}

/// Where this process reads checkout-trust declarations from, and the `GIT_CONFIG_COUNT` it
/// passes on to its children (captured when the value is built).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredTrust {
    source: Source,
    inherited_count: Option<OsString>,
}

impl DeclaredTrust {
    /// This device's declaration file, from the process environment.
    pub fn device() -> Self {
        Self::device_with(|name| std::env::var_os(name))
    }

    /// This device's declaration file as [`device_key_path::resolve_path_with`] finds the key
    /// from `lookup`, which also supplies the inherited `GIT_CONFIG_COUNT`. An unresolvable key
    /// home declares nothing and keeps why.
    pub fn device_with<F>(lookup: F) -> Self
    where
        F: Fn(&str) -> Option<OsString>,
    {
        let source = match device_key_path::resolve_path_with(&lookup)
            .and_then(|key| trust_file_beside(&key))
        {
            Ok(file) => Source::File(file),
            Err(error) => Source::Unresolved(format!("device trust file not located: {error}")),
        };
        Self {
            source,
            inherited_count: lookup("GIT_CONFIG_COUNT"),
        }
    }

    /// The declarations in `file`: for tests, and for a caller that already resolved the key.
    pub fn at(file: impl Into<PathBuf>) -> Self {
        Self {
            source: Source::File(file.into()),
            inherited_count: std::env::var_os("GIT_CONFIG_COUNT"),
        }
    }

    /// No declarations at all: every rendering is empty and nothing is reported as unapplied.
    pub fn none() -> Self {
        Self {
            source: Source::Nothing,
            inherited_count: std::env::var_os("GIT_CONFIG_COUNT"),
        }
    }

    /// The same declarations, with `count` as the `GIT_CONFIG_COUNT` this process passes on.
    pub fn inheriting(mut self, count: Option<OsString>) -> Self {
        self.inherited_count = count;
        self
    }

    /// The declaration file read, when there is one.
    pub fn file(&self) -> Option<&Path> {
        match &self.source {
            Source::File(file) => Some(file),
            Source::Nothing | Source::Unresolved(_) => None,
        }
    }

    fn read(&self, cwd: &Path) -> Reading {
        let file = match &self.source {
            Source::File(file) => file,
            Source::Nothing => return Reading::Undeclared,
            Source::Unresolved(why) => return Reading::Unusable(why.clone()),
        };
        let text = match read_in_custody(file) {
            Ok(Some(text)) => text,
            Ok(None) => return Reading::Undeclared,
            Err(error) => return Reading::Unusable(error.to_string()),
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
            Reading::Undeclared | Reading::Unusable(_) => None,
        }
    }

    /// Why a declaration could not be applied for `cwd`: the key home did not resolve, or the
    /// file is unreadable or refused by the custody rule. `None` when trust applies, and when
    /// none was declared (no file yet, another checkout, not a checkout) — the ordinary states.
    pub fn unapplied(&self, cwd: &Path) -> Option<String> {
        match self.read(cwd) {
            Reading::Unusable(why) => Some(why),
            Reading::Trusted(_) | Reading::Undeclared => None,
        }
    }

    /// The `-c safe.directory=<root>` pair for `cwd`'s checkout, or nothing when undeclared.
    pub fn git_args(&self, cwd: &Path) -> Vec<String> {
        self.root(cwd)
            .map(|root| args_for(&root))
            .unwrap_or_default()
    }

    /// The same trust as `GIT_CONFIG_*` entries appended after `existing` (the caller's
    /// `GIT_CONFIG_COUNT`), or nothing when undeclared.
    pub fn git_config_env(&self, cwd: &Path, existing: Option<&str>) -> Vec<(String, String)> {
        self.root(cwd)
            .map(|root| config_env(existing, &root))
            .unwrap_or_default()
    }

    /// [`DeclaredTrust::git_config_env`] after the `GIT_CONFIG_COUNT` this process passes on
    /// (read as bytes; one that is not UTF-8 is malformed and adds nothing, see
    /// [`config_env_os`]).
    pub fn git_config_env_inherited(&self, cwd: &Path) -> Vec<(String, String)> {
        self.root(cwd)
            .map(|root| config_env_os(self.inherited_count.as_deref(), &root))
            .unwrap_or_default()
    }
}

/// The declaration file's text, read only when its directory and the file itself hold the
/// device directory's custody rule. `Ok(None)`: no file yet. The file is stat'ed through the
/// descriptor it is read from, so the mode judged is the mode of the bytes read.
fn read_in_custody(file: &Path) -> io::Result<Option<String>> {
    device_key_path::check_key_dir(file)?;
    let mut handle = match std::fs::File::open(file) {
        Ok(handle) => handle,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let meta = handle.metadata()?;
    if let Some(refused) = file_refusal(file, &meta) {
        return Err(refused);
    }
    let mut text = String::new();
    handle.read_to_string(&mut text)?;
    Ok(Some(text))
}

#[cfg(unix)]
fn file_refusal(file: &Path, meta: &std::fs::Metadata) -> Option<io::Error> {
    device_key_path::custody_refusal(
        "the checkout-trust declaration",
        file,
        meta,
        "can grant git trust to a checkout they plant",
    )
}

#[cfg(not(unix))]
fn file_refusal(_file: &Path, _meta: &std::fs::Metadata) -> Option<io::Error> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch checkout and a declaration file outside it.
    struct Fixture {
        _dir: tempfile::TempDir,
        repo: PathBuf,
        other: PathBuf,
        file: PathBuf,
    }

    fn fixture() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().canonicalize().unwrap();
        let repo = base.join("repo");
        let other = base.join("other");
        for checkout in [&repo, &other] {
            std::fs::create_dir_all(checkout.join(".git")).unwrap();
            std::fs::create_dir_all(checkout.join("src/deep")).unwrap();
        }
        let file = base.join("device").join(TRUST_FILE_NAME);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        Fixture {
            _dir: dir,
            repo,
            other,
            file,
        }
    }

    #[test]
    fn a_declared_checkout_renders_both_ways_from_any_depth() {
        let f = fixture();
        std::fs::write(&f.file, format!("{}\n", f.repo.display())).unwrap();
        let trust = DeclaredTrust::at(&f.file);
        let cwd = f.repo.join("src/deep");
        assert_eq!(trust.root(&cwd), Some(f.repo.clone()));
        assert_eq!(
            trust.git_args(&cwd),
            vec![
                "-c".to_owned(),
                format!("safe.directory={}", f.repo.display())
            ]
        );
        assert_eq!(
            trust.git_config_env(&cwd, None),
            vec![
                ("GIT_CONFIG_KEY_0".into(), "safe.directory".into()),
                ("GIT_CONFIG_VALUE_0".into(), f.repo.display().to_string()),
                ("GIT_CONFIG_COUNT".into(), "1".into()),
            ]
        );
    }

    #[test]
    fn no_declaration_adds_nothing() {
        let f = fixture();
        // Absent file, empty file, and no file configured at all.
        for trust in [
            DeclaredTrust::at(&f.file),
            DeclaredTrust::none(),
            DeclaredTrust::at(f.file.with_file_name("missing")),
        ] {
            assert!(trust.git_args(&f.repo).is_empty());
            assert!(trust.git_config_env(&f.repo, None).is_empty());
        }
        std::fs::write(&f.file, "").unwrap();
        let trust = DeclaredTrust::at(&f.file);
        assert!(trust.git_args(&f.repo).is_empty());
        assert!(trust.git_config_env(&f.repo, Some("2")).is_empty());
    }

    #[test]
    fn a_declaration_for_another_checkout_adds_nothing() {
        let f = fixture();
        std::fs::write(&f.file, format!("{}\n", f.other.display())).unwrap();
        let trust = DeclaredTrust::at(&f.file);
        assert!(trust.root(&f.repo).is_none());
        assert!(trust.git_args(&f.repo).is_empty());
        assert!(trust.git_config_env(&f.repo, None).is_empty());
        // The declared one still reads as declared.
        assert_eq!(trust.root(&f.other), Some(f.other.clone()));
    }

    #[test]
    fn a_parent_declaration_never_trusts_a_nested_checkout() {
        let f = fixture();
        let nested = f.repo.join("vendor/sub");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join(".git"), "gitdir: ../../.git/modules/sub\n").unwrap();
        std::fs::write(&f.file, format!("{}\n", f.repo.display())).unwrap();
        let trust = DeclaredTrust::at(&f.file);
        assert!(trust.git_args(&nested).is_empty());
        assert!(!trust.git_args(&f.repo).is_empty());
    }

    #[test]
    fn a_wildcard_or_prefix_line_is_not_a_declaration() {
        let f = fixture();
        let parent = f.repo.parent().unwrap().display().to_string();
        std::fs::write(
            &f.file,
            format!("*\n{parent}\n{}-sibling\n", f.repo.display()),
        )
        .unwrap();
        let trust = DeclaredTrust::at(&f.file);
        assert!(trust.git_args(&f.repo).is_empty());
        assert!(trust.git_config_env(&f.repo, None).is_empty());
    }

    #[test]
    fn config_env_appends_after_the_callers_entries() {
        let root = Path::new("/projects/elohim");
        let after_two = config_env(Some("2"), root);
        assert_eq!(
            after_two,
            vec![
                ("GIT_CONFIG_KEY_2".into(), "safe.directory".into()),
                ("GIT_CONFIG_VALUE_2".into(), "/projects/elohim".into()),
                ("GIT_CONFIG_COUNT".into(), "3".into()),
            ]
        );
        assert_eq!(config_env(Some(" 1 "), root)[0].0, "GIT_CONFIG_KEY_1");
        assert!(config_env(Some("bogus"), root).is_empty());
        assert!(config_env(Some(&usize::MAX.to_string()), root).is_empty());
    }

    #[test]
    fn the_declaration_file_sits_beside_the_device_key() {
        assert_eq!(
            trust_file_beside(Path::new("/home/u/.config/elohim/device/ed25519.seed")).unwrap(),
            PathBuf::from("/home/u/.config/elohim/device/trusted-checkouts")
        );
        assert!(trust_file_beside(Path::new("/")).is_err());
    }

    fn chmod(path: &Path, mode: u32) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    }

    fn declare(f: &Fixture) -> DeclaredTrust {
        std::fs::write(&f.file, format!("{}\n", f.repo.display())).unwrap();
        chmod(&f.file, 0o600);
        chmod(f.file.parent().unwrap(), 0o700);
        DeclaredTrust::at(&f.file).inheriting(None)
    }

    #[test]
    fn a_private_declaration_and_one_under_our_own_group_are_honoured() {
        let f = fixture();
        let trust = declare(&f);
        assert_eq!(trust.root(&f.repo), Some(f.repo.clone()));
        assert_eq!(trust.unapplied(&f.repo), None);
        // A Kubernetes fsGroup volume: group-writable, but by a group this process holds (the
        // group a fresh file is created with is this process's own).
        chmod(f.file.parent().unwrap(), 0o2770);
        chmod(&f.file, 0o660);
        assert_eq!(trust.root(&f.repo), Some(f.repo.clone()));
    }

    #[test]
    fn a_world_writable_declaration_or_directory_grants_nothing_and_says_why() {
        let f = fixture();
        let trust = declare(&f);
        chmod(&f.file, 0o666);
        assert!(trust.git_args(&f.repo).is_empty());
        assert!(trust.git_config_env_inherited(&f.repo).is_empty());
        let why = trust.unapplied(&f.repo).expect("a refused file is named");
        assert!(
            why.contains("checkout-trust declaration") && why.contains("everyone"),
            "{why}"
        );

        chmod(&f.file, 0o600);
        chmod(f.file.parent().unwrap(), 0o777);
        assert!(trust.git_args(&f.repo).is_empty());
        let why = trust
            .unapplied(&f.repo)
            .expect("a refused directory is named");
        assert!(why.contains("device key directory"), "{why}");
    }

    /// The group rule as the file reader applies it: only the predicate can be staged without
    /// root (a file cannot be given to a group this process is not in), so it is checked here
    /// and the file path shares it through `custody_refusal`.
    #[test]
    fn a_group_this_process_does_not_hold_is_a_stranger() {
        use device_key_path::{stranger_may_write, Stranger};
        assert_eq!(
            stranger_may_write(0o620, 4_242_424, &device_key_path::own_groups()),
            Some(Stranger::ForeignGroup)
        );
    }

    fn lookup(pairs: Vec<(&'static str, OsString)>) -> impl Fn(&str) -> Option<OsString> {
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.clone())
        }
    }

    #[test]
    fn device_follows_the_key_path_to_the_file_beside_it() {
        let f = fixture();
        declare(&f);
        let key = f.file.with_file_name("ed25519.seed");
        let trust = DeclaredTrust::device_with(lookup(vec![(
            device_key_path::DEVICE_KEY_ENV,
            key.clone().into_os_string(),
        )]));
        assert_eq!(trust.file(), Some(f.file.as_path()));
        assert_eq!(trust.root(&f.repo), Some(f.repo.clone()));

        let xdg = DeclaredTrust::device_with(lookup(vec![("XDG_CONFIG_HOME", "/xdg".into())]));
        assert_eq!(
            xdg.file(),
            Some(Path::new("/xdg/elohim/device/trusted-checkouts"))
        );
    }

    #[test]
    fn an_override_in_a_world_writable_directory_grants_nothing() {
        let f = fixture();
        declare(&f);
        chmod(f.file.parent().unwrap(), 0o777);
        let trust = DeclaredTrust::device_with(lookup(vec![(
            device_key_path::DEVICE_KEY_ENV,
            f.file.with_file_name("ed25519.seed").into_os_string(),
        )]));
        assert!(trust.git_args(&f.repo).is_empty());
        assert!(trust.unapplied(&f.repo).unwrap().contains("everyone"));
    }

    #[test]
    fn an_unresolvable_key_home_grants_nothing_and_says_why() {
        let f = fixture();
        for env in [
            vec![(
                device_key_path::DEVICE_KEY_ENV,
                OsString::from("relative/key"),
            )],
            vec![],
        ] {
            let trust = DeclaredTrust::device_with(lookup(env));
            assert!(trust.git_args(&f.repo).is_empty());
            let why = trust.unapplied(&f.repo).expect("named");
            assert!(why.starts_with("device trust file not located"), "{why}");
        }
        // The ordinary states are not reported: no file yet, or nothing declared by construction.
        assert_eq!(DeclaredTrust::at(&f.file).unapplied(&f.repo), None);
        assert_eq!(DeclaredTrust::none().unapplied(&f.repo), None);
    }

    #[test]
    fn the_inherited_count_is_appended_after_and_a_non_utf8_count_adds_nothing() {
        use std::os::unix::ffi::OsStringExt;
        let f = fixture();
        let trust = declare(&f).inheriting(Some("2".into()));
        assert_eq!(
            trust.git_config_env_inherited(&f.repo),
            vec![
                ("GIT_CONFIG_KEY_2".into(), "safe.directory".into()),
                ("GIT_CONFIG_VALUE_2".into(), f.repo.display().to_string()),
                ("GIT_CONFIG_COUNT".into(), "3".into()),
            ]
        );
        let from_lookup = DeclaredTrust::device_with(lookup(vec![
            (
                device_key_path::DEVICE_KEY_ENV,
                f.file.with_file_name("k").into_os_string(),
            ),
            ("GIT_CONFIG_COUNT", "1".into()),
        ]));
        assert_eq!(
            from_lookup.git_config_env_inherited(&f.repo)[0].0,
            "GIT_CONFIG_KEY_1"
        );
        let bytes = declare(&f).inheriting(Some(OsString::from_vec(vec![b'1', 0xff])));
        assert!(bytes.git_config_env_inherited(&f.repo).is_empty());
        assert!(config_env_os(Some(OsStr::new("x")), &f.repo).is_empty());
    }

    #[test]
    fn a_symlinked_cwd_resolves_to_the_real_root_which_must_be_the_declared_one() {
        let f = fixture();
        let link = f.repo.parent().unwrap().join("link");
        std::os::unix::fs::symlink(&f.repo, &link).unwrap();
        let trust = declare(&f);
        assert_eq!(trust.root(&link.join("src")), Some(f.repo.clone()));
        // A declaration naming only the symlink path never matches: roots are canonical.
        std::fs::write(&f.file, format!("{}\n", link.display())).unwrap();
        assert_eq!(trust.root(&link), None);
    }

    /// `str::lines` strips a trailing `\r` and `Path` equality ignores a trailing `/`: both lines
    /// still name exactly the declared root, never a different path.
    #[test]
    fn a_crlf_or_trailing_slash_line_names_the_same_root() {
        let f = fixture();
        let trust = declare(&f);
        for line in [
            format!("{}\r\n", f.repo.display()),
            format!("{}/\n", f.repo.display()),
        ] {
            std::fs::write(&f.file, line).unwrap();
            assert_eq!(trust.root(&f.repo), Some(f.repo.clone()));
        }
        std::fs::write(&f.file, format!("{}\r\n", f.other.display())).unwrap();
        assert_eq!(trust.root(&f.repo), None);
    }

    #[test]
    #[ignore = "needs root to give a file to a group this process does not hold"]
    fn a_declaration_writable_by_a_foreign_group_grants_nothing() {
        let f = fixture();
        let trust = declare(&f);
        let stranger = 4_242_424;
        std::os::unix::fs::chown(&f.file, None, Some(stranger)).expect("chown needs root");
        chmod(&f.file, 0o620);
        assert!(trust.git_args(&f.repo).is_empty());
        let why = trust.unapplied(&f.repo).unwrap();
        assert!(why.contains("a group this process is not in"), "{why}");
        chmod(&f.file, 0o640);
        assert_eq!(
            trust.root(&f.repo),
            Some(f.repo.clone()),
            "group-read only is fine"
        );
        std::os::unix::fs::chown(f.file.parent().unwrap(), None, Some(stranger)).unwrap();
        chmod(f.file.parent().unwrap(), 0o770);
        assert!(trust
            .unapplied(&f.repo)
            .unwrap()
            .contains("device key directory"));
    }
}
