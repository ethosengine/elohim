use super::linux::{capture, prepare, Phase};
use super::*;
use eprfs_core::{load_verified_tree, BlobCid, EprfsStorage, FetchPolicy, TreeEntryKind};
use eprfs_storage::DirectoryStorage;
use std::{
    ffi::OsStr,
    os::unix::{
        ffi::OsStrExt,
        fs::{symlink, PermissionsExt},
    },
};

struct Fixture {
    _temp: tempfile::TempDir,
    source: PathBuf,
    store: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        std::fs::create_dir(&source).unwrap();
        let store = temp.path().join("store");
        Self {
            _temp: temp,
            source,
            store,
        }
    }

    fn write(&self, name: impl AsRef<Path>, bytes: &[u8]) {
        let path = self.source.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    async fn capture(&self, paths: &[&str]) -> Result<CaptureReport, CaptureError> {
        capture_tree(
            &self.source,
            &self.store,
            &paths.iter().map(PathBuf::from).collect::<Vec<_>>(),
            &TreeLimits::default(),
        )
        .await
    }

    fn no_store(&self) {
        assert!(!self.store.exists(), "refusal must precede store creation");
    }
}

#[tokio::test]
async fn selection_refusals_leave_store_absent() {
    let f = Fixture::new();
    f.write("good", b"good");
    f.write("dir/file", b"file");
    for bad in [
        "",
        ".",
        "..",
        "../good",
        "/good",
        "dir/../good",
        "./good",
        "dir/./file",
        "dir//file",
        "good/",
        "*.rs",
        "g?od",
        "[good]",
        "{a,b}",
        "x\0y",
        ".git/config",
        "dir/.git/config",
        ".eprfs/object",
        ".brit/index",
        "dir",
        "missing",
    ] {
        assert!(f.capture(&["good", bad]).await.is_err(), "accepted {bad:?}");
        f.no_store();
    }
    for paths in [vec![], vec!["good", "good"], vec!["dir", "dir/file"]] {
        assert!(f.capture(&paths).await.is_err());
        f.no_store();
    }
    assert_eq!(std::fs::read(f.source.join("good")).unwrap(), b"good");
}

#[tokio::test]
async fn refuses_special_files_and_symlink_ancestors_without_following_targets() {
    let f = Fixture::new();
    f.write("good", b"good");
    f.write("real/file", b"private");
    symlink("real", f.source.join("alias")).unwrap();
    let fifo = f.source.join("fifo");
    rustix::fs::mknodat(
        rustix::fs::CWD,
        &fifo,
        rustix::fs::FileType::Fifo,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
        0,
    )
    .unwrap();
    for bad in ["alias/file", "fifo"] {
        assert!(f.capture(&["good", bad]).await.is_err());
        f.no_store();
    }
    // A symlink itself is data, even if absolute, dangling, looping or escaping.
    for (name, target) in [
        ("absolute", "/etc/passwd"),
        ("dangling", "missing"),
        ("loop", "loop"),
        ("escape", "../../outside"),
    ] {
        symlink(target, f.source.join(name)).unwrap();
    }
    let report = f
        .capture(&["absolute", "dangling", "loop", "escape"])
        .await
        .unwrap();
    assert_eq!(report.symlinks, 4);
    let tree = load_verified_tree(
        &DirectoryStorage::open(&f.store).unwrap(),
        &report.root,
        FetchPolicy::LocalOnly,
        &TreeLimits::default(),
    )
    .await
    .unwrap();
    assert_eq!(tree.leaves.len(), 4);
    assert_eq!(
        tree.leaf(&BlobCid::compute_raw(b"/etc/passwd"))
            .unwrap()
            .as_ref(),
        b"/etc/passwd"
    );
}

#[tokio::test]
async fn source_store_overlap_and_symlinked_root_chains_refuse() {
    let f = Fixture::new();
    f.write("good", b"good");
    for store in [
        f.source.clone(),
        f.source.join("objects"),
        f.source.parent().unwrap().into(),
    ] {
        assert!(
            capture_tree(&f.source, &store, &["good".into()], &TreeLimits::default())
                .await
                .is_err()
        );
    }
    let alias = f.source.parent().unwrap().join("alias");
    symlink(&f.source, &alias).unwrap();
    assert!(
        capture_tree(&alias, &f.store, &["good".into()], &TreeLimits::default())
            .await
            .is_err()
    );
    assert!(capture_tree(
        &f.source,
        alias.join("objects"),
        &["good".into()],
        &TreeLimits::default()
    )
    .await
    .is_err());
    let external = f.source.parent().unwrap().join("external");
    std::fs::create_dir(&external).unwrap();
    symlink(&external, &f.store).unwrap();
    assert!(f.capture(&["good"]).await.is_err());
    assert_eq!(std::fs::read_dir(&external).unwrap().count(), 0);
    assert!(!f.source.join("objects").exists());
}

#[tokio::test]
async fn representation_is_canonical_and_hardlink_names_remain_distinct() {
    let f = Fixture::new();
    f.write("nested/run", b"\0binary\xff");
    std::fs::set_permissions(
        f.source.join("nested/run"),
        std::fs::Permissions::from_mode(0o751),
    )
    .unwrap();
    std::fs::hard_link(
        f.source.join("nested/run"),
        f.source.join("nested/hardlink"),
    )
    .unwrap();
    let raw_name = PathBuf::from(OsStr::from_bytes(b"nested/raw-\xff"));
    f.write(&raw_name, b"hello world");
    symlink(
        OsStr::from_bytes(b"../literal-\xff"),
        f.source.join("nested/link"),
    )
    .unwrap();
    let paths = vec![
        "nested/run".into(),
        raw_name.clone(),
        "nested/link".into(),
        "nested/hardlink".into(),
    ];
    let a = capture_tree(&f.source, &f.store, &paths, &TreeLimits::default())
        .await
        .unwrap();
    let mut reverse = paths;
    reverse.reverse();
    let b = capture_tree(&f.source, &f.store, &reverse, &TreeLimits::default())
        .await
        .unwrap();
    assert_eq!(a, b);
    assert_eq!((a.files, a.symlinks, a.directories), (3, 1, 1));
    assert_eq!(
        a.bytes_captured,
        2 * b"\0binary\xff".len() + b"hello world".len() + b"../literal-\xff".len()
    );
    let tree = load_verified_tree(
        &DirectoryStorage::open(&f.store).unwrap(),
        &a.root,
        FetchPolicy::LocalOnly,
        &TreeLimits::default(),
    )
    .await
    .unwrap();
    let child = match &tree.root_node().entries[0].kind {
        TreeEntryKind::Directory { tree: cid } => &tree.nodes[cid.as_blob_cid()],
        other => panic!("expected directory, got {other:?}"),
    };
    assert_eq!(child.entries.len(), 4);
    for name in [b"run".as_slice(), b"hardlink"] {
        let entry = child
            .entries
            .iter()
            .find(|entry| entry.name == name)
            .unwrap();
        assert!(
            matches!(&entry.kind, TreeEntryKind::File { executable: true, blob }
            if blob.as_blob_cid() == &BlobCid::compute_raw(b"\0binary\xff"))
        );
    }
    // Independent frozen Tree v1 vector (also shared by core's codec tests).
    let g = Fixture::new();
    let name = PathBuf::from(OsStr::from_bytes(b"\xff"));
    g.write(&name, b"hello world");
    let captured = capture_tree(&g.source, &g.store, &[name], &TreeLimits::default())
        .await
        .unwrap();
    assert_eq!(
        captured.root.to_string(),
        "bafyreihedxqhkuhibpz2uhoknm5ov2p4ubsjr5csvg4tdajfdapsyotrou"
    );
}

#[tokio::test]
async fn observed_open_read_and_ancestor_races_refuse_without_store_mutation() {
    for phase in [
        Phase::BeforeOpen,
        Phase::Opened,
        Phase::Reading,
        Phase::Read,
        Phase::Recheck,
    ] {
        let f = Fixture::new();
        f.write("dir/file", b"aaaa");
        let mut ran = false;
        let result = capture(
            &f.source,
            &f.store,
            &["dir/file".into()],
            &TreeLimits::default(),
            |at, _| {
                if at == phase && !ran {
                    ran = true;
                    std::fs::write(f.source.join("dir/file"), b"changed size").unwrap();
                }
            },
        )
        .await;
        assert!(ran);
        assert!(
            matches!(result, Err(CaptureError::Changed(_))),
            "{phase:?}: {result:?}"
        );
        f.no_store();
    }
    // Replace an ancestor with an escaping symlink after initial inspection.
    let f = Fixture::new();
    f.write("dir/file", b"inside");
    let outside = f.source.parent().unwrap().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("file"), b"outside").unwrap();
    let result = capture(
        &f.source,
        &f.store,
        &["dir/file".into()],
        &TreeLimits::default(),
        |at, _| {
            if at == Phase::BeforeOpen {
                std::fs::rename(f.source.join("dir"), f.source.join("moved")).unwrap();
                symlink(&outside, f.source.join("dir")).unwrap();
            }
        },
    )
    .await;
    assert!(result.is_err());
    f.no_store();
    assert_eq!(std::fs::read(outside.join("file")).unwrap(), b"outside");
}

#[tokio::test]
async fn leaf_replacement_and_symlink_retarget_are_observed() {
    for phase in [Phase::BeforeOpen, Phase::Opened, Phase::Read] {
        let f = Fixture::new();
        f.write("file", b"inside");
        let result = capture(
            &f.source,
            &f.store,
            &["file".into()],
            &TreeLimits::default(),
            |at, _| {
                if at == phase {
                    std::fs::rename(f.source.join("file"), f.source.join("old")).unwrap();
                    symlink("/etc/passwd", f.source.join("file")).unwrap();
                }
            },
        )
        .await;
        assert!(result.is_err());
        f.no_store();
    }
    let f = Fixture::new();
    symlink("old", f.source.join("link")).unwrap();
    let result = capture(
        &f.source,
        &f.store,
        &["link".into()],
        &TreeLimits::default(),
        |at, _| {
            if at == Phase::Opened {
                std::fs::remove_file(f.source.join("link")).unwrap();
                symlink("new", f.source.join("link")).unwrap();
            }
        },
    )
    .await;
    assert!(result.is_err());
    f.no_store();
}

#[tokio::test]
async fn same_size_rewrite_with_restored_mtime_and_mode_change_are_observed() {
    for change_mode in [false, true] {
        let f = Fixture::new();
        f.write("file", b"aaaa");
        let file = f.source.join("file");
        let modified = std::fs::metadata(&file).unwrap().modified().unwrap();
        let result = capture(
            &f.source,
            &f.store,
            &["file".into()],
            &TreeLimits::default(),
            |at, _| {
                if at == Phase::Read {
                    if change_mode {
                        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755))
                            .unwrap();
                    } else {
                        std::fs::write(&file, b"bbbb").unwrap();
                        std::fs::File::open(&file)
                            .unwrap()
                            .set_times(std::fs::FileTimes::new().set_modified(modified))
                            .unwrap();
                    }
                }
            },
        )
        .await;
        assert!(matches!(result, Err(CaptureError::Changed(_))));
        f.no_store();
    }
}

#[tokio::test]
async fn earlier_file_change_while_reading_later_file_is_observed() {
    let f = Fixture::new();
    f.write("a", b"aaaa");
    f.write("z", b"zzzz");
    let result = capture(
        &f.source,
        &f.store,
        &["a".into(), "z".into()],
        &TreeLimits::default(),
        |at, name| {
            if at == Phase::Read && name == Path::new("z") {
                std::fs::write(f.source.join("a"), b"bbbb").unwrap();
            }
        },
    )
    .await;
    assert!(matches!(result, Err(CaptureError::Changed(_))));
    f.no_store();
}

#[test]
fn concurrent_captures_keep_identical_roots() {
    let f = Fixture::new();
    f.write("file", b"concurrent");
    let barrier = std::sync::Barrier::new(8);
    let roots = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| {
                scope.spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();
                    rt.block_on(capture(
                        &f.source,
                        &f.store,
                        &["file".into()],
                        &TreeLimits::default(),
                        |phase, _| {
                            if phase == Phase::Recheck {
                                barrier.wait();
                            }
                        },
                    ))
                    .unwrap()
                    .root
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert!(roots.iter().all(|r| r == &roots[0]));
    assert_eq!(std::fs::read_dir(&f.store).unwrap().count(), 2);
}

#[tokio::test]
async fn object_and_expanded_closure_limits_refuse_before_writes() {
    let f = Fixture::new();
    let file = std::fs::File::create(f.source.join("large")).unwrap();
    file.set_len(MAX_CAPTURE_OBJECT_BYTES as u64 + 1).unwrap();
    assert!(matches!(
        f.capture(&["large"]).await,
        Err(CaptureError::Budget("object bytes"))
    ));
    f.no_store();
    file.set_len(MAX_CAPTURE_OBJECT_BYTES as u64).unwrap();
    let mut paths = vec!["large".into()];
    // Charge distinct hardlink names, not unique inodes or deduplicated objects.
    for i in 0..16 {
        let name = format!("alias-{i}");
        std::fs::hard_link(f.source.join("large"), f.source.join(&name)).unwrap();
        paths.push(PathBuf::from(name));
    }
    let result = capture_tree(
        &f.source,
        &f.store,
        &paths,
        &TreeLimits {
            max_total_bytes: usize::MAX,
            ..TreeLimits::default()
        },
    )
    .await;
    assert!(matches!(result, Err(CaptureError::Budget("closure bytes"))));
    f.no_store();
    let g = Fixture::new();
    g.write("a", b"1234");
    g.write("b", b"1234");
    assert!(capture_tree(
        &g.source,
        &g.store,
        &["a".into(), "b".into()],
        &TreeLimits {
            max_total_bytes: 7,
            ..TreeLimits::default()
        }
    )
    .await
    .is_err());
    g.no_store();
}

#[tokio::test]
async fn tree_shape_limits_are_checked_before_any_storage_mutation() {
    let f = Fixture::new();
    f.write("deep/a/file", b"file");
    f.write("deep/b/file", b"file");
    let paths = ["deep/a/file".into(), "deep/b/file".into()];
    for limits in [
        TreeLimits {
            max_depth: 1,
            ..TreeLimits::default()
        },
        TreeLimits {
            max_total_entries: 3,
            ..TreeLimits::default()
        },
        TreeLimits {
            max_path_bytes: 2,
            ..TreeLimits::default()
        },
        TreeLimits {
            max_entries_per_node: 1,
            ..TreeLimits::default()
        },
        TreeLimits {
            max_node_bytes: 1,
            ..TreeLimits::default()
        },
        TreeLimits {
            max_pending_entries: 1,
            ..TreeLimits::default()
        },
        TreeLimits {
            max_total_bytes: 30,
            ..TreeLimits::default()
        },
    ] {
        assert!(
            capture_tree(&f.source, &f.store, &paths, &limits)
                .await
                .is_err(),
            "{limits:?}"
        );
        f.no_store();
    }
}

#[tokio::test]
async fn refusal_preserves_existing_store_and_corruption_is_not_a_miss() {
    let f = Fixture::new();
    f.write("file", b"captured");
    let a = f.capture(&["file"]).await.unwrap();
    let before: Vec<_> = std::fs::read_dir(&f.store)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert!(f.capture(&["file", "../escape"]).await.is_err());
    let after: Vec<_> = std::fs::read_dir(&f.store)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(before, after);
    let leaf = f.store.join(BlobCid::compute_raw(b"captured").to_string());
    std::fs::write(&leaf, b"corrupt").unwrap();
    assert!(f.capture(&["file"]).await.is_err());
    assert_eq!(std::fs::read(&leaf).unwrap(), b"corrupt");
    assert!(f.store.join(a.root.to_string()).exists());
    assert_eq!(std::fs::read(f.source.join("file")).unwrap(), b"captured");
}

#[tokio::test]
async fn capture_does_not_execute_metadata_hooks() {
    let f = Fixture::new();
    f.write("file", b"data");
    let marker = f.source.parent().unwrap().join("executed");
    let malicious = format!("#!/bin/sh\ntouch {}\n", marker.display());
    f.write(".git/hooks/post-checkout", malicious.as_bytes());
    f.write(".eprfs/hooks/capture", malicious.as_bytes());
    for name in [".git/hooks/post-checkout", ".eprfs/hooks/capture"] {
        std::fs::set_permissions(f.source.join(name), std::fs::Permissions::from_mode(0o755))
            .unwrap();
    }
    f.capture(&["file"]).await.unwrap();
    assert!(!marker.exists());
}

#[tokio::test]
async fn prepared_write_propagates_failure_and_cancellation_without_touching_prior_objects() {
    use bytes::Bytes;
    use eprfs_core::{AttestationDraft, BlobHandle, BlobPresence, EprRecord, EprRef, EprfsError};
    use eprfs_storage::MemoryStorage;
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct FailSecond {
        store: MemoryStorage,
        puts: AtomicUsize,
        stall: bool,
    }
    #[async_trait::async_trait]
    impl EprfsStorage for FailSecond {
        async fn resolve_epr(&self, r: &EprRef) -> eprfs_core::Result<EprRecord> {
            self.store.resolve_epr(r).await
        }
        async fn has_blob(&self, c: &BlobCid) -> eprfs_core::Result<BlobPresence> {
            self.store.has_blob(c).await
        }
        async fn fetch_blob(&self, c: &BlobCid, p: FetchPolicy) -> eprfs_core::Result<BlobHandle> {
            self.store.fetch_blob(c, p).await
        }
        async fn put_blob(&self, _: Bytes) -> eprfs_core::Result<BlobCid> {
            panic!("legacy put_blob must never be used")
        }
        async fn put_blob_verified(&self, c: &BlobCid, b: Bytes) -> eprfs_core::Result<()> {
            if self.puts.fetch_add(1, Ordering::SeqCst) == 1 {
                if self.stall {
                    std::future::pending::<()>().await;
                }
                return Err(EprfsError::Storage("injected write failure".into()));
            }
            self.store.put_blob_verified(c, b).await
        }
        async fn publish_attestation(&self, _: AttestationDraft) -> eprfs_core::Result<EprRef> {
            panic!("capture cannot publish")
        }
    }
    let f = Fixture::new();
    f.write("a", b"first");
    f.write("b", b"second");
    let (prepared, _) = prepare(
        &f.source,
        &f.store,
        &["a".into(), "b".into()],
        &TreeLimits::default(),
        |_, _| {},
    )
    .unwrap();
    let store = FailSecond {
        store: MemoryStorage::default(),
        puts: AtomicUsize::new(0),
        stall: false,
    };
    let prior = BlobCid::compute_raw(b"prior");
    store
        .store
        .put_blob_verified(&prior, Bytes::from_static(b"prior"))
        .await
        .unwrap();
    assert!(prepared
        .write(&store)
        .await
        .unwrap_err()
        .to_string()
        .contains("injected write failure"));
    assert_eq!(
        store
            .fetch_blob(&prior, FetchPolicy::LocalOnly)
            .await
            .unwrap()
            .bytes
            .as_ref(),
        b"prior"
    );
    assert_eq!(
        store.has_blob(&prepared.report.root).await.unwrap(),
        BlobPresence::Missing
    );
    assert_eq!(
        store
            .has_blob(&BlobCid::compute_raw(b"first"))
            .await
            .unwrap(),
        BlobPresence::Local
    );
    f.no_store();
    let interrupted = FailSecond {
        store: MemoryStorage::default(),
        puts: AtomicUsize::new(0),
        stall: true,
    };
    interrupted
        .store
        .put_blob_verified(&prior, Bytes::from_static(b"prior"))
        .await
        .unwrap();
    let mut pending = Box::pin(prepared.write(&interrupted));
    use std::future::Future;
    let state = std::future::poll_fn(|cx| std::task::Poll::Ready(pending.as_mut().poll(cx))).await;
    assert!(state.is_pending());
    drop(pending);
    assert_eq!(
        interrupted.has_blob(&prepared.report.root).await.unwrap(),
        BlobPresence::Missing
    );
    assert_eq!(
        interrupted
            .fetch_blob(&prior, FetchPolicy::LocalOnly)
            .await
            .unwrap()
            .bytes
            .as_ref(),
        b"prior"
    );
    assert_eq!(
        interrupted
            .has_blob(&BlobCid::compute_raw(b"first"))
            .await
            .unwrap(),
        BlobPresence::Local
    );
    assert_eq!(std::fs::read(f.source.join("a")).unwrap(), b"first");
}

#[test]
fn exactly_one_object_limit_is_accepted_by_bounded_source_reads() {
    let f = Fixture::new();
    let file = std::fs::File::create(f.source.join("large")).unwrap();
    file.set_len(MAX_CAPTURE_OBJECT_BYTES as u64).unwrap();
    let (prepared, recheck) = prepare(
        &f.source,
        &f.store,
        &["large".into()],
        &TreeLimits::default(),
        |_, _| {},
    )
    .unwrap();
    recheck().unwrap();
    assert_eq!(prepared.report.bytes_captured, 64 * 1024 * 1024);
    assert_eq!(prepared.report.files, 1);
    f.no_store();
}

#[tokio::test]
async fn mutation_between_read_chunks_and_source_root_replacement_refuse() {
    let f = Fixture::new();
    f.write("file", &vec![b'a'; 128 * 1024]);
    let mut changed = false;
    let result = capture(
        &f.source,
        &f.store,
        &["file".into()],
        &TreeLimits::default(),
        |phase, _| {
            if phase == Phase::Reading && !changed {
                changed = true;
                std::fs::write(f.source.join("file"), vec![b'b'; 128 * 1024]).unwrap();
            }
        },
    )
    .await;
    assert!(changed);
    assert!(matches!(result, Err(CaptureError::Changed(_))));
    f.no_store();
    let moved = f.source.parent().unwrap().join("moved");
    let result = capture(
        &f.source,
        &f.store,
        &["file".into()],
        &TreeLimits::default(),
        |phase, _| {
            if phase == Phase::Recheck {
                std::fs::rename(&f.source, &moved).unwrap();
                std::fs::create_dir(&f.source).unwrap();
                std::fs::write(f.source.join("file"), b"replacement").unwrap();
            }
        },
    )
    .await;
    assert!(result.is_err());
    f.no_store();
}

#[test]
fn relative_roots_remain_bound_when_process_cwd_changes() {
    const CHILD: &str = "EPRFS_CAPTURE_CWD_TEST_CHILD";
    if std::env::var_os(CHILD).is_none() {
        // cwd is process-global. Isolate its mutation from the test harness's
        // parallel tests, including the host's default multi-thread test run.
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "capture::tests::relative_roots_remain_bound_when_process_cwd_changes",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    let f = Fixture::new();
    f.write("file", b"selected");
    let other = f.source.parent().unwrap().join("other");
    std::fs::create_dir(&other).unwrap();
    let original = std::env::current_dir().unwrap();
    std::env::set_current_dir(f.source.parent().unwrap()).unwrap();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let report = rt
        .block_on(capture(
            Path::new("source"),
            Path::new("store"),
            &["file".into()],
            &TreeLimits::default(),
            |phase, _| {
                if phase == Phase::Recheck {
                    std::env::set_current_dir(&other).unwrap();
                }
            },
        ))
        .unwrap();
    std::env::set_current_dir(original).unwrap();
    assert!(f.store.join(report.root.to_string()).exists());
    assert!(!other.join("store").exists());
}
