//! Append-only publication recovery and the explicitly limited dep-info checks.
use super::*;
use std::os::unix::fs::PermissionsExt;

fn script(f: &Foreign, name: &str, body: &str) -> PathBuf {
    let path = f.dir.path().join(name);
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn notes(f: &Foreign) -> Vec<u8> {
    success(
        f.command("git")
            .args(["for-each-ref", "refs/notes/brit/build"])
            .output()
            .unwrap(),
    )
    .stdout
}

#[test]
fn artifact_failed_attestation_preserves_history_and_reports_orphans() {
    let _cargo = CARGO.lock().unwrap_or_else(|e| e.into_inner());
    for publish_first in [false, true] {
        let Some(f) = Foreign::try_new() else { return };
        f.build();
        success(f.describe());
        let input = f.input();
        success(f.put(&input, true));
        let before = std::fs::read(f.root.join(".eprfs/status/flows.jsonl")).unwrap();
        let notes_before = notes(&f);
        // A second commit supplies an independent publication, preserving the first record.
        f.git(&["commit", "--allow-empty", "-qm", "next artifact"]);
        let input = f.input();
        let publish = if publish_first {
            format!("'{}' \"$@\" || exit 2;", f.brit.display())
        } else {
            String::new()
        };
        let fail = script(&f, "fail-brit", &format!(
            "case \" $* \" in *' build put '*) {publish} echo failed-to-publish >&2; exit 1;; *) exec '{}' \"$@\";; esac", f.brit.display()));
        let out = f
            .put_command(&input, true)
            .env("BRIT_BUILD_REF_BIN", fail)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3), "{}", text(&out));
        assert!(
            text(&out).contains("orphan content-addressed blobs remain"),
            "{}",
            text(&out)
        );
        assert_eq!(
            std::fs::read(f.root.join(".eprfs/status/flows.jsonl")).unwrap(),
            before
        );
        if publish_first {
            assert_ne!(
                notes(&f),
                notes_before,
                "published witness must survive its publisher's error"
            );
        } else {
            assert_eq!(notes(&f), notes_before);
        }
        success(f.put(&input, true));
        let completed = std::fs::read(f.root.join(".eprfs/status/flows.jsonl")).unwrap();
        assert!(completed.starts_with(&before));
        assert_eq!(
            SidecarFlowStore::open(&f.root)
                .unwrap()
                .events()
                .unwrap()
                .len(),
            2
        );
    }
}

#[test]
fn artifact_attested_not_produced_retry_is_idempotent_and_never_removes_flow_records() {
    let _cargo = CARGO.lock().unwrap_or_else(|e| e.into_inner());
    let Some(f) = Foreign::try_new() else { return };
    f.build();
    success(f.describe());
    success(f.put(&f.input(), true));
    let log = f.root.join(".eprfs/status/flows.jsonl");
    let before = std::fs::read(&log).unwrap();
    f.git(&["commit", "--allow-empty", "-qm", "next artifact"]);
    let input = f.input();
    let fail = script(&f, "limit-after-attestation", &format!(
        "case \" $* \" in *' build put '*) '{}' \"$@\" || exit 2; prlimit --pid \"$PPID\" --fsize=0:unlimited;; *) exec '{}' \"$@\";; esac",
        f.brit.display(), f.brit.display()));
    // Ignore SIGXFSZ so the actual append returns EFBIG. Brit imposes the limit only AFTER
    // storing the signed note, so destination preflight and all content writes run normally.
    let launch = script(&f, "ignore-xfsz", "trap '' XFSZ\nexec \"$@\"");
    let mut command = f.command(launch);
    let out = command
        .arg(env!("CARGO_BIN_EXE_epr"))
        .env("BRIT_BUILD_REF_BIN", fail)
        .args(["artifact", "put", "--method"])
        .arg(&f.method)
        .args(["--input", &input, "--bytes"])
        .arg(&f.binary)
        .args([
            "--project",
            "hello",
            "--repository",
            "example/hello",
            "--json",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3), "{}", text(&out));
    assert!(
        text(&out)
            .contains("attested, not produced: re-run put with the same arguments to complete"),
        "{}",
        text(&out)
    );
    assert_eq!(std::fs::read(&log).unwrap(), before);
    let attested = notes(&f);
    // If a retry tried to re-publish, this wrapper would refuse it. Reads still verify via Brit.
    let read_only = script(&f, "no-republish", &format!(
        "case \" $* \" in *' build put '*) echo unexpected-republication >&2; exit 1;; *) exec '{}' \"$@\";; esac", f.brit.display()));
    success(
        f.put_command(&input, true)
            .env("BRIT_BUILD_REF_BIN", &read_only)
            .output()
            .unwrap(),
    );
    let completed = std::fs::read(&log).unwrap();
    assert!(completed.starts_with(&before));
    assert!(completed.len() > before.len());
    assert_eq!(
        SidecarFlowStore::open(&f.root)
            .unwrap()
            .events()
            .unwrap()
            .len(),
        2
    );
    success(
        f.put_command(&input, true)
            .env("BRIT_BUILD_REF_BIN", &read_only)
            .output()
            .unwrap(),
    );
    assert_eq!(notes(&f), attested, "no notes republished or removed");
    assert_eq!(
        std::fs::read(log).unwrap(),
        completed,
        "Produce appended once"
    );
}

#[test]
fn artifact_stale_dep_info_and_different_target_are_missing_facts() {
    let _cargo = CARGO.lock().unwrap_or_else(|e| e.into_inner());
    for wrong_target in [false, true] {
        let Some(f) = Foreign::try_new() else { return };
        f.build();
        success(f.describe());
        let input = f.input();
        let dep = f.binary.with_extension("d");
        let expected = if wrong_target {
            let other = f.dir.path().join("other-binary");
            std::fs::copy(&f.binary, &other).unwrap();
            std::fs::write(
                &dep,
                format!(
                    "{}: {}\n",
                    other.display(),
                    f.root.join("src/main.rs").display()
                ),
            )
            .unwrap();
            "not recorded binary"
        } else {
            let modified = std::fs::metadata(&dep).unwrap().modified().unwrap();
            std::fs::File::options()
                .write(true)
                .open(&f.binary)
                .unwrap()
                .set_modified(modified + std::time::Duration::from_secs(2))
                .unwrap();
            "is newer than dep-info"
        };
        let out = f.put(&input, true);
        assert_eq!(out.status.code(), Some(3), "{}", text(&out));
        for name in [expected, dep.to_str().unwrap(), f.binary.to_str().unwrap()] {
            assert!(text(&out).contains(name), "{name}: {}", text(&out));
        }
        f.absent(&f.descriptor(), &input);
    }
}
