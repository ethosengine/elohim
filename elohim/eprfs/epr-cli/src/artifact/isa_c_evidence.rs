//! The C level of the build that produced the bytes, read from that build's own compile commands
//! (backlog steward-device-arrival row 29.2; L2a review ruling 1): the `env:isa-c` marker means
//! every cc-built crate in the build has affirmative evidence of the declared baseline, or is a
//! named [`EXCEPTIONS`] entry.
//!
//! The evidence is cc's own command log. With `CC_ENABLE_DEBUG_OUTPUT` set (a declared gate
//! export, pool-policy.json declared_targets `env`), cc 1.11.1 prints `running: <command>` for
//! every command it spawns (command_helpers.rs:644, lib.rs:2160 for its flag probes) to the build
//! script's stdout, which cargo keeps beside the script's `OUT_DIR` (`<out_dir>/../output`, or
//! `<out_dir>/../run/stdout` in cargo's newer build-dir layout), and cc reruns the script when the
//! variable changes (`cargo:rerun-if-env-changed=CC_ENABLE_DEBUG_OUTPUT`, command_helpers.rs:153).
//! The producer replays the gate's build with `--message-format=json` (a no-op over fresh units,
//! which still emit `compiler-artifact` and `build-script-executed`) and hands put those lines.
//!
//! The rule, per build script of that build:
//! - every `running:` line is a compile (`-c` among its options, which are read only up to a bare
//!   `--`, an `-o` output, no `-E` or `-S`), one of cc's own probes, or an archiver line
//!   ([`classify`]); anything else refuses, and so does a compile carrying an option whose effect
//!   the check cannot read (`@file`, `-Xclang`, `-Xassembler`, `-Wa,…`: [`opaque`]);
//! - each compile carries at least one `-march`, every `-march` is the declared one, and no other
//!   `-m` token selects an ISA: `-m64`, `-mtune=*` and `-mno-*` do not; every other `-m` token
//!   refuses (`-mavx2`, `-msse4.2`, `-mcpu=*`, `-march=native` among them);
//! - a script whose `OUT_DIR` holds `.o` or `.a` files and that logged no compile has no evidence
//!   and refuses, and so does one holding an object older than the run its log records, or an
//!   object no logged command produced ([`log_findings`]: a stale or restored log);
//! - an [`EXCEPTIONS`] crate (matched by name, crates.io source and locked version) is judged by
//!   its own exception instead, which still reads its commands: CPUID-dispatched assembly
//!   compiles only the sources it names (and cc's flag probe), and a crate that compiles nothing
//!   on a declared target compiled nothing.
//!
//! Residuals, recorded rather than solved: an assembly unit's instructions are its source's, not
//! its `-march`'s, so a compile of a `.S` at the declared `-march` is evidence of the command, not
//! of the instructions (ring's CPUID-dispatched assembly is accepted on that evidence); a
//! compiler wrapper's injected options are not seen (an option outside the `-m` namespace that the
//! log does show and the check can name refuses as unjudgeable, ruling 3); every
//! build script in the replayed build is judged, which is a superset of one bin's closure; and the
//! C source itself can select instructions the command does not show: `__attribute__((target))`,
//! `#pragma GCC target` and function multiversioning (`target_clones`) are not read, so a compile
//! at the declared `-march` is evidence of the command's baseline, not of every function in it.
//!
//! These are limits of command evidence: a build's commands can show what it asked the compiler
//! for, never every instruction the bytes hold. The second line is the receiver's: before it
//! adopts a drawn release, the sibling runs the staged `epr doctor --exercise` on its own CPU,
//! which executes the C paths the bin depends on (ring's x25519, SQLite) where a missing
//! instruction would trap.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use eprfs_core::{BlobCid, MethodDescriptor};

use super::{cargo_project, host, isa_c_archive, CRATE_INPUT};

const DECLARED: &str = "missing declared-target";

/// What the build of these bytes must show (re-review ruling 1): the cargo package id of the bin's
/// package (the crate at `--crate`), and every package in that package's dependency closure for
/// the declared triple, as `name@version`, each of which the build must have compiled.
#[derive(Debug, Clone, Default)]
pub(super) struct Expected {
    pub package: String,
    pub closure: BTreeSet<String>,
}

/// The [`Expected`] roster of `descriptor`'s build, from the lock's resolve for its triple.
///
/// The coordinator's decision (L2a re-review, ruling 1): the raw lock closure over-states, since
/// it holds platform-gated packages never built on Linux (iana-time-zone-haiku is in epr's lock
/// closure and is not compiled), so "every lock-closure package has a row" would refuse every
/// real build. The decision was `cargo metadata --format-version 1 --locked --offline
/// --filter-platform <triple>` in the crate's workspace (put already requires a clean,
/// HEAD-matching selected input set) and the bin package's normal and build closure in its
/// resolve; that names the bin's package id here, and the closure itself comes from `cargo tree`
/// (below: the resolve over-stated on the real build). If either cannot run, the roster is
/// unknown and put refuses with the reason.
pub(super) fn expected(repo: &Path, descriptor: &MethodDescriptor) -> Result<Expected, String> {
    let crate_dir = descriptor
        .non_path_inputs
        .get(CRATE_INPUT)
        .map_or(".", String::as_str);
    let crate_path = repo.join(crate_dir);
    let features = descriptor
        .flags
        .iter()
        .find_map(|f| f.strip_prefix("--features="));
    let metadata = cargo_project::metadata(&crate_path, host::target(descriptor), features)
        .map_err(|e| {
            format!(
                "{DECLARED}: the build's roster is unread: cargo metadata for {crate_dir} \
                 failed: {e}"
            )
        })?;
    let manifest = crate_path.join("Cargo.toml");
    let same = |path: &str| {
        let path = Path::new(path);
        path == manifest
            || path
                .canonicalize()
                .ok()
                .is_some_and(|p| Some(p) == manifest.canonicalize().ok())
    };
    let package = metadata["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|p| p["manifest_path"].as_str().is_some_and(same))
        .and_then(|p| p["id"].as_str())
        .ok_or_else(|| {
            format!(
                "{DECLARED}: the build's roster is unread: cargo metadata names no package at \
                 {crate_dir}/Cargo.toml"
            )
        })?
        .to_owned();
    // The closure, as the bin's own build resolves it (the coordinator's caution 1, ratified):
    // cargo metadata resolves features for the whole workspace, so its resolve named optional
    // dependencies the gate's build of the bin never compiles (sha1, sha3, ripemd and some 150
    // more refused the real build on this host); `cargo tree -p <pkg> -e normal,build --target
    // <triple>` resolves features as that build does. Never the stream, which is what is checked.
    let spec = format!("{}@{}", package_of(&package).0, version_of(&package));
    let mut args = vec![
        "tree",
        "--locked",
        "--offline",
        "-p",
        spec.as_str(),
        "-e",
        "normal,build",
        "--target",
        host::target(descriptor),
        "--prefix",
        "none",
        "-f",
        "{p}",
    ];
    if let Some(features) = features {
        args.extend(["--features", features]);
    }
    let tree = super::tool_line("cargo", &args, &crate_path).map_err(|e| {
        format!("{DECLARED}: the build's roster is unread: cargo tree for {spec} failed: {e}")
    })?;
    let closure = tree_closure(&tree)?;
    if closure.is_empty() {
        return Err(format!(
            "{DECLARED}: the build's roster is unread: cargo tree for {spec} names no package"
        ));
    }
    Ok(Expected { package, closure })
}

/// The `name@version` of every package a `cargo tree --prefix none -f '{p}'` listing names:
/// `name vX.Y.Z`, optionally followed by a source in parentheses and `(*)` for a repeat.
fn tree_closure(tree: &str) -> Result<BTreeSet<String>, String> {
    let mut closure = BTreeSet::new();
    for line in tree.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let mut words = line.split_whitespace();
        let (Some(name), Some(version)) = (words.next(), words.next()) else {
            return Err(format!(
                "{DECLARED}: the build's roster is unread: cargo tree printed {line:?}"
            ));
        };
        let Some(version) = version.strip_prefix('v') else {
            return Err(format!(
                "{DECLARED}: the build's roster is unread: cargo tree printed {line:?}"
            ));
        };
        closure.insert(format!("{name}@{version}"));
    }
    Ok(closure)
}

/// How an exception is held against the build's own commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExceptionKind {
    /// Its compiles are assembly units, each written for one level and chosen at run time by
    /// CPUID; their `-m` flags let the assembler accept the higher paths.
    DispatchedAssembly,
    /// Its build script compiles nothing on a declared target.
    CompilesNothing,
}

/// A crates.io crate judged by its own exception rather than by the `-march` rule.
#[derive(Debug)]
pub(super) struct Exception {
    pub package: &'static str,
    /// The locked version reviewed (re-review ruling 3); another version is judged by the rule.
    pub version: &'static str,
    pub kind: ExceptionKind,
    /// For dispatched assembly, the reviewed sources exactly as its build.rs names them (cc's
    /// flag probe aside); any other compile under the exception refuses.
    pub sources: &'static [&'static str],
    /// One sentence, held against the crate's build.rs by genesis/orchestrator/c-baseline-pin.test.mjs.
    pub reason: &'static str,
}

/// The named exceptions (ruling 1(d), re-review ruling 3); genesis/orchestrator/
/// c-baseline-pin.test.mjs reads this list, holds each version against Cargo.lock and each
/// reason and source list against the locked crate's build.rs.
pub(super) const EXCEPTIONS: &[Exception] = &[
    Exception {
        package: "blake3",
        version: "1.8.5",
        kind: ExceptionKind::DispatchedAssembly,
        sources: &[
            "c/blake3_sse2_x86-64_unix.S",
            "c/blake3_sse41_x86-64_unix.S",
            "c/blake3_avx2_x86-64_unix.S",
            "c/blake3_avx512_x86-64_unix.S",
        ],
        reason:
            "On x86_64 Linux without the pure or prefer_intrinsics features, blake3's build.rs \
                 compiles only its SSE2, SSE4.1, AVX2 and AVX-512 assembly (.S), passing \
                 -mavx512f and -mavx512vl so the assembler accepts the AVX-512 file, and the \
                 crate chooses each path at run time by CPUID.",
    },
    Exception {
        package: "stacker",
        version: "0.1.25",
        kind: ExceptionKind::CompilesNothing,
        sources: &[],
        reason: "stacker's build.rs runs cc only for src/arch/windows.c when the target is \
                 Windows, so it compiles nothing on a declared Linux target.",
    },
    Exception {
        package: "generator",
        version: "0.8.11",
        kind: ExceptionKind::CompilesNothing,
        sources: &[],
        reason: "generator's build.rs runs cc only for its powerpc64 context-switch assembly, so \
                 it compiles nothing on a declared x86_64 target.",
    },
    Exception {
        package: "iana-time-zone-haiku",
        version: "0.1.2",
        kind: ExceptionKind::CompilesNothing,
        sources: &[],
        reason: "iana-time-zone depends on iana-time-zone-haiku only under cfg(target_os = \
                 \"haiku\"), so its build script never runs, and nothing is compiled, for a \
                 declared Linux target.",
    },
];

const CRATES_IO: &str = "registry+https://github.com/rust-lang/crates.io-index";

/// A cargo package id as its name and source, in either spelling cargo writes:
/// `<source>#<name>@<version>` (or `<source>#<version>` when the name is the URL's last path
/// segment), or the older `<name> <version> (<source>)`.
fn package_of(id: &str) -> (String, String) {
    if let Some((source, fragment)) = id.rsplit_once('#') {
        let name = match fragment.split_once('@') {
            Some((name, _)) => name.to_owned(),
            None => source
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .to_owned(),
        };
        return (name, source.to_owned());
    }
    let mut parts = id.split_whitespace();
    let name = parts.next().unwrap_or_default().to_owned();
    let source = id
        .split_once('(')
        .and_then(|(_, rest)| rest.strip_suffix(')'))
        .unwrap_or_default()
        .to_owned();
    (name, source)
}

/// A cargo package id's version, in either spelling [`package_of`] reads; empty when it names
/// none.
fn version_of(id: &str) -> String {
    match id.rsplit_once('#') {
        Some((_, fragment)) => fragment
            .rsplit_once('@')
            .map_or(fragment, |(_, version)| version)
            .to_owned(),
        None => id.split_whitespace().nth(1).unwrap_or_default().to_owned(),
    }
}

/// The words of one `running: <command>` line: cc prints the program and each argument with
/// `{:?}` (command_helpers.rs:699), a Rust string literal each. `None` when the line is not that.
pub(super) fn command_words(line: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut chars = line.chars().peekable();
    loop {
        while chars.peek() == Some(&' ') {
            chars.next();
        }
        match chars.next() {
            None => return Some(words),
            Some('"') => {}
            Some(_) => return None,
        }
        let mut word = String::new();
        loop {
            match chars.next()? {
                '"' => break,
                '\\' => match chars.next()? {
                    'n' => word.push('\n'),
                    't' => word.push('\t'),
                    'r' => word.push('\r'),
                    '0' => word.push('\0'),
                    '\\' => word.push('\\'),
                    '"' => word.push('"'),
                    '\'' => word.push('\''),
                    'x' => {
                        let hex: String = [chars.next()?, chars.next()?].iter().collect();
                        let byte = u8::from_str_radix(&hex, 16).ok()?;
                        word.push(if byte.is_ascii() {
                            char::from(byte)
                        } else {
                            char::REPLACEMENT_CHARACTER
                        });
                    }
                    'u' => {
                        if chars.next()? != '{' {
                            return None;
                        }
                        let mut hex = String::new();
                        loop {
                            match chars.next()? {
                                '}' => break,
                                c => hex.push(c),
                            }
                        }
                        word.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                    }
                    _ => return None,
                },
                c => word.push(c),
            }
        }
        words.push(word);
    }
}

/// A command's options: its words after the program, up to a bare `--` (re-review ruling 3:
/// after it every word is an operand, never an option).
fn options(words: &[String]) -> &[String] {
    let rest = words.get(1..).unwrap_or_default();
    let end = rest.iter().position(|w| w == "--").unwrap_or(rest.len());
    &rest[..end]
}

/// What a logged `running:` line is (re-review ruling 3).
#[derive(Debug, PartialEq, Eq)]
enum Command {
    /// A compile: `-c` among its options, an `-o` output, and no `-E` or `-S` (cc's object
    /// compile, lib.rs:2640; its flag probe compiles `<OUT_DIR>/flag_check.c` the same way,
    /// lib.rs:1948/1966/2139, and is judged like any compile).
    Compile,
    /// One of cc 1.11.1's own probes, which compile nothing that reaches the bytes: family
    /// detection `<cc> -E <OUT_DIR>/…detect_compiler_family.c` (tool.rs:309/320), the cl-style
    /// probe `<cc> [args…] -?` (tool.rs:267), `<cc> --version` (tool.rs:238, and the archiver's
    /// at lib.rs:4846/4919), and `<cc> --print-search-dirs` (lib.rs:5769).
    Probe,
    /// cc's archiver or index call (lib.rs:4032-4046, 3930-3939) or a ranlib.
    Archiver,
}

/// The [`Command`] a logged line is, or why the check cannot classify it.
fn classify(words: &[String]) -> Result<Command, String> {
    let program = words
        .first()
        .and_then(|p| Path::new(p).file_name())
        .and_then(|p| p.to_str())
        .unwrap_or_default();
    let opts = options(words);
    let is = |list: &[&str]| opts.len() == list.len() && opts.iter().zip(list).all(|(a, b)| a == b);
    if program == "ar" || program == "llvm-ar" || program.ends_with("-ar") {
        return if is(&["--version"])
            || (opts
                .iter()
                .any(|w| matches!(w.as_str(), "cq" | "cqD" | "s" | "sD"))
                && !opts.iter().any(|w| w == "-o"))
        {
            Ok(Command::Archiver)
        } else {
            Err("an archiver call that is not cc's".into())
        };
    }
    if program == "ranlib" || program.ends_with("-ranlib") {
        return Ok(Command::Archiver);
    }
    // cc 1.11.1's probes, each exactly as cc builds it (hardening 1): the program and the
    // arguments cc gives it, nothing else, so no `-o` and no flag rides along on one.
    let args: Vec<&str> = words.iter().skip(1).map(String::as_str).collect();
    let family = |file: &str| file.ends_with("detect_compiler_family.c");
    let probe = match args.as_slice() {
        // Family detection: `<cc> -E <file>` (tool.rs:320), or `<cc> -E -- <file>` after a
        // -Wslash-u-filename warning (tool.rs:338-341).
        ["-E", file] | ["-E", "--", file] => family(file),
        // The cl-style probe `<cc> -?` (tool.rs:263-268; cc passes the compiler's own arguments
        // before it, none for a plain CC), `<cc> --version` (tool.rs:236-238) and
        // `<cc> --print-search-dirs` (lib.rs:5768-5769).
        ["-?"] | ["--version"] | ["--print-search-dirs"] => true,
        _ => false,
    };
    if probe {
        return Ok(Command::Probe);
    }
    let has = |flag: &str| opts.iter().any(|w| w == flag);
    if has("-c") && !has("-E") && !has("-S") && opts.windows(2).any(|w| w[0] == "-o") {
        return Ok(Command::Compile);
    }
    Err("neither a compile, one of cc's probes nor an archiver call".into())
}

/// The option of a compile that hides what it passes to the compiler, assembler or a file:
/// a response file (`@file`), `-Xclang`, `-Xassembler` or a `-Wa,` list (re-review ruling 3).
fn opaque(words: &[String]) -> Option<&str> {
    options(words)
        .iter()
        .find(|w| {
            w.starts_with('@') || *w == "-Xclang" || *w == "-Xassembler" || w.starts_with("-Wa,")
        })
        .map(String::as_str)
}

/// Every reason one compile command is not at `march` alone.
pub(super) fn compile_findings(package: &str, words: &[String], march: &str) -> Vec<String> {
    let mut findings = Vec::new();
    let mut marched = false;
    let shown = || {
        let source = words.last().map_or("", String::as_str);
        format!("{package}'s compile of {source}")
    };
    for word in options(words) {
        if word.starts_with("-march=") {
            marched = true;
            if word != march {
                findings.push(format!(
                    "{DECLARED}: the C is not at {march}: {} carries {word}",
                    shown()
                ));
            }
        } else if word.starts_with("-m")
            && word != "-m64"
            && !word.starts_with("-mtune=")
            && !word.starts_with("-mno-")
        {
            findings.push(format!(
                "{DECLARED}: the C is not at {march} alone: {} carries {word}",
                shown()
            ));
        }
    }
    if !marched {
        findings.push(format!(
            "{DECLARED}: the C level is unread: {} carries no -march",
            shown()
        ));
    }
    findings
}

/// The build script's stdout as cargo keeps it, beside its `OUT_DIR`.
fn script_output(out_dir: &Path) -> Option<PathBuf> {
    let unit = out_dir.parent()?;
    [unit.join("output"), unit.join("run").join("stdout")]
        .into_iter()
        .find(|path| path.is_file())
}

/// Compiled objects and static archives under `dir`, relative and sorted.
fn objects(root: &Path) -> Result<Vec<String>, String> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    let unread = |dir: &Path, e: std::io::Error| {
        format!(
            "{DECLARED}: no evidence: {} does not read: {e}",
            dir.display()
        )
    };
    while let Some(dir) = pending.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|e| unread(&dir, e))?;
        for entry in entries {
            let entry = entry.map_err(|e| unread(&dir, e))?;
            let path = entry.path();
            let kind = entry.file_type().map_err(|e| unread(&path, e))?;
            if kind.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "o" || ext == "a") {
                let shown = path.strip_prefix(root).unwrap_or(&path);
                found.push(shown.display().to_string());
            }
        }
    }
    found.sort();
    Ok(found)
}

/// Whether a compile is one an exception reviewed (re-review ruling 3): its source is one of the
/// exception's named sources, or cc's flag-support probe in the script's `OUT_DIR`
/// (`flag_check.c`, lib.rs:1948/1966, never archived). No suffix admits a source by itself.
fn reviewed(words: &[String], out_dir: &Path, sources: &[&str]) -> bool {
    let Some(source) = words.last() else {
        return false;
    };
    let probe = Path::new(source).parent() == Some(out_dir)
        && Path::new(source)
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                matches!(name, "flag_check.c" | "flag_check.cpp" | "flag_check.cu")
            });
    probe || sources.contains(&source.as_str())
}

/// What one build script showed: its package, its compile commands, the objects it left.
struct Script {
    package: String,
    source: String,
    version: String,
    out_dir: PathBuf,
    /// The script's stdout as cargo kept it.
    output: PathBuf,
    /// Every logged command; `compiles` are those carrying `-c`.
    commands: Vec<Vec<String>>,
    compiles: Vec<Vec<String>>,
    objects: Vec<String>,
}

/// A stale log must not pass (coordinator ruling on the L2a rework, point 3). Cargo reruns a
/// build script when its fingerprint or a `rerun-if-*` input changes (cc declares
/// `rerun-if-env-changed=CC_ENABLE_DEBUG_OUTPUT`, command_helpers.rs:153) and writes the
/// script's output when it exits, stamping its mtime with the time the script STARTED (observed
/// on this host with cargo 1.100: `run/stdout` carries exactly `run/invoked.timestamp`'s mtime,
/// while the objects it logged are seconds newer), so a fresh build pairs that output with what
/// the run left in OUT_DIR. A target directory restored from elsewhere can break the pairing
/// without cargo noticing; these checks cover it: (a) no object or archive in OUT_DIR is older
/// than the run its output records (one left by an earlier run, or copied in, is), and (b) every
/// `.o` is the `-o` of a logged compile and every `.a` the archive a logged archiver command
/// CREATED (cc 1.11.1 logs `ar` through the same `running:` line, lib.rs:4032 and
/// command_helpers.rs:644). Any object with no logged producer is no evidence.
///
/// Re-review ruling 2: every member of such an archive is the output (the basename of the `-o`)
/// of a judged compile of the same unit, and where that object still sits at its `-o` path, the
/// member is its bytes. cc's `Build::object` chains prebuilt objects from anywhere into the
/// archive (cc-1.11.1 lib.rs:818, :3852); such a member is allowed only as a named
/// [`PREBUILT_MEMBERS`] entry.
fn log_findings(script: &Script) -> Vec<String> {
    let mut findings = Vec::new();
    let logged = std::fs::metadata(&script.output).and_then(|m| m.modified());
    for object in &script.objects {
        let path = script.out_dir.join(object);
        let made = std::fs::metadata(&path).and_then(|m| m.modified());
        let stale = match (&logged, made) {
            (Ok(started), Ok(made)) => made < *started,
            _ => true,
        };
        if stale {
            findings.push(format!(
                "{DECLARED}: no evidence: {}'s build script output {} records a run, and \
                 {object} is older than the run (left by another run, or a restored cache)",
                script.package,
                script.output.display()
            ));
            break;
        }
    }
    // Every judged compile's output, by basename, with the path it was written to.
    let outputs: Vec<(String, &str)> = script
        .compiles
        .iter()
        .filter_map(|words| {
            let path = words.windows(2).find(|w| w[0] == "-o")?[1].as_str();
            let name = Path::new(path).file_name()?.to_str()?.to_owned();
            Some((name, path))
        })
        .collect();
    for object in &script.objects {
        let path = script.out_dir.join(object).display().to_string();
        if object.ends_with(".o") {
            if !outputs.iter().any(|(_, written)| *written == path) {
                findings.push(format!(
                    "{DECLARED}: no evidence: {}'s OUT_DIR holds {object}, which no logged \
                     command produced",
                    script.package
                ));
            }
            continue;
        }
        if !script
            .commands
            .iter()
            .any(|words| archive_created(words).is_some_and(|created| created == path))
        {
            findings.push(format!(
                "{DECLARED}: no evidence: {}'s OUT_DIR holds {object}, which no logged archiver \
                 command created",
                script.package
            ));
            continue;
        }
        let members = std::fs::read(&path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| isa_c_archive::members(&bytes));
        let members = match members {
            Ok(members) => members,
            Err(why) => {
                findings.push(format!(
                    "{DECLARED}: no evidence: {}'s {object} does not read: {why}",
                    script.package
                ));
                continue;
            }
        };
        for member in members {
            let compiled: Vec<&str> = outputs
                .iter()
                .filter(|(name, _)| *name == member.name)
                .map(|(_, written)| *written)
                .collect();
            if compiled.is_empty() {
                let named = PREBUILT_MEMBERS.iter().any(|p| {
                    p.package == script.package
                        && p.version == script.version
                        && p.member == member.name
                        && script.source == CRATES_IO
                        // Allowed only with its reason (the pin test holds it).
                        && !p.reason.trim().is_empty()
                });
                if !named {
                    findings.push(format!(
                        "{DECLARED}: no evidence: {}'s member {} of {object} is not the output \
                         of a judged compile (a prebuilt object chained in by Build::object?)",
                        script.package, member.name
                    ));
                }
                continue;
            }
            // Where the compile's object is still at its -o path, the member is its bytes.
            let held: Vec<Vec<u8>> = compiled
                .iter()
                .filter_map(|p| std::fs::read(p).ok())
                .collect();
            if !held.is_empty() && !held.contains(&member.data) {
                findings.push(format!(
                    "{DECLARED}: no evidence: {}'s member {} of {object} is not the bytes its \
                     compile left at its -o path",
                    script.package, member.name
                ));
            }
        }
    }
    findings
}

/// The archive a logged `running:` line creates: cc's archiver call `<ar> [flags…] cq|cqD <dst>
/// <objs…>` (cc-1.11.1 lib.rs:4032, :4035, :4038, :4046), `<ar>` named `ar`, `*-ar` or `llvm-ar`.
/// An index call (`ar s|sD <dst>`, lib.rs:3930–3939) or any other command creates nothing.
pub(super) fn archive_created(words: &[String]) -> Option<&str> {
    let program = Path::new(words.first()?).file_name()?.to_str()?;
    if !(program == "ar" || program == "llvm-ar" || program.ends_with("-ar")) {
        return None;
    }
    let mode = words.iter().skip(1).position(|w| w == "cq" || w == "cqD")?;
    words.get(mode + 2).map(String::as_str)
}

/// A prebuilt archive member allowed by name, with its reason (re-review ruling 2); none today.
/// genesis/orchestrator/c-baseline-pin.test.mjs reads this list.
#[derive(Debug)]
pub(super) struct PrebuiltMember {
    pub package: &'static str,
    pub version: &'static str,
    pub member: &'static str,
    pub reason: &'static str,
}

pub(super) const PREBUILT_MEMBERS: &[PrebuiltMember] = &[];

fn read_script(message: &serde_json::Value, root: &Path) -> Result<Script, String> {
    let id = message["package_id"].as_str().unwrap_or_default();
    let (package, source) = package_of(id);
    let out_dir = message["out_dir"].as_str().map(PathBuf::from).ok_or_else(|| {
        format!("{DECLARED}: no evidence: the build-script-executed message of {id} names no out_dir")
    })?;
    // Ruling 1: an out_dir that does not exist, or lies outside the executable's target
    // directory, refuses; it never reads as an empty unit.
    let real = out_dir.canonicalize().map_err(|e| {
        format!(
            "{DECLARED}: no evidence: {package}'s out_dir {} does not exist: {e}",
            out_dir.display()
        )
    })?;
    if !real.is_dir() || !real.starts_with(root) {
        return Err(format!(
            "{DECLARED}: no evidence: {package}'s out_dir {} lies outside the target directory \
             {} of the executable these messages built",
            out_dir.display(),
            root.display()
        ));
    }
    let output = script_output(&out_dir).ok_or_else(|| {
        format!(
            "{DECLARED}: no evidence: {package}'s build script output beside {} does not read",
            out_dir.display()
        )
    })?;
    let text = std::fs::read(&output).map_err(|e| {
        format!(
            "{DECLARED}: no evidence: {} does not read: {e}",
            output.display()
        )
    })?;
    let text = String::from_utf8_lossy(&text);
    let mut commands = Vec::new();
    let mut compiles = Vec::new();
    for line in text.lines() {
        let Some(command) = line.strip_prefix("running: ") else {
            continue;
        };
        let words = command_words(command).ok_or_else(|| {
            format!(
                "{DECLARED}: no evidence: {package}'s build script logged a command that does \
                 not read: {}",
                line.chars().take(160).collect::<String>()
            )
        })?;
        let shown = || line.chars().take(200).collect::<String>();
        match classify(&words) {
            Ok(Command::Compile) => {
                if let Some(form) = opaque(&words) {
                    return Err(format!(
                        "{DECLARED}: unjudgeable: {package}'s compile of {} carries {form}, whose \
                         effect on the instruction set the check cannot read",
                        words.last().map_or("", String::as_str)
                    ));
                }
                compiles.push(words.clone());
            }
            Ok(Command::Probe | Command::Archiver) => {}
            Err(why) => {
                return Err(format!(
                    "{DECLARED}: no evidence: {package}'s build script logged a command the check \
                     cannot classify ({why}): {}",
                    shown()
                ))
            }
        }
        commands.push(words);
    }
    Ok(Script {
        package,
        source,
        version: version_of(id),
        objects: objects(&out_dir)?,
        out_dir,
        output,
        commands,
        compiles,
    })
}

fn judge_exception(script: &Script, exception: &Exception) -> Vec<String> {
    match exception.kind {
        ExceptionKind::DispatchedAssembly => script
            .compiles
            .iter()
            .filter(|words| !reviewed(words, &script.out_dir, exception.sources))
            .map(|words| {
                format!(
                    "{DECLARED}: {}'s exception covers its CPUID-dispatched assembly, and it \
                     compiled {}, which it does not review",
                    script.package,
                    words.last().map_or("", String::as_str)
                )
            })
            .collect(),
        ExceptionKind::CompilesNothing => {
            if script.compiles.is_empty() && script.objects.is_empty() {
                Vec::new()
            } else {
                vec![format!(
                    "{DECLARED}: {}'s exception says it compiles nothing on a declared target, \
                     and its build logged {} compile(s) and left {} object(s) or archive(s)",
                    script.package,
                    script.compiles.len(),
                    script.objects.len()
                )]
            }
        }
    }
}

fn judge_script(script: &Script, march: &str) -> Vec<String> {
    if script.compiles.is_empty() {
        if script.objects.is_empty() {
            return Vec::new();
        }
        return vec![format!(
            "{DECLARED}: no evidence: {}'s build script left {} in its OUT_DIR and logged no \
             `running:` compile (was CC_ENABLE_DEBUG_OUTPUT set for the build?)",
            script.package,
            script.objects.join(", ")
        )];
    }
    script
        .compiles
        .iter()
        .flat_map(|words| compile_findings(&script.package, words, march))
        .collect()
}

/// Judge the build `messages` (cargo `--message-format=json` lines) at `march`, bound to `bytes`
/// as the `bin` executable it produced: `Ok(notes)` when every build script has affirmative
/// evidence or holds its exception, else every reason it does not.
pub(super) fn judge(
    messages: &Path,
    bytes: &[u8],
    bin: &str,
    march: &str,
    expected: &Expected,
) -> Result<Vec<String>, Vec<String>> {
    let text = std::fs::read_to_string(messages).map_err(|e| {
        vec![format!(
            "{DECLARED}: the build messages {} do not read: {e}",
            messages.display()
        )]
    })?;
    let mut parsed = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line).map_err(|e| {
            vec![format!(
                "{DECLARED}: the build messages {} are not cargo's --message-format=json: line \
                 {}: {e}",
                messages.display(),
                n + 1
            )]
        })?;
        parsed.push(value);
    }
    // Re-review ruling 1: the stream proves it is the whole, successful build of these bytes.
    if !parsed
        .last()
        .is_some_and(|m| m["reason"] == "build-finished" && m["success"] == true)
    {
        return Err(vec![format!(
            "{DECLARED}: the build messages {} do not end in a build-finished row with success \
             true, so they are not a whole, successful build",
            messages.display()
        )]);
    }
    let kind_of = |m: &serde_json::Value, kind: &str| {
        m["target"]["kind"]
            .as_array()
            .is_some_and(|kinds| kinds.iter().any(|k| k == kind))
    };
    let artifacts: Vec<&serde_json::Value> = parsed
        .iter()
        .filter(|m| m["reason"] == "compiler-artifact")
        .collect();
    let named: Vec<&serde_json::Value> = artifacts
        .iter()
        .copied()
        .filter(|m| m["target"]["name"] == bin && kind_of(m, "bin"))
        .collect();
    let executables: Vec<&str> = named
        .iter()
        .filter(|m| m["package_id"] == expected.package.as_str())
        .filter_map(|m| m["executable"].as_str())
        .collect();
    if executables.is_empty() {
        let others: Vec<&str> = named
            .iter()
            .filter_map(|m| m["package_id"].as_str())
            .collect();
        return Err(vec![format!(
            "{DECLARED}: the build messages {} name no executable for the bin {bin} of the \
             package put, {}{}, so they are not the build of these bytes",
            messages.display(),
            expected.package,
            if others.is_empty() {
                String::new()
            } else {
                format!(" (only of {})", others.join(", "))
            }
        )]);
    }
    let want = BlobCid::compute_bytes(bytes);
    let Some(exe) = executables
        .iter()
        .find(|path| std::fs::read(path).is_ok_and(|built| BlobCid::compute_bytes(&built) == want))
    else {
        return Err(vec![format!(
            "{DECLARED}: the build messages' {bin} executable ({}) is not --bytes, so they are \
             not the build of these bytes",
            executables.join(", ")
        )]);
    };
    // The executable's `<target>/<profile>/` directory: every unit of its build lies under it.
    let Some(root) = Path::new(exe)
        .parent()
        .and_then(|dir| dir.canonicalize().ok())
    else {
        return Err(vec![format!(
            "{DECLARED}: the {bin} executable {exe} has no readable target directory"
        )]);
    };
    let mut missing = Vec::new();
    let mut notes = Vec::new();
    // The roster (ruling 1): every package the lock's resolve puts in the bin's closure for the
    // declared triple was compiled, and every compiled build script ran and is judged below.
    let compiled: BTreeSet<String> = artifacts
        .iter()
        .filter_map(|m| m["package_id"].as_str())
        .map(|id| format!("{}@{}", package_of(id).0, version_of(id)))
        .collect();
    for id in &expected.closure {
        if !compiled.contains(id) {
            missing.push(format!(
                "{DECLARED}: the build messages name no compiler-artifact row for {id}, which \
                 the bin's dependency closure for the declared triple holds, so they are not the \
                 whole build"
            ));
        }
    }
    let ran: BTreeSet<&str> = parsed
        .iter()
        .filter(|m| m["reason"] == "build-script-executed")
        .filter_map(|m| m["package_id"].as_str())
        .collect();
    let scripted: BTreeSet<&str> = artifacts
        .iter()
        .filter(|m| kind_of(m, "custom-build"))
        .filter_map(|m| m["package_id"].as_str())
        .collect();
    for id in scripted.difference(&ran) {
        missing.push(format!(
            "{DECLARED}: no evidence: {id} has a custom-build compile and no \
             build-script-executed row, so its build script's commands are not in the stream"
        ));
    }
    // Every build-script-executed unit counts, build-dependencies and proc-macros included, not
    // only the bin's target-side closure (coordinator ruling on L2a rework, point 1): a declared
    // target is a native build (host triple == target triple; put refuses any other, `isa_c::native`,
    // and requires the triple to be the one rustc reports), so the same CFLAGS_<triple> apply to
    // host-side C; over-checking can only refuse, never admit; and the JSON messages do not mark
    // host from target units, so restricting to the target closure would need cargo's unstable
    // unit graph.
    for message in parsed
        .iter()
        .filter(|m| m["reason"] == "build-script-executed")
    {
        let script = match read_script(message, &root) {
            Ok(script) => script,
            Err(reason) => {
                missing.push(reason);
                continue;
            }
        };
        let exception = EXCEPTIONS.iter().find(|e| {
            e.package == script.package && script.source == CRATES_IO && e.version == script.version
        });
        let mut findings = match exception {
            Some(exception) => judge_exception(&script, exception),
            None => judge_script(&script, march),
        };
        findings.extend(log_findings(&script));
        if !findings.is_empty() {
            missing.extend(findings);
            continue;
        }
        match exception {
            Some(exception) => notes.push(format!(
                "isa-c: {} excepted ({} compile(s)): {}",
                script.package,
                script.compiles.len(),
                exception.reason
            )),
            None if !script.compiles.is_empty() => notes.push(format!(
                "isa-c: {}: {} compile(s) at {march}",
                script.package,
                script.compiles.len()
            )),
            None => {}
        }
    }
    if missing.is_empty() {
        Ok(notes)
    } else {
        Err(missing)
    }
}

#[cfg(test)]
#[path = "isa_c_evidence_tests.rs"]
mod tests;
