use super::*;

use std::path::Path;
use std::process::Command;

/// Run this host's `ar` (binutils, as cc runs it) in `dir`.
fn ar(dir: &Path, args: &[&str]) {
    let out = Command::new("ar")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "ar {args:?}: {out:?}");
}

/// One header as GNU ar writes it, for hand-built archives.
fn header(name: &str, size: usize) -> Vec<u8> {
    format!("{name:<16}{:<12}{:<6}{:<6}{:<8}{size:<10}`\n", 0, 0, 0, 644).into_bytes()
}

fn member(name: &str, data: &[u8]) -> Vec<u8> {
    let mut bytes = header(name, data.len());
    bytes.extend_from_slice(data);
    if data.len() % 2 == 1 {
        bytes.push(b'\n');
    }
    bytes
}

#[test]
fn archive_members_read_what_a_real_ar_wrote_long_names_included() {
    let dir = tempfile::TempDir::new().unwrap();
    let long = "0af7afa8371890fd-blake3_sse2_x86-64_unix.o";
    std::fs::write(dir.path().join(long), b"first object").unwrap();
    std::fs::write(dir.path().join("x.o"), b"second").unwrap();
    // cc's own form (lib.rs:4032): `ar cqD <dst> <objs>`, then `ar sD <dst>`.
    ar(dir.path(), &["cqD", "libx.a", long, "x.o"]);
    ar(dir.path(), &["sD", "libx.a"]);
    let bytes = std::fs::read(dir.path().join("libx.a")).unwrap();
    let found = members(&bytes).unwrap();
    assert_eq!(
        found,
        [
            Member {
                name: long.into(),
                data: b"first object".to_vec()
            },
            Member {
                name: "x.o".into(),
                data: b"second".to_vec()
            },
        ]
    );
}

#[test]
fn archive_members_skip_the_symbol_tables_and_resolve_the_long_name_table() {
    let mut bytes = b"!<arch>\n".to_vec();
    bytes.extend(member("/", b"\0\0\0\0"));
    bytes.extend(member("/SYM64/", b"\0\0\0\0\0\0\0\0"));
    let table = b"a-very-long-object-name.o/\nanother-long-object-name.o/\n";
    bytes.extend(member("//", table));
    bytes.extend(member("/27", b"second"));
    bytes.extend(member("/0", b"first"));
    bytes.extend(member("short.o/", b"s"));
    let names: Vec<String> = members(&bytes)
        .unwrap()
        .into_iter()
        .map(|m| m.name)
        .collect();
    assert_eq!(
        names,
        [
            "another-long-object-name.o",
            "a-very-long-object-name.o",
            "short.o"
        ]
    );
}

/// Every form the reader does not know refuses by name, never as an archive with no members.
#[test]
fn archive_members_refuse_what_they_cannot_read_by_name() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("x.o"), b"object").unwrap();
    ar(dir.path(), &["cqT", "libthin.a", "x.o"]);
    let thin = std::fs::read(dir.path().join("libthin.a")).unwrap();
    let mut bsd = b"!<arch>\n".to_vec();
    bsd.extend(member("#1/8", b"x.o\0\0\0\0\0"));
    let mut early = b"!<arch>\n".to_vec();
    early.extend(member("/0", b"x"));
    let mut odd = b"!<arch>\n".to_vec();
    odd.extend(member("noslash", b"x"));
    let mut bad = b"!<arch>\n".to_vec();
    bad.extend(member("x.o/", b"data"));
    let len = bad.len();
    bad[len - 6] = b'X';
    let mut truncated = b"!<arch>\n".to_vec();
    truncated.extend(header("x.o/", 100));
    for (bytes, why) in [
        (thin, "thin archive"),
        (bsd, "BSD member name"),
        (early, "before any // long-name table"),
        (odd, "does not know"),
        (bad, "terminator"),
        (truncated, "runs past the archive's end"),
        (b"not an archive".to_vec(), "no !<arch> magic"),
    ] {
        let found = members(&bytes).unwrap_err();
        assert!(found.contains(why), "{why}: {found}");
    }
}
