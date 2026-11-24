//! One payload-store root opened by several processes (artifact-store E4, ruling R4).
//!
//! The races that matter cross a process boundary, so these tests spawn real processes: this
//! test binary re-executes itself (`child_process_role`, selected by [`ROLE`]) and the child
//! plays one role against the shared root. Run in the ordinary way, `child_process_role` sees
//! no role and returns at once.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use elohim_blob_store::{BlobStore, BlobStoreError, Exists, OpenMode};
use elohim_epr::address::{compute_bytes_cid, ByteCid};

/// The environment variable naming the child's role.
const ROLE: &str = "ELOHIM_BLOB_STORE_CHILD_ROLE";
/// The root the child opens.
const ROOT: &str = "ELOHIM_BLOB_STORE_CHILD_ROOT";
/// A role's parameter (a seed, a count, a path).
const ARG: &str = "ELOHIM_BLOB_STORE_CHILD_ARG";

/// A deterministic byte stream: xorshift64* from `seed`.
fn bytes(seed: u64, len: usize) -> Vec<u8> {
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut out = Vec::with_capacity(len + 8);
    while out.len() < len {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        out.extend_from_slice(&state.wrapping_mul(0x2545_F491_4F6C_DD1D).to_le_bytes());
    }
    out.truncate(len);
    out
}

/// A payload that cuts into several chunks.
fn chunked(seed: u64) -> Vec<u8> {
    bytes(seed, 6 * 1024 * 1024)
}

/// `chunked(base)` followed by a tail of its own: its leading chunks are `chunked(base)`'s.
fn extended(base: u64, tail: u64) -> Vec<u8> {
    let mut data = chunked(base);
    data.extend_from_slice(&bytes(tail, 2 * 1024 * 1024));
    data
}

fn private_root() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    dir
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
}

/// Spawn this test binary as a child playing `role` against `root`.
fn spawn(role: &str, root: &Path, arg: &str) -> Child {
    Command::new(std::env::current_exe().unwrap())
        .args([
            "child_process_role",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(ROLE, role)
        .env(ROOT, root)
        .env(ARG, arg)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap()
}

fn succeeded(child: Child, role: &str) {
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "child {role} failed: {:?}",
        output.status
    );
}

/// Wait (bounded) for `path` to exist.
fn wait_for(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(60);
    while !path.exists() {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {}",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Open the owner, retrying while another instance (a finishing child) still holds the lock.
async fn open_owner_eventually(root: &Path) -> BlobStore {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        match BlobStore::open_owner(root).await {
            Ok(store) => return store,
            Err(BlobStoreError::OwnerHeld { .. }) if Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            Err(error) => panic!("owner open: {error}"),
        }
    }
}

/// Every file under `root` (relative path → bytes), lock files aside.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    std::fs::read(&path).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

/// How many files under `root/<area>` (two fan-out levels) are named `name`.
fn copies(root: &Path, area: &str, name: &str) -> usize {
    snapshot(root)
        .keys()
        .filter(|path| {
            path.starts_with(area) && path.file_name().and_then(|n| n.to_str()) == Some(name)
        })
        .count()
}

fn files_under(root: &Path, area: &str) -> usize {
    snapshot(root)
        .keys()
        .filter(|path| path.starts_with(area))
        .count()
}

/// The child half of every multi-process test. A no-op unless [`ROLE`] is set.
#[test]
fn child_process_role() {
    let Ok(role) = std::env::var(ROLE) else {
        return;
    };
    let root = PathBuf::from(std::env::var(ROOT).unwrap());
    let arg = std::env::var(ARG).unwrap_or_default();
    runtime().block_on(async move {
        match role.as_str() {
            // Store one payload (seed `arg`) through a shared instance and read it back.
            "store-shared" => {
                let (seed, inline) = match arg.split_once(':') {
                    Some((seed, "inline")) => (seed.parse().unwrap(), true),
                    _ => (arg.parse().unwrap(), false),
                };
                let data = if inline {
                    bytes(seed, 512)
                } else {
                    chunked(seed)
                };
                let store = BlobStore::open_shared(&root).await.unwrap();
                let stored = store.store(&data).await.unwrap();
                assert_eq!(&store.get(&stored.cid).await.unwrap()[..], &data[..]);
            }
            // Store `extended(1, 100 + i)` for i in 0..arg through a shared instance, each
            // read back whole.
            "store-extended-loop" => {
                let store = BlobStore::open_shared(&root).await.unwrap();
                for i in 0..arg.parse::<u64>().unwrap() {
                    let data = extended(1, 100 + i);
                    let stored = store.store(&data).await.unwrap();
                    assert_eq!(&store.get(&stored.cid).await.unwrap()[..], &data[..]);
                }
            }
            // Open the owner, then hold it until `<arg>.release` appears.
            "hold-owner" => {
                let store = BlobStore::open_owner(&root).await.unwrap();
                std::fs::write(format!("{arg}.ready"), b"").unwrap();
                wait_for(Path::new(&format!("{arg}.release")));
                drop(store);
            }
            // Stream `extended(1, 2)` through a shared instance, pausing after its prefix (the
            // chunks `extended(1, 3)` shares) until `<arg>.go` appears; then read it back.
            "stream-paused" => {
                let store = BlobStore::open_shared(&root).await.unwrap();
                let (mut tx, rx) = tokio::io::duplex(64 * 1024);
                let signal = arg.clone();
                let feeder = tokio::spawn(async move {
                    use tokio::io::AsyncWriteExt;
                    tx.write_all(&chunked(1)).await.unwrap();
                    std::fs::write(format!("{signal}.ready"), b"").unwrap();
                    let go = PathBuf::from(format!("{signal}.go"));
                    while !go.exists() {
                        tokio::time::sleep(Duration::from_millis(20)).await;
                    }
                    tx.write_all(&bytes(2, 2 * 1024 * 1024)).await.unwrap();
                });
                let stored = store.store_stream(rx).await.unwrap();
                feeder.await.unwrap();
                let data = extended(1, 2);
                assert_eq!(stored.cid, compute_bytes_cid(&data));
                assert_eq!(&store.get(&stored.cid).await.unwrap()[..], &data[..]);
            }
            // Open the owner (its startup sweep runs under the exclusive lock) and exit.
            "open-owner-once" => {
                let store = open_owner_eventually(&root).await;
                assert_eq!(store.open_mode(), Some(OpenMode::Owner));
            }
            other => panic!("unknown child role {other}"),
        }
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_instances_storing_one_chunk_converge_to_one_verified_copy() {
    // Real processes: three children store the same chunked payload, and three the same
    // inline (single-chunk) payload, each through its own shared instance, all at once; two
    // in-process shared instances race them.
    let dir = private_root();
    let root = dir.path();
    let children: Vec<(Child, &str)> = (0..3)
        .flat_map(|_| {
            [
                (spawn("store-shared", root, "7"), "store-shared 7"),
                (
                    spawn("store-shared", root, "8:inline"),
                    "store-shared 8:inline",
                ),
            ]
        })
        .collect();
    let data = chunked(7);
    let small = bytes(8, 512);
    let (a, b) = (
        BlobStore::open_shared(root).await.unwrap(),
        BlobStore::open_shared(root).await.unwrap(),
    );
    let (ra, rb) = tokio::join!(a.store(&data), b.store(&data));
    let (ra, rb) = (ra.unwrap(), rb.unwrap());
    assert_eq!(ra.cid, rb.cid);
    assert_eq!(ra.chunks, rb.chunks);
    assert!(ra.chunks.len() > 1, "fixture must be chunked");
    a.store(&small).await.unwrap();
    for (child, role) in children {
        tokio::task::spawn_blocking(move || succeeded(child, role))
            .await
            .unwrap();
    }

    let owner = BlobStore::open_owner(root).await.unwrap();
    let cid = compute_bytes_cid(&data);
    let small_cid = compute_bytes_cid(&small);
    assert_eq!(owner.exists(&cid).await, Exists::Verified);
    assert_eq!(&owner.get(&cid).await.unwrap()[..], &data[..]);
    assert_eq!(owner.exists(&small_cid).await, Exists::Verified);
    // One copy of each: a marker per payload under blobs/, one file per distinct chunk under
    // chunks/ — the single-chunk payload's bytes included (one canonical location per CID).
    assert_eq!(copies(root, "blobs", &cid.to_string()), 1);
    assert_eq!(copies(root, "blobs", &small_cid.to_string()), 1);
    assert_eq!(copies(root, "chunks", &small_cid.to_string()), 1);
    let distinct: std::collections::HashSet<ByteCid> = ra.chunks.iter().map(|c| c.cid).collect();
    for chunk in &distinct {
        assert_eq!(
            copies(root, "chunks", &chunk.to_string()),
            1,
            "chunk {chunk}"
        );
    }
    assert_eq!(files_under(root, "chunks"), distinct.len() + 1);
    assert_eq!(files_under(root, "blobs"), 2);
    assert_eq!(files_under(root, "index"), 2, "a layout record per payload");
    assert_eq!(files_under(root, "tmp"), 0, "no staging residue");
    assert_eq!(files_under(root, "quarantine"), 0);
}

#[tokio::test]
async fn a_shared_instance_never_moves_a_file() {
    let dir = private_root();
    let root = dir.path();
    let data = chunked(21);
    let small = bytes(22, 300);
    let (held, inline) = {
        let owner = BlobStore::open_owner(root).await.unwrap();
        (
            owner.store(&data).await.unwrap(),
            owner.store(&small).await.unwrap(),
        )
    };
    // Same-length corruption of one chunk and of the inline payload.
    let victim = held.chunks[1].cid;
    for (area, cid) in [("chunks", victim), ("blobs", inline.cid)] {
        let path = elohim_blob_store::fanout_path(&root.join(area), &cid.to_string());
        let mut bytes = std::fs::read(&path).unwrap();
        bytes[3] ^= 0x01;
        std::fs::write(&path, bytes).unwrap();
    }
    let before = snapshot(root);

    let shared = BlobStore::open_shared(root).await.unwrap();
    assert!(matches!(
        shared.get(&held.cid).await,
        Err(BlobStoreError::HashMismatch { .. })
    ));
    assert!(matches!(
        shared.get_range(&held.cid, 0, 10).await,
        Err(BlobStoreError::HashMismatch { .. })
    ));
    assert!(matches!(
        shared.get_chunk(&victim).await,
        Err(BlobStoreError::HashMismatch { .. })
    ));
    assert!(matches!(
        shared.get(&inline.cid).await,
        Err(BlobStoreError::HashMismatch { .. })
    ));
    assert_eq!(shared.exists(&held.cid).await, Exists::Corrupt);
    assert_eq!(shared.exists(&inline.cid).await, Exists::Corrupt);
    assert!(!shared.holds_verified(&held.cid));
    assert!(!shared.list_units().unwrap().contains(&victim));
    // A store over a held copy that fails verification overwrites nothing.
    assert!(matches!(
        shared.store(&data).await,
        Err(BlobStoreError::HashMismatch { .. })
    ));
    assert!(matches!(
        shared.store(&small).await,
        Err(BlobStoreError::HashMismatch { .. })
    ));
    // The destructive operations are the owner's.
    for refused in [
        shared.delete(&held.cid).await,
        shared.quarantine(&held.cid, "asked").await,
        shared.scrub(),
    ] {
        assert!(
            matches!(refused, Err(BlobStoreError::SharedInstance { .. })),
            "{refused:?}"
        );
    }
    assert_eq!(
        snapshot(root),
        before,
        "a shared instance moved or rewrote a file"
    );

    // The owner condemns what the shared instance only refused, and heals it.
    let owner = BlobStore::open_owner(root).await.unwrap();
    assert_eq!(owner.exists(&held.cid).await, Exists::Quarantined);
    assert!(!owner.store(&data).await.unwrap().already_existed);
    assert_eq!(&shared.get(&held.cid).await.unwrap()[..], &data[..]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_second_owner_is_refused_while_the_first_holds_the_lock() {
    let dir = private_root();
    let root = dir.path().join("store");
    let signal = dir.path().join("signal");
    let signal_text = signal.to_string_lossy().into_owned();
    // The first owner lives in another process.
    let child = spawn("hold-owner", &root, &signal_text);
    let ready = PathBuf::from(format!("{signal_text}.ready"));
    tokio::task::spawn_blocking(move || wait_for(&ready))
        .await
        .unwrap();
    match BlobStore::open_owner(&root).await {
        Err(BlobStoreError::OwnerHeld { root: refused }) => assert_eq!(refused, root),
        other => panic!("a second owner was admitted: {:?}", other.map(|_| ())),
    }
    // A shared instance opens beside the owner.
    let shared = BlobStore::open_shared(&root).await.unwrap();
    assert_eq!(shared.open_mode(), Some(OpenMode::Shared));
    std::fs::write(format!("{signal_text}.release"), b"").unwrap();
    tokio::task::spawn_blocking(move || succeeded(child, "hold-owner"))
        .await
        .unwrap();

    // Released (the process is gone): ownership passes, and is exclusive in-process too. The
    // opens after a release retry briefly: another test in this binary forking a child at that
    // moment shares the just-closed lock description until the child execs (std opens files
    // close-on-exec), which is a few microseconds of `OwnerHeld`.
    let owner = open_owner_eventually(&root).await;
    assert!(matches!(
        BlobStore::open_owner(&root).await,
        Err(BlobStoreError::OwnerHeld { .. })
    ));
    drop(owner);
    open_owner_eventually(&root).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn owner_gc_never_collects_a_chunk_a_shared_writer_is_publishing() {
    // The child streams a payload whose leading chunks the owner's payload already holds, so
    // its publication skips them: they are on disk, named by no layout record but the owner's.
    // Mid-publication (prefix done, marker not yet written) the child pauses, and the owner
    // deletes its own payload, collecting every chunk no layout record names. The delete must
    // wait for the publication; the child's payload must come out whole.
    let dir = private_root();
    let root = dir.path().to_path_buf();
    let signal = dir.path().join("signal").to_string_lossy().into_owned();
    let owner = std::sync::Arc::new(BlobStore::open_owner(&root).await.unwrap());
    let base = extended(1, 3);
    let published = extended(1, 2);
    let held = owner.store(&base).await.unwrap();
    assert!(held.chunks.len() > 2, "fixture must be chunked");

    let writer = spawn("stream-paused", &root, &signal);
    let ready = PathBuf::from(format!("{signal}.ready"));
    tokio::task::spawn_blocking(move || wait_for(&ready))
        .await
        .unwrap();
    let deleter = {
        let owner = owner.clone();
        let cid = held.cid;
        tokio::spawn(async move {
            owner.delete(&cid).await.unwrap();
            Instant::now()
        })
    };
    tokio::time::sleep(Duration::from_millis(500)).await;
    let resumed = Instant::now();
    std::fs::write(format!("{signal}.go"), b"").unwrap();
    tokio::task::spawn_blocking(move || succeeded(writer, "stream-paused"))
        .await
        .unwrap();
    let deleted = deleter.await.unwrap();
    assert!(
        deleted > resumed,
        "the owner collected chunks while a shared publication was in flight"
    );

    let cid = compute_bytes_cid(&published);
    assert_eq!(owner.exists(&cid).await, Exists::Verified);
    assert_eq!(&owner.get(&cid).await.unwrap()[..], &published[..]);
    assert_eq!(owner.exists(&held.cid).await, Exists::Absent);
    assert_eq!(files_under(&root, "quarantine"), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_owner_opening_never_sweeps_a_shared_writers_staging() {
    // Owners open (and sweep `tmp/`) over and over in other processes while a shared writer
    // stages chunk after chunk: every one of its renames must find its staging file.
    let dir = private_root();
    let root = dir.path().to_path_buf();
    let rounds = 10u64;
    let mut writer = spawn("store-extended-loop", &root, &rounds.to_string());
    let mut opens = 0;
    loop {
        let opener = spawn("open-owner-once", &root, "");
        tokio::task::spawn_blocking(move || succeeded(opener, "open-owner-once"))
            .await
            .unwrap();
        opens += 1;
        if writer.try_wait().unwrap().is_some() {
            break;
        }
    }
    succeeded(writer, "store-extended-loop");
    assert!(opens > 1, "the owners' sweeps raced the writer");
    let owner = open_owner_eventually(&root).await;
    for i in 0..rounds {
        let cid = compute_bytes_cid(&extended(1, 100 + i));
        assert_eq!(
            owner.exists(&cid).await,
            Exists::Verified,
            "publication {i}"
        );
    }
}

/// Who holds `lock` open now, for a refusal's message: every process with a descriptor on it
/// (from `/proc/*/fd`, with its state and command line) and the kernel's lock rows for its
/// inode. A child forked by another test and not yet exec'd shows here with this binary's
/// command line.
#[cfg(target_os = "linux")]
fn lock_holders(lock: &Path) -> String {
    use std::os::unix::fs::MetadataExt;
    let target = std::fs::canonicalize(lock).unwrap_or_else(|_| lock.to_path_buf());
    let mut out = vec![format!("this test process is pid {}", std::process::id())];
    for proc_dir in std::fs::read_dir("/proc").into_iter().flatten().flatten() {
        let Ok(fds) = std::fs::read_dir(proc_dir.path().join("fd")) else {
            continue;
        };
        for fd in fds.flatten() {
            if std::fs::read_link(fd.path()).is_ok_and(|path| path == target) {
                let field =
                    |name: &str| std::fs::read(proc_dir.path().join(name)).unwrap_or_default();
                let state = String::from_utf8_lossy(&field("status"))
                    .lines()
                    .find(|line| line.starts_with("State:"))
                    .unwrap_or("State: ?")
                    .to_owned();
                out.push(format!(
                    "pid {} fd {} ({state}) {}",
                    proc_dir.file_name().to_string_lossy(),
                    fd.file_name().to_string_lossy(),
                    String::from_utf8_lossy(&field("cmdline")).replace('\0', " ")
                ));
            }
        }
    }
    let mut children = Vec::new();
    for task in std::fs::read_dir("/proc/self/task")
        .into_iter()
        .flatten()
        .flatten()
    {
        let listed = std::fs::read_to_string(task.path().join("children")).unwrap_or_default();
        for pid in listed.split_whitespace() {
            let exe = std::fs::read_link(format!("/proc/{pid}/exe")).unwrap_or_default();
            children.push(format!("{pid} {}", exe.display()));
        }
    }
    out.push(format!(
        "children of this process: [{}]",
        children.join(", ")
    ));
    let inode = std::fs::metadata(lock)
        .map_or(0, |meta| meta.ino())
        .to_string();
    let rows = std::fs::read_to_string("/proc/locks").unwrap_or_default();
    out.extend(
        rows.lines()
            .filter(|row| {
                row.split_whitespace()
                    .any(|f| f.rsplit(':').next() == Some(&inode) && f.contains(':'))
            })
            .map(|row| format!("lock {row}")),
    );
    out.join("; ")
}

#[cfg(not(target_os = "linux"))]
fn lock_holders(_lock: &Path) -> String {
    "(holders are read from /proc on Linux only)".to_owned()
}

/// A record the owner stored, then damaged on disk: corrupt (same length, one bit) or
/// unreadable (a directory where the file was). Returns the record's bytes, CID and path.
async fn damaged_record(root: &Path, unreadable: bool) -> (Vec<u8>, cid::Cid, PathBuf) {
    let canonical = serde_ipld_dagcbor::to_vec(&vec![7u8, 8, 9, 10]).unwrap();
    let cid = {
        let owner = BlobStore::open_owner(root).await.unwrap();
        owner.store_record(&canonical).await.unwrap().cid
    };
    let path = elohim_blob_store::fanout_path(&root.join("records"), &cid.to_string());
    if unreadable {
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
    } else {
        let mut bytes = std::fs::read(&path).unwrap();
        bytes[1] ^= 0x01;
        std::fs::write(&path, bytes).unwrap();
    }
    (canonical, cid, path)
}

#[tokio::test]
async fn a_shared_instance_never_replaces_a_record() {
    for unreadable in [false, true] {
        let dir = private_root();
        let (canonical, cid, path) = damaged_record(dir.path(), unreadable).await;
        let before = snapshot(dir.path());
        let shared = BlobStore::open_shared(dir.path()).await.unwrap();
        let refused = shared.store_record(&canonical).await;
        if unreadable {
            assert!(
                matches!(refused, Err(BlobStoreError::HeldCopyUnreadable { .. })),
                "{refused:?}"
            );
            assert!(path.is_dir(), "the unreadable record was replaced");
        } else {
            assert!(
                matches!(refused, Err(BlobStoreError::HashMismatch { .. })),
                "{refused:?}"
            );
        }
        assert!(shared.get_record(&cid).await.is_err());
        assert_eq!(
            snapshot(dir.path()),
            before,
            "a shared instance replaced a record"
        );

        // The owner keeps today's behaviour: it rewrites the record.
        if !unreadable {
            let owner = match BlobStore::open_owner(dir.path()).await {
                Ok(owner) => owner,
                Err(error) => {
                    let holders = lock_holders(&dir.path().join(elohim_blob_store::OWNER_LOCK));
                    let refused_at = Instant::now();
                    let mut cleared = None;
                    while cleared.is_none() && refused_at.elapsed() < Duration::from_secs(2) {
                        tokio::time::sleep(Duration::from_millis(5)).await;
                        if BlobStore::open_owner(dir.path()).await.is_ok() {
                            cleared = Some(refused_at.elapsed());
                        }
                    }
                    panic!(
                        "{error:?}; owner.lock holders when refused: {holders}; \
                         a retry every 5 ms opened the owner after {cleared:?}"
                    )
                }
            };
            owner.store_record(&canonical).await.unwrap();
            assert_eq!(&owner.get_record(&cid).await.unwrap()[..], &canonical[..]);
        }
    }
}

/// An owner's lock ends with the owner even while a child forked from this process has not yet
/// exec'd. Such a child holds a duplicate of every descriptor until its exec closes them, so a
/// lock released only by closing the file stays held through it. Other tests in this binary
/// spawn children at any moment; under push load that window read as `OwnerHeld` in
/// `a_shared_instance_never_replaces_a_record`.
#[cfg(unix)]
#[tokio::test]
async fn an_owner_lock_ends_with_the_owner_despite_a_forked_child() {
    use std::os::unix::process::CommandExt;
    let dir = private_root();
    let owner = BlobStore::open_owner(dir.path()).await.unwrap();
    let mut ready = [0; 2];
    // SAFETY: `ready` has room for the two descriptors `pipe` writes.
    assert_eq!(unsafe { libc::pipe(ready.as_mut_ptr()) }, 0);
    let [read_ready, write_ready] = ready;
    // Fork from another thread, as a sibling test would: `spawn` returns only after the exec.
    let forker = std::thread::spawn(move || {
        let mut command = Command::new("true");
        // SAFETY: the hook uses only async-signal-safe libc calls on inherited descriptors,
        // then holds the forked child before exec with the descriptors it inherited.
        unsafe {
            command.pre_exec(move || {
                libc::close(read_ready);
                let byte = [1_u8];
                if libc::write(write_ready, byte.as_ptr().cast(), byte.len()) != 1 {
                    return Err(std::io::Error::last_os_error());
                }
                let pause = libc::timespec {
                    tv_sec: 1,
                    tv_nsec: 500_000_000,
                };
                libc::nanosleep(&pause, std::ptr::null_mut());
                libc::close(write_ready);
                Ok(())
            });
        }
        command.status().unwrap()
    });
    let mut byte = [0_u8];
    // SAFETY: both descriptors came from `pipe`; the buffer holds the one byte requested.
    assert_eq!(
        unsafe { libc::read(read_ready, byte.as_mut_ptr().cast(), byte.len()) },
        1
    );
    // SAFETY: the parent is finished with both of its pipe descriptors.
    unsafe {
        libc::close(read_ready);
        libc::close(write_ready);
    }
    drop(owner);
    let reopened = BlobStore::open_owner(dir.path()).await;
    let holders = lock_holders(&dir.path().join(elohim_blob_store::OWNER_LOCK));
    assert!(forker.join().unwrap().success());
    assert!(
        reopened.is_ok(),
        "{:?}; owner.lock holders: {holders}",
        reopened.err()
    );
}

#[cfg(feature = "tier")]
#[tokio::test]
async fn a_shared_tier_never_replaces_a_record() {
    use eprfs_core::{BlobCid, EprfsStorage};
    for unreadable in [false, true] {
        let dir = private_root();
        let (canonical, cid, path) = damaged_record(dir.path(), unreadable).await;
        let before = snapshot(dir.path());
        let tier = elohim_blob_store::tier::BlobStoreTier::at(dir.path(), OpenMode::Shared)
            .await
            .unwrap();
        let blob = BlobCid::parse(&cid.to_string()).unwrap();
        assert!(tier
            .put_blob_verified(&blob, bytes::Bytes::from(canonical.clone()))
            .await
            .is_err());
        assert_eq!(snapshot(dir.path()), before, "the tier replaced a record");
        assert_eq!(path.is_dir(), unreadable);
    }
}
