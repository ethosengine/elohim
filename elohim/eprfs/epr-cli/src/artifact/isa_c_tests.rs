use super::*;

use super::super::host::fixture;

fn bytes(producers: &[&str]) -> Vec<u8> {
    let mut strings = b"clang LLVM (rustc version 1.100.0)\0".to_vec();
    for producer in producers {
        strings.extend_from_slice(producer.as_bytes());
        strings.push(0);
    }
    fixture::artifact_elf_format(b"Linker: LLD 22.1.2\0", true, true, Some(&strings))
}

/// gcc 14's record of ring's x25519 unit, at `march`, as the held shem/ethosengine builds show it.
fn ring_unit(march: &str) -> String {
    format!(
        "GNU C11 14.3.1 20251022 (Red Hat 14.3.1-4) -m64 -mtune=generic {march} -g3 -O2 \
         -std=c11 -ffunction-sections -fdata-sections -fPIC -fvisibility=hidden"
    )
}

#[test]
fn isa_c_reads_the_declared_march_from_every_gnu_c_producer() {
    let v1 = ring_unit("-march=x86-64");
    assert_eq!(producers(&bytes(&[&v1]), "-march=x86-64"), Ok(1));
    // Repeated units are one string; -mno-*, -m64 and -mtune= carry no ISA.
    let quiet = ring_unit("-march=x86-64 -mno-avx -mno-red-zone -mtune=znver2");
    assert_eq!(
        producers(&bytes(&[&v1, &v1, &quiet]), "-march=x86-64"),
        Ok(2)
    );
    // A GNU C++ unit is read too.
    let cxx = "GNU C++17 14.3.1 -march=x86-64 -O2".to_string();
    assert_eq!(producers(&bytes(&[&cxx]), "-march=x86-64"), Ok(1));
    // No .debug_str, or one with no GNU C string: nothing read, for the caller to judge.
    let plain = fixture::artifact_elf(b"Linker: LLD 22.1.2\0");
    assert_eq!(producers(&plain, "-march=x86-64"), Ok(0));
    assert_eq!(producers(&bytes(&[]), "-march=x86-64"), Ok(0));
}

#[test]
fn isa_c_refuses_a_producer_off_the_declared_march() {
    let v1 = ring_unit("-march=x86-64");
    for (producer, token) in [
        (ring_unit("-march=x86-64-v3"), "-march=x86-64-v3"),
        (ring_unit("-march=x86-64 -mavx2"), "-mavx2"),
        (ring_unit("-march=x86-64 -msse4.2"), "-msse4.2"),
        (
            ring_unit("-march=x86-64-v2 -march=x86-64"),
            "-march=x86-64-v2",
        ),
        (ring_unit(""), "no -march"),
    ] {
        let found = producers(&bytes(&[&v1, &producer]), "-march=x86-64").unwrap_err();
        assert!(
            found.iter().any(|why| why.contains(token)),
            "{token}: {found:?}"
        );
    }
    let found = producers(b"not an object", "-march=x86-64").unwrap_err();
    assert!(found[0].contains("do not parse"), "{found:?}");
}

fn lock(text: &str) -> toml::Value {
    text.parse().unwrap()
}

#[test]
fn isa_c_walks_the_lock_from_the_bins_package() {
    let lock = lock(
        r#"
version = 4
[[package]]
name = "elohim-epr-cli"
version = "0.1.0"
dependencies = ["eprfs-peer", "serde 1.0.0"]
[[package]]
name = "eprfs-peer"
version = "0.1.0"
dependencies = ["rustls 0.23.0 (registry+https://github.com/rust-lang/crates.io-index)"]
[[package]]
name = "rustls"
version = "0.23.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
dependencies = ["ring"]
[[package]]
name = "rustls"
version = "0.21.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
dependencies = ["openssl-sys"]
[[package]]
name = "ring"
version = "0.17.14"
source = "registry+https://github.com/rust-lang/crates.io-index"
[[package]]
name = "serde"
version = "1.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
[[package]]
name = "elohim-device-runtime"
version = "0.1.0"
dependencies = ["serde", "rustls 0.21.0"]
"#,
    );
    assert_eq!(
        lock_closure_holds(&lock, "elohim-epr-cli", "ring"),
        Ok(true)
    );
    // The versioned edge selects one rustls; the other's closure holds no ring.
    assert_eq!(
        lock_closure_holds(&lock, "elohim-device-runtime", "ring"),
        Ok(false)
    );
    assert_eq!(lock_closure_holds(&lock, "eprfs-peer", "ring"), Ok(true));
    // A registry package is never the bin's own; an absent one is an error, not "no ring".
    assert!(lock_closure_holds(&lock, "rustls", "ring").is_err());
    assert!(lock_closure_holds(&lock, "absent", "ring").is_err());
}

fn descriptor(inputs: &[(&str, &str)]) -> MethodDescriptor {
    MethodDescriptor {
        name: super::super::MESH_BUILD.into(),
        tools: BTreeMap::new(),
        flags: vec![
            "--bin=epr".into(),
            "--target=x86_64-unknown-linux-gnu".into(),
        ],
        params: None,
        exec: None,
        step: Some("mesh-build:eprfs".into()),
        embedder: None,
        view: Vec::new(),
        non_path_inputs: inputs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
        deterministic: false,
    }
}

const DECLARED_V1: [(&str, &str); 4] = [
    ("target.isa", "x86-64"),
    ("target.cflags", "-march=x86-64"),
    ("CFLAGS_x86_64_unknown_linux_gnu", "-march=x86-64"),
    ("CXXFLAGS_x86_64_unknown_linux_gnu", "-march=x86-64"),
];

#[test]
fn isa_c_accepts_only_a_declaration_the_record_applies() {
    assert!(matches!(declared(&descriptor(&[("CC", "gcc")])), Ok(None)));
    let realized = descriptor(&DECLARED_V1);
    let found = declared(&realized).unwrap().unwrap();
    assert_eq!((found.isa, found.march), ("x86-64", "-march=x86-64"));
    // The C compiler itself (CC) is chosen, not flagged: it may be recorded beside.
    let mut with_cc = DECLARED_V1.to_vec();
    with_cc.push(("CC", "gcc"));
    assert!(declared(&descriptor(&with_cc)).unwrap().is_some());
    for (change, why) in [
        (("target.cflags", "-O2"), "names no single -march"),
        (
            ("target.cflags", "-march=x86-64 -march=x86-64"),
            "names no single -march",
        ),
        (("target.isa", "x86-64-v2"), "not target.isa x86-64-v2"),
        (
            ("CFLAGS_x86_64_unknown_linux_gnu", "-march=x86-64 -O2"),
            "not -march=x86-64",
        ),
        (("CPPFLAGS", "-DX"), "C-family variable CPPFLAGS"),
        (("CC_x86_64_unknown_linux_gnu", "clang"), "CC_x86_64"),
        (("target.libc", "glibc"), "target.libc"),
    ] {
        let mut inputs = DECLARED_V1.to_vec();
        inputs.retain(|(name, _)| *name != change.0);
        inputs.push(change);
        let Err(missing) = declared(&descriptor(&inputs)) else {
            panic!("{change:?} was accepted");
        };
        assert!(
            missing.iter().any(|m| m.contains(why)),
            "{why}: {missing:?}"
        );
    }
}
