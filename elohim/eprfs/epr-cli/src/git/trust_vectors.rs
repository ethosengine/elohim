//! Mints the checkout-trust vector file every reader of the declaration is held to.
//!
//! The cases' inputs are written here; their expected outputs are computed by this module's own
//! reader (`declaration_with`, `args_for`, `config_env_os` — the bodies `trust_args` and
//! `trust_env` compose — and `device_key::parse_groups`), and the whole document is compared byte
//! for byte with `tests/fixtures/declared-trust-vectors.json`, serialized canonically (object keys
//! sorted, whatever serde_json's map-order feature). A hand edit of the file fails this test; a
//! change to the reader that moves an answer fails it too. Re-mint with
//! `EPR_MINT_TRUST_VECTORS=1 cargo test -p elohim-epr-cli --lib trust_vectors` — that run writes
//! the file and then FAILS on purpose, so an environment that leaves the variable set can never
//! report green — then re-run without it, and bring ark's reader (which runs the same file) along.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use serde_json::{json, Value};

use super::{args_for, config_env_os, declaration_with, trust_file_beside, Declaration};
use crate::device_key::{own_groups, parse_groups};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/declared-trust-vectors.json"
);

/// How long one case may take before the reader is taken to hang (a FIFO open that blocks).
const DEADLINE: Duration = Duration::from_secs(20);

/// The tree every case stands in, relative to its scratch base.
enum Node {
    Dir(&'static str),
    File(&'static str, &'static str),
    Symlink(&'static str, &'static str),
}

const TREE: &[Node] = &[
    Node::Dir("repo/.git"),
    Node::Dir("repo/src/deep"),
    Node::Dir("repo/vendor/sub"),
    Node::File("repo/vendor/sub/.git", "gitdir: ../../.git/modules/sub\n"),
    Node::Dir("other/.git"),
    Node::Symlink("link", "repo"),
    Node::Dir("wt"),
    Node::File("wt/.git", "gitdir: /nowhere/.git/worktrees/wt\n"),
    Node::Dir("plain"),
];

/// What a case puts where the declaration is read.
#[derive(Clone, Copy)]
enum TrustFile {
    Text(&'static str, u32),
    TextThenBytes(&'static str, &'static [u8], u32),
    Filled(usize, u32),
    Directory,
    Fifo,
    Symlink(&'static str),
}

/// The device directory a case creates: a directory with a mode, or a regular file there.
#[derive(Clone, Copy)]
enum DeviceDir {
    Dir(&'static str, u32),
    RegularFile(&'static str, u32),
}

/// The inherited `GIT_CONFIG_COUNT` a case passes on.
#[derive(Clone, Copy)]
enum Count {
    Unset,
    Utf8(&'static str),
    Bytes(&'static [u8]),
}

#[derive(Clone, Copy)]
struct Case {
    name: &'static str,
    env: &'static [(&'static str, &'static str)],
    device_dir: Option<DeviceDir>,
    trust_file: Option<TrustFile>,
    cwd: &'static str,
    count: Count,
}

const KEY_ENV: &str = "ELOHIM_DEVICE_KEY_FILE";
const KEY: &str = "{base}/device/ed25519.seed";
const DEVICE: &str = "{base}/device";
const HOME_DEVICE: &str = "{base}/home/.config/elohim/device";
const REPO: &str = "{base}/repo\n";
const CAP: usize = 1 << 20;

fn case(name: &'static str) -> Case {
    Case {
        name,
        env: &[(KEY_ENV, KEY)],
        device_dir: Some(DeviceDir::Dir(DEVICE, 0o700)),
        trust_file: Some(TrustFile::Text(REPO, 0o600)),
        cwd: "{base}/repo/src/deep",
        count: Count::Unset,
    }
}

fn text(text: &'static str) -> Option<TrustFile> {
    Some(TrustFile::Text(text, 0o600))
}

fn counted(name: &'static str, count: &'static str) -> Case {
    Case {
        count: Count::Utf8(count),
        ..case(name)
    }
}

fn at_home(name: &'static str, env: &'static [(&'static str, &'static str)]) -> Case {
    Case {
        env,
        device_dir: Some(DeviceDir::Dir(HOME_DEVICE, 0o700)),
        ..case(name)
    }
}

fn cases() -> Vec<Case> {
    vec![
        case("declared checkout, deep cwd"),
        counted("inherited count 2", "2"),
        counted("inherited count malformed", "bogus"),
        Case {
            count: Count::Bytes(&[b'1', 0xff]),
            ..case("inherited count not utf-8")
        },
        counted("inherited count overflows", "18446744073709551615"),
        counted("inherited count padded", " 1 "),
        counted("inherited count with plus sign", "+1"),
        counted("inherited count negative", "-1"),
        counted("inherited count empty", ""),
        counted("inherited count zero", "0"),
        Case {
            trust_file: None,
            ..case("no declaration file")
        },
        Case {
            device_dir: None,
            trust_file: None,
            ..case("no device directory")
        },
        Case {
            device_dir: Some(DeviceDir::RegularFile(DEVICE, 0o600)),
            trust_file: None,
            ..case("device directory is a regular file")
        },
        Case {
            trust_file: text("{base}/other\n"),
            ..case("another checkout declared")
        },
        Case {
            cwd: "{base}/repo/vendor/sub",
            ..case("nested checkout under a declared parent")
        },
        Case {
            trust_file: text("*\n{base}\n{base}/repo-sibling\n"),
            ..case("wildcard, parent and prefix lines")
        },
        Case {
            trust_file: text(""),
            ..case("empty declaration")
        },
        Case {
            trust_file: text("\n\n  {base}/repo\n{base}/repo  \n\t{base}/repo\n\n"),
            ..case("blank and padded lines")
        },
        Case {
            trust_file: text("{base}/other\n{base}/repo"),
            ..case("last line without a newline")
        },
        Case {
            trust_file: text("{base}/repo\r\n"),
            ..case("crlf line")
        },
        Case {
            trust_file: text("{base}/repo/\n"),
            ..case("trailing slash line")
        },
        Case {
            trust_file: text("{base}/./repo\n"),
            ..case("dot segment line")
        },
        Case {
            trust_file: Some(TrustFile::TextThenBytes(REPO, &[0xff, 0xfe, b'\n'], 0o600)),
            ..case("declaration with non-utf-8 bytes")
        },
        Case {
            cwd: "{base}/link/src",
            ..case("symlinked cwd resolves to the declared real root")
        },
        Case {
            cwd: "{base}/link",
            trust_file: text("{base}/link\n"),
            ..case("only the symlink path declared")
        },
        Case {
            cwd: "{base}/missing/here",
            ..case("cwd does not exist")
        },
        Case {
            cwd: "{base}/plain",
            ..case("cwd inside no checkout")
        },
        Case {
            trust_file: Some(TrustFile::Text(REPO, 0o666)),
            ..case("world-writable declaration")
        },
        Case {
            device_dir: Some(DeviceDir::Dir(DEVICE, 0o777)),
            ..case("world-writable device directory")
        },
        Case {
            device_dir: Some(DeviceDir::Dir(DEVICE, 0o2770)),
            trust_file: Some(TrustFile::Text(REPO, 0o660)),
            ..case("group-writable by this process's own group")
        },
        Case {
            trust_file: Some(TrustFile::Directory),
            ..case("a directory in the declaration's place")
        },
        Case {
            trust_file: Some(TrustFile::Fifo),
            ..case("a fifo in the declaration's place")
        },
        Case {
            trust_file: Some(TrustFile::Symlink("{base}/repo")),
            ..case("declaration is a symlink to a directory")
        },
        Case {
            trust_file: Some(TrustFile::Symlink("/dev/zero")),
            ..case("declaration is a symlink to an endless device")
        },
        Case {
            trust_file: Some(TrustFile::Filled(CAP + 1, 0o600)),
            ..case("declaration over the size cap")
        },
        Case {
            trust_file: Some(TrustFile::Filled(CAP, 0o600)),
            ..case("declaration at the size cap is read")
        },
        Case {
            env: &[],
            ..case("no key home at all")
        },
        Case {
            env: &[(KEY_ENV, ""), ("XDG_CONFIG_HOME", ""), ("HOME", "")],
            ..case("every key home variable empty")
        },
        Case {
            env: &[(KEY_ENV, "relative/ed25519.seed")],
            ..case("relative key override")
        },
        Case {
            env: &[(KEY_ENV, "{base}/repo/keys/ed25519.seed")],
            ..case("key override inside a repository")
        },
        Case {
            env: &[(KEY_ENV, "{base}/wt/keys/ed25519.seed")],
            ..case("key override inside a linked worktree")
        },
        Case {
            env: &[(KEY_ENV, "/")],
            ..case("key override with no parent")
        },
        Case {
            env: &[("XDG_CONFIG_HOME", "{base}/xdg"), ("HOME", "{base}/home")],
            device_dir: Some(DeviceDir::Dir("{base}/xdg/elohim/device", 0o700)),
            ..case("xdg config home")
        },
        at_home("home config", &[("HOME", "{base}/home")]),
        at_home(
            "relative xdg falls through to home",
            &[("XDG_CONFIG_HOME", "relative"), ("HOME", "{base}/home")],
        ),
        at_home(
            "empty xdg falls through to home",
            &[("XDG_CONFIG_HOME", ""), ("HOME", "{base}/home")],
        ),
        at_home(
            "empty key override falls through to home",
            &[(KEY_ENV, ""), ("HOME", "{base}/home")],
        ),
    ]
}

/// `/proc/self/status` texts the group parse is held to in both readers.
const GROUP_TEXTS: &[(&str, &str)] = &[
    (
        "a normal status",
        "Name:\tcat\nUmask:\t0022\nState:\tR (running)\nTgid:\t41\nUid:\t0\t0\t0\t0\n\
         Gid:\t0\t0\t0\t0\nFDSize:\t64\nGroups:\t0 1234 \nNStgid:\t41\n",
    ),
    (
        "four differing gid fields",
        "Gid:\t100\t200\t300\t400\nGroups:\t500\n",
    ),
    ("an empty groups line", "Gid:\t5\t5\t5\t5\nGroups:\t\n"),
    ("no groups line", "Uid:\t7\t7\t7\t7\nGid:\t7\t7\t7\t8\n"),
    (
        "garbage fields and lines",
        "garbage\nGids:\t3\nGid: x 9 -1 4294967296 10\nGroups:\tabc 11 0x10\n",
    ),
    ("empty text", ""),
];

/// Replace the scratch base with `{base}` and a custody message's numeric group with `{gid}`,
/// so a message reads the same on every host.
fn normalize(message: &str, base: &str) -> String {
    let message = message.replace(base, "{base}");
    let mut out = String::with_capacity(message.len());
    let mut rest = message.as_str();
    while let Some(at) = rest.find("group ") {
        out.push_str(&rest[..at + "group ".len()]);
        rest = &rest[at + "group ".len()..];
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        if digits > 0 && rest[digits..].starts_with(')') {
            out.push_str("{gid}");
            rest = &rest[digits..];
        }
    }
    out.push_str(rest);
    out
}

/// Refuse a scratch base that would make a case answer by accident: inside a checkout (an
/// override would be refused as in-repository), or on a filesystem whose new files take a group
/// this process does not hold (every group-writable case would read as a custody refusal).
fn assert_hermetic(base: &Path) {
    if let Some(repo) = base.ancestors().find(|dir| dir.join(".git").exists()) {
        panic!(
            "the scratch directory {} lies inside the checkout at {}; set TMPDIR outside every \
             repository to run the trust vectors",
            base.display(),
            repo.display()
        );
    }
}

fn assert_own_group(path: &Path) {
    let gid = std::fs::metadata(path).unwrap().gid();
    assert!(
        own_groups().contains(&gid),
        "{} was created with group {gid}, which this process does not hold (a setgid scratch \
         directory?); set TMPDIR to a directory whose new files take one of this process's groups",
        path.display()
    );
}

fn materialize(base: &Path, case: &Case) {
    assert_hermetic(base);
    let b = base.display().to_string();
    let at = |template: &str| PathBuf::from(template.replace("{base}", &b));
    for node in TREE {
        match node {
            Node::Dir(dir) => std::fs::create_dir_all(base.join(dir)).unwrap(),
            Node::File(file, text) => std::fs::write(base.join(file), text).unwrap(),
            Node::Symlink(link, to) => {
                std::os::unix::fs::symlink(base.join(to), base.join(link)).unwrap()
            }
        }
    }
    let chmod = |path: &Path, mode: u32| {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap()
    };
    match case.device_dir {
        None => {}
        Some(DeviceDir::RegularFile(path, mode)) => {
            let path = at(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, "not a directory\n").unwrap();
            chmod(&path, mode);
        }
        Some(DeviceDir::Dir(dir, mode)) => {
            let dir = at(dir);
            std::fs::create_dir_all(&dir).unwrap();
            assert_own_group(&dir);
            let file = dir.join("trusted-checkouts");
            match case.trust_file {
                None => {}
                Some(TrustFile::Text(content, file_mode)) => {
                    std::fs::write(&file, content.replace("{base}", &b)).unwrap();
                    chmod(&file, file_mode);
                    assert_own_group(&file);
                }
                Some(TrustFile::TextThenBytes(content, bytes, file_mode)) => {
                    let mut all = content.replace("{base}", &b).into_bytes();
                    all.extend_from_slice(bytes);
                    std::fs::write(&file, all).unwrap();
                    chmod(&file, file_mode);
                }
                Some(TrustFile::Filled(len, file_mode)) => {
                    std::fs::write(&file, vec![b'a'; len]).unwrap();
                    chmod(&file, file_mode);
                }
                Some(TrustFile::Directory) => std::fs::create_dir(&file).unwrap(),
                Some(TrustFile::Fifo) => {
                    let made = std::process::Command::new("mkfifo")
                        .arg(&file)
                        .status()
                        .unwrap();
                    assert!(made.success(), "mkfifo {}", file.display());
                }
                Some(TrustFile::Symlink(to)) => std::os::unix::fs::symlink(at(to), &file).unwrap(),
            }
            chmod(&dir, mode);
        }
    }
}

fn input_of(case: &Case) -> Value {
    json!({
        "env": case.env.iter().map(|(k, v)| ((*k).to_owned(), json!(v))).collect::<serde_json::Map<_, _>>(),
        "deviceDir": match case.device_dir {
            None => Value::Null,
            Some(DeviceDir::Dir(path, mode)) => json!({ "path": path, "mode": format!("{mode:o}") }),
            Some(DeviceDir::RegularFile(path, mode)) => {
                json!({ "path": path, "mode": format!("{mode:o}"), "regularFile": true })
            }
        },
        "trustFile": match case.trust_file {
            None => Value::Null,
            Some(TrustFile::Text(text, mode)) => json!({ "text": text, "mode": format!("{mode:o}") }),
            Some(TrustFile::TextThenBytes(text, bytes, mode)) => {
                json!({ "text": text, "trailingBytes": bytes, "mode": format!("{mode:o}") })
            }
            Some(TrustFile::Filled(len, mode)) => json!({ "filledBytes": len, "mode": format!("{mode:o}") }),
            Some(TrustFile::Directory) => json!({ "directory": true }),
            Some(TrustFile::Fifo) => json!({ "fifo": true }),
            Some(TrustFile::Symlink(to)) => json!({ "symlinkTo": to }),
        },
        "cwd": case.cwd,
        "inheritedCount": match case.count {
            Count::Unset => Value::Null,
            Count::Utf8(text) => json!({ "utf8": text }),
            Count::Bytes(bytes) => json!({ "bytes": bytes }),
        },
    })
}

fn answer(case: Case) -> Value {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().canonicalize().unwrap();
    materialize(&base, &case);
    let b = base.display().to_string();
    let sub = |s: &str| s.replace("{base}", &b);
    let env: BTreeMap<String, OsString> = case
        .env
        .iter()
        .map(|(k, v)| ((*k).to_owned(), OsString::from(sub(v))))
        .collect();
    let lookup = |name: &str| env.get(name).cloned();
    let count = match case.count {
        Count::Unset => None,
        Count::Utf8(text) => Some(OsString::from(text)),
        Count::Bytes(bytes) => Some(OsString::from_vec(bytes.to_vec())),
    };
    let cwd = PathBuf::from(sub(case.cwd));

    let trust_file = crate::device_key::resolve_path_with(lookup)
        .and_then(|key| trust_file_beside(&key))
        .ok()
        .map(|file| normalize(&file.display().to_string(), &b));
    let (root, git_args, config_env, class, message) = match declaration_with(&cwd, &lookup) {
        Declaration::Trusted(root) => (
            Some(normalize(&root.display().to_string(), &b)),
            args_for(&root),
            config_env_os(count.as_deref(), &root),
            None,
            None,
        ),
        Declaration::Undeclared => (None, Vec::new(), Vec::new(), None, None),
        Declaration::Unapplied(why) => (
            None,
            Vec::new(),
            Vec::new(),
            Some(why.class()),
            Some(normalize(why.reason(), &b)),
        ),
    };
    json!({
        "name": case.name,
        "input": input_of(&case),
        "expect": {
            "trustFile": trust_file,
            "root": root,
            "gitArgs": git_args.iter().map(|a| normalize(a, &b)).collect::<Vec<_>>(),
            "configEnv": config_env.iter().map(|(k, v)| [k.clone(), normalize(v, &b)]).collect::<Vec<_>>(),
            "unapplied": class,
            "unappliedMessage": message,
        },
    })
}

/// One case's answer, or a panic naming the case if the reader does not answer in time.
fn answer_within_deadline(case: Case) -> Value {
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = send.send(answer(case));
    });
    receive.recv_timeout(DEADLINE).unwrap_or_else(|error| {
        panic!(
            "case `{}` gave no answer within {DEADLINE:?} ({error}): the reader hangs, or the \
             case panicked (see above)",
            case.name
        )
    })
}

/// `value` with every object's keys inserted in sorted order, so serialization is canonical
/// whether or not serde_json's `preserve_order` feature is on.
fn canonical(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let sorted: BTreeMap<&String, &Value> = map.iter().collect();
            Value::Object(
                sorted
                    .into_iter()
                    .map(|(k, v)| (k.clone(), canonical(v)))
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(items.iter().map(canonical).collect()),
        other => other.clone(),
    }
}

fn minted() -> String {
    let tree: Vec<Value> = TREE
        .iter()
        .map(|node| match node {
            Node::Dir(dir) => json!({ "dir": dir }),
            Node::File(file, text) => json!({ "file": file, "text": text }),
            Node::Symlink(link, to) => json!({ "symlink": link, "to": to }),
        })
        .collect();
    let document = json!({
        "format": "elohim-declared-trust-vectors@2",
        "contract": "elohim/eprfs/epr-cli/src/git.rs (module doc: the checkout-trust declaration)",
        "mintedBy": "elohim/eprfs/epr-cli/src/git/trust_vectors.rs; re-mint with \
                     EPR_MINT_TRUST_VECTORS=1 (that run fails on purpose; re-run without it), \
                     never edit by hand",
        "materialize": "make every tree node under a fresh scratch {base}; then, when deviceDir \
                        is a directory, create it (parents default), put trustFile inside, apply \
                        the file's mode, then the directory's; a regularFile deviceDir is a file \
                        with that mode. Messages replace {base} and a custody message's numeric \
                        group with {gid}.",
        "tree": tree,
        "cases": cases().into_iter().map(answer_within_deadline).collect::<Vec<_>>(),
        "groupParse": GROUP_TEXTS
            .iter()
            .map(|(name, status)| json!({ "name": name, "status": status, "groups": parse_groups(status) }))
            .collect::<Vec<_>>(),
    });
    let mut text = serde_json::to_string_pretty(&canonical(&document)).unwrap();
    text.push('\n');
    text
}

#[test]
fn the_vector_file_is_what_this_reader_answers() {
    let text = minted();
    if std::env::var_os("EPR_MINT_TRUST_VECTORS").is_some() {
        std::fs::write(FIXTURE, &text).unwrap();
        panic!(
            "EPR_MINT_TRUST_VECTORS is set: wrote {FIXTURE}. This run fails on purpose; re-run \
             without the variable to check the file, then bring ark's reader along"
        );
    }
    let on_disk = std::fs::read_to_string(FIXTURE).unwrap_or_default();
    assert!(
        on_disk == text,
        "{FIXTURE} differs from what epr-cli's reader answers; re-mint it with \
         EPR_MINT_TRUST_VECTORS=1 and bring ark's reader along"
    );
}

/// The predicate's central branch: group-writable by a group this process does not hold.
#[test]
#[ignore = "needs root to chgrp a scratch directory"]
fn a_declaration_writable_by_a_foreign_group_grants_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().canonicalize().unwrap();
    materialize(&base, &case("foreign group"));
    let device = base.join("device");
    let file = device.join("trusted-checkouts");
    let stranger = (4_242_424..)
        .find(|gid| !own_groups().contains(gid))
        .unwrap();
    let key = device.join("ed25519.seed").into_os_string();
    let lookup = |name: &str| (name == "ELOHIM_DEVICE_KEY_FILE").then(|| key.clone());
    let cwd = base.join("repo");
    let custody = |what: &str| match declaration_with(&cwd, &lookup) {
        Declaration::Unapplied(why) => {
            assert_eq!(why.class(), "custody");
            assert!(
                why.reason().starts_with(what)
                    && why.reason().contains("a group this process is not in"),
                "{}",
                why.reason()
            );
        }
        other => panic!("expected a custody refusal for {what}, got {other:?}"),
    };

    std::os::unix::fs::chown(&file, None, Some(stranger))
        .expect("chgrp of the scratch declaration needs root");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o620)).unwrap();
    custody("the checkout-trust declaration");

    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o640)).unwrap();
    assert!(matches!(
        declaration_with(&cwd, &lookup),
        Declaration::Trusted(_)
    ));

    std::os::unix::fs::chown(&device, None, Some(stranger))
        .expect("chgrp of the scratch directory needs root");
    std::fs::set_permissions(&device, std::fs::Permissions::from_mode(0o770)).unwrap();
    custody("the device key directory");
}
