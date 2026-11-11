use super::*;
use std::os::unix::fs::{symlink, MetadataExt};
use std::time::{Duration, SystemTime};

fn tree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("usr/include")).unwrap();
    fs::create_dir_all(dir.path().join("lib64")).unwrap();
    fs::write(
        dir.path().join("usr/include/features.h"),
        b"#define __GLIBC__ 2\n#define __GLIBC_MINOR__ 39\n",
    )
    .unwrap();
    fs::write(dir.path().join("lib64/libc.so.6"), b"library").unwrap();
    dir
}

#[test]
fn pack_is_relocation_metadata_and_creation_order_independent() {
    let a = tree();
    let b = tree();
    let file = b.path().join("lib64/libc.so.6");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o777)).unwrap();
    fs::File::open(&file)
        .unwrap()
        .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(42))
        .unwrap();
    // Root can supply two actual owners. Other callers still exercise mtime/mode; the format
    // has no ownership fields and pack never reads uid/gid.
    if fs::metadata(&file).unwrap().uid() == 0 {
        std::os::unix::fs::chown(&file, Some(1234), Some(1234)).unwrap();
    }
    fs::write(a.path().join("z"), b"last").unwrap();
    fs::write(a.path().join("a"), b"first").unwrap();
    fs::write(b.path().join("a"), b"first").unwrap();
    fs::write(b.path().join("z"), b"last").unwrap();
    let (left, manifest) = pack(a.path(), "x86_64-unknown-linux-gnu").unwrap();
    let (right, other) = pack(b.path(), "x86_64-unknown-linux-gnu").unwrap();
    assert_eq!(left, right);
    assert_eq!(manifest.cid().unwrap(), other.cid().unwrap());
    assert_eq!(manifest.libc_version.as_deref(), Some("2.39"));
    fs::write(a.path().join("z"), b"changed").unwrap();
    let (changed, _) = pack(a.path(), "x86_64-unknown-linux-gnu").unwrap();
    assert_ne!(
        BlobCid::compute_bytes(&left),
        BlobCid::compute_bytes(&changed)
    );
}

#[test]
fn symlinks_dereference_inside_and_refuse_escape_or_cycle() {
    let dir = tree();
    symlink("libc.so.6", dir.path().join("lib64/alias")).unwrap();
    let (_, manifest) = pack(dir.path(), "test").unwrap();
    assert_eq!(
        manifest
            .files
            .iter()
            .find(|f| f.path == "lib64/alias")
            .unwrap()
            .cid,
        manifest
            .files
            .iter()
            .find(|f| f.path == "lib64/libc.so.6")
            .unwrap()
            .cid
    );
    let outside = tempfile::tempdir().unwrap();
    symlink(outside.path(), dir.path().join("escape")).unwrap();
    assert!(pack(dir.path(), "test")
        .unwrap_err()
        .to_string()
        .contains("escapes"));
    fs::remove_file(dir.path().join("escape")).unwrap();
    symlink(".", dir.path().join("cycle")).unwrap();
    assert!(pack(dir.path(), "test")
        .unwrap_err()
        .to_string()
        .contains("cycle"));
}

#[test]
fn script_absolute_operands_are_rebased_and_missing_host_files_refuse() {
    let dir = tree();
    fs::write(
        dir.path().join("lib64/libgcc_s.so"),
        b"/* GNU ld script */\nGROUP ( /lib64/libc.so.6 AS_NEEDED ( libc.so.6 ) )",
    )
    .unwrap();
    let (_, manifest) = pack(dir.path(), "test").unwrap();
    assert_eq!(manifest.scripts[0].operands, ["lib64/libc.so.6"]);
    fs::write(
        dir.path().join("lib64/libgcc_s.so"),
        b"INPUT ( /etc/passwd )",
    )
    .unwrap();
    assert!(pack(dir.path(), "test")
        .unwrap_err()
        .to_string()
        .contains("absent inside"));
    fs::write(
        dir.path().join("lib64/libgcc_s.so"),
        b"INPUT ( ../../outside )",
    )
    .unwrap();
    assert!(pack(dir.path(), "test").is_err());
}

#[test]
fn materialize_verifies_every_file_and_hits_without_replacing() {
    let dir = tree();
    let cache = tempfile::tempdir().unwrap();
    let (archive, _) = pack(dir.path(), "test").unwrap();
    let root = materialize(&archive, cache.path()).unwrap();
    assert!(root.ends_with(format!("{}/root", BlobCid::compute_bytes(&archive))));
    let inode = fs::metadata(&root).unwrap().ino();
    assert_eq!(materialize(&archive, cache.path()).unwrap(), root);
    assert_eq!(fs::metadata(&root).unwrap().ino(), inode);
    assert_eq!(
        fs::metadata(root.join("lib64/libc.so.6"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o444
    );
    let mut corrupt = archive.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    assert!(materialize(&corrupt, cache.path()).is_err());
    let file = root.join("lib64/libc.so.6");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
    fs::write(&file, b"corrupt").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o444)).unwrap();
    assert!(materialize(&archive, cache.path()).is_err());
    // Restore directory permissions so a non-root test runner can clean its own cache.
    fs::set_permissions(root.parent().unwrap(), fs::Permissions::from_mode(0o755)).unwrap();
    for path in [
        root.join("usr/include"),
        root.join("usr"),
        root.join("lib64"),
        root,
    ] {
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

/// ELF64 with one PT_NOTE and no section headers, so extraction cannot rely on section names.
fn isa_fixture() -> Vec<u8> {
    let mut bytes = vec![0u8; 152];
    bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    bytes[16..18].copy_from_slice(&3u16.to_le_bytes());
    bytes[18..20].copy_from_slice(&62u16.to_le_bytes());
    bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
    bytes[32..40].copy_from_slice(&64u64.to_le_bytes());
    bytes[52..54].copy_from_slice(&64u16.to_le_bytes());
    bytes[54..56].copy_from_slice(&56u16.to_le_bytes());
    bytes[56..58].copy_from_slice(&1u16.to_le_bytes());
    bytes[64..68].copy_from_slice(&4u32.to_le_bytes());
    bytes[72..80].copy_from_slice(&120u64.to_le_bytes());
    bytes[96..104].copy_from_slice(&32u64.to_le_bytes());
    bytes[112..120].copy_from_slice(&8u64.to_le_bytes());
    bytes[120..124].copy_from_slice(&4u32.to_le_bytes());
    bytes[124..128].copy_from_slice(&16u32.to_le_bytes());
    bytes[128..132].copy_from_slice(&5u32.to_le_bytes());
    bytes[132..136].copy_from_slice(b"GNU\0");
    bytes[136..140].copy_from_slice(&0xc0008002u32.to_le_bytes());
    bytes[140..144].copy_from_slice(&4u32.to_le_bytes());
    bytes[144..148].copy_from_slice(&7u32.to_le_bytes());
    bytes
}

#[test]
fn isa_notes_record_v3_payload_and_tampered_evidence_refuses() {
    let dir = tree();
    fs::write(dir.path().join("lib64/Scrt1.o"), isa_fixture()).unwrap();
    let (archive, manifest) = pack(dir.path(), "x86_64-unknown-linux-gnu").unwrap();
    assert_eq!(
        manifest.isa_notes[0].needed,
        ["x86-64-baseline", "x86-64-v2", "x86-64-v3"]
    );
    assert_eq!(verify(&archive).unwrap(), manifest);
    let mut false_manifest = manifest;
    false_manifest.isa_notes.clear();
    let mut remaining = &archive[8..];
    sized(&mut remaining, Limit::ManifestBytes).unwrap();
    let json = false_manifest.canonical_bytes().unwrap();
    let mut forged = MAGIC.to_vec();
    forged.extend_from_slice(&(json.len() as u64).to_le_bytes());
    forged.extend(json);
    forged.extend(remaining);
    assert!(verify(&forged)
        .unwrap_err()
        .to_string()
        .contains("evidence disagrees"));
}

#[test]
fn script_comments_unsupported_first_and_empty_operands_cannot_bypass_validation() {
    let dir = tree();
    let script = dir.path().join("lib64/libc.so");
    for text in [
        "# comment\nINPUT ( /outside/missing.so )",
        "# comment\nINPUT ( ../../outside )",
        "SEARCH_DIR(/lib64) INPUT(libc.so.6)",
        "INPUT ()",
        "GROUP(AS_NEEDED())",
        "INPUT(/* empty */)",
        "INPUT ( missing.so )",
        "OUTPUT_FORMAT(SEARCH_DIR(/lib64)) INPUT(libc.so.6)",
    ] {
        fs::write(&script, text).unwrap();
        assert!(pack(dir.path(), "test").is_err(), "accepted {text}");
    }
    fs::write(&script, b"# \xff\nINPUT(/outside/missing.so)").unwrap();
    assert!(pack(dir.path(), "test").is_err());
    fs::write(
        &script,
        "# leading\n/* first */ INPUT/* adjacent */(libc.so.6/* operand */) /* trailing */",
    )
    .unwrap();
    let (archive, manifest) = pack(dir.path(), "test").unwrap();
    assert_eq!(manifest.scripts[0].operands, ["lib64/libc.so.6"]);
    assert_eq!(verify(&archive).unwrap(), manifest);
}

fn assert_limit<T: std::fmt::Debug>(result: ArtifactResult<T>, expected: Limit) {
    assert!(
        matches!(result, Err(ArtifactError::SysrootLimit(actual)) if actual == expected),
        "{result:?}"
    );
}

#[test]
fn expanded_acyclic_symlink_fanout_refuses_count_and_depth() {
    for (levels, expected) in [(16, Limit::Members), (65, Limit::Depth)] {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..levels {
            let path = dir.path().join(format!("d{i:02}"));
            fs::create_dir(&path).unwrap();
            if i + 1 < levels {
                symlink(format!("../d{:02}", i + 1), path.join("a")).unwrap();
                symlink(format!("../d{:02}", i + 1), path.join("b")).unwrap();
            }
        }
        let root = fs::canonicalize(dir.path()).unwrap();
        assert_limit(
            walk(
                &root,
                &root.join("d00"),
                "d00",
                &mut Vec::new(),
                &mut Members::new(),
                &mut Budget::default(),
            ),
            expected,
        );
    }
}

#[test]
fn manifest_count_and_byte_prefixes_refuse_before_decode_or_copy() {
    use limits::*;
    let mut prefix = MAGIC.to_vec();
    prefix.extend_from_slice(&(MAX_MANIFEST_BYTES + 1).to_le_bytes());
    assert_limit(verify(&prefix), Limit::ManifestBytes);
    let json = format!(
        "{{\"files\":[{}]}}",
        vec!["{}"; MAX_MEMBERS as usize + 1].join(",")
    );
    prefix.truncate(8);
    prefix.extend_from_slice(&(json.len() as u64).to_le_bytes());
    prefix.extend_from_slice(json.as_bytes());
    assert_limit(verify(&prefix), Limit::Members);
    let length = (MAX_FILE_BYTES + 1).to_le_bytes();
    assert_limit(
        file_body(&mut length.as_slice(), &mut Budget::default()),
        Limit::FileBytes,
    );
    let length = 1u64.to_le_bytes();
    assert_limit(
        file_body(
            &mut length.as_slice(),
            &mut Budget {
                members: 0,
                bytes: MAX_TOTAL_BYTES,
            },
        ),
        Limit::TotalBytes,
    );
}

#[test]
fn byte_limits_accept_exact_maximum_and_refuse_next_byte() {
    use limits::*;
    for limit in [
        Limit::Members,
        Limit::Depth,
        Limit::ManifestBytes,
        Limit::FileBytes,
        Limit::TotalBytes,
        Limit::ArchiveBytes,
    ] {
        limit.check(limit.maximum()).unwrap();
        assert_limit(limit.check(limit.maximum() + 1), limit);
    }
    // Exercise the actual encoder at exactly the maximum and one byte beyond it.
    let mut manifest = Manifest {
        version: 1,
        triple: String::new(),
        files: vec![],
        scripts: vec![],
        isa_notes: vec![],
        libc_version: None,
    };
    let overhead = manifest.canonical_bytes().unwrap().len();
    manifest.triple = "x".repeat(MAX_MANIFEST_BYTES as usize - overhead);
    assert_eq!(
        manifest.canonical_bytes().unwrap().len() as u64,
        MAX_MANIFEST_BYTES
    );
    manifest.triple.push('x');
    assert_limit(manifest.canonical_bytes(), Limit::ManifestBytes);
    // Sparse files avoid disk pressure; the accepted per-file boundary exercises the real reader.
    let file = tempfile::NamedTempFile::new().unwrap();
    file.as_file().set_len(MAX_FILE_BYTES).unwrap();
    assert_eq!(
        limits::read_file(file.path(), 0).unwrap().len() as u64,
        MAX_FILE_BYTES
    );
    file.as_file().set_len(MAX_FILE_BYTES + 1).unwrap();
    assert_limit(limits::read_file(file.path(), 0), Limit::FileBytes);
    file.as_file().set_len(1).unwrap();
    assert_eq!(
        limits::read_file(file.path(), MAX_TOTAL_BYTES - 1).unwrap(),
        [0]
    );
    assert_limit(
        limits::read_file(file.path(), MAX_TOTAL_BYTES),
        Limit::TotalBytes,
    );
    file.as_file().set_len(MAX_ARCHIVE_BYTES + 1).unwrap();
    assert_limit(limits::read_archive(file.path()), Limit::ArchiveBytes);
}

#[test]
fn ordinary_header_macros_are_not_linker_script_candidates() {
    let dir = tree();
    fs::write(
        dir.path().join("usr/include/unistd.h"),
        b"#define N 1\n__SC_COMP(N, sys_io, compat_io)\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("usr/include/sysmacros.h"),
        b"#define N 1\nDECLARE_RC_STRUCT(N)\n",
    )
    .unwrap();
    let (archive, manifest) = pack(dir.path(), "test").unwrap();
    assert!(manifest.scripts.is_empty());
    verify(&archive).unwrap();
}
