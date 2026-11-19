//! The bundled-C exercise: run, in this process, the C code a release links and no Rust check
//! reaches (row 29.2 of the native-throughput sprint).
//!
//! A release built on one device can carry C compiled for a newer CPU than another device has:
//! shem's peer daemon died with SIGILL in ring's plain-C X25519, built on a device whose C
//! compiler defaulted to x86-64-v3. The exercise runs that path and two of its neighbours: one
//! ring X25519 key agreement, one ring Ed25519 sign and verify, and one SQLite round trip with a
//! date and a float format. An instruction this CPU lacks kills the process by signal, and the
//! process that ran it (`epr doctor --exercise`) is the evidence. It runs two paths out of many;
//! a clean run proves those paths, not every byte.
//!
//! [`judge`] runs another executable's exercise with a timeout, for a release this device holds
//! but did not build (a sibling's draw, or an explicit land or pin of one).

use std::{
    io::Read,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

/// A fault the exercise injects, for the tests that must see it fail: honoured only with
/// `EPR_UNDER_TEST=1`, and it can only make the exercise fail, never pass.
#[cfg(feature = "peer")]
const FAULT_ENV: &str = "EPR_EXERCISE_FAULT";

/// One exercise step that ran and did not give the answer it must.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// `x25519`, `ed25519` or `sqlite`.
    pub step: &'static str,
    pub detail: String,
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.step, self.detail)
    }
}

/// Run every step in order. A step that traps never returns: the process dies by signal.
#[cfg(feature = "peer")]
pub fn run() -> Result<(), Failure> {
    let fault = (std::env::var("EPR_UNDER_TEST").as_deref() == Ok("1"))
        .then(|| std::env::var(FAULT_ENV).ok())
        .flatten();
    x25519()?;
    ed25519()?;
    match fault.as_deref() {
        // A process killed by a signal, as a CPU without an instruction kills it.
        Some("abort") => std::process::abort(),
        Some("fail") => {
            return Err(Failure {
                step: "sqlite",
                detail: format!("{FAULT_ENV}=fail"),
            })
        }
        _ => {}
    }
    sqlite()
}

#[cfg(feature = "peer")]
fn fail(step: &'static str, detail: impl std::fmt::Display) -> Failure {
    Failure {
        step,
        detail: detail.to_string(),
    }
}

/// Two fresh X25519 keypairs agree on one secret.
#[cfg(feature = "peer")]
fn x25519() -> Result<(), Failure> {
    use ring::agreement::{agree_ephemeral, EphemeralPrivateKey, UnparsedPublicKey, X25519};
    let rng = ring::rand::SystemRandom::new();
    let key = || {
        let private = EphemeralPrivateKey::generate(&X25519, &rng)
            .map_err(|_| fail("x25519", "no key generated"))?;
        let public = private
            .compute_public_key()
            .map_err(|_| fail("x25519", "no public key computed"))?;
        Ok::<_, Failure>((private, public.as_ref().to_vec()))
    };
    let (a, a_public) = key()?;
    let (b, b_public) = key()?;
    let ab = agree_ephemeral(
        a,
        &UnparsedPublicKey::new(&X25519, &b_public),
        <[u8]>::to_vec,
    )
    .map_err(|_| fail("x25519", "agreement A to B refused"))?;
    let ba = agree_ephemeral(
        b,
        &UnparsedPublicKey::new(&X25519, &a_public),
        <[u8]>::to_vec,
    )
    .map_err(|_| fail("x25519", "agreement B to A refused"))?;
    if ab != ba || ab.iter().all(|byte| *byte == 0) {
        return Err(fail("x25519", "the two sides derived different secrets"));
    }
    Ok(())
}

/// One Ed25519 signature verifies, and the same signature over another message does not.
#[cfg(feature = "peer")]
fn ed25519() -> Result<(), Failure> {
    use ring::signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519};
    let rng = ring::rand::SystemRandom::new();
    let pkcs8 =
        Ed25519KeyPair::generate_pkcs8(&rng).map_err(|_| fail("ed25519", "no key generated"))?;
    let pair = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref())
        .map_err(|_| fail("ed25519", "the generated key does not parse"))?;
    let message = b"epr doctor --exercise";
    let signature = pair.sign(message);
    let public = UnparsedPublicKey::new(&ED25519, pair.public_key().as_ref());
    public
        .verify(message, signature.as_ref())
        .map_err(|_| fail("ed25519", "a fresh signature does not verify"))?;
    if public
        .verify(b"another message", signature.as_ref())
        .is_ok()
    {
        return Err(fail("ed25519", "a signature verified over another message"));
    }
    Ok(())
}

/// An in-memory SQLite database: create, insert, then select in order with a date function, a
/// `strftime` and a float formatted as text.
#[cfg(feature = "peer")]
fn sqlite() -> Result<(), Failure> {
    let wrong = |e: rusqlite::Error| fail("sqlite", e);
    let db = rusqlite::Connection::open_in_memory().map_err(wrong)?;
    db.execute_batch(
        "CREATE TABLE draw (id INTEGER PRIMARY KEY, amount REAL NOT NULL, day TEXT NOT NULL);
         INSERT INTO draw (id, amount, day) VALUES
           (1, 2.5, '2026-11-19'), (2, 160.1, '2026-02-28'), (3, 0.75, '2024-12-31');",
    )
    .map_err(wrong)?;
    let mut query = db
        .prepare(
            "SELECT id, amount * 2, printf('%.2f', amount), date(day, '+1 day'),
                    strftime('%Y/%m/%d', day)
             FROM draw ORDER BY amount DESC",
        )
        .map_err(wrong)?;
    let rows: Vec<(i64, f64, String, String, String)> = query
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .map_err(wrong)?
        .collect::<Result<_, _>>()
        .map_err(wrong)?;
    let expected: [(i64, f64, &str, &str, &str); 3] = [
        (2, 320.2, "160.10", "2026-03-01", "2026/02/28"),
        (1, 5.0, "2.50", "2026-11-20", "2026/11/19"),
        (3, 1.5, "0.75", "2025-01-01", "2024/12/31"),
    ];
    let matches = rows.len() == expected.len()
        && rows.iter().zip(expected).all(|(row, want)| {
            row.0 == want.0
                && row.1.to_bits() == want.1.to_bits()
                && row.2 == want.2
                && row.3 == want.3
                && row.4 == want.4
        });
    if !matches {
        return Err(fail("sqlite", format!("unexpected rows {rows:?}")));
    }
    Ok(())
}

/// `epr doctor --exercise`: one line and exit 0, or the failing step and exit 1.
#[cfg(feature = "peer")]
pub fn main() -> std::process::ExitCode {
    let started = Instant::now();
    match run() {
        Ok(()) => {
            println!(
                "exercise: ok (x25519, ed25519, sqlite) in {:.1} ms",
                started.elapsed().as_secs_f64() * 1000.0
            );
            std::process::ExitCode::SUCCESS
        }
        Err(failure) => {
            eprintln!("exercise: failed at {failure}");
            std::process::ExitCode::FAILURE
        }
    }
}

/// How another executable's `doctor --exercise` ended, when it did not end well.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Killed by this signal: an instruction this CPU lacks, or another fatal fault.
    Signal(i32),
    /// Still running when the timeout came; it was killed.
    Timeout(Duration),
    /// Exited non-zero; a release from before the exercise has no such verb.
    Exit(i32, String),
    /// It could not be started at all.
    Unstarted(String),
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Signal(signal) => write!(
                f,
                "signal: `doctor --exercise` died by signal {signal}{}",
                if *signal == libc::SIGILL {
                    " (SIGILL: an instruction this CPU does not have)"
                } else {
                    ""
                }
            ),
            Self::Timeout(after) => write!(
                f,
                "timeout: `doctor --exercise` was still running after {} ms and was killed",
                after.as_millis()
            ),
            Self::Exit(code, said) => write!(
                f,
                "exit: `doctor --exercise` exited {code} (a release from before the exercise has \
                 no such verb){}",
                if said.is_empty() {
                    String::new()
                } else {
                    format!(": {said}")
                }
            ),
            Self::Unstarted(why) => write!(f, "unstarted: `doctor --exercise`: {why}"),
        }
    }
}

/// How long a staged or held release may take to exercise. Measured on a Xeon E5-2667 device
/// (x86-64-v2, no AVX2) at load average 13.6 on 24 CPUs (2026-11-19): five runs of the whole
/// debug `epr doctor --exercise` process took 11 ms of wall time each, the steps 1.5 to 1.9 ms
/// of it. Ten seconds is about 900 times the slowest, room for a cold page cache under a 160 MB
/// binary on a loaded device; a release that hangs costs one follow this long before it builds
/// for itself.
pub const TIMEOUT: Duration = Duration::from_secs(10);

/// Run `program doctor --exercise` and wait at most `timeout`. Its output is read but never
/// trusted: only how it ended is the answer.
pub fn judge(program: &Path, timeout: Duration) -> Result<(), Verdict> {
    use std::os::unix::process::ExitStatusExt;
    let mut child = spawn(program)?;
    let mut stderr = child.stderr.take();
    let reader = std::thread::spawn(move || {
        let mut text = String::new();
        if let Some(pipe) = stderr.as_mut() {
            // Bounded: a few lines say which step failed.
            let _ = pipe.take(4096).read_to_string(&mut text);
        }
        text
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Verdict::Timeout(timeout));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(error) => return Err(Verdict::Unstarted(error.to_string())),
        }
    };
    let said = reader.join().unwrap_or_default();
    if let Some(signal) = status.signal() {
        return Err(Verdict::Signal(signal));
    }
    match status.code() {
        Some(0) => Ok(()),
        Some(code) => Err(Verdict::Exit(code, said.trim().replace('\n', "; "))),
        None => Err(Verdict::Unstarted("no exit status".into())),
    }
}

/// Start `program doctor --exercise`. A file just written can still be open for writing in a
/// child another thread forked a moment ago (ETXTBSY); that clears in milliseconds.
fn spawn(program: &Path) -> Result<std::process::Child, Verdict> {
    let mut attempts = 0;
    loop {
        let mut command = Command::new(program);
        command
            .args(["doctor", "--exercise"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        if let Some(dir) = program.parent() {
            command.current_dir(dir);
        }
        match command.spawn() {
            Ok(child) => return Ok(child),
            Err(error) if error.raw_os_error() == Some(libc::ETXTBSY) && attempts < 50 => {
                attempts += 1;
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(error) => {
                return Err(Verdict::Unstarted(format!(
                    "{}: {error}",
                    program.display()
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn script(dir: &Path, body: &str) -> std::path::PathBuf {
        let path = dir.join("epr");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    #[cfg(feature = "peer")]
    #[test]
    fn the_exercise_passes_on_this_host() {
        assert_eq!(run(), Ok(()));
    }

    #[test]
    fn a_release_whose_exercise_dies_by_signal_is_named_as_a_signal_death() {
        let tmp = tempfile::tempdir().unwrap();
        let program = script(tmp.path(), "kill -ILL $$");
        let verdict = judge(&program, TIMEOUT).unwrap_err();
        assert_eq!(verdict, Verdict::Signal(libc::SIGILL));
        assert!(verdict.to_string().starts_with("signal: "), "{verdict}");
        assert!(verdict.to_string().contains("SIGILL"), "{verdict}");
    }

    #[test]
    fn a_release_without_the_verb_is_named_by_its_exit() {
        let tmp = tempfile::tempdir().unwrap();
        let program = script(tmp.path(), "echo 'unknown command `doctor`' >&2; exit 2");
        let verdict = judge(&program, TIMEOUT).unwrap_err();
        assert!(
            matches!(&verdict, Verdict::Exit(2, said) if said.contains("unknown command")),
            "{verdict:?}"
        );
        assert!(verdict.to_string().starts_with("exit: "), "{verdict}");
    }

    #[test]
    fn a_release_whose_exercise_does_not_finish_is_killed_at_the_timeout() {
        let tmp = tempfile::tempdir().unwrap();
        let program = script(tmp.path(), "exec sleep 30");
        let started = Instant::now();
        let verdict = judge(&program, Duration::from_millis(300)).unwrap_err();
        assert_eq!(verdict, Verdict::Timeout(Duration::from_millis(300)));
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "killed promptly"
        );
        assert!(verdict.to_string().starts_with("timeout: "), "{verdict}");
    }

    #[test]
    fn a_release_whose_exercise_exits_zero_passes_and_a_missing_one_is_unstarted() {
        let tmp = tempfile::tempdir().unwrap();
        let program = script(tmp.path(), "test \"$1 $2\" = 'doctor --exercise'");
        assert_eq!(judge(&program, TIMEOUT), Ok(()));
        let missing = judge(&tmp.path().join("absent"), TIMEOUT).unwrap_err();
        assert!(matches!(missing, Verdict::Unstarted(_)), "{missing:?}");
    }
}
