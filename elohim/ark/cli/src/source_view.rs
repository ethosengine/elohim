//! `ark task source-root`: a candidate task input that is a projected view of real source files.
//!
//! The legacy material input ([`crate::material`]) is a tree whose leaves are the closure
//! material's JSON fields, so its root moves when any keyed field moves, whatever the gate reads.
//! This is the other shape, computed beside it in shadow: the concrete repository-relative files a
//! gate reads, captured from a checkout through eprfs-local's [`capture_tree`] (which refuses
//! anything but explicit regular files and literal symlinks), then projected under a [`View`]
//! through eprfs-core's canonical projection. Nothing decides on it yet: `gate-task.mjs` prints
//! it next to the legacy key so the two can be compared over landings.
//!
//! The store is the caller's (a scratch directory in shadow); captured objects stay in it. A
//! narrow projection's re-encoded spine nodes are returned by eprfs-core and not persisted here:
//! the shadow needs the root's CID, not a readable projected tree.

use std::{
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
};

use eprfs_core::{
    project_view, walk_spine, BatchOptions, BlobCid, FetchPolicy, ProjectedRoot, TreeLimits,
    TreePath, View,
};
use eprfs_storage::DirectoryStorage;
use serde::Serialize;

use crate::Failure;

/// The largest file list `source-root` reads; a gate's read set is far smaller.
const MAX_LIST_BYTES: u64 = 16 * 1024 * 1024;

/// What one capture-and-project gives: the captured tree's root, its projection under the view,
/// and how much was read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceRoot {
    pub(crate) captured: BlobCid,
    pub(crate) projected: ProjectedRoot,
    pub(crate) files: usize,
    pub(crate) bytes: usize,
}

/// Refuses a selection path that is absolute, empty, or names `.` or `..` anywhere, before
/// anything is read: a source view never reaches outside its checkout. eprfs-local's capture
/// refuses these too; this names the refusal in the source view's own terms.
fn check_relative(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("an empty path is not a source file".into());
    }
    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            Component::RootDir | Component::Prefix(_) => {
                return Err(format!(
                    "{} is absolute; a source path is repository-relative",
                    path.display()
                ))
            }
            Component::CurDir | Component::ParentDir => {
                return Err(format!(
                    "{} escapes or re-enters the checkout root",
                    path.display()
                ))
            }
        }
    }
    Ok(())
}

/// Stats `path` one component at a time without following a symlink, so no stat ever passes
/// through a symlinked ancestor out of the checkout: an ancestor that is a symlink is refused, a
/// final component that is a directory is refused, and a final symlink is kept as its text.
/// eprfs-local's capture repeats these checks on descriptors (NOFOLLOW) when it reads.
fn check_no_symlink_ancestor(checkout: &Path, path: &Path) -> Result<(), String> {
    let components: Vec<_> = path.components().collect();
    let mut at = checkout.to_path_buf();
    for (index, component) in components.iter().enumerate() {
        at.push(component);
        let meta =
            fs::symlink_metadata(&at).map_err(|error| format!("{}: {error}", path.display()))?;
        let last = index + 1 == components.len();
        if !last && meta.file_type().is_symlink() {
            return Err(format!(
                "{} passes through a symlink; a source view never follows one",
                path.display()
            ));
        }
        if last && meta.is_dir() {
            return Err(format!(
                "{} is a directory; a source view names concrete files",
                path.display()
            ));
        }
    }
    Ok(())
}

/// Captures `files` (repository-relative, each a regular file or a literal symlink under
/// `checkout`) into the directory store at `store`, and projects the captured tree under `view`.
///
/// The whole view verifies the entire captured closure and is the captured root. A narrower
/// view walks the verified spine locally and re-encodes it (eprfs-core's `project_view`), so a
/// captured file outside the view does not move the projected root. Directories, absolute paths
/// and `..` are refused; the store must not lie inside the checkout (capture refuses that).
pub(crate) async fn project_sources(
    checkout: &Path,
    store: &Path,
    files: &[PathBuf],
    view: &View,
    limits: &TreeLimits,
) -> Result<SourceRoot, String> {
    if files.is_empty() {
        return Err("a source view names at least one file".into());
    }
    if view.is_empty() {
        return Err("a source view's projection names at least one path".into());
    }
    for path in files {
        check_relative(path)?;
        check_no_symlink_ancestor(checkout, path)?;
    }
    let report = eprfs_local::capture_tree(checkout, store, files, limits)
        .await
        .map_err(|error| format!("capture: {error}"))?;
    let storage = DirectoryStorage::open(store).map_err(|error| format!("store: {error}"))?;
    let projected = if view == &View::whole() {
        ProjectedRoot::verify_whole_tree(
            &storage,
            &report.root,
            view,
            FetchPolicy::LocalOnly,
            limits,
        )
        .await
        .map_err(|error| format!("projection: {error}"))?
    } else {
        let spine = walk_spine(
            &storage,
            &report.root,
            view,
            &BatchOptions::from(FetchPolicy::LocalOnly),
            limits,
        )
        .await
        .map_err(|error| format!("projection: {error}"))?;
        project_view(&spine, limits)
            .map(|(projected, _)| projected)
            .map_err(|error| format!("projection: {error}"))?
    };
    Ok(SourceRoot {
        captured: report.root,
        projected,
        files: report.files + report.symlinks,
        bytes: report.bytes_captured,
    })
}

/// Reads the file list: a JSON array of repository-relative path strings.
fn read_list(path: &Path) -> Result<Vec<PathBuf>, Failure> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .and_then(|file| file.take(MAX_LIST_BYTES + 1).read_to_end(&mut bytes))
        .map_err(|error| Failure::data(format!("reading {}: {error}", path.display())))?;
    if bytes.len() as u64 > MAX_LIST_BYTES {
        return Err(Failure::data("source file list exceeds 16 MiB"));
    }
    let names: Vec<String> = serde_json::from_slice(&bytes)
        .map_err(|error| Failure::data(format!("source file list: {error}")))?;
    Ok(names.into_iter().map(PathBuf::from).collect())
}

/// The view `--view` names; none is the whole captured tree.
fn view_of(paths: &[String]) -> Result<View, Failure> {
    if paths.is_empty() {
        return Ok(View::whole());
    }
    paths
        .iter()
        .map(|path| {
            TreePath::from_slash_bytes(path.as_bytes())
                .map_err(|error| Failure::data(format!("view path {path}: {error}")))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(View::from_tree_paths)
}

/// `ark task source-root`: prints `{captured, projected, files, bytes}` for the listed files.
pub(crate) fn source_root(
    checkout: &Path,
    files: &Path,
    store: &Path,
    view: &[String],
) -> Result<u8, Failure> {
    let files = read_list(files)?;
    let view = view_of(view)?;
    let root = futures::executor::block_on(project_sources(
        checkout,
        store,
        &files,
        &view,
        &TreeLimits::default(),
    ))
    .map_err(Failure::data)?;
    let json = serde_json::to_string(&root).map_err(|error| Failure::data(error.to_string()))?;
    println!("{json}");
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Checkout {
        _temp: tempfile::TempDir,
        root: PathBuf,
        store: PathBuf,
    }

    impl Checkout {
        fn new() -> Self {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("checkout");
            fs::create_dir(&root).unwrap();
            let store = temp.path().join("store");
            Self {
                _temp: temp,
                root,
                store,
            }
        }

        fn write(&self, name: &str, bytes: &[u8]) {
            let path = self.root.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }

        /// A fresh store each time, so a root never depends on what an earlier capture left.
        fn project(&self, files: &[&str], view: &View) -> Result<SourceRoot, String> {
            let _ = fs::remove_dir_all(&self.store);
            futures::executor::block_on(project_sources(
                &self.root,
                &self.store,
                &files.iter().map(PathBuf::from).collect::<Vec<_>>(),
                view,
                &TreeLimits::default(),
            ))
        }
    }

    fn sample() -> Checkout {
        let c = Checkout::new();
        c.write(
            "genesis/orchestrator/gate-task.mjs",
            b"export const a = 1;\n",
        );
        c.write("genesis/orchestrator/suite-reads.json", b"{}\n");
        c.write("package.json", b"{\"name\":\"x\"}\n");
        c.write("unrelated/notes.md", b"not read\n");
        c
    }

    const FILES: [&str; 3] = [
        "genesis/orchestrator/gate-task.mjs",
        "genesis/orchestrator/suite-reads.json",
        "package.json",
    ];

    #[test]
    fn changing_an_included_file_changes_the_root() {
        let c = sample();
        let before = c.project(&FILES, &View::whole()).unwrap();
        assert_eq!(before.files, 3);
        assert_eq!(before.projected.cid(), &before.captured);
        assert_eq!(before.projected.cid().as_cid().codec(), 0x71);
        c.write(
            "genesis/orchestrator/gate-task.mjs",
            b"export const a = 2;\n",
        );
        let after = c.project(&FILES, &View::whole()).unwrap();
        assert_ne!(before.projected, after.projected);
    }

    #[test]
    fn changing_a_file_outside_the_view_does_not_change_the_root() {
        let c = sample();
        // Not listed at all: never captured.
        let before = c.project(&FILES, &View::whole()).unwrap();
        c.write("unrelated/notes.md", b"changed\n");
        c.write("genesis/orchestrator/other.mjs", b"new and unlisted\n");
        assert_eq!(before, c.project(&FILES, &View::whole()).unwrap());

        // Captured, but outside a narrower view: the captured root moves, the projection does not.
        let narrow = View::from_tree_paths([TreePath::from_slash_bytes(b"genesis").unwrap()]);
        let wide = ["genesis/orchestrator/gate-task.mjs", "package.json"];
        let first = c.project(&wide, &narrow).unwrap();
        assert_ne!(first.projected.cid(), &first.captured);
        c.write("package.json", b"{\"name\":\"y\"}\n");
        let second = c.project(&wide, &narrow).unwrap();
        assert_ne!(first.captured, second.captured);
        assert_eq!(first.projected, second.projected);
        c.write(
            "genesis/orchestrator/gate-task.mjs",
            b"export const a = 3;\n",
        );
        assert_ne!(
            first.projected,
            c.project(&wide, &narrow).unwrap().projected
        );
    }

    #[test]
    fn two_independent_checkouts_with_identical_content_give_the_same_root() {
        let a = sample();
        let b = sample();
        // Different mtimes and different stores: neither enters the address.
        std::thread::sleep(std::time::Duration::from_millis(20));
        b.write("package.json", b"{\"name\":\"x\"}\n");
        let ra = a.project(&FILES, &View::whole()).unwrap();
        let rb = b.project(&FILES, &View::whole()).unwrap();
        assert_ne!(a.root, b.root);
        assert_eq!(ra, rb);
        // The executable bit is content.
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            b.root.join("package.json"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        assert_ne!(
            ra.projected,
            b.project(&FILES, &View::whole()).unwrap().projected
        );
    }

    #[test]
    fn a_path_escaping_the_root_or_a_directory_is_refused() {
        let c = sample();
        fs::write(c.root.parent().unwrap().join("outside"), b"secret").unwrap();
        for bad in [
            "../outside",
            "genesis/../../outside",
            "/etc/passwd",
            "./package.json",
            "",
        ] {
            let error = c.project(&[bad], &View::whole()).unwrap_err();
            assert!(
                error.contains("escapes") || error.contains("absolute") || error.contains("empty"),
                "{bad}: {error}"
            );
            assert!(!c.store.exists(), "{bad}: refused before the store exists");
        }
        let error = c
            .project(&["genesis/orchestrator"], &View::whole())
            .unwrap_err();
        assert!(error.contains("is a directory"), "{error}");
        assert!(!c.store.exists());
        // An ancestor that is a symlink is refused before anything is stat'ed through it.
        std::os::unix::fs::symlink("..", c.root.join("up")).unwrap();
        let error = c.project(&["up/outside"], &View::whole()).unwrap_err();
        assert!(error.contains("passes through a symlink"), "{error}");
        assert!(!c.store.exists());
        // A symlink is captured as its text, never followed out of the checkout.
        std::os::unix::fs::symlink("../outside", c.root.join("link")).unwrap();
        let linked = c.project(&["link"], &View::whole()).unwrap();
        assert_eq!(linked.files, 1);
        assert!(c.project(&[], &View::whole()).is_err());
    }
}
