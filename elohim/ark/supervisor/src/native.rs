//! The Native driver: a declared child becomes an operating-system process.
//!
//! Children are spawned with [`std::process::Command`] and never with `tokio::process`
//! (spec §12 item 19): the envelope owns every death, and a runtime that reaps its own
//! children would consume the exit status a witness is made of. The handle returned by
//! `spawn` is dropped once its pipes have been taken; [`crate::reaper`] does the waiting.

use std::{
    fs::File,
    io::{Read, Write},
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use ark_core::{
    berth::{Berth, PassphraseSource},
    manifest::{ArtifactRef, ChildSpec, ProcessKind, StdinSource},
    passport::EffectiveTier,
    sample::ProcessSample,
};
use nix::{
    sys::signal::{kill, Signal},
    unistd::{access, AccessFlags, Pid},
};
use sha2::{Digest, Sha256};

use crate::{
    driver::{Driver, DriverError, Fingerprint, Started},
    reaper::proc_status_sample,
};

/// Bytes hashed per read while fingerprinting an artifact.
///
/// A conductor binary is hundreds of megabytes; streaming it in fixed chunks keeps the
/// pre-spawn check bounded in memory whatever the artifact's size.
const HASH_CHUNK_BYTES: usize = 1024 * 1024;

/// The environment variables that survive a scrub.
///
/// `env_scrub` exists so a child inherits nothing it was not given, but a process with no
/// `PATH` cannot resolve a helper and one with no `HOME` writes dotfiles into `/`. These two
/// are passed through from the parent when the parent has them, and are documented here
/// because they are the entire exception.
const SCRUB_SURVIVORS: [&str; 2] = ["PATH", "HOME"];

/// The cgroup-v2 delegation file, probed read-only for writability.
const CGROUP_SUBTREE_CONTROL: &str = "/sys/fs/cgroup/cgroup.subtree_control";

/// Starts children as native operating-system processes on this host.
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeDriver;

/// Streams a file through SHA-256 and returns its lowercase hexadecimal digest.
///
/// This is the passport's arithmetic: the digest is taken over the bytes that are about to
/// be executed, so what the manifest pinned and what the host runs are the same claim.
pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; HASH_CHUNK_BYTES];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

impl Driver for NativeDriver {
    fn fingerprint(&self) -> Fingerprint {
        Fingerprint {
            hostname: proc_line("/proc/sys/kernel/hostname"),
            kernel: proc_line("/proc/sys/kernel/osrelease"),
            // `access` asks the kernel a question and writes nothing; false on every host
            // that has not delegated a cgroup subtree to this process, which is all of S0.
            cgroup_v2_delegated: access(CGROUP_SUBTREE_CONTROL, AccessFlags::W_OK).is_ok(),
            effective_tier: EffectiveTier::None,
        }
    }

    fn start(&self, spec: &ChildSpec, berth: &Berth) -> Result<Started, DriverError> {
        if spec.kind != ProcessKind::Native {
            return Err(DriverError::UnsupportedKind(spec.kind.clone()));
        }

        let pinned = match &spec.artifact {
            ArtifactRef::Channel { channel_id } => {
                return Err(DriverError::ChannelUnresolvedInS0 {
                    channel_id: channel_id.clone(),
                })
            }
            ArtifactRef::Pinned { sha256, .. } => sha256.clone(),
        };

        let declared = berth
            .artifacts
            .get(&spec.name)
            .cloned()
            // No entry at all: there is no path to name, which the empty path records.
            .ok_or_else(|| DriverError::ArtifactMissing(PathBuf::new()))?;
        // Absolutised BEFORE the hash, so that the file hashed here and the file `exec`ed
        // below cannot be two different files. The child is spawned with
        // `current_dir(data_root)`, and a relative program is resolved by the kernel against
        // that new directory — so a relative artifact would be hashed against the
        // supervisor's cwd and executed against the berth's.
        let path = absolute_artifact_path(&declared)?;

        let actual = match sha256_file(&path) {
            Ok(digest) => digest,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(DriverError::ArtifactMissing(path))
            }
            Err(error) => {
                return Err(DriverError::Spawn(format!(
                    "reading {} to hash it: {error}",
                    path.display()
                )))
            }
        };
        if actual != pinned {
            // Refused before any process exists: a mismatch is exit 66, never a warning.
            return Err(DriverError::ArtifactHashMismatch {
                expected: pinned,
                actual,
                path,
            });
        }

        let mut command = self.command(spec, berth, &path)?;
        let passphrase = match spec.stdin {
            StdinSource::Passphrase => {
                command.stdin(Stdio::piped());
                Some(passphrase(berth)?)
            }
            StdinSource::Null => {
                command.stdin(Stdio::null());
                None
            }
        };

        let mut child = command
            .spawn()
            .map_err(|error| DriverError::Spawn(format!("{}: {error}", path.display())))?;
        let started_at_epoch_ms = epoch_ms();
        let pid = child.id();

        match take_handles(&mut child, passphrase.as_deref()) {
            Ok((stdout, stderr)) => Ok(Started {
                pid,
                stdout,
                stderr,
                artifact_sha256: actual,
                artifact_path: path,
                started_at_epoch_ms,
            }),
            Err(error) => {
                // The child exists but the caller will never hold it: kill and reap it here
                // rather than return an error that leaks a process nobody supervises.
                let _ = child.kill();
                let _ = child.wait();
                Err(error)
            }
        }
    }

    fn signal(&self, pid: u32, signal: i32) -> Result<(), DriverError> {
        let signal = Signal::try_from(signal)
            .map_err(|error| DriverError::Signal(format!("signal {signal}: {error}")))?;
        kill(Pid::from_raw(pid as i32), signal)
            .map_err(|error| DriverError::Signal(format!("kill({pid}): {error}")))
    }

    fn stats(&self, pid: u32) -> Option<ProcessSample> {
        proc_status_sample(pid)
    }
}

impl NativeDriver {
    /// Builds the command for a verified artifact, resolving every template against the berth.
    fn command(
        &self,
        spec: &ChildSpec,
        berth: &Berth,
        path: &Path,
    ) -> Result<Command, DriverError> {
        let resolve = |template: &str| {
            berth
                .resolve_template(&spec.name, path, template)
                .map_err(|error| DriverError::Template(error.to_string()))
        };

        let mut argv = Vec::with_capacity(spec.argv.len());
        for arg in &spec.argv {
            argv.push(resolve(arg)?);
        }

        // The program is the file that was hashed, not `argv[0]`: the passport's claim is
        // about bytes, so the declared `argv[0]` is passed as a name and never as a lookup.
        let mut command = Command::new(path);
        if let Some(argv0) = argv.first() {
            command.arg0(argv0);
        }
        command.args(argv.iter().skip(1));

        if spec.env_scrub {
            command.env_clear();
            for key in SCRUB_SURVIVORS {
                // `var_os`, not `var`: an environment value is bytes, not text, and a `HOME`
                // or `PATH` that is not valid UTF-8 must be passed through unchanged rather
                // than silently dropped by a lossy decode the child never asked for.
                if let Some(value) = std::env::var_os(key) {
                    command.env(key, value);
                }
            }
        }
        for (key, value) in &spec.env {
            command.env(key, resolve(value)?);
        }

        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        if !berth.data_root.as_os_str().is_empty() {
            command.current_dir(&berth.data_root);
        }
        Ok(command)
    }
}

/// Resolves an artifact path to an absolute one before anything is hashed or executed.
///
/// [`std::fs::canonicalize`] is asked first because it answers with the file the kernel will
/// actually open — symlinks resolved, `..` collapsed — which is the identity the passport
/// claims. When it fails the path is merely joined onto the current directory: a missing
/// artifact must still reach [`DriverError::ArtifactMissing`] naming a path someone can look
/// at, rather than being swallowed here as a resolution failure.
fn absolute_artifact_path(declared: &Path) -> Result<PathBuf, DriverError> {
    match std::fs::canonicalize(declared) {
        Ok(resolved) => Ok(resolved),
        Err(_) => std::env::current_dir()
            .map(|cwd| cwd.join(declared))
            .map_err(|error| {
                DriverError::Spawn(format!(
                    "resolving {} against the current directory: {error}",
                    declared.display()
                ))
            }),
    }
}

/// Writes the passphrase, closes stdin, and takes the output pipes.
///
/// Closing stdin is not incidental: a conductor reading a passphrase waits on EOF, so the
/// handle is dropped here rather than handed on to anyone who might hold it open.
fn take_handles(
    child: &mut Child,
    passphrase: Option<&str>,
) -> Result<(std::process::ChildStdout, std::process::ChildStderr), DriverError> {
    if let Some(secret) = passphrase {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| DriverError::Spawn("stdin pipe was not created".to_string()))?;
        stdin
            .write_all(secret.as_bytes())
            .and_then(|()| stdin.write_all(b"\n"))
            .map_err(|error| DriverError::Spawn(format!("writing the passphrase: {error}")))?;
    }

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| DriverError::Spawn("stdout pipe was not created".to_string()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| DriverError::Spawn("stderr pipe was not created".to_string()))?;
    Ok((stdout, stderr))
}

/// Resolves the berth's passphrase to the exact bytes a child will read.
fn passphrase(berth: &Berth) -> Result<String, DriverError> {
    match &berth.passphrase {
        PassphraseSource::Empty => Ok(String::new()),
        PassphraseSource::Literal(secret) => Ok(secret.clone()),
        PassphraseSource::File(path) => std::fs::read_to_string(path)
            .map(strip_one_terminal_line_ending)
            .map_err(|error| {
                DriverError::Spawn(format!("passphrase file {}: {error}", path.display()))
            }),
    }
}

/// Removes exactly one trailing `\n` or `\r\n` and nothing else.
///
/// A passphrase file written by an editor ends in a newline that is not part of the secret,
/// and the driver appends its own terminator — so one line ending comes off. Everything else
/// stays: leading spaces, trailing spaces, interior tabs and blank lines are all secret
/// material, and `trim()` here would silently unlock a conductor with the wrong passphrase
/// (or, worse, lock one out of a passphrase it was created with).
fn strip_one_terminal_line_ending(mut contents: String) -> String {
    if contents.ends_with('\n') {
        contents.pop();
        if contents.ends_with('\r') {
            contents.pop();
        }
    }
    contents
}

fn proc_line(path: &str) -> String {
    std::fs::read_to_string(path)
        .map(|value| value.trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

fn epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn sha256_streams_a_file_larger_than_one_chunk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("artifact");
        // Two chunks and a remainder, so the streaming loop is exercised rather than a
        // single read that would pass whatever the chunk size was.
        let bytes = vec![0x61u8; HASH_CHUNK_BYTES * 2 + 7];
        std::fs::write(&path, &bytes).unwrap();

        let mut expected = Sha256::new();
        expected.update(&bytes);

        assert_eq!(
            sha256_file(&path).unwrap(),
            hex::encode(expected.finalize())
        );
    }

    #[test]
    fn a_missing_berth_entry_is_named_before_any_hashing() {
        let spec = ChildSpec {
            name: "absent".into(),
            argv: vec!["{artifact}".into()],
            ..Default::default()
        };

        assert!(matches!(
            NativeDriver.start(&spec, &Berth::default()),
            Err(DriverError::ArtifactMissing(path)) if path.as_os_str().is_empty()
        ));
    }

    #[test]
    fn a_non_native_kind_is_refused() {
        let spec = ChildSpec {
            name: "wasm".into(),
            kind: ProcessKind::Wasm,
            ..Default::default()
        };

        assert!(matches!(
            NativeDriver.start(&spec, &Berth::default()),
            Err(DriverError::UnsupportedKind(ProcessKind::Wasm))
        ));
    }

    #[test]
    fn the_scrub_keeps_path_and_home_and_resolves_env_templates() {
        let dir = tempfile::tempdir().unwrap();
        let artifact = PathBuf::from("/bin/sh");
        let spec = ChildSpec {
            name: "child".into(),
            argv: vec!["{artifact}".into()],
            env: BTreeMap::from([("ARK_DATA".to_string(), "{data_root}/store".to_string())]),
            ..Default::default()
        };
        let berth = Berth {
            data_root: dir.path().into(),
            ..Default::default()
        };

        let command = NativeDriver.command(&spec, &berth, &artifact).unwrap();
        let env: BTreeMap<_, _> = command
            .get_envs()
            .filter_map(|(key, value)| Some((key.to_owned(), value?.to_owned())))
            .collect();

        assert_eq!(command.get_program(), artifact.as_os_str());
        assert_eq!(
            env.get(std::ffi::OsStr::new("ARK_DATA")).unwrap(),
            std::ffi::OsStr::new(&format!("{}/store", dir.path().display()))
        );
        for key in SCRUB_SURVIVORS {
            // Compared against `var_os`, and byte-for-byte: the survivor a child receives is
            // the parent's raw value, never a lossy re-encoding of it, and a value that is
            // not valid UTF-8 survives rather than reading as absent.
            assert_eq!(
                env.get(std::ffi::OsStr::new(key)).cloned(),
                std::env::var_os(key),
                "{key} should survive the scrub exactly as the parent holds it"
            );
        }
    }

    #[test]
    fn an_unknown_template_is_reported_as_a_template_error() {
        let spec = ChildSpec {
            name: "child".into(),
            argv: vec!["{nowhere}".into()],
            ..Default::default()
        };

        assert!(matches!(
            NativeDriver.command(&spec, &Berth::default(), Path::new("/bin/sh")),
            Err(DriverError::Template(_))
        ));
    }

    #[test]
    fn passphrase_sources_resolve_to_their_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let from_file = |name: &str, contents: &str| {
            let path = dir.path().join(name);
            std::fs::write(&path, contents).unwrap();
            passphrase(&Berth {
                passphrase: PassphraseSource::File(path),
                ..Default::default()
            })
            .unwrap()
        };

        assert_eq!(passphrase(&Berth::default()).unwrap(), "");
        assert_eq!(
            passphrase(&Berth {
                passphrase: PassphraseSource::Literal("  a literal  ".into()),
                ..Default::default()
            })
            .unwrap(),
            "  a literal  "
        );

        // Exactly one terminal line ending comes off — the one an editor added — and every
        // other byte is secret material the child must receive unchanged.
        assert_eq!(from_file("unix", "  from file  \n"), "  from file  ");
        assert_eq!(from_file("dos", "  from file  \r\n"), "  from file  ");
        assert_eq!(from_file("bare", "  from file  "), "  from file  ");
        assert_eq!(from_file("blank-line", "secret\n\n"), "secret\n");
        assert_eq!(
            from_file("interior", "two\twords\nand more\n"),
            "two\twords\nand more"
        );
        assert_eq!(from_file("only-newline", "\n"), "");
    }

    #[test]
    fn the_fingerprint_reads_this_host() {
        let fingerprint = NativeDriver.fingerprint();

        assert!(!fingerprint.hostname.is_empty());
        assert!(!fingerprint.kernel.is_empty());
        assert_eq!(fingerprint.effective_tier, EffectiveTier::None);
    }
}
