use super::*;

use std::collections::BTreeSet;

const MARCH: &str = "-march=x86-64";
const BYTES: &[u8] = b"\x7fELF the bin these messages built";

/// The locked version of a crate as the fixtures name it: the exceptions' own, else 1.0.0.
fn version(package: &str) -> &'static str {
    match package {
        "blake3" => "1.8.5",
        "stacker" => "0.1.25",
        "generator" => "0.8.11",
        "iana-time-zone-haiku" => "0.1.2",
        _ => "1.0.0",
    }
}

fn id(package: &str) -> String {
    format!("{CRATES_IO}#{package}@{}", version(package))
}

/// A package as the roster names it (`cargo tree`'s `name vX.Y.Z`): `name@version`.
fn key(package: &str) -> String {
    format!("{package}@{}", version(package))
}

/// The roster parser reads `cargo tree --prefix none -f '{p}'` lines, sources and repeats too.
#[test]
fn evidence_reads_the_roster_from_cargo_trees_listing() {
    let tree = "elohim-epr-cli v0.3.3 (/repo/elohim/eprfs/epr-cli)\nring v0.17.14\n\
                serde_derive v1.0.228 (proc-macro)\nring v0.17.14 (*)\n\n";
    assert_eq!(
        tree_closure(tree).unwrap(),
        BTreeSet::from([
            "elohim-epr-cli@0.3.3".to_owned(),
            "ring@0.17.14".to_owned(),
            "serde_derive@1.0.228".to_owned(),
        ])
    );
    assert!(tree_closure("ring 0.17.14\n").is_err());
    assert!(tree_closure("ring\n").is_err());
}

/// One build of `bin` under a scratch target directory, as cargo's `--message-format=json`
/// stream names it: the bin's executable under `<target>/debug`, per build script its lib and
/// custom-build compiler-artifact rows, its build-script-executed row (the `running:` lines and
/// the objects it left), and a final `build-finished`. The expected roster is the bin's package
/// and every script's package.
struct Build {
    dir: tempfile::TempDir,
    bin: String,
    lines: Vec<String>,
    closure: BTreeSet<String>,
    /// The closure packages the lock's metadata gives a custom-build target (hardening 4).
    scripted: BTreeSet<String>,
    finished: Option<bool>,
}

impl Build {
    fn new(bin: &str) -> Self {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("debug")).unwrap();
        let exe = dir.path().join("debug").join(bin);
        std::fs::write(&exe, BYTES).unwrap();
        let package = format!("path+file:///repo/crate#{bin}@0.1.0");
        let lines = vec![serde_json::json!({
            "reason": "compiler-artifact",
            "package_id": package,
            "target": {"kind": ["bin"], "name": bin},
            "executable": exe,
            "fresh": true,
        })
        .to_string()];
        Self {
            dir,
            bin: bin.to_owned(),
            lines,
            closure: [format!("{bin}@0.1.0")].into(),
            scripted: BTreeSet::new(),
            finished: Some(true),
        }
    }

    fn root(&self) -> PathBuf {
        self.dir.path().join("debug")
    }

    /// A build script of `package` (crates.io) that logged `running` and left `objects`; `run`
    /// keeps the output in cargo's newer `run/stdout`, else in `output`.
    fn script(self, package: &str, running: &[&str], objects: &[&str], run: bool) -> Self {
        let root = self.root();
        self.script_in(&root, package, running, objects, run)
    }

    /// Drop every row of `reason` for `package` from the stream.
    fn without(mut self, reason: &str, package: &str) -> Self {
        let package_id = id(package);
        self.lines.retain(|line| {
            let row: serde_json::Value = serde_json::from_str(line).unwrap();
            !(row["reason"] == reason && row["package_id"] == package_id.as_str())
        });
        self
    }

    /// Drop `package`'s compiler-artifact row of target `kind` from the stream.
    fn without_kind(mut self, kind: &str, package: &str) -> Self {
        let package_id = id(package);
        self.lines.retain(|line| {
            let row: serde_json::Value = serde_json::from_str(line).unwrap();
            !(row["reason"] == "compiler-artifact"
                && row["package_id"] == package_id.as_str()
                && row["target"]["kind"][0] == kind)
        });
        self
    }

    fn finished(mut self, finished: Option<bool>) -> Self {
        self.finished = finished;
        self
    }

    /// [`Build::script`] with its unit under `root` (another target directory when not
    /// [`Build::root`]).
    fn script_in(
        mut self,
        root: &Path,
        package: &str,
        running: &[&str],
        objects: &[&str],
        run: bool,
    ) -> Self {
        let unit = root.join("build").join(package).join("0123");
        let out = unit.join("out");
        std::fs::create_dir_all(&out).unwrap();
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
        // As cargo leaves it: the output stamped with the script's start (cargo sets its mtime to
        // the invocation time), and every object the run wrote no older than that.
        std::fs::write(&output, stdout).unwrap();
        let started = std::time::SystemTime::now() - std::time::Duration::from_secs(2);
        std::fs::File::options()
            .write(true)
            .open(&output)
            .unwrap()
            .set_modified(started)
            .unwrap();
        for object in objects.iter().filter(|o| !o.ends_with(".a")) {
            std::fs::write(out.join(object), b"obj").unwrap();
        }
        // An archive is a real one, as cc's `ar cqD` writes it from the unit's objects.
        for archive in objects.iter().filter(|o| o.ends_with(".a")) {
            let members: Vec<PathBuf> = objects
                .iter()
                .filter(|o| o.ends_with(".o"))
                .map(|o| out.join(o))
                .collect();
            ar(&out, archive, &members);
        }
        let package_id = id(package);
        for kind in ["custom-build", "lib"] {
            self.lines.push(
                serde_json::json!({
                    "reason": "compiler-artifact",
                    "package_id": package_id,
                    "target": {"kind": [kind], "name": package},
                    "executable": null,
                    "fresh": true,
                })
                .to_string(),
            );
        }
        self.lines.push(
            serde_json::json!({
                "reason": "build-script-executed",
                "package_id": package_id,
                "out_dir": out,
            })
            .to_string(),
        );
        self.closure.insert(key(package));
        self
    }

    fn judge(&self, bin: &str, bytes: &[u8]) -> Result<Vec<String>, Vec<String>> {
        let messages = self.dir.path().join("messages.json");
        let mut lines = self.lines.clone();
        if let Some(success) = self.finished {
            lines.push(
                serde_json::json!({"reason": "build-finished", "success": success}).to_string(),
            );
        }
        std::fs::write(&messages, lines.join("\n") + "\n").unwrap();
        let expected = Expected {
            package: format!("path+file:///repo/crate#{}@0.1.0", self.bin),
            closure: self.closure.clone(),
            scripted: self.scripted.clone(),
        };
        judge(&messages, bytes, bin, MARCH, &expected)
    }
}

/// This host's binutils `ar` in cc's form (lib.rs:4032): `ar cqD <archive> <members>`.
fn ar(dir: &Path, archive: &str, members: &[PathBuf]) {
    let out = std::process::Command::new("ar")
        .arg("cqD")
        .arg(archive)
        .args(members)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "ar cqD {archive}: {out:?}");
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

/// [`compile`] writing `$OUT/<object>`.
fn compile_to(flags: &str, source: &str, object: &str) -> String {
    compile(flags, source).replace("\"$OUT/x.o\"", &format!("\"$OUT/{object}\""))
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
        compile_to(
            "-march=x86-64 -mtune=generic -mno-red-zone -march=x86-64",
            "curve25519.c",
            "curve25519.o",
        ),
        compile_to("-march=x86-64", "aesni-x86_64-elf.S", "aesni.o"),
        ARCHIVE.to_owned(),
    ];
    let ring: Vec<&str> = ring.iter().map(String::as_str).collect();
    let build = Build::new("epr")
        .script("ring", &ring, &["curve25519.o", "aesni.o", "libx.a"], true)
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
    let none = Expected::default();
    let found = judge(&messages, BYTES, "epr", MARCH, &none).unwrap_err();
    assert!(
        has(&found, "not cargo's --message-format=json"),
        "{found:?}"
    );
    let found = judge(&build.dir.path().join("absent"), BYTES, "epr", MARCH, &none).unwrap_err();
    assert!(has(&found, "do not read"), "{found:?}");
}

/// Re-review ruling 1: the stream must prove it is the whole build that produced these bytes. An
/// executable-only stream, or one missing a unit's rows, or one whose build did not finish, is not.
#[test]
fn evidence_refuses_a_stream_that_is_not_the_whole_build() {
    let ring = [compile(MARCH, "a.c"), ARCHIVE.to_owned()];
    let ring: Vec<&str> = ring.iter().map(String::as_str).collect();
    let full = || Build::new("epr").script("ring", &ring, &["x.o", "libx.a"], true);
    assert!(full().judge("epr", BYTES).is_ok(), "the whole build passes");
    // Only the bin's own compiler-artifact row: nothing judged.
    let mut only = Build::new("epr");
    only.closure.insert(id("ring"));
    let found = only.judge("epr", BYTES).unwrap_err();
    assert!(
        has(&found, "name no compiler-artifact row for"),
        "{found:?}"
    );
    // One build script's build-script-executed row removed (its custom-build compile stays).
    let found = full()
        .without("build-script-executed", "ring")
        .judge("epr", BYTES)
        .unwrap_err();
    assert!(
        has(
            &found,
            "custom-build compile and no build-script-executed row"
        ),
        "{found:?}"
    );
    // Both of one package's rows removed: the lock's roster names it.
    let found = full()
        .without("build-script-executed", "ring")
        .without("compiler-artifact", "ring")
        .judge("epr", BYTES)
        .unwrap_err();
    assert!(
        has(
            &found,
            &format!("compiler-artifact row for {}", key("ring"))
        ),
        "{found:?}"
    );
    // A build that did not finish, or did not succeed.
    for finished in [None, Some(false)] {
        let found = full().finished(finished).judge("epr", BYTES).unwrap_err();
        assert!(has(&found, "build-finished"), "{finished:?}: {found:?}");
    }
    // The bin row of another package than the one put.
    let mut other = full();
    other.bin = "elsewhere".into();
    let found = other.judge("epr", BYTES).unwrap_err();
    assert!(
        has(&found, "path+file:///repo/crate#elsewhere@0.1.0"),
        "{found:?}"
    );
}

/// Ruling 1: an out_dir outside the executable's target directory, or one that does not exist,
/// refuses; it never reads as an empty unit.
#[test]
fn evidence_refuses_an_out_dir_outside_the_target_or_absent() {
    let elsewhere = tempfile::TempDir::new().unwrap();
    let found = Build::new("epr")
        .script_in(elsewhere.path(), "ring", &[], &[], true)
        .judge("epr", BYTES)
        .unwrap_err();
    assert!(has(&found, "outside the target directory"), "{found:?}");
    let build = Build::new("epr").script("ring", &[], &[], true);
    std::fs::remove_dir_all(out_dir(&build, "ring")).unwrap();
    let found = build.judge("epr", BYTES).unwrap_err();
    assert!(has(&found, "does not exist"), "{found:?}");
}

/// blake3's AVX-512 assembly and cc's flag probe carry -mavx512f: its exception holds while every
/// compile is assembly or the probe, and refuses once it compiles C.
#[test]
fn an_exception_crate_passes_on_its_own_terms_and_no_further() {
    let blake3 = [
        DETECT.to_owned(),
        compile_to(
            "-march=x86-64 -mavx512f -march=x86-64",
            "$OUT/flag_check.c",
            "flag_check",
        ),
        compile_to(
            "-march=x86-64 -std=c11",
            "c/blake3_avx2_x86-64_unix.S",
            "avx2.o",
        ),
        compile_to(
            "-march=x86-64 -mavx512f -mavx512vl",
            "c/blake3_avx512_x86-64_unix.S",
            "avx512.o",
        ),
        ARCHIVE.to_owned(),
    ];
    let blake3: Vec<&str> = blake3.iter().map(String::as_str).collect();
    let notes = Build::new("epr")
        .script("blake3", &blake3, &["avx2.o", "avx512.o", "libx.a"], true)
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
    std::fs::remove_file(build.root().join("build/ring/0123/run/stdout")).unwrap();
    let found = build.judge("epr", BYTES).unwrap_err();
    assert!(
        has(&found, "no evidence: ring's build script output"),
        "{found:?}"
    );
}

fn out_dir(build: &Build, package: &str) -> PathBuf {
    build.root().join("build").join(package).join("0123/out")
}

/// The coordinator's point 3: a stale log must not pass. Cargo stamps a script's output with the
/// time the script started, so every object the run wrote is no older than that; an object older
/// than its log (left by another run, or a cache restored from elsewhere) is no evidence, for an
/// exception crate too. A fresh run (the helper's order) passes.
#[test]
fn evidence_refuses_an_object_older_than_the_run_its_log_records() {
    let ring = [compile(MARCH, "a.c"), ARCHIVE.to_owned()];
    let ring: Vec<&str> = ring.iter().map(String::as_str).collect();
    for package in ["ring", "blake3"] {
        let build = Build::new("epr").script(package, &ring, &["x.o", "libx.a"], true);
        let earlier = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
        std::fs::File::options()
            .write(true)
            .open(out_dir(&build, package).join("x.o"))
            .unwrap()
            .set_modified(earlier)
            .unwrap();
        let found = build.judge("epr", BYTES).unwrap_err();
        assert!(
            has(
                &found,
                &format!("no evidence: {package}'s build script output")
            ),
            "{package}: {found:?}"
        );
        assert!(
            has(&found, "x.o is older than the run"),
            "{package}: {found:?}"
        );
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

/// Re-review ruling 2: archive members are reconciled with the unit's judged compiles. cc's
/// `Build::object` chains a prebuilt object from outside OUT_DIR into the archive (cc-1.11.1
/// lib.rs:818, :3852); a genuine archive holding one baseline object and one unjudged external
/// member refuses, and so does a same-named member whose bytes are not the compile's output.
#[test]
fn evidence_reconciles_every_archive_member_with_a_judged_compile() {
    let running = [compile(MARCH, "a.c"), ARCHIVE.to_owned()];
    let running: Vec<&str> = running.iter().map(String::as_str).collect();
    // One baseline object and one external member, archived by a genuine `ar cqD`.
    let build = Build::new("epr").script("ring", &running, &["x.o"], true);
    let out = out_dir(&build, "ring");
    let elsewhere = tempfile::TempDir::new().unwrap();
    let prebuilt = elsewhere.path().join("prebuilt-avx2.o");
    std::fs::write(&prebuilt, b"vpaddd").unwrap();
    ar(&out, "libx.a", &[out.join("x.o"), prebuilt]);
    let found = build.judge("epr", BYTES).unwrap_err();
    assert!(
        has(
            &found,
            "member prebuilt-avx2.o of libx.a is not the output of a judged compile"
        ),
        "{found:?}"
    );
    // A same-named member whose bytes are not what the compile left at its -o path.
    let build = Build::new("epr").script("ring", &running, &["x.o", "libx.a"], true);
    std::fs::write(out_dir(&build, "ring").join("x.o"), b"other").unwrap();
    let found = build.judge("epr", BYTES).unwrap_err();
    assert!(
        has(
            &found,
            "member x.o of libx.a is not the bytes its compile left"
        ),
        "{found:?}"
    );
    // An archive only an index call (`ar sD`) names was not created by a logged archiver
    // command (any other command naming it refuses as unclassifiable, ruling 3).
    let index = [
        compile(MARCH, "a.c"),
        "\"ar\" \"sD\" \"$OUT/libx.a\"".to_owned(),
    ];
    let index: Vec<&str> = index.iter().map(String::as_str).collect();
    let found = Build::new("epr")
        .script("ring", &index, &["x.o", "libx.a"], true)
        .judge("epr", BYTES)
        .unwrap_err();
    assert!(
        has(
            &found,
            "OUT_DIR holds libx.a, which no logged archiver command created"
        ),
        "{found:?}"
    );
    // An archive that is not one, or not one this reader reads, refuses by name.
    let build = Build::new("epr").script("ring", &running, &["x.o", "libx.a"], true);
    std::fs::write(out_dir(&build, "ring").join("libx.a"), b"!<thin>\n").unwrap();
    let found = build.judge("epr", BYTES).unwrap_err();
    assert!(
        has(&found, "libx.a does not read: a thin archive"),
        "{found:?}"
    );
}

/// Re-review ruling 3: a compile whose effective options the check cannot read refuses as
/// unjudgeable, and says so: a response file, `-Xclang` (a hidden target feature), `-Xassembler`
/// and a `-Wa,` list.
#[test]
fn evidence_refuses_a_compile_whose_options_it_cannot_read() {
    for (flags, form) in [
        (
            "-march=x86-64 -Xclang -target-feature -Xclang +avx2",
            "-Xclang",
        ),
        ("-march=x86-64 -Wa,-mavx2", "-Wa,-mavx2"),
        ("-march=x86-64 -Xassembler -mavx2", "-Xassembler"),
        ("-march=x86-64 @extra.rsp", "@extra.rsp"),
    ] {
        let found = Build::new("epr")
            .script("ring", &[&compile(flags, "a.c")], &["x.o"], true)
            .judge("epr", BYTES)
            .unwrap_err();
        assert!(
            has(
                &found,
                &format!("unjudgeable: ring's compile of a.c carries {form}")
            ),
            "{form}: {found:?}"
        );
    }
}

/// Hardening 4: a closure package the lock's metadata gives a build script must have its
/// build-script-executed row, even when the stream also dropped its custom-build compile.
#[test]
fn evidence_refuses_a_scripted_package_whose_script_row_is_gone() {
    let ring = [compile(MARCH, "a.c")];
    let ring: Vec<&str> = ring.iter().map(String::as_str).collect();
    let mut build = Build::new("epr")
        .script("ring", &ring, &["x.o"], true)
        .without("build-script-executed", "ring")
        .without_kind("custom-build", "ring");
    build.scripted.insert(key("ring"));
    let found = build.judge("epr", BYTES).unwrap_err();
    assert!(
        has(
            &found,
            "ring@1.0.0 has a build script and the stream no build-script-executed row"
        ),
        "{found:?}"
    );
}

/// Hardening 3: an archive member whose judged object is no longer at its -o path has nothing to
/// be compared with, and refuses instead of passing by name; two compiles of one unit writing the
/// same object name cannot tell their members apart, and refuse.
#[test]
fn evidence_refuses_a_member_it_cannot_compare_or_tell_apart() {
    let running = [compile(MARCH, "a.c"), ARCHIVE.to_owned()];
    let running: Vec<&str> = running.iter().map(String::as_str).collect();
    let build = Build::new("epr").script("ring", &running, &["x.o", "libx.a"], true);
    std::fs::remove_file(out_dir(&build, "ring").join("x.o")).unwrap();
    let found = build.judge("epr", BYTES).unwrap_err();
    assert!(
        has(
            &found,
            "member x.o of libx.a has no object at its compile's -o path to compare"
        ),
        "{found:?}"
    );
    let twice = [
        compile_to(MARCH, "a.c", "x.o"),
        compile_to(MARCH, "sub/a.c", "sub/x.o"),
    ];
    let twice: Vec<&str> = twice.iter().map(String::as_str).collect();
    let build = Build::new("epr");
    std::fs::create_dir_all(out_dir(&build, "ring").join("sub")).unwrap();
    let found = build
        .script("ring", &twice, &["x.o", "sub/x.o"], true)
        .judge("epr", BYTES)
        .unwrap_err();
    assert!(
        has(&found, "2 compiles write an object named x.o"),
        "{found:?}"
    );
}

/// Hardening 2: more options whose effect on the instruction set the check cannot read refuse as
/// unjudgeable: a specs file, a compiler plugin, another compiler-program prefix, a `--march=`
/// spelling, a bare target feature and a wrapper.
#[test]
fn evidence_refuses_more_options_it_cannot_read() {
    for (flags, form) in [
        ("-march=x86-64 -specs extra.specs", "-specs"),
        ("-march=x86-64 -specs=extra.specs", "-specs=extra.specs"),
        ("-march=x86-64 -fplugin plugin.so", "-fplugin"),
        ("-march=x86-64 -fplugin=plugin.so", "-fplugin=plugin.so"),
        ("-march=x86-64 -B /opt/other/bin", "-B"),
        ("-march=x86-64 -B/opt/other/bin", "-B/opt/other/bin"),
        ("-march=x86-64 --march=x86-64-v3", "--march=x86-64-v3"),
        ("-march=x86-64 -target-feature +avx2", "-target-feature"),
        ("-march=x86-64 -wrapper valgrind", "-wrapper"),
    ] {
        let found = Build::new("epr")
            .script("ring", &[&compile(flags, "a.c")], &["x.o"], true)
            .judge("epr", BYTES)
            .unwrap_err();
        assert!(
            has(
                &found,
                &format!("unjudgeable: ring's compile of a.c carries {form}")
            ),
            "{form}: {found:?}"
        );
    }
}

/// Ruling 3: every logged command is a recognised compile, one of cc 1.11.1's own probes, or an
/// archiver line; anything else refuses. Options are read only up to a bare `--`.
#[test]
fn evidence_refuses_a_command_it_cannot_classify() {
    for (line, what) in [
        ("\"gcc\" \"-E\" \"$OUT/other.c\"", "-E"),
        ("\"gcc\" \"-S\" \"-o\" \"$OUT/x.s\" \"a.c\"", "-S"),
        (
            "\"gcc\" \"-march=x86-64\" \"-o\" \"$OUT/x.o\" \"--\" \"-c\" \"a.c\"",
            "--",
        ),
        ("\"nm\" \"$OUT/libx.a\"", "nm"),
        (
            "\"gcc\" \"-march=x86-64\" \"a.c\" \"-o\" \"$OUT/a.out\"",
            "a.out",
        ),
    ] {
        let found = Build::new("epr")
            .script("ring", &[line], &[], true)
            .judge("epr", BYTES)
            .unwrap_err();
        assert!(
            has(&found, "logged a command the check cannot classify"),
            "{what}: {found:?}"
        );
    }
    // cc's own probes pass: family detection, the cl-style probe, the version probes.
    let probes = [
        DETECT,
        "\"gcc\" \"-?\"",
        "\"gcc\" \"--version\"",
        "\"ar\" \"--version\"",
        "\"gcc\" \"--print-search-dirs\"",
    ];
    assert!(Build::new("epr")
        .script("ring", &probes, &[], true)
        .judge("epr", BYTES)
        .is_ok());
}

/// Hardening 1: a probe matches cc 1.11.1's exact shape, the arguments cc gives it and nothing
/// else, so no `-o` and no flag can ride along on one; the second -E family form (`-E --
/// <file>`, tool.rs:338-341) passes.
#[test]
fn a_probe_matches_ccs_exact_shape() {
    for line in [
        "\"gcc\" \"-o\" \"$OUT/x.o\" \"-?\"",
        "\"gcc\" \"-march=x86-64-v3\" \"-?\"",
        "\"gcc\" \"-o\" \"$OUT/x.o\" \"--version\"",
        "\"gcc\" \"--version\" \"--version\"",
        "\"gcc\" \"-o\" \"$OUT/x.o\" \"--print-search-dirs\"",
        "\"gcc\" \"-o\" \"$OUT/x.o\" \"-E\" \"$OUT/detect_compiler_family.c\"",
        "\"gcc\" \"-mavx2\" \"-E\" \"$OUT/detect_compiler_family.c\"",
        "\"ar\" \"-o\" \"$OUT/x.o\" \"--version\"",
    ] {
        let found = Build::new("epr")
            .script("ring", &[line], &[], true)
            .judge("epr", BYTES)
            .unwrap_err();
        assert!(
            has(&found, "logged a command the check cannot classify"),
            "{line}: {found:?}"
        );
    }
    let delimited = "\"gcc\" \"-E\" \"--\" \"$OUT/detect_compiler_family.c\"";
    assert!(Build::new("epr")
        .script("ring", &[delimited], &[], true)
        .judge("epr", BYTES)
        .is_ok());
}

/// Ruling 3: an exception matches the package's name, source and locked version, and blake3's
/// covers exactly the assembly its build.rs names for x86_64 unix and cc's probe.
#[test]
fn an_exception_holds_only_for_its_version_and_reviewed_sources() {
    let extra = compile("-march=x86-64 -mavx512f", "c/blake3_avx10_x86-64_unix.S");
    let found = Build::new("epr")
        .script("blake3", &[&extra], &["x.o"], true)
        .judge("epr", BYTES)
        .unwrap_err();
    assert!(
        has(
            &found,
            "compiled c/blake3_avx10_x86-64_unix.S, which it does not review"
        ),
        "{found:?}"
    );
    // Another version of blake3 is judged by the rule, which refuses its -mavx512f.
    let reviewed = compile("-march=x86-64 -mavx512f", "c/blake3_avx512_x86-64_unix.S");
    let mut build = Build::new("epr").script("blake3", &[&reviewed], &["x.o"], true);
    let current = id("blake3");
    let other = current.replace("@1.8.5", "@1.8.6");
    build.lines = build
        .lines
        .iter()
        .map(|l| l.replace(&current, &other))
        .collect();
    build.closure = build
        .closure
        .iter()
        .map(|l| l.replace("blake3@1.8.5", "blake3@1.8.6"))
        .collect();
    let found = build.judge("epr", BYTES).unwrap_err();
    assert!(has(&found, "carries -mavx512f"), "{found:?}");
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
