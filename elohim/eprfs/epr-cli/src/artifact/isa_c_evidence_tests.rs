use super::*;

const MARCH: &str = "-march=x86-64";
const BYTES: &[u8] = b"\x7fELF the bin these messages built";

/// One build of `bin` under a scratch target directory: its executable, and per build script the
/// `running:` lines and the objects it left.
struct Build {
    dir: tempfile::TempDir,
    lines: Vec<String>,
}

impl Build {
    fn new(bin: &str) -> Self {
        let dir = tempfile::TempDir::new().unwrap();
        let exe = dir.path().join(bin);
        std::fs::write(&exe, BYTES).unwrap();
        let lines = vec![serde_json::json!({
            "reason": "compiler-artifact",
            "package_id": format!("path+file:///repo/crate#{bin}@0.1.0"),
            "target": {"kind": ["bin"], "name": bin},
            "executable": exe,
            "fresh": true,
        })
        .to_string()];
        Self { dir, lines }
    }

    /// A build script of `package` (crates.io) that logged `running` and left `objects`; `run`
    /// keeps the output in cargo's newer `run/stdout`, else in `output`.
    fn script(mut self, package: &str, running: &[&str], objects: &[&str], run: bool) -> Self {
        let unit = self.dir.path().join("build").join(package).join("0123");
        let out = unit.join("out");
        std::fs::create_dir_all(&out).unwrap();
        for object in objects {
            std::fs::write(out.join(object), b"obj").unwrap();
        }
        let stdout = running
            .iter()
            .map(|line| {
                format!(
                    "running: {}\n",
                    line.replace("$OUT", &out.display().to_string())
                )
            })
            .collect::<String>()
            + "cargo:rerun-if-env-changed=CC_ENABLE_DEBUG_OUTPUT\n";
        let output = if run {
            std::fs::create_dir_all(unit.join("run")).unwrap();
            unit.join("run/stdout")
        } else {
            unit.join("output")
        };
        std::fs::write(output, stdout).unwrap();
        self.lines.push(
            serde_json::json!({
                "reason": "build-script-executed",
                "package_id": format!("{CRATES_IO}#{package}@1.0.0"),
                "out_dir": out,
            })
            .to_string(),
        );
        self
    }

    fn judge(&self, bin: &str, bytes: &[u8]) -> Result<Vec<String>, Vec<String>> {
        let messages = self.dir.path().join("messages.json");
        std::fs::write(&messages, self.lines.join("\n") + "\n").unwrap();
        judge(&messages, bytes, bin, MARCH)
    }
}

/// A gcc compile as cc 1.11.1 logs it, with `flags` before the output and the source last.
fn compile(flags: &str, source: &str) -> String {
    let flags: String = flags
        .split_whitespace()
        .map(|f| format!(" {f:?}"))
        .collect();
    format!(
        "\"gcc\" \"-O2\" \"-ffunction-sections\" \"-fPIC\" \"-m64\"{flags} \"-o\" \"$OUT/x.o\" \
         \"-c\" {source:?}"
    )
}

const ARCHIVE: &str = "\"ar\" \"cqD\" \"$OUT/libx.a\" \"$OUT/x.o\"";
const DETECT: &str = "\"gcc\" \"-E\" \"$OUT/detect_compiler_family.c\"";

fn has(found: &[String], needle: &str) -> bool {
    found.iter().any(|f| f.contains(needle))
}

#[test]
fn evidence_passes_every_compile_at_the_declared_march() {
    let ring = [
        DETECT.to_owned(),
        compile(
            "-march=x86-64 -mtune=generic -mno-red-zone -march=x86-64",
            "curve25519.c",
        ),
        compile("-march=x86-64", "aesni-x86_64-elf.S"),
        ARCHIVE.to_owned(),
    ];
    let ring: Vec<&str> = ring.iter().map(String::as_str).collect();
    let build = Build::new("epr")
        .script("ring", &ring, &["x.o", "libx.a"], true)
        .script("serde", &[], &[], false)
        .script(
            "libsqlite3-sys",
            &[&compile("-march=x86-64", "sqlite3.c")],
            &["x.o"],
            false,
        );
    let notes = build.judge("epr", BYTES).unwrap();
    assert!(
        has(&notes, "ring: 2 compile(s) at -march=x86-64"),
        "{notes:?}"
    );
    assert!(has(&notes, "libsqlite3-sys: 1 compile(s)"), "{notes:?}");
}

/// The review's regression (ruling 1(f)): one compile at the baseline and one above it in the same
/// build refuses, whichever crate carries the higher one.
#[test]
fn evidence_refuses_one_compile_above_the_baseline_beside_one_at_it() {
    for (above, token) in [
        ("-march=x86-64 -mavx2", "-mavx2"),
        ("-march=x86-64-v3", "-march=x86-64-v3"),
        ("-march=x86-64 -msse4.2", "-msse4.2"),
        ("-march=x86-64 -mcpu=znver2", "-mcpu=znver2"),
        ("-march=native", "-march=native"),
        ("-march=x86-64 -march=x86-64-v2", "-march=x86-64-v2"),
        ("-O2", "carries no -march"),
    ] {
        let same = [compile(MARCH, "a.c"), compile(above, "b.c")];
        let same: Vec<&str> = same.iter().map(String::as_str).collect();
        let found = Build::new("epr")
            .script("ring", &same, &["x.o"], true)
            .judge("epr", BYTES)
            .unwrap_err();
        assert!(has(&found, token), "{above}: {found:?}");
        assert!(has(&found, "ring's compile of b.c"), "{above}: {found:?}");
        let other = Build::new("epr")
            .script("ring", &[&compile(MARCH, "a.c")], &["x.o"], true)
            .script("zstd-sys", &[&compile(above, "zstd.c")], &["x.o"], false)
            .judge("epr", BYTES)
            .unwrap_err();
        assert!(
            has(&other, "zstd-sys's compile of zstd.c"),
            "{above}: {other:?}"
        );
    }
}

#[test]
fn evidence_refuses_objects_with_no_logged_compile() {
    let found = Build::new("epr")
        .script(
            "libsqlite3-sys",
            &[ARCHIVE],
            &["sqlite3.o", "libsqlite3.a"],
            true,
        )
        .judge("epr", BYTES)
        .unwrap_err();
    assert!(
        has(&found, "no evidence: libsqlite3-sys's build script left"),
        "{found:?}"
    );
    assert!(has(&found, "libsqlite3.a"), "{found:?}");
}

#[test]
fn evidence_is_bound_to_the_bytes_by_the_bins_executable() {
    let build = Build::new("epr").script("ring", &[&compile(MARCH, "a.c")], &["x.o"], true);
    let found = build.judge("epr", b"other bytes").unwrap_err();
    assert!(has(&found, "executable"), "{found:?}");
    assert!(has(&found, "is not --bytes"), "{found:?}");
    let found = build.judge("bin-launch", BYTES).unwrap_err();
    assert!(
        has(&found, "name no executable for the bin bin-launch"),
        "{found:?}"
    );
    let messages = build.dir.path().join("not-json");
    std::fs::write(&messages, "   Compiling ring v0.17.14\n").unwrap();
    let found = judge(&messages, BYTES, "epr", MARCH).unwrap_err();
    assert!(
        has(&found, "not cargo's --message-format=json"),
        "{found:?}"
    );
    let found = judge(&build.dir.path().join("absent"), BYTES, "epr", MARCH).unwrap_err();
    assert!(has(&found, "do not read"), "{found:?}");
}

/// blake3's AVX-512 assembly and cc's flag probe carry -mavx512f: its exception holds while every
/// compile is assembly or the probe, and refuses once it compiles C.
#[test]
fn an_exception_crate_passes_on_its_own_terms_and_no_further() {
    let blake3 = [
        DETECT.to_owned(),
        compile("-march=x86-64 -mavx512f -march=x86-64", "$OUT/flag_check.c"),
        compile("-march=x86-64 -std=c11", "c/blake3_avx2_x86-64_unix.S"),
        compile(
            "-march=x86-64 -mavx512f -mavx512vl",
            "c/blake3_avx512_x86-64_unix.S",
        ),
        ARCHIVE.to_owned(),
    ];
    let blake3: Vec<&str> = blake3.iter().map(String::as_str).collect();
    let notes = Build::new("epr")
        .script("blake3", &blake3, &["x.o", "libx.a"], true)
        .judge("epr", BYTES)
        .unwrap();
    assert!(has(&notes, "blake3 excepted (3 compile(s))"), "{notes:?}");
    let mut intrinsics = blake3.clone();
    let c = compile("-march=x86-64 -mavx512f", "c/blake3_avx512.c");
    intrinsics.push(&c);
    let found = Build::new("epr")
        .script("blake3", &intrinsics, &["x.o"], true)
        .judge("epr", BYTES)
        .unwrap_err();
    assert!(has(&found, "blake3's exception covers its CPUID-dispatched assembly, and it compiled c/blake3_avx512.c"), "{found:?}");
    // The same commands under another name are judged by the rule.
    let found = Build::new("epr")
        .script("blake3-fork", &blake3, &["x.o"], true)
        .judge("epr", BYTES)
        .unwrap_err();
    assert!(has(&found, "-mavx512f"), "{found:?}");
}

#[test]
fn bin_launch_with_only_exception_crates_passes() {
    let notes = Build::new("bin-launch")
        .script("stacker", &[], &[], true)
        .script("generator", &[], &[], false)
        .script(
            "blake3",
            &[&compile(
                "-march=x86-64 -mavx512f",
                "c/blake3_avx512_x86-64_unix.S",
            )],
            &["x.o"],
            true,
        )
        .judge("bin-launch", BYTES)
        .unwrap();
    assert!(has(&notes, "stacker excepted (0 compile(s))"), "{notes:?}");
    assert!(has(&notes, "generator excepted"), "{notes:?}");
    // A "compiles nothing" crate that compiled something does not hold its exception.
    let found = Build::new("bin-launch")
        .script(
            "stacker",
            &[&compile(MARCH, "src/arch/windows.c")],
            &["x.o"],
            true,
        )
        .judge("bin-launch", BYTES)
        .unwrap_err();
    assert!(
        has(&found, "stacker's exception says it compiles nothing"),
        "{found:?}"
    );
}

#[test]
fn evidence_reads_cc_command_lines_and_cargo_package_ids() {
    assert_eq!(
        command_words(r#""gcc" "-DX=\"a b\"" "c\\d" "\u{e9}""#),
        Some(vec![
            "gcc".into(),
            "-DX=\"a b\"".into(),
            "c\\d".into(),
            "\u{e9}".into()
        ])
    );
    assert_eq!(command_words(r#"gcc -c x.c"#), None);
    assert_eq!(command_words(r#""gcc" "-c"#), None);
    assert_eq!(
        package_of("registry+https://github.com/rust-lang/crates.io-index#ring@0.17.14"),
        ("ring".into(), CRATES_IO.into())
    );
    assert_eq!(
        package_of("path+file:///repo/elohim/eprfs/epr-cli#0.3.3").0,
        "epr-cli"
    );
    assert_eq!(
        package_of("ring 0.17.14 (registry+https://github.com/rust-lang/crates.io-index)"),
        ("ring".into(), CRATES_IO.into())
    );
    // A script whose output is in neither place cargo keeps it has no evidence.
    let build = Build::new("epr").script("ring", &[], &[], true);
    std::fs::remove_file(build.dir.path().join("build/ring/0123/run/stdout")).unwrap();
    let found = build.judge("epr", BYTES).unwrap_err();
    assert!(
        has(&found, "no evidence: ring's build script output"),
        "{found:?}"
    );
}

fn out_dir(build: &Build, package: &str) -> PathBuf {
    build
        .dir
        .path()
        .join("build")
        .join(package)
        .join("0123/out")
}

/// The coordinator's point 3: a stale log must not pass. A log older than an object it sits
/// beside (a cache restored from elsewhere) is no evidence, for an exception crate too.
#[test]
fn evidence_refuses_a_log_older_than_an_object_it_sits_beside() {
    let ring = [compile(MARCH, "a.c"), ARCHIVE.to_owned()];
    let ring: Vec<&str> = ring.iter().map(String::as_str).collect();
    for package in ["ring", "blake3"] {
        let build = Build::new("epr").script(package, &ring, &["x.o", "libx.a"], true);
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(3600);
        std::fs::File::options()
            .write(true)
            .open(out_dir(&build, package).join("x.o"))
            .unwrap()
            .set_modified(later)
            .unwrap();
        let found = build.judge("epr", BYTES).unwrap_err();
        assert!(
            has(
                &found,
                &format!("no evidence: {package}'s build script output")
            ),
            "{package}: {found:?}"
        );
        assert!(has(&found, "is older than x.o"), "{package}: {found:?}");
    }
}

/// Point 3(b): every object is the `-o` of a logged compile and every static library the
/// archive of a logged archiver command (cc 1.11.1 logs `ar` through the same `running:` line,
/// lib.rs:4032 and command_helpers.rs:644); one with no logged producer is no evidence.
#[test]
fn evidence_refuses_an_object_or_archive_no_logged_command_produced() {
    let ring = [compile(MARCH, "a.c"), ARCHIVE.to_owned()];
    let ring: Vec<&str> = ring.iter().map(String::as_str).collect();
    for (extra, package) in [
        ("extra.o", "ring"),
        ("libother.a", "ring"),
        ("extra.o", "blake3"),
        ("nested/extra.o", "ring"),
    ] {
        let objects = ["x.o", "libx.a", extra];
        let build = Build::new("epr");
        std::fs::create_dir_all(out_dir(&build, package).join("nested")).unwrap();
        let found = build
            .script(package, &ring, &objects, true)
            .judge("epr", BYTES)
            .unwrap_err();
        assert!(
            has(
                &found,
                &format!("no evidence: {package}'s OUT_DIR holds {extra}")
            ),
            "{extra}: {found:?}"
        );
    }
}

/// Every exception's reason is one sentence (genesis/orchestrator/c-baseline-pin.test.mjs holds
/// each against the crate's build.rs).
#[test]
fn every_exception_names_its_reason_in_one_sentence() {
    for exception in EXCEPTIONS {
        let reason = exception.reason;
        assert!(reason.len() > 40, "{}: {reason}", exception.package);
        assert!(reason.ends_with('.'), "{}: {reason}", exception.package);
        assert_eq!(
            reason.matches(". ").count(),
            0,
            "{}: one sentence: {reason}",
            exception.package
        );
    }
}
