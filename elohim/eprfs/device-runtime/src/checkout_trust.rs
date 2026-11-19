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

use std::path::{Path, PathBuf};

use crate::device_key_path;

/// The declaration file's name, beside the device key.
pub const TRUST_FILE_NAME: &str = "trusted-checkouts";

/// The declaration file for a device key at `key`: `<key dir>/trusted-checkouts`.
pub fn trust_file_beside(key: &Path) -> std::io::Result<PathBuf> {
    key.parent()
        .map(|p| p.join(TRUST_FILE_NAME))
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "device key has no directory",
            )
        })
}

/// This device's declaration file, located from the device key path ([`device_key_path`]).
pub fn trust_file() -> std::io::Result<PathBuf> {
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
pub fn declared_at(root: &Path, file: &Path) -> bool {
    std::fs::read_to_string(file)
        .ok()
        .is_some_and(|text| text.lines().any(|line| Path::new(line) == root))
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

/// Where this process reads checkout-trust declarations from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredTrust {
    file: Option<PathBuf>,
}

impl DeclaredTrust {
    /// This device's declaration file. A device with no resolvable key home declares nothing.
    pub fn device() -> Self {
        Self {
            file: trust_file().ok(),
        }
    }

    /// The declarations in `file`: for tests, and for a caller that already resolved the key.
    pub fn at(file: impl Into<PathBuf>) -> Self {
        Self {
            file: Some(file.into()),
        }
    }

    /// No declarations at all: every rendering is empty.
    pub fn none() -> Self {
        Self { file: None }
    }

    /// The checkout holding `cwd`, when this device declared that exact checkout trusted.
    pub fn root(&self, cwd: &Path) -> Option<PathBuf> {
        let file = self.file.as_deref()?;
        let root = checkout(cwd)?;
        declared_at(&root, file).then_some(root)
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

    /// [`DeclaredTrust::git_config_env`] after the `GIT_CONFIG_COUNT` this process passes on.
    pub fn git_config_env_inherited(&self, cwd: &Path) -> Vec<(String, String)> {
        let existing = std::env::var("GIT_CONFIG_COUNT").ok();
        self.git_config_env(cwd, existing.as_deref())
    }
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
}
