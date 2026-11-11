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
    fs::write(dir.path().join("lib64/libc.so.6"), b"!<arch>\n").unwrap();
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

#[test]
fn evidence_budget_charges_every_retained_string_and_refuses_before_growth() {
    let mut members = Members::new();
    members.insert("lib64/libreal.so".into(), Some(isa_fixture()));
    members.insert(
        "lib64/libscript.a".into(),
        Some(b"INPUT(libreal.so)".to_vec()),
    );
    members.insert(
        "usr/include/features.h".into(),
        Some(b"#define __GLIBC__ 2\n#define __GLIBC_MINOR__ 39".to_vec()),
    );
    let mut budget = EvidenceBudget::default();
    let result = manifest_with_budget("test", &members, &mut budget).unwrap();
    let strings = result.triple.len()
        + result.files.iter().map(|f| f.path.len()).sum::<usize>()
        + result
            .scripts
            .iter()
            .map(|s| s.path.len() + s.operands.iter().map(String::len).sum::<usize>())
            .sum::<usize>()
        + result
            .isa_notes
            .iter()
            .map(|n| n.path.len() + n.needed.iter().map(String::len).sum::<usize>())
            .sum::<usize>()
        + result.libc_version.as_ref().unwrap().len();
    assert_eq!(budget.bytes, strings as u64);
    budget.bytes = limits::MAX_MANIFEST_BYTES - 1;
    assert_eq!(budget.retain("x").unwrap(), "x");
    assert_limit(budget.retain("y"), Limit::ManifestBytes);
    assert_eq!(budget.bytes, limits::MAX_MANIFEST_BYTES);
}

#[test]
fn many_scripts_with_deep_resolved_targets_refuse_on_shared_counter() {
    // Review shape: 5,000 targets, 500 scripts, a 2,509-byte prefix. Use in-memory
    // members to avoid filesystem PATH_MAX. ar headers make targets binary under P2.
    let prefix = format!("lib/{}", vec!["d".repeat(249); 10].join("/"));
    assert_eq!(prefix.len(), 2503);
    let prefix = format!("{prefix}/child");
    assert_eq!(prefix.len(), 2509);
    let mut members = Members::new();
    for i in 0..5000 {
        members.insert(format!("{prefix}/t{i:04}"), Some(b"!<arch>\n".to_vec()));
    }
    let script = format!(
        "INPUT({})",
        (0..5000)
            .map(|i| format!("t{i:04}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    for i in 0..500 {
        members.insert(
            format!("{prefix}/s{i:04}"),
            Some(script.as_bytes().to_vec()),
        );
    }
    let mut budget = EvidenceBudget::default();
    assert_limit(
        manifest_with_budget("test", &members, &mut budget),
        Limit::ManifestBytes,
    );
    let path_bytes = (prefix.len() + 6) as u64;
    // First script completes; second stops at the first charge over budget. No RSS probe:
    // triple + two Member paths + one Script path + accepted operand resolutions.
    let accepted_operands = (limits::MAX_MANIFEST_BYTES - 4 - 3 * path_bytes) / path_bytes;
    assert!(accepted_operands > 5000 && accepted_operands < 10000);
    assert_eq!(budget.bytes, 4 + (3 + accepted_operands) * path_bytes);
    assert!(budget.bytes <= limits::MAX_MANIFEST_BYTES);
    assert!(budget.bytes + path_bytes > limits::MAX_MANIFEST_BYTES);
}

#[test]
fn script_operand_cap_counts_duplicates_and_format_operands() {
    let members = Members::from([("lib/x".into(), Some(b"!<arch>\n".to_vec()))]);
    for directive in ["INPUT", "OUTPUT_FORMAT"] {
        let script = format!(
            "{directive}({})",
            vec!["x"; limits::MAX_SCRIPT_OPERANDS as usize].join(" ")
        );
        evidence::script(
            "lib/script",
            script.as_bytes(),
            &members,
            &mut EvidenceBudget::default(),
        )
        .unwrap();
        let excess = format!("{script} {directive}(x)");
        assert_limit(
            evidence::script(
                "lib/script",
                excess.as_bytes(),
                &members,
                &mut EvidenceBudget::default(),
            ),
            Limit::ScriptOperands,
        );
    }
}

#[test]
fn all_nonbinary_library_files_are_validated_regardless_of_name_or_first_token() {
    for directory in ["lib", "usr/lib64", "nested/lib32/deep", "usr/libx32"] {
        for bytes in [
            &b"FORCE_COMMON_ALLOCATION\nINPUT(/outside/missing.so)"[..],
            &b"# \xff\nINPUT(/outside/missing.so)"[..],
            &b"OUTPUT_FORMAT(elf64-x86-64) FORCE_COMMON_ALLOCATION"[..],
            &b"INPUT()"[..],
            &b"INPUT(\"\")"[..],
            &b"/* no directive */"[..],
        ] {
            let path = format!("{directory}/libfoo.a");
            let members = Members::from([(path.clone(), Some(bytes.to_vec()))]);
            assert!(
                matches!(manifest("test", &members), Err(ArtifactError::SysrootScript { path: actual, .. }) if actual == path)
            );
        }
    }
    let members = Members::from([
        ("usr/lib64/libelf.so".into(), Some(isa_fixture())),
        ("lib/libarchive.a".into(), Some(b"!<arch>\n".to_vec())),
        (
            "usr/include/header.h".into(),
            Some(b"# \xff\nINPUT(outside)".to_vec()),
        ),
        ("library/libfoo.a".into(), Some(b"not a candidate".to_vec())),
    ]);
    let result = manifest("test", &members).unwrap();
    assert!(result.scripts.is_empty());
    assert_eq!(result.isa_notes.len(), 1);
}

#[test]
fn library_data_types_are_explicit_refusals_not_silent_exclusions() {
    for (path, bytes) in [
        ("usr/lib64/glibc.pc", "prefix=/usr\nName: glibc"),
        (
            "usr/lib64/libexample.la",
            "# libtool archive\ndlname='libexample.so'",
        ),
        (
            "usr/lib64/gconv/gconv-modules",
            "module UTF-8// INTERNAL ISO8859-1 1",
        ),
        ("usr/lib64/gconv/gconv-modules.cache", "\0\u{fffd}"),
        ("usr/lib64/cmake/ZLIB/ZLIB.cmake", "set(ZLIB_FOUND TRUE)"),
        ("usr/lib64/python3.12/data.py", "import sys"),
        ("usr/lib64/libfreebl3.chk", "checksum"),
    ] {
        let members = Members::from([(path.into(), Some(bytes.as_bytes().to_vec()))]);
        assert!(
            matches!(manifest("test", &members), Err(ArtifactError::SysrootScript { path: actual, .. }) if actual == path)
        );
    }
}

#[test]
fn real_glibc_library_and_header_sysroot_packs_when_installed() {
    if !Path::new("/usr/lib64/libc.so").exists() {
        eprintln!("real glibc fixture unavailable on this device");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let mut paths = vec![
        "/usr/lib64/libc.so",
        "/usr/lib64/libm.so",
        "/lib64/libc.so.6",
        "/usr/lib64/libc_nonshared.a",
        "/lib64/ld-linux-x86-64.so.2",
        "/lib64/libm.so.6",
        "/lib64/libmvec.so.1",
        "/usr/include/features.h",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect::<Vec<_>>();
    let mut gcc_scripts = 0;
    let gcc_root = Path::new("/usr/lib/gcc");
    if gcc_root.exists() {
        for target in fs::read_dir(gcc_root).unwrap() {
            for version in fs::read_dir(target.unwrap().path()).unwrap() {
                let version = version.unwrap().path();
                let script = version.join("libgcc_s.so");
                if script.is_file() {
                    paths.extend([script, version.join("libgcc.a")]);
                    gcc_scripts += 1;
                }
            }
        }
        if gcc_scripts > 0 {
            paths.push("/lib64/libgcc_s.so.1".into());
        }
    }
    for source in paths {
        let destination = dir.path().join(source.strip_prefix("/").unwrap());
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::copy(source, destination).unwrap();
    }
    let (archive, manifest) = pack(dir.path(), "x86_64-unknown-linux-gnu").unwrap();
    assert_eq!(manifest.scripts.len(), 2 + gcc_scripts);
    assert_eq!(verify(&archive).unwrap(), manifest);
    eprintln!("real glibc pack+verify: libc.so, libm.so, {gcc_scripts} gcc libgcc_s.so scripts");
}

#[test]
fn only_named_pkgconfig_data_is_excluded_and_remains_content_addressed() {
    let dir = tree();
    let path = dir.path().join("usr/lib64/pkgconfig/glibc.pc");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "prefix=/usr\nName: glibc").unwrap();
    let (archive, manifest) = pack(dir.path(), "test").unwrap();
    assert!(manifest.scripts.is_empty());
    assert!(manifest
        .files
        .iter()
        .any(|f| f.path == "usr/lib64/pkgconfig/glibc.pc"));
    assert_eq!(verify(&archive).unwrap(), manifest);
    fs::write(&path, "prefix=/changed").unwrap();
    assert_ne!(pack(dir.path(), "test").unwrap().0, archive);
    // Directory alone is not an exclusion: an archive-looking filename still parses.
    fs::write(path.with_file_name("libfoo.a"), "FORCE_COMMON_ALLOCATION").unwrap();
    assert!(matches!(
        pack(dir.path(), "test"),
        Err(ArtifactError::SysrootScript { .. })
    ));
}
