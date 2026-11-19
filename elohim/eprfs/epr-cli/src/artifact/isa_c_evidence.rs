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
//! - a compile command is a `running:` line with a `-c` argument (cc's archiver, family-detection
//!   `-E` and version probes carry none);
//! - each compile carries at least one `-march`, every `-march` is the declared one, and no other
//!   `-m` token selects an ISA: `-m64`, `-mtune=*` and `-mno-*` do not; every other `-m` token
//!   refuses (`-mavx2`, `-msse4.2`, `-mcpu=*`, `-march=native` among them);
//! - a script whose `OUT_DIR` holds `.o` or `.a` files and that logged no compile has no evidence
//!   and refuses, and so does one holding an object older than the run its log records, or an
//!   object no logged command produced ([`log_findings`]: a stale or restored log);
//! - an [`EXCEPTIONS`] crate is judged by its own exception instead, which still reads its
//!   commands: CPUID-dispatched assembly compiles only assembly (and cc's flag probe), and a crate
//!   that compiles nothing on a declared target compiled nothing.
//!
//! Residuals, recorded rather than solved: an assembly unit's instructions are its source's, not
//! its `-march`'s, so a compile of a `.S` at the declared `-march` is evidence of the command, not
//! of the instructions (ring's CPUID-dispatched assembly is accepted on that evidence); an option
//! outside the `-m` namespace (`-Wa,…`, `-Xassembler`, a response file `@file`) is not read; every
//! build script in the replayed build is judged, which is a superset of one bin's closure; and the
//! C source itself can select instructions the command does not show: `__attribute__((target))`,
//! `#pragma GCC target` and function multiversioning (`target_clones`) are not read, so a compile
//! at the declared `-march` is evidence of the command's baseline, not of every function in it.
use std::path::{Path, PathBuf};

use eprfs_core::BlobCid;

const DECLARED: &str = "missing declared-target";

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
    pub kind: ExceptionKind,
    /// One sentence, held against the crate's build.rs by genesis/orchestrator/c-baseline-pin.test.mjs.
    pub reason: &'static str,
}

/// The named exceptions (ruling 1(d)); genesis/orchestrator/c-baseline-pin.test.mjs reads this
/// list and holds each reason against the locked crate's build.rs.
pub(super) const EXCEPTIONS: &[Exception] = &[
    Exception {
        package: "blake3",
        kind: ExceptionKind::DispatchedAssembly,
        reason:
            "On x86_64 Linux without the pure or prefer_intrinsics features, blake3's build.rs \
                 compiles only its SSE2, SSE4.1, AVX2 and AVX-512 assembly (.S), passing \
                 -mavx512f and -mavx512vl so the assembler accepts the AVX-512 file, and the \
                 crate chooses each path at run time by CPUID.",
    },
    Exception {
        package: "stacker",
        kind: ExceptionKind::CompilesNothing,
        reason: "stacker's build.rs runs cc only for src/arch/windows.c when the target is \
                 Windows, so it compiles nothing on a declared Linux target.",
    },
    Exception {
        package: "generator",
        kind: ExceptionKind::CompilesNothing,
        reason: "generator's build.rs runs cc only for its powerpc64 context-switch assembly, so \
                 it compiles nothing on a declared x86_64 target.",
    },
    Exception {
        package: "iana-time-zone-haiku",
        kind: ExceptionKind::CompilesNothing,
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

/// Every reason one compile command is not at `march` alone.
pub(super) fn compile_findings(package: &str, words: &[String], march: &str) -> Vec<String> {
    let mut findings = Vec::new();
    let mut marched = false;
    let shown = || {
        let source = words.last().map_or("", String::as_str);
        format!("{package}'s compile of {source}")
    };
    for word in words.iter().skip(1) {
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
fn objects(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_type().is_ok_and(|t| t.is_dir()) {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "o" || ext == "a") {
                let shown = path.strip_prefix(root).unwrap_or(&path);
                found.push(shown.display().to_string());
            }
        }
    }
    found.sort();
    found
}

/// Whether a compile's source is assembly, or cc's flag-support probe in the script's `OUT_DIR`
/// (lib.rs:1948, 1966: `flag_check.c`, never archived).
fn assembly_or_probe(words: &[String], out_dir: &Path) -> bool {
    let Some(source) = words.last().map(Path::new) else {
        return false;
    };
    let assembly = source
        .extension()
        .is_some_and(|ext| ext == "S" || ext == "s" || ext == "asm");
    let probe = source.parent() == Some(out_dir)
        && source
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("flag_check."));
    assembly || probe
}

/// What one build script showed: its package, its compile commands, the objects it left.
struct Script {
    package: String,
    source: String,
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
/// `.o` is the `-o` of a logged compile and every `.a` an argument of a
/// logged archiver command (cc 1.11.1 logs `ar` through the same `running:` line, lib.rs:4032
/// and command_helpers.rs:644). Any object with no logged producer is no evidence.
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
    for object in &script.objects {
        let path = script.out_dir.join(object).display().to_string();
        let produced = if object.ends_with(".o") {
            script
                .compiles
                .iter()
                .any(|words| words.windows(2).any(|w| w[0] == "-o" && w[1] == path))
        } else {
            script
                .commands
                .iter()
                .filter(|words| !words.iter().skip(1).any(|word| word == "-c"))
                .any(|words| words.iter().skip(1).any(|word| *word == path))
        };
        if !produced {
            findings.push(format!(
                "{DECLARED}: no evidence: {}'s OUT_DIR holds {object}, which no logged command \
                 produced",
                script.package
            ));
        }
    }
    findings
}

fn read_script(message: &serde_json::Value) -> Result<Script, String> {
    let id = message["package_id"].as_str().unwrap_or_default();
    let (package, source) = package_of(id);
    let out_dir = message["out_dir"].as_str().map(PathBuf::from).ok_or_else(|| {
        format!("{DECLARED}: no evidence: the build-script-executed message of {id} names no out_dir")
    })?;
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
        if words.iter().skip(1).any(|word| word == "-c") {
            compiles.push(words.clone());
        }
        commands.push(words);
    }
    Ok(Script {
        package,
        source,
        objects: objects(&out_dir),
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
            .filter(|words| !assembly_or_probe(words, &script.out_dir))
            .map(|words| {
                format!(
                    "{DECLARED}: {}'s exception covers its CPUID-dispatched assembly, and it \
                     compiled {}",
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
    let executables: Vec<&str> = parsed
        .iter()
        .filter(|m| m["reason"] == "compiler-artifact" && m["target"]["name"] == bin)
        .filter(|m| {
            m["target"]["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|k| k == "bin"))
        })
        .filter_map(|m| m["executable"].as_str())
        .collect();
    if executables.is_empty() {
        return Err(vec![format!(
            "{DECLARED}: the build messages {} name no executable for the bin {bin}, so they are \
             not the build of these bytes",
            messages.display()
        )]);
    }
    let want = BlobCid::compute_bytes(bytes);
    if !executables
        .iter()
        .any(|path| std::fs::read(path).is_ok_and(|built| BlobCid::compute_bytes(&built) == want))
    {
        return Err(vec![format!(
            "{DECLARED}: the build messages' {bin} executable ({}) is not --bytes, so they are \
             not the build of these bytes",
            executables.join(", ")
        )]);
    }
    let mut missing = Vec::new();
    let mut notes = Vec::new();
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
        let script = match read_script(message) {
            Ok(script) => script,
            Err(reason) => {
                missing.push(reason);
                continue;
            }
        };
        let exception = EXCEPTIONS
            .iter()
            .find(|e| e.package == script.package && script.source == CRATES_IO);
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
