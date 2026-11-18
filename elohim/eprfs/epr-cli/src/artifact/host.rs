//! Host inputs observed before a build, and the linker witness checked before recording it.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use object::{Object, ObjectSection};

use super::{resolve_tool, ArtifactError, ArtifactResult, BlobCid, MethodDescriptor, PATH_INPUT};

/// Startup and support objects selected by the gcc driver, not rustc's native-library set.
const DRIVER_INPUTS: [&str; 8] = [
    "crt1.o",
    "Scrt1.o",
    "crti.o",
    "crtn.o",
    "crtbeginS.o",
    "crtendS.o",
    "libgcc.a",
    "libc_nonshared.a",
];

fn probe(tool: &Path, args: &[&str], path: &str) -> ArtifactResult<Output> {
    probe_until(tool, args, path, None)
}

/// [`probe`], killing the child's whole process group once `deadline` passes.
fn probe_until(
    tool: &Path,
    args: &[&str],
    path: &str,
    deadline: Option<Instant>,
) -> ArtifactResult<Output> {
    let mut command = Command::new(tool);
    command
        .args(args)
        .stdin(Stdio::null())
        .env_clear()
        .env("PATH", path)
        .env("LANG", "C");
    let ran = |e: std::io::Error| {
        ArtifactError::Gap(format!(
            "{} {} does not run: {e}",
            tool.display(),
            args.join(" ")
        ))
    };
    let output = match deadline {
        None => command.output().map_err(ran)?,
        Some(deadline) => match bounded_output(&mut command, deadline).map_err(ran)? {
            Bounded::Done(output) => output,
            Bounded::Expired => {
                return Err(ArtifactError::Gap(format!(
                    "{} {} exceeded the probe deadline",
                    tool.display(),
                    args.join(" ")
                )))
            }
            Bounded::TooLarge => {
                return Err(ArtifactError::Gap(format!(
                    "{} {} wrote more than {PROBE_OUTPUT_LIMIT} bytes",
                    tool.display(),
                    args.join(" ")
                )))
            }
        },
    };
    if !output.status.success() {
        return Err(ArtifactError::Gap(format!(
            "{} {} failed: {}",
            tool.display(),
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(output)
}

/// Bytes a deadline-bounded probe may write to each of stdout and stderr; gcc's predefined-macro
/// dump, the largest answer the sysroot probe asks for, is a few tens of KiB.
const PROBE_OUTPUT_LIMIT: u64 = 4 * 1024 * 1024;

enum Bounded {
    Done(Output),
    Expired,
    TooLarge,
}

/// Kill the child's whole process group and reap the child without a blocking wait: a few
/// non-blocking tries, then the process is left to be reaped at exit rather than waited for.
fn kill_group(child: &mut std::process::Child) {
    let group = i32::try_from(child.id()).unwrap_or(i32::MAX);
    // SAFETY: kill(2) takes plain integers and has no memory effects; the group is this child's
    // own (process_group(0) at spawn). An error (ESRCH: the group is already empty) is benign.
    unsafe { libc::kill(-group, libc::SIGKILL) };
    for _ in 0..50 {
        if !matches!(child.try_wait(), Ok(None)) {
            return;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// Run `command` under one `deadline` covering the child's life AND the drain of its pipes: a
/// descendant that keeps a pipe open after the child exits cannot hold the caller, because the
/// readers are never joined, only received from until the deadline. Each pipe's capture is capped
/// at [`PROBE_OUTPUT_LIMIT`]. The child leads its own process group, and on every exit path the
/// group is killed, so neither a stuck nor a lingering descendant outlives the probe.
fn bounded_output(command: &mut Command, deadline: Instant) -> std::io::Result<Bounded> {
    use std::os::unix::process::CommandExt;
    use std::sync::mpsc;
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()?;
    let (sender, received) = mpsc::channel::<(bool, Vec<u8>, bool)>();
    let pipes: [(bool, Option<Box<dyn Read + Send>>); 2] = [
        (true, child.stdout.take().map(|p| Box::new(p) as _)),
        (false, child.stderr.take().map(|p| Box::new(p) as _)),
    ];
    for (is_stdout, pipe) in pipes {
        let sender = sender.clone();
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            if let Some(pipe) = pipe {
                // A read error ends the capture; the exit status still decides the probe.
                let _ = pipe.take(PROBE_OUTPUT_LIMIT + 1).read_to_end(&mut bytes);
            }
            let over = bytes.len() as u64 > PROBE_OUTPUT_LIMIT;
            // The receiver may have given up at its deadline; nothing waits for this send.
            let _ = sender.send((is_stdout, bytes, over));
        });
    }
    drop(sender);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            kill_group(&mut child);
            return Ok(Bounded::Expired);
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    let (mut stdout, mut stderr) = (None, None);
    while stdout.is_none() || stderr.is_none() {
        let left = deadline.saturating_duration_since(Instant::now());
        match received.recv_timeout(left) {
            Ok((_, _, true)) => {
                kill_group(&mut child);
                return Ok(Bounded::TooLarge);
            }
            Ok((true, bytes, false)) => stdout = Some(bytes),
            Ok((false, bytes, false)) => stderr = Some(bytes),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                kill_group(&mut child);
                return Ok(Bounded::Expired);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    // The child is reaped; kill whatever of its group lingers, holding no pipe.
    kill_group(&mut child);
    Ok(Bounded::Done(Output {
        status,
        stdout: stdout.unwrap_or_default(),
        stderr: stderr.unwrap_or_default(),
    }))
}

/// Limits on the files the sysroot probe and its re-check read: one wall-clock deadline (shared
/// with the probe's children), a per-file size and a total over all files.
pub(crate) struct ReadBudget {
    pub deadline: Instant,
    pub per_file: u64,
    pub remaining: u64,
}

/// libc.so.6 is a few MiB and libgcc.a under ten; nothing the sysroot names approaches this.
pub(crate) const SYSROOT_FILE_LIMIT: u64 = 64 * 1024 * 1024;
/// Over every file one observation reads: a few times the real sum (about 20 MiB here).
pub(crate) const SYSROOT_TOTAL_LIMIT: u64 = 256 * 1024 * 1024;

/// Why a bounded read did not return bytes: the file is gone (a change, for the re-check), or
/// it is outside the bounds (the observation cannot be made).
enum ReadRefused {
    Missing,
    Outside(String),
}

/// Read `file` within `budget`: the last component is never followed, the open never blocks on a
/// FIFO or device, only a regular file within the per-file and remaining bytes is read, and the
/// deadline is checked before the open and between chunks.
fn bounded_read(file: &Path, budget: &mut ReadBudget) -> Result<Vec<u8>, ReadRefused> {
    use std::os::unix::fs::OpenOptionsExt;
    let expired = || {
        ReadRefused::Outside(format!(
            "reading {} exceeded the probe deadline",
            file.display()
        ))
    };
    if Instant::now() >= budget.deadline {
        return Err(expired());
    }
    let handle = match std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(file)
    {
        Ok(handle) => handle,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(ReadRefused::Missing),
        Err(e) => return Err(ReadRefused::Outside(format!("{}: {e}", file.display()))),
    };
    let meta = handle
        .metadata()
        .map_err(|e| ReadRefused::Outside(format!("{}: {e}", file.display())))?;
    if !meta.is_file() {
        return Err(ReadRefused::Outside(format!(
            "{} is not a regular file",
            file.display()
        )));
    }
    let limit = budget.per_file.min(budget.remaining);
    if meta.len() > limit {
        return Err(ReadRefused::Outside(format!(
            "{} is {} bytes, over the {limit}-byte read bound",
            file.display(),
            meta.len()
        )));
    }
    let mut bytes = Vec::new();
    let mut handle = handle.take(limit + 1);
    let mut chunk = vec![0u8; 1024 * 1024];
    loop {
        if Instant::now() >= budget.deadline {
            return Err(expired());
        }
        let read = handle
            .read(&mut chunk)
            .map_err(|e| ReadRefused::Outside(format!("{}: {e}", file.display())))?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..read]);
    }
    if bytes.len() as u64 > limit {
        return Err(ReadRefused::Outside(format!(
            "{} grew past the {limit}-byte read bound",
            file.display()
        )));
    }
    budget.remaining -= bytes.len() as u64;
    Ok(bytes)
}

fn output_bytes(output: Output) -> Vec<u8> {
    [output.stdout, output.stderr].concat()
}

mod native_libraries;
mod selections;
pub(super) use selections::{codegen_options, merge_configs, resolve, target, Selections};

fn tool_version(
    lines: &mut String,
    label: &str,
    tool: &Path,
    path: &str,
    deadline: Option<Instant>,
) -> ArtifactResult<()> {
    let output = probe_until(tool, &["--version"], path, deadline)?;
    let version = BlobCid::compute_bytes(&output_bytes(output));
    lines.push_str(&format!("tool:{label}\t{}\t{version}\n", tool.display()));
    Ok(())
}

fn driver_defaults(
    lines: &mut String,
    label: &str,
    tool: &Path,
    path: &str,
    deadline: Option<Instant>,
) -> ArtifactResult<()> {
    let output = probe_until(tool, &["-E", "-dM", "-x", "c", "-"], path, deadline)?;
    let macros = String::from_utf8(output.stdout).map_err(|e| {
        ArtifactError::Gap(format!("{} macro dump is not UTF-8: {e}", tool.display()))
    })?;
    let mut macros: Vec<_> = macros.lines().collect();
    macros.sort_unstable();
    let defaults = BlobCid::compute_bytes(macros.join("\n").as_bytes());
    lines.push_str(&format!(
        "tool:{label}:target\t{}\t{defaults}\n",
        tool.display()
    ));
    Ok(())
}

/// Content-address one link input, and the referents of a GNU ld script. With a `budget` (the
/// sysroot probe) every read is bounded by it ([`bounded_read`]); without one (the artifact key)
/// files are read as before.
fn link_input(
    lines: &mut String,
    label: &str,
    file: &Path,
    depth: usize,
    mut budget: Option<&mut ReadBudget>,
) -> ArtifactResult<()> {
    if depth > 4 {
        return Err(ArtifactError::Gap(format!(
            "link input {label}: linker script recursion exceeds depth 4"
        )));
    }
    let resolved = std::fs::canonicalize(file).map_err(|e| {
        ArtifactError::Gap(format!(
            "link input {label} at {} does not resolve: {e}",
            file.display()
        ))
    })?;
    let bytes = match budget.as_deref_mut() {
        Some(budget) => bounded_read(&resolved, budget).map_err(|refused| match refused {
            ReadRefused::Missing => ArtifactError::Gap(format!(
                "link input {label} at {} does not read: missing",
                resolved.display()
            )),
            ReadRefused::Outside(why) => ArtifactError::Gap(why),
        })?,
        None => std::fs::read(&resolved).map_err(|e| {
            ArtifactError::Gap(format!(
                "link input {label} at {} does not read: {e}",
                resolved.display()
            ))
        })?,
    };
    lines.push_str(&format!(
        "link:{label}\t{}\t{}\n",
        resolved.display(),
        BlobCid::compute_bytes(&bytes)
    ));
    if let Ok(text) = std::str::from_utf8(&bytes) {
        if text.contains("GROUP") || text.contains("INPUT") || text.starts_with("/* GNU ld script")
        {
            // Remove C comments before looking for paths; paths in explanatory prose are not inputs.
            let mut uncommented = String::new();
            let mut rest = text;
            while let Some((before, comment)) = rest.split_once("/*") {
                uncommented.push_str(before);
                uncommented.push(' ');
                rest = comment.split_once("*/").map_or("", |(_, tail)| tail);
            }
            uncommented.push_str(rest);
            let referents: std::collections::BTreeSet<_> = uncommented
                .split(|c: char| {
                    c.is_whitespace() || matches!(c, '(' | ')' | ',' | ';' | '\'' | '"')
                })
                .filter(|word| word.starts_with('/'))
                .collect();
            for referent in referents {
                link_input(
                    lines,
                    &format!("{label}>{referent}"),
                    Path::new(referent),
                    depth + 1,
                    budget.as_deref_mut(),
                )?;
            }
        }
    }
    Ok(())
}

/// Probe the effective final-link driver and the independently selected build-script C compiler.
/// An explicit driver without fuse-ld also keys PATH's `ld` as an external linker. No selection,
/// or a final bare fuse-ld=lld, uses rustc's bundled linker identity instead.
pub(super) fn tool_lines(
    descriptor: &MethodDescriptor,
    selections: &Selections,
) -> ArtifactResult<String> {
    let Some(path) = descriptor.non_path_inputs.get(PATH_INPUT) else {
        if selections.driver != "cc" || selections.external_linker.is_some() {
            return Err(ArtifactError::Gap(
                "selected linker has no recorded PATH".into(),
            ));
        }
        return Ok(String::new());
    };
    let driver = resolve_tool(&selections.driver, path);
    if driver.is_none() && selections.driver != "cc" {
        return Err(ArtifactError::Gap(format!(
            "selected linker {} does not resolve under recorded PATH",
            selections.driver
        )));
    }
    let mut lines = String::new();
    if let Some(driver) = &driver {
        tool_version(&mut lines, "link-driver", driver, path, None)?;
        driver_defaults(&mut lines, "link-driver", driver, path, None)?;
        for input in DRIVER_INPUTS {
            if let Some(file) = native_libraries::driver_file(driver, input, path, None)? {
                link_input(&mut lines, input, Path::new(&file), 0, None)?;
            } else {
                lines.push_str(&format!("link:{input}\t-\n"));
            }
        }
        for name in native_libraries::probe(descriptor, path)? {
            let shared = format!("lib{name}.so");
            if let Some(file) = native_libraries::driver_file(driver, &shared, path, None)? {
                link_input(&mut lines, &shared, Path::new(&file), 0, None)?;
            } else {
                let archive = format!("lib{name}.a");
                if let Some(file) = native_libraries::driver_file(driver, &archive, path, None)? {
                    link_input(&mut lines, &archive, Path::new(&file), 0, None)?;
                } else {
                    lines.push_str(&format!("link:lib{name}\t-\n"));
                }
            }
        }
    } else {
        lines.push_str("tool:link-driver\t-\n");
    }
    let cc_name = descriptor
        .non_path_inputs
        .get("CC")
        .map_or("cc", String::as_str);
    let cc = resolve_tool(cc_name, path);
    if cc.is_none() && descriptor.non_path_inputs.contains_key("CC") {
        return Err(ArtifactError::Gap(format!(
            "build-script C compiler CC={cc_name} does not resolve under recorded PATH"
        )));
    }
    if let Some(cc) = cc {
        if driver.as_ref() != Some(&cc) {
            tool_version(&mut lines, "CC", &cc, path, None)?;
            driver_defaults(&mut lines, "CC", &cc, path, None)?;
        }
    }
    if let Some(name) = &selections.external_linker {
        let label = "external-linker";
        let resolved = resolve_tool(name, path).ok_or_else(|| {
            ArtifactError::Gap(format!(
                "selected linker {label}={name} does not resolve under recorded PATH"
            ))
        })?;
        tool_version(&mut lines, label, &resolved, path, None)?;
    }
    Ok(lines)
}

/// Libraries the sysroot identity reads beyond [`DRIVER_INPUTS`], as the C driver resolves them.
/// Their GNU ld scripts pull in, by content, libc.so.6, the loader, libc_nonshared.a, libm.so.6,
/// libmvec.so.1, libgcc_s.so.1 and libgcc.a. A fixed list, so no compiler other than the C driver
/// decides what is read: a Rust toolchain switch cannot move the identity.
const SYSROOT_LIBRARIES: [&str; 3] = ["libc.so", "libm.so", "libgcc_s.so"];

/// This host's sysroot identity lines, read with the artifact key's own primitives: the C link
/// driver `cc` under `path` (canonical path, `--version` digest, predefined-macro digest), then
/// each of [`DRIVER_INPUTS`] and [`SYSROOT_LIBRARIES`] as `cc -print-file-name` resolves it,
/// content-addressed through any linker script. Sorted; absolute canonical paths as the key's
/// input document spells them. One `deadline` covers every child, its pipes and every file read;
/// files are bounded by [`SYSROOT_FILE_LIMIT`] and [`SYSROOT_TOTAL_LIMIT`].
///
/// What it does NOT see: a same-path gcc rebuild whose `--version` output, predefined macros and
/// selected inputs are unchanged (cc1, collect2, as and ld are not hashed, nor is the gcc binary
/// itself); libraries outside these lists, including any a Rust build adds; header files; linker
/// script operands that are not absolute paths.
pub(crate) fn sysroot_lines(path: &str, deadline: Instant) -> ArtifactResult<Vec<String>> {
    sysroot_lines_within(
        path,
        &mut ReadBudget {
            deadline,
            per_file: SYSROOT_FILE_LIMIT,
            remaining: SYSROOT_TOTAL_LIMIT,
        },
    )
}

fn sysroot_lines_within(path: &str, budget: &mut ReadBudget) -> ArtifactResult<Vec<String>> {
    let deadline = Some(budget.deadline);
    let driver = resolve_tool("cc", path)
        .ok_or_else(|| ArtifactError::Gap("no C link driver cc under PATH".into()))?;
    let mut lines = String::new();
    tool_version(&mut lines, "link-driver", &driver, path, deadline)?;
    driver_defaults(&mut lines, "link-driver", &driver, path, deadline)?;
    for input in DRIVER_INPUTS.iter().chain(SYSROOT_LIBRARIES.iter()) {
        match native_libraries::driver_file(&driver, input, path, deadline)? {
            Some(file) => link_input(&mut lines, input, Path::new(&file), 0, Some(&mut *budget))?,
            None => lines.push_str(&format!("link:{input}\t-\n")),
        }
    }
    let mut lines: Vec<String> = lines.lines().map(str::to_owned).collect();
    lines.sort_unstable();
    check_sysroot_lines(&lines).map_err(ArtifactError::Gap)?;
    Ok(lines)
}

/// More lines than a sysroot observation can emit (two driver lines, eleven inputs, a few
/// linker-script referents each).
const SYSROOT_LINES_LIMIT: usize = 64;

/// An absolute path with no `.`/`..` components.
fn clean_absolute(path: &str) -> bool {
    path.starts_with('/')
        && Path::new(path).components().all(|c| {
            matches!(
                c,
                std::path::Component::RootDir | std::path::Component::Normal(_)
            )
        })
}

/// `link:<input>` with `<input>` one the probe asks for, followed by zero or more
/// `><absolute path>` linker-script referents.
fn sysroot_link_label(label: &str) -> bool {
    let Some(rest) = label.strip_prefix("link:") else {
        return false;
    };
    let mut parts = rest.split('>');
    let input = parts.next().unwrap_or_default();
    DRIVER_INPUTS
        .iter()
        .chain(SYSROOT_LIBRARIES.iter())
        .any(|known| *known == input)
        && parts.all(clean_absolute)
}

/// The grammar of [`sysroot_lines`]: sorted and unique; each line either `link:<input>\t-` (not
/// found) or `<label>\t<absolute path>\t<CID>` with a label the probe emits; at least the driver
/// line and one link input with a path. A record read from disk is checked against it before
/// any path in it is used.
pub(crate) fn check_sysroot_lines(lines: &[String]) -> Result<(), String> {
    if lines.len() > SYSROOT_LINES_LIMIT {
        return Err(format!(
            "{} lines, more than a sysroot observation emits",
            lines.len()
        ));
    }
    let (mut driver, mut linked) = (false, false);
    for (index, line) in lines.iter().enumerate() {
        let fields: Vec<&str> = line.split('\t').collect();
        let valid = match fields[..] {
            [label, "-"] => sysroot_link_label(label) && !label.contains('>'),
            [label, path, cid] if clean_absolute(path) && BlobCid::parse(cid).is_ok() => {
                if label == "tool:link-driver" {
                    driver = true;
                    true
                } else if label == "tool:link-driver:target" {
                    true
                } else {
                    let ok = sysroot_link_label(label);
                    linked |= ok;
                    ok
                }
            }
            _ => false,
        };
        if !valid {
            return Err(format!("line {} is not a sysroot identity line", index + 1));
        }
    }
    if !lines.windows(2).all(|pair| pair[0] < pair[1]) {
        return Err("lines are not sorted and unique".into());
    }
    if !driver || !linked {
        return Err("no driver line or no link input with a path".into());
    }
    Ok(())
}

/// Re-check recorded [`sysroot_lines`] without running any program, within one `deadline` and
/// the same per-file and total read bounds: the lines are checked against the grammar first,
/// then every `link:` file is read and content-addressed again and every `tool:` path must still
/// be a regular file at its canonical path. Returns the labels that changed (a file now missing
/// or no longer at its canonical path is a change); `Err` when the check cannot be made (bad
/// grammar, a file outside the bounds, the deadline). A cheap, partial check: a driver whose
/// output changed at the same path, or an input recorded as absent that has since appeared, is
/// only seen by a full observation.
pub(crate) fn sysroot_drift(lines: &[String], deadline: Instant) -> Result<Vec<String>, String> {
    sysroot_drift_within(
        lines,
        &mut ReadBudget {
            deadline,
            per_file: SYSROOT_FILE_LIMIT,
            remaining: SYSROOT_TOTAL_LIMIT,
        },
    )
}

fn sysroot_drift_within(lines: &[String], budget: &mut ReadBudget) -> Result<Vec<String>, String> {
    check_sysroot_lines(lines)?;
    let mut drifted = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split('\t').collect();
        let [label, file, cid] = fields[..] else {
            continue;
        };
        let path = Path::new(file);
        // A recorded path is canonical; one that no longer resolves to itself has moved.
        let same = if std::fs::canonicalize(path).ok().as_deref() != Some(path) {
            false
        } else if label.starts_with("tool:") {
            std::fs::symlink_metadata(path).is_ok_and(|meta| meta.is_file())
        } else {
            match bounded_read(path, budget) {
                Ok(bytes) => BlobCid::compute_bytes(&bytes).to_string() == cid,
                Err(ReadRefused::Missing) => false,
                Err(ReadRefused::Outside(why)) => return Err(why),
            }
        };
        if !same {
            drifted.push(label.to_owned());
        }
    }
    Ok(drifted)
}

fn linker_identity(text: &str) -> Option<(&str, &str)> {
    for (marker, kind) in [
        ("LLD ", "LLD"),
        ("GNU ld ", "GNU ld"),
        ("GNU gold ", "GNU gold"),
        ("mold ", "mold"),
    ] {
        if let Some((_, rest)) = text.split_once(marker) {
            // GNU's vendor annotation can precede its version number.
            let version = rest
                .split_whitespace()
                .find(|word| word.starts_with(|c: char| c.is_ascii_digit()))?;
            return Some((kind, version));
        }
    }
    None
}

/// Read only ELF section bytes. Missing, malformed or unaccounted linker evidence refuses before
/// `put` writes an attestation or a custody record; an old descriptor must be described again.
/// External GNU ld normally emits no linker witness. Its keyed --version proves which tool was
/// selected, not which tool produced these bytes: without a witness this build cannot be recorded.
/// Do not substitute the driver/compiler comment or the selected tool's version for that evidence.
pub(super) fn check_linker(
    descriptor: &MethodDescriptor,
    selections: &Selections,
    bytes: &[u8],
) -> ArtifactResult<()> {
    let kind = object::FileKind::parse(bytes).map_err(|e| {
        ArtifactError::Gap(format!(
            "artifact format does not read: {e}; no .comment Linker line"
        ))
    })?;
    if !matches!(kind, object::FileKind::Elf32 | object::FileKind::Elf64) {
        return Err(ArtifactError::Gap(format!(
            "artifact format {kind:?} is not ELF"
        )));
    }
    let elf = object::File::parse(bytes).map_err(|e| {
        ArtifactError::Gap(format!(
            "artifact ELF does not read: {e}; no .comment Linker line"
        ))
    })?;
    let llvm = descriptor
        .tools
        .get("rustc")
        .and_then(|rustc| {
            rustc
                .lines()
                .find_map(|line| line.strip_prefix("LLVM version: "))
        })
        .map(str::trim);
    let mut accounted = Vec::new();
    if let Some(version) = llvm.filter(|v| !v.is_empty() && selections.bundled_lld_authorized) {
        accounted.push(format!("LLD {version} (rustc LLVM)"));
    }
    if let Some(name) = &selections.external_linker {
        let label = "external-linker";
        let path = descriptor.non_path_inputs.get(PATH_INPUT).ok_or_else(|| {
            ArtifactError::Gap(format!(
                "selected linker {label}={name} has no recorded PATH"
            ))
        })?;
        let resolved = resolve_tool(name, path).ok_or_else(|| {
            ArtifactError::Gap(format!("selected linker {label}={name} does not resolve"))
        })?;
        let version =
            String::from_utf8_lossy(&output_bytes(probe(&resolved, &["--version"], path)?))
                .into_owned();
        if let Some((kind, version)) = linker_identity(&version) {
            accounted.push(format!("{kind} {version} ({label}={name})"));
        }
    }
    let missing_witness = || {
        ArtifactError::Gap(
            if accounted
                .iter()
                .any(|expected| matches!(linker_identity(expected), Some(("GNU ld", _))))
            {
                "an external GNU ld leaves no linker witness; this build cannot be recorded".into()
            } else {
                "artifact has no .comment Linker line".into()
            },
        )
    };
    let comment = elf
        .section_by_name(".comment")
        .ok_or_else(missing_witness)?
        .data()
        .map_err(|e| ArtifactError::Gap(format!("artifact .comment does not read: {e}")))?;
    let observed: Vec<&str> = comment
        .split(|byte| *byte == 0)
        .filter_map(|bytes| std::str::from_utf8(bytes).ok())
        .flat_map(str::lines)
        .filter_map(|line| {
            line.strip_prefix("Linker: ").or_else(|| {
                ["mold ", "GNU gold ", "GNU ld "]
                    .iter()
                    .any(|prefix| line.starts_with(prefix))
                    .then_some(line)
            })
        })
        .collect();
    if observed.is_empty() {
        return Err(missing_witness());
    }
    for line in observed {
        let identity = linker_identity(line);
        if identity.is_none()
            || !accounted
                .iter()
                .any(|expected| linker_identity(expected) == identity)
        {
            return Err(ArtifactError::Gap(format!(
                "artifact Linker: {line}; keyed linker: {}",
                if accounted.is_empty() {
                    "none (no recorded rustc LLVM version or identifiable selected linker; describe again)".into()
                } else {
                    accounted.join(", ")
                }
            )));
        }
    }
    Ok(())
}

/// Root of the installed compiler selected by the same recorded selector as native-lib probing.
pub(super) fn compiler_root(descriptor: &MethodDescriptor) -> ArtifactResult<std::path::PathBuf> {
    let path = descriptor
        .non_path_inputs
        .get(PATH_INPUT)
        .ok_or_else(|| ArtifactError::Gap("no compiler PATH".into()))?;
    let name = descriptor
        .non_path_inputs
        .get("RUSTC")
        .map_or("rustc", String::as_str);
    let executable = super::resolve_tool_path(name, path)
        .ok_or_else(|| ArtifactError::Gap("compiler unresolved".into()))?;
    let compiler = native_libraries::compiler(&executable, descriptor)?;
    compiler
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| ArtifactError::Gap("compiler has no toolchain root".into()))
}

#[cfg(test)]
#[path = "../../tests/support/artifact_elf.rs"]
mod fixture;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn check_linker(descriptor: &MethodDescriptor, bytes: &[u8]) -> ArtifactResult<()> {
        let selection = resolve(
            &mut descriptor.non_path_inputs.clone(),
            &toml::Value::Table(Default::default()),
            target(descriptor),
        )?;
        super::check_linker(descriptor, &selection, bytes)
    }

    fn descriptor() -> MethodDescriptor {
        MethodDescriptor {
            name: "mesh-build".into(),
            tools: [("rustc".into(), "LLVM version: 22.1.2".into())].into(),
            flags: vec![],
            params: None,
            exec: None,
            step: None,
            embedder: None,
            view: vec![],
            non_path_inputs: BTreeMap::new(),
            deterministic: false,
        }
    }

    #[test]
    fn artifact_elf_class_and_endian_are_generic_and_other_formats_refuse_by_name() {
        for is64 in [false, true] {
            for little in [false, true] {
                let bytes = fixture::artifact_elf_format(b"Linker: LLD 22.1.2\0", is64, little);
                check_linker(&descriptor(), &bytes).unwrap();
            }
        }
        let mut macho = vec![0; 32];
        macho[..4].copy_from_slice(&[0xcf, 0xfa, 0xed, 0xfe]);
        let error = check_linker(&descriptor(), &macho).unwrap_err();
        assert!(error.to_string().contains("MachO64 is not ELF"), "{error}");
    }

    #[test]
    fn artifact_actual_linker_comment_formats_and_bundled_lld_authority() {
        use std::os::unix::fs::PermissionsExt;
        let tools = tempfile::tempdir().unwrap();
        let linker = tools.path().join("selected-linker");
        let mut descriptor = descriptor();
        descriptor
            .non_path_inputs
            .insert(PATH_INPUT.into(), tools.path().to_string_lossy().into());
        for (version, comment) in [
            (
                "mold 2.40.0 (compatible with GNU ld)",
                "mold 2.40.0 (compatible with GNU ld)",
            ),
            (
                "GNU gold (GNU Binutils 2.41) 1.16",
                "GNU gold (GNU Binutils 2.41) 1.16",
            ),
            ("GNU ld (GNU Binutils) 2.41", "GNU ld (GNU Binutils) 2.41"),
        ] {
            std::fs::write(&linker, format!("#!/bin/sh\necho '{version}'\n")).unwrap();
            std::fs::set_permissions(&linker, std::fs::Permissions::from_mode(0o755)).unwrap();
            fixture::wait_for_stub(&linker);
            descriptor.non_path_inputs.insert(
                "RUSTFLAGS".into(),
                format!("-C link-arg=-fuse-ld={}", linker.display()),
            );
            check_linker(&descriptor, &fixture::artifact_elf(comment.as_bytes())).unwrap();
            let bundled = fixture::artifact_elf(b"Linker: LLD 22.1.2\0");
            assert!(
                check_linker(&descriptor, &bundled).is_err(),
                "external selection cannot authorize bundled LLD"
            );
        }
        for flags in [
            "",
            "-C link-arg=-fuse-ld=lld",
            "-C link-arg=-fuse-ld=bfd -C link-arg=-fuse-ld=lld",
            "-Clinker=gcc -C link-arg=-fuse-ld=lld",
        ] {
            descriptor
                .non_path_inputs
                .insert("RUSTFLAGS".into(), flags.into());
            check_linker(&descriptor, &fixture::artifact_elf(b"Linker: LLD 22.1.2\0")).unwrap();
        }
        std::os::unix::fs::symlink(&linker, tools.path().join("ld")).unwrap();
        descriptor
            .non_path_inputs
            .insert("RUSTFLAGS".into(), "-Clinker=gcc".into());
        let bundled = fixture::artifact_elf(b"Linker: LLD 22.1.2\0");
        assert!(check_linker(&descriptor, &bundled).is_err());
        let comment_only = fixture::artifact_elf(b"GCC: 14.3.1\0");
        let mut no_comment = comment_only.clone();
        let name = no_comment
            .windows(8)
            .position(|bytes| bytes == b".comment")
            .unwrap();
        no_comment[name..name + 8].copy_from_slice(b".unknown");
        for bytes in [comment_only, no_comment] {
            let error = check_linker(&descriptor, &bytes).unwrap_err();
            assert!(matches!(error, ArtifactError::Gap(_)));
            assert!(
                error.to_string().contains(
                    "an external GNU ld leaves no linker witness; this build cannot be recorded"
                ),
                "{error}"
            );
        }
        descriptor.non_path_inputs.insert(
            "RUSTFLAGS".into(),
            "--codegen=link-self-contained=-linker".into(),
        );
        let error =
            check_linker(&descriptor, &fixture::artifact_elf(b"Linker: LLD 22.1.2\0")).unwrap_err();
        assert!(error.to_string().contains("link-self-contained"));
        descriptor.non_path_inputs.clear();
        assert!(check_linker(&descriptor, &fixture::artifact_elf(b"GCC: 14\0")).is_err());
    }

    #[test]
    fn artifact_linker_witness_matches_recorded_rustc_llvm() {
        let output = Command::new("rustc").arg("-vV").output().unwrap();
        assert!(output.status.success());
        let rustc = String::from_utf8(output.stdout).unwrap();
        let llvm = rustc
            .lines()
            .find_map(|line| line.strip_prefix("LLVM version: "))
            .unwrap()
            .to_string();
        let mut descriptor = MethodDescriptor {
            name: "mesh-build".into(),
            tools: [("rustc".into(), rustc)].into(),
            flags: vec![],
            params: None,
            exec: None,
            step: None,
            embedder: None,
            view: vec![],
            non_path_inputs: BTreeMap::new(),
            deterministic: false,
        };
        let valid = fixture::artifact_elf(format!("Linker: LLD {llvm} (toolchain)\0").as_bytes());
        check_linker(&descriptor, &valid).unwrap();
        for comment in ["Linker: GNU ld 2.41\0", "Linker: LLD 0.0.0\0"] {
            let err =
                check_linker(&descriptor, &fixture::artifact_elf(comment.as_bytes())).unwrap_err();
            assert!(matches!(err, ArtifactError::Gap(_)));
            let message = err.to_string();
            assert!(
                message.contains(comment.trim_end_matches('\0')),
                "{message}"
            );
            assert!(message.contains(&format!("LLD {llvm}")), "{message}");
        }
        for bytes in [
            fixture::artifact_elf(b"GCC: 14\0"),
            Vec::new(),
            valid[..100].to_vec(),
        ] {
            assert!(matches!(
                check_linker(&descriptor, &bytes),
                Err(ArtifactError::Gap(_))
            ));
        }
        let mixed =
            fixture::artifact_elf(format!("Linker: LLD {llvm}\0Linker: GNU ld 2.41\0").as_bytes());
        assert!(check_linker(&descriptor, &mixed).is_err());
        descriptor.tools.clear();
        assert!(check_linker(&descriptor, &valid)
            .unwrap_err()
            .to_string()
            .contains("describe again"));
    }

    /// A fake toolchain: `cc` resolves inputs from `lib` (else echoes the name, as gcc does for
    /// an unknown file), prints `version`, and dumps one macro. No rustc anywhere on PATH.
    struct FakeSysroot {
        root: tempfile::TempDir,
    }
    impl FakeSysroot {
        fn new() -> Self {
            let fake = Self {
                root: tempfile::tempdir().unwrap(),
            };
            std::fs::create_dir(fake.lib()).unwrap();
            for name in DRIVER_INPUTS {
                fake.put(name, name.as_bytes());
            }
            fake.put("libc.so.6", b"\x7fELF libc 2.39");
            fake.put("libc_nonshared.a", b"!<arch> nonshared");
            fake.put("ld-linux-x86-64.so.2", b"\x7fELF loader");
            fake.put("libm.so", b"\x7fELF libm");
            let lib = fake.lib().display().to_string();
            fake.put(
                "libc.so",
                format!(
                    "/* GNU ld script */\nOUTPUT_FORMAT(elf64-x86-64)\nGROUP ( {lib}/libc.so.6 \
                     {lib}/libc_nonshared.a  AS_NEEDED ( {lib}/ld-linux-x86-64.so.2 ) )\n"
                )
                .as_bytes(),
            );
            fake.driver("cc 14.3.1");
            fake
        }
        fn lib(&self) -> std::path::PathBuf {
            self.root.path().join("lib")
        }
        fn put(&self, name: &str, bytes: &[u8]) {
            std::fs::write(self.lib().join(name), bytes).unwrap();
        }
        fn driver(&self, body: &str) {
            use std::os::unix::fs::PermissionsExt;
            let file = self.root.path().join("gcc");
            let staged = self.root.path().join("gcc.new");
            std::fs::write(
                &staged,
                format!(
                    "#!/bin/sh\ncase \"$1\" in\n  -print-file-name=*) n=\"${{1#-print-file-name=}}\"; \
                     if [ -e '{lib}'/\"$n\" ]; then echo '{lib}'/\"$n\"; else echo \"$n\"; fi ;;\n  \
                     --version) echo '{version}' ;;\n  *) {body_macros} ;;\nesac\n",
                    lib = self.lib().display(),
                    version = if body.starts_with("sleep_ ") { "cc 14 (stalling)" } else { body },
                    // `sleep_ <command>` makes every probe but `--version` run <command>.
                    body_macros = body
                        .strip_prefix("sleep_ ")
                        .unwrap_or("echo '#define __GNUC__ 14'"),
                ),
            )
            .unwrap();
            std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755)).unwrap();
            std::fs::rename(&staged, &file).unwrap();
            // `--version` answers at once even for the sleeping driver.
            fixture::wait_for_stub(&file);
        }
        /// A PATH directory named `name` holding a `cc` link to the fake driver.
        fn bin(&self, name: &str) -> String {
            let dir = self.root.path().join(name);
            if !dir.exists() {
                std::fs::create_dir(&dir).unwrap();
                std::os::unix::fs::symlink(self.root.path().join("gcc"), dir.join("cc")).unwrap();
            }
            dir.display().to_string()
        }
        fn lines(&self, path: &str) -> Vec<String> {
            sysroot_lines(path, Instant::now() + Duration::from_secs(20)).unwrap()
        }
    }

    fn drift(lines: &[String]) -> Vec<String> {
        sysroot_drift(lines, Instant::now() + Duration::from_secs(20)).unwrap()
    }

    fn sleep_bin() -> &'static str {
        ["/usr/bin/sleep", "/bin/sleep"]
            .into_iter()
            .find(|p| Path::new(p).exists())
            .unwrap()
    }

    /// The identity names the driver and its inputs by canonical path and content: PATH order,
    /// spelling and unrelated entries do not move it, no rustc is consulted, and a change to a
    /// crt object or to a library behind a linker script does.
    #[test]
    fn sysroot_lines_are_the_drivers_inputs_by_content_not_path_or_rustc() {
        let fake = FakeSysroot::new();
        let (a, b) = (fake.bin("a"), fake.bin("b"));
        let unrelated = fake.root.path().join("unrelated");
        std::fs::create_dir(&unrelated).unwrap();
        let unrelated = unrelated.display().to_string();
        let first = fake.lines(&a);
        for path in [
            format!("{b}:{a}"),
            format!("{unrelated}:{a}"),
            format!("{b}:{unrelated}"),
        ] {
            assert_eq!(fake.lines(&path), first, "{path}");
        }
        // Labels keep the linker script's own spelling; the path field is canonical.
        let lib = fake.lib().display().to_string();
        let real = std::fs::canonicalize(fake.lib())
            .unwrap()
            .display()
            .to_string();
        assert!(first
            .iter()
            .any(|l| l.starts_with(&format!("link:crt1.o\t{real}/crt1.o\t"))));
        assert!(first
            .iter()
            .any(|l| l.starts_with(&format!("link:libc.so>{lib}/libc.so.6\t{real}/libc.so.6\t"))));
        assert!(first.contains(&"link:libgcc_s.so\t-".to_owned()));
        assert!(first.iter().all(|l| !l.contains("rustc")));
        assert!(drift(&first).is_empty());

        fake.put("libc.so.6", b"\x7fELF libc 2.40");
        let libc = fake.lines(&a);
        assert_ne!(libc, first);
        assert_eq!(drift(&first), vec![format!("link:libc.so>{lib}/libc.so.6")]);
        fake.put("crt1.o", b"another crt1");
        assert_ne!(fake.lines(&a), libc);
        assert!(drift(&libc).contains(&"link:crt1.o".to_owned()));
        // A driver whose output changes at the same path moves the identity, but only a full
        // observation sees it: the cheap re-check runs no program.
        let before = fake.lines(&a);
        fake.driver("cc 14.3.2");
        assert_ne!(fake.lines(&a), before);
        assert!(drift(&before).is_empty());
        let empty = fake.root.path().join("empty").display().to_string();
        assert!(
            sysroot_lines(&empty, Instant::now() + Duration::from_secs(5))
                .unwrap_err()
                .to_string()
                .contains("no C link driver cc")
        );
    }

    /// One deadline covers the child and its pipes: a driver that EXITS while a descendant keeps
    /// the pipe open, and a driver that never exits, are both cut off at the deadline, and the
    /// descendant is killed with the group rather than left running.
    #[test]
    fn sysroot_probe_deadline_covers_a_descendant_holding_the_pipe() {
        let fake = FakeSysroot::new();
        let a = fake.bin("a");
        let marker = fake.root.path().join("descendant.pid");
        for body in [
            // The driver exits at once; its background child holds stdout and stderr open.
            format!(
                "sleep_ /bin/sh -c 'echo $$ > {}; exec {} 30' & exit 0",
                marker.display(),
                sleep_bin()
            ),
            // The driver itself waits on the sleeper.
            format!("sleep_ {} 30; true", sleep_bin()),
        ] {
            fake.driver(&body);
            let started = Instant::now();
            let error = sysroot_lines(&a, Instant::now() + Duration::from_millis(300)).unwrap_err();
            let elapsed = started.elapsed();
            assert!(
                error.to_string().contains("exceeded the probe deadline"),
                "{body}: {error}"
            );
            assert!(elapsed < Duration::from_secs(3), "{body}: {elapsed:?}");
        }
        // The exiting driver's descendant was killed with its process group.
        let pid: i32 = std::fs::read_to_string(&marker)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let gone = (0..100).any(|_| {
            std::thread::sleep(Duration::from_millis(10));
            !Path::new(&format!("/proc/{pid}")).exists()
                || std::fs::read_to_string(format!("/proc/{pid}/stat"))
                    .is_ok_and(|stat| stat.contains(") Z "))
        });
        assert!(gone, "descendant {pid} outlived the probe");
    }

    /// A probe's captured output is capped; a driver that floods its pipe is refused.
    #[test]
    fn sysroot_probe_output_is_capped() {
        let fake = FakeSysroot::new();
        let a = fake.bin("a");
        let head = ["/usr/bin/head", "/bin/head"]
            .into_iter()
            .find(|p| Path::new(p).exists())
            .unwrap();
        fake.driver(&format!(
            "sleep_ {head} -c {} /dev/zero",
            PROBE_OUTPUT_LIMIT + 1
        ));
        let error = sysroot_lines(&a, Instant::now() + Duration::from_secs(20)).unwrap_err();
        assert!(error.to_string().contains("wrote more than"), "{error}");
    }

    /// Files are read within the per-file and total bounds, never through a link or from a
    /// non-regular file, for the probe and for the re-check alike.
    #[test]
    fn sysroot_reads_are_bounded_per_file_and_in_total() {
        let fake = FakeSysroot::new();
        let a = fake.bin("a");
        let lines = fake.lines(&a);
        let budget = |per_file, remaining| ReadBudget {
            deadline: Instant::now() + Duration::from_secs(20),
            per_file,
            remaining,
        };
        let large = sysroot_lines_within(&a, &mut budget(8, SYSROOT_TOTAL_LIMIT)).unwrap_err();
        assert!(large.to_string().contains("read bound"), "{large}");
        let total = sysroot_lines_within(&a, &mut budget(SYSROOT_FILE_LIMIT, 40)).unwrap_err();
        assert!(total.to_string().contains("read bound"), "{total}");
        let recheck = sysroot_drift_within(&lines, &mut budget(8, SYSROOT_TOTAL_LIMIT));
        assert!(recheck.unwrap_err().contains("read bound"));
        let expired = sysroot_drift_within(
            &lines,
            &mut ReadBudget {
                deadline: Instant::now(),
                per_file: SYSROOT_FILE_LIMIT,
                remaining: SYSROOT_TOTAL_LIMIT,
            },
        );
        assert!(expired.unwrap_err().contains("exceeded the probe deadline"));
        // A FIFO in place of a recorded file is outside the bounds: not read, never waited on.
        let crt = fake.lib().join("crt1.o");
        std::fs::remove_file(&crt).unwrap();
        assert!(std::process::Command::new("mkfifo")
            .arg(&crt)
            .status()
            .unwrap()
            .success());
        let fifo = sysroot_drift(&lines, Instant::now() + Duration::from_secs(20));
        assert!(fifo.unwrap_err().contains("not a regular file"));
        // A file that is gone is a change, not an error.
        std::fs::remove_file(&crt).unwrap();
        assert_eq!(drift(&lines), vec!["link:crt1.o".to_owned()]);
    }

    /// The re-check refuses a recorded line outside the probe's grammar before using any path.
    #[test]
    fn sysroot_recorded_lines_must_match_the_probe_grammar() {
        let fake = FakeSysroot::new();
        let lines = fake.lines(&fake.bin("a"));
        assert!(check_sysroot_lines(&lines).is_ok());
        let cid = BlobCid::compute_bytes(b"x").to_string();
        let driver = format!("tool:link-driver\t/usr/bin/gcc\t{cid}");
        let with = |extra: &str| {
            let mut lines = vec![driver.clone(), extra.to_owned()];
            lines.sort();
            lines
        };
        assert!(
            check_sysroot_lines(&with(&format!("link:crt1.o\t/usr/lib/crt1.o\t{cid}"))).is_ok()
        );
        for bad in [
            format!("link:crt1.o\tusr/lib/crt1.o\t{cid}"),
            format!("link:crt1.o\t/usr/lib/../../etc/shadow\t{cid}"),
            format!("link:secret\t/etc/shadow\t{cid}"),
            format!("link:libc.so>etc/x\t/usr/lib/x\t{cid}"),
            "link:crt1.o\t/usr/lib/crt1.o\tnot-a-cid".to_owned(),
            "link:crt1.o\t/usr/lib/crt1.o".to_owned(),
            "link:libc.so>/x\t-".to_owned(),
            format!("link:crt1.o\t/usr/lib/crt1.o\t{cid}\textra"),
        ] {
            assert!(check_sysroot_lines(&with(&bad)).is_err(), "{bad}");
        }
        // No driver line, no link line with a path, nothing at all, or unsorted: refused.
        let link = format!("link:crt1.o\t/usr/lib/crt1.o\t{cid}");
        assert!(check_sysroot_lines(std::slice::from_ref(&link)).is_err());
        assert!(check_sysroot_lines(&with("link:crt1.o\t-")).is_err());
        assert!(check_sysroot_lines(&[]).is_err());
        assert!(check_sysroot_lines(&[driver, link]).is_err());
    }
}
