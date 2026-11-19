//! The bundled-C exercise: run, in this process, the C code a release links and no Rust check
//! reaches (row 29.2 of the native-throughput sprint).
//!
//! A release built on one device can carry C compiled for a newer CPU than another device has:
//! shem's peer daemon died with SIGILL in ring's plain-C X25519, built on a device whose C
//! compiler defaulted to x86-64-v3. The exercise runs that path and two of its neighbours: one
//! ring X25519 key agreement between fresh keys, one Ed25519 signature checked against the RFC
//! 8032 test vector, and one SQLite round trip with a date and a float format compared exactly.
//! A trap kills the process by signal, and the process that ran it (`epr doctor --exercise`) is
//! the evidence. It runs three paths out of many; a clean run proves those paths, not every
//! byte. On a CPU with ADX and BMI2, ring's X25519 takes its assembly path, so there the
//! exercise reaches the C only through Ed25519 and SQLite.
//!
//! [`judge`] runs another executable's exercise with a timeout, for a release this device holds
//! but did not build (a sibling's draw, or an explicit land or pin of one).
//!
//! What is not judged here (each needs device-runtime, a storage build input): a held record for
//! the HEAD being followed that fails landing health is returned by the store before the local
//! build, so a device holding such a record repeats the refusal until the record is removed; and
//! `EPR_PIN_CID`, existing session pins, bootstrap reuse of a current or cached candidate,
//! `bin ingest <path>`, a caller-supplied closure and an `EPR_BIN` candidate are never exercised.

use std::{
    io::Read,
    path::Path,
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

/// One exercise step that ran and did not give the answer it must, or could not run at all (key
/// generation fails when the operating system has no randomness to give).
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
    x25519()?;
    ed25519_against(&RFC8032_TEST_1_SIGNATURE)?;
    sqlite_against(&SQLITE_ROWS)
}

#[cfg(feature = "peer")]
fn fail(step: &'static str, detail: impl std::fmt::Display) -> Failure {
    Failure {
        step,
        detail: detail.to_string(),
    }
}

/// Two fresh X25519 keypairs agree on one secret. ring offers no fixed-key agreement outside its
/// test module, so this is the two-way check, not a known answer.
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

/// RFC 8032 section 7.1, TEST 1: the secret key (seed), its public key, and the signature of the
/// empty message.
#[cfg(feature = "peer")]
const RFC8032_TEST_1_SEED: [u8; 32] =
    hex32("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60");
#[cfg(feature = "peer")]
const RFC8032_TEST_1_PUBLIC: [u8; 32] =
    hex32("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a");
#[cfg(feature = "peer")]
const RFC8032_TEST_1_SIGNATURE: [u8; 64] = hex64(
    "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065\
     224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
);

#[cfg(feature = "peer")]
const fn nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        _ => panic!("not a lowercase hex digit"),
    }
}

#[cfg(feature = "peer")]
const fn hex<const N: usize>(text: &str) -> [u8; N] {
    let bytes = text.as_bytes();
    assert!(bytes.len() == 2 * N, "hex length");
    let mut out = [0u8; N];
    let mut i = 0;
    while i < N {
        out[i] = (nibble(bytes[2 * i]) << 4) | nibble(bytes[2 * i + 1]);
        i += 1;
    }
    out
}

#[cfg(feature = "peer")]
const fn hex32(text: &str) -> [u8; 32] {
    hex::<32>(text)
}

#[cfg(feature = "peer")]
const fn hex64(text: &str) -> [u8; 64] {
    hex::<64>(text)
}

/// The RFC 8032 key signs the empty message to exactly `expected`; that signature verifies, and
/// does not verify over another message.
#[cfg(feature = "peer")]
fn ed25519_against(expected: &[u8; 64]) -> Result<(), Failure> {
    use ring::signature::{Ed25519KeyPair, UnparsedPublicKey, ED25519};
    let pair =
        Ed25519KeyPair::from_seed_and_public_key(&RFC8032_TEST_1_SEED, &RFC8032_TEST_1_PUBLIC)
            .map_err(|e| fail("ed25519", format!("the RFC 8032 key does not load: {e}")))?;
    let signature = pair.sign(b"");
    if signature.as_ref() != expected.as_slice() {
        return Err(fail(
            "ed25519",
            "the RFC 8032 test 1 signature came out different",
        ));
    }
    let public = UnparsedPublicKey::new(&ED25519, RFC8032_TEST_1_PUBLIC);
    public
        .verify(b"", signature.as_ref())
        .map_err(|_| fail("ed25519", "the RFC 8032 signature does not verify"))?;
    if public
        .verify(b"another message", signature.as_ref())
        .is_ok()
    {
        return Err(fail("ed25519", "a signature verified over another message"));
    }
    Ok(())
}

/// One row the SQLite step must return: id, amount doubled, amount formatted, the next day and a
/// `strftime` of the day.
#[cfg(feature = "peer")]
type Row = (i64, f64, &'static str, &'static str, &'static str);

#[cfg(feature = "peer")]
const SQLITE_ROWS: [Row; 3] = [
    (2, 320.2, "160.10", "2026-03-01", "2026/02/28"),
    (1, 5.0, "2.50", "2026-11-20", "2026/11/19"),
    (3, 1.5, "0.75", "2025-01-01", "2024/12/31"),
];

/// An in-memory SQLite database: create, insert, then select in order with a date function, a
/// `strftime` and a float formatted as text, compared exactly with `expected`.
#[cfg(feature = "peer")]
fn sqlite_against(expected: &[Row]) -> Result<(), Failure> {
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

/// What `epr doctor --exercise` prints for `outcome`: the success line for stdout, or the failure
/// line for stderr.
pub fn report(outcome: Result<(), Failure>, took: Duration) -> Result<String, String> {
    match outcome {
        Ok(()) => Ok(format!(
            "exercise: ok (x25519, ed25519, sqlite) in {:.1} ms",
            took.as_secs_f64() * 1000.0
        )),
        Err(failure) => Err(format!("exercise: failed at {failure}")),
    }
}

/// `epr doctor --exercise`: one line and exit 0, or the failing step and exit 1.
#[cfg(feature = "peer")]
pub fn main() -> std::process::ExitCode {
    let started = Instant::now();
    match report(run(), started.elapsed()) {
        Ok(line) => {
            println!("{line}");
            std::process::ExitCode::SUCCESS
        }
        Err(line) => {
            eprintln!("{line}");
            std::process::ExitCode::FAILURE
        }
    }
}

/// How another executable's `doctor --exercise` ended, when it did not end well.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Killed by this signal.
    Signal(i32),
    /// Still running, or its stderr still held open by a process it started, when the timeout
    /// came; its whole process group was killed.
    Timeout(Duration),
    /// Exited non-zero; a release from before the exercise has no such verb.
    Exit(i32, String),
    /// It could not be started at all, or its state could not be read.
    Unstarted(String),
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Signal(signal) => write!(
                f,
                "signal: `doctor --exercise` died with signal {signal}{}",
                if *signal == libc::SIGILL {
                    " (SIGILL, illegal instruction)"
                } else {
                    ""
                }
            ),
            Self::Timeout(after) => write!(
                f,
                "timeout: `doctor --exercise` had not finished, or its output was still held \
                 open, after {} ms; its process group was killed",
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

/// Run `program doctor --exercise` in a process group of its own and judge how it ended.
///
/// The bound it keeps: it returns within `timeout`, plus the time to send SIGKILL to the group and
/// reap the direct child. The wait for the exit and the drain of stderr share that one deadline,
/// so a process the exercise started that keeps stderr open cannot hold it longer; at the
/// deadline, and on every return, the whole group is killed. Output is read only to quote a
/// failing step, never trusted: how the process ended is the answer.
pub fn judge(program: &Path, timeout: Duration) -> Result<(), Verdict> {
    use std::os::unix::process::ExitStatusExt;
    let started = Instant::now();
    let mut child = spawn(program)?;
    let group = rustix::process::Pid::from_child(&child);
    let kill_group = || {
        // ESRCH once every member has gone is the expected answer, not an error.
        let _ = rustix::process::kill_process_group(group, rustix::process::Signal::KILL);
    };
    let (sender, said) = mpsc::channel();
    let stderr = child.stderr.take();
    std::thread::spawn(move || {
        let mut kept = Vec::new();
        if let Some(mut pipe) = stderr {
            // Drain to the end so a chatty child never blocks on a full pipe; keep the first
            // few lines, enough to name a failing step.
            let mut chunk = [0u8; 4096];
            while let Ok(read) = pipe.read(&mut chunk) {
                if read == 0 {
                    break;
                }
                let room = 4096usize.saturating_sub(kept.len());
                kept.extend_from_slice(&chunk[..read.min(room)]);
            }
        }
        let _ = sender.send(String::from_utf8_lossy(&kept).into_owned());
    });
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() >= timeout => {
                kill_group();
                let _ = child.wait();
                return Err(Verdict::Timeout(timeout));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(error) => {
                kill_group();
                let _ = child.wait();
                return Err(Verdict::Unstarted(error.to_string()));
            }
        }
    };
    // The pipe closes when every holder has gone; a process the exercise started may hold it.
    let said = said.recv_timeout(timeout.saturating_sub(started.elapsed()));
    kill_group();
    let Ok(said) = said else {
        return Err(Verdict::Timeout(timeout));
    };
    if let Some(signal) = status.signal() {
        return Err(Verdict::Signal(signal));
    }
    match status.code() {
        Some(0) => Ok(()),
        Some(code) => Err(Verdict::Exit(code, said.trim().replace('\n', "; "))),
        None => Err(Verdict::Unstarted("no exit status".into())),
    }
}

/// Start `program doctor --exercise` as the leader of a new process group. A file just written
/// can still be open for writing in a child another thread forked a moment ago (ETXTBSY); that
/// clears in milliseconds.
fn spawn(program: &Path) -> Result<std::process::Child, Verdict> {
    use std::os::unix::process::CommandExt;
    let mut attempts = 0;
    loop {
        let mut command = Command::new(program);
        command
            .args(["doctor", "--exercise"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .process_group(0);
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

    /// Every live (non-zombie) process whose process group is `group`, from `/proc`.
    fn live_members(group: &str) -> Vec<String> {
        let mut live = Vec::new();
        for entry in std::fs::read_dir("/proc").unwrap().flatten() {
            let Ok(stat) = std::fs::read_to_string(entry.path().join("stat")) else {
                continue;
            };
            // pid (comm) state ppid pgrp ...: split after the command's closing parenthesis.
            let Some((_, rest)) = stat.rsplit_once(") ") else {
                continue;
            };
            let fields: Vec<&str> = rest.split_whitespace().collect();
            if fields.get(2) == Some(&group) && fields.first() != Some(&"Z") {
                live.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
        live
    }

    #[cfg(feature = "peer")]
    #[test]
    fn the_exercise_passes_on_this_host() {
        assert_eq!(run(), Ok(()));
    }

    #[cfg(feature = "peer")]
    #[test]
    fn a_wrong_expected_answer_fails_its_step() {
        // Each known-answer step can fail: a no-op exercise would not.
        let mut signature = RFC8032_TEST_1_SIGNATURE;
        signature[0] ^= 1;
        let failure = ed25519_against(&signature).unwrap_err();
        assert_eq!(failure.step, "ed25519");
        assert!(failure.detail.contains("came out different"), "{failure}");
        let mut rows = SQLITE_ROWS;
        rows[0].2 = "160.11";
        assert_eq!(sqlite_against(&rows).unwrap_err().step, "sqlite");
        assert_eq!(sqlite_against(&rows[..2]).unwrap_err().step, "sqlite");
    }

    #[test]
    fn the_report_is_one_line_either_way() {
        let ok = report(Ok(()), Duration::from_micros(1700)).unwrap();
        assert_eq!(ok, "exercise: ok (x25519, ed25519, sqlite) in 1.7 ms");
        let failed = report(
            Err(Failure {
                step: "sqlite",
                detail: "unexpected rows []".into(),
            }),
            Duration::ZERO,
        )
        .unwrap_err();
        assert_eq!(failed, "exercise: failed at sqlite: unexpected rows []");
    }

    #[test]
    fn a_release_whose_exercise_dies_by_signal_is_named_as_a_signal_death() {
        let tmp = tempfile::tempdir().unwrap();
        let program = script(tmp.path(), "kill -ILL $$");
        let verdict = judge(&program, TIMEOUT).unwrap_err();
        assert_eq!(verdict, Verdict::Signal(libc::SIGILL));
        let said = verdict.to_string();
        assert!(said.starts_with("signal: "), "{said}");
        assert!(
            said.contains("died with signal 4 (SIGILL, illegal instruction)"),
            "{said}"
        );
        assert!(
            !said.contains("CPU"),
            "the observer knows the signal, not the cause: {said}"
        );
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
            started.elapsed() < Duration::from_secs(5),
            "killed promptly"
        );
        assert!(verdict.to_string().starts_with("timeout: "), "{verdict}");
    }

    #[test]
    fn a_child_that_keeps_stderr_open_cannot_hold_the_judge_past_its_deadline() {
        // The exercise exits at once, but a process it started keeps stderr open and sleeps.
        // The judge must still answer within its deadline, and leave no member of the group.
        let tmp = tempfile::tempdir().unwrap();
        let group_file = tmp.path().join("group");
        let sleeper_file = tmp.path().join("sleeper");
        let program = script(
            tmp.path(),
            &format!(
                "echo $$ > '{}'; sleep 30 & echo $! > '{}'; exit 0",
                group_file.display(),
                sleeper_file.display()
            ),
        );
        let deadline = Duration::from_millis(500);
        let started = Instant::now();
        let verdict = judge(&program, deadline).unwrap_err();
        let took = started.elapsed();
        assert_eq!(verdict, Verdict::Timeout(deadline));
        assert!(took < deadline + Duration::from_secs(2), "took {took:?}");
        let read = |path: &Path| std::fs::read_to_string(path).unwrap().trim().to_owned();
        let (group, sleeper) = (read(&group_file), read(&sleeper_file));
        // SIGKILL is delivered asynchronously; give the kernel a moment to finish the members.
        let alive = |pid: &str| {
            std::fs::read_to_string(format!("/proc/{pid}/stat"))
                .ok()
                .and_then(|stat| stat.rsplit_once(") ").map(|(_, rest)| rest.to_owned()))
                .is_some_and(|rest| !rest.starts_with('Z'))
        };
        let mut live = live_members(&group);
        for _ in 0..100 {
            if live.is_empty() && !alive(&sleeper) {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
            live = live_members(&group);
        }
        assert!(live.is_empty(), "group {group} still has {live:?}");
        assert!(
            !alive(&sleeper),
            "the process holding stderr ({sleeper}) survived"
        );
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
