//! Mints the checkout-trust vector file every reader of the declaration is held to.
//!
//! The cases' inputs are written here; their expected outputs are computed by this module's own
//! reader (`declaration_with`, `args_for`, `config_env_os` — the bodies `trust_args` and
//! `trust_env` compose), and the whole document is compared byte for byte with
//! `tests/fixtures/declared-trust-vectors.json`. A hand edit of the file fails this test; a change
//! to the reader that moves an answer fails it too, until the file is re-minted with
//! `EPR_MINT_TRUST_VECTORS=1 cargo test -p elohim-epr-cli --lib trust_vectors` and ark's reader
//! (which runs the same file) is brought along.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::{args_for, config_env_os, declaration_with, trust_file_beside, Declaration};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/declared-trust-vectors.json"
);

/// A file the case writes as the declaration: its text, or a directory in its place.
enum TrustFile {
    Text(&'static str, u32),
    Directory,
}

/// The inherited `GIT_CONFIG_COUNT` a case passes on.
enum Count {
    Unset,
    Utf8(&'static str),
    Bytes(&'static [u8]),
}

struct Case {
    name: &'static str,
    env: &'static [(&'static str, &'static str)],
    device_dir: Option<(&'static str, u32)>,
    trust_file: Option<TrustFile>,
    cwd: &'static str,
    count: Count,
}

const KEY_ENV: &str = "ELOHIM_DEVICE_KEY_FILE";
const KEY: &str = "{base}/device/ed25519.seed";
const DEVICE: &str = "{base}/device";
const REPO: &str = "{base}/repo\n";

fn case(name: &'static str) -> Case {
    Case {
        name,
        env: &[(KEY_ENV, KEY)],
        device_dir: Some((DEVICE, 0o700)),
        trust_file: Some(TrustFile::Text(REPO, 0o600)),
        cwd: "{base}/repo/src/deep",
        count: Count::Unset,
    }
}

fn cases() -> Vec<Case> {
    vec![
        case("declared checkout, deep cwd"),
        Case {
            count: Count::Utf8("2"),
            ..case("inherited count 2")
        },
        Case {
            count: Count::Utf8("bogus"),
            ..case("inherited count malformed")
        },
        Case {
            count: Count::Bytes(&[b'1', 0xff]),
            ..case("inherited count not utf-8")
        },
        Case {
            count: Count::Utf8("18446744073709551615"),
            ..case("inherited count overflows")
        },
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
            trust_file: Some(TrustFile::Text("{base}/other\n", 0o600)),
            ..case("another checkout declared")
        },
        Case {
            cwd: "{base}/repo/vendor/sub",
            ..case("nested checkout under a declared parent")
        },
        Case {
            trust_file: Some(TrustFile::Text("*\n{base}\n{base}/repo-sibling\n", 0o600)),
            ..case("wildcard, parent and prefix lines")
        },
        Case {
            trust_file: Some(TrustFile::Text("{base}/repo\r\n", 0o600)),
            ..case("crlf line")
        },
        Case {
            trust_file: Some(TrustFile::Text("{base}/repo/\n", 0o600)),
            ..case("trailing slash line")
        },
        Case {
            cwd: "{base}/link/src",
            ..case("symlinked cwd resolves to the declared real root")
        },
        Case {
            cwd: "{base}/link",
            trust_file: Some(TrustFile::Text("{base}/link\n", 0o600)),
            ..case("only the symlink path declared")
        },
        Case {
            trust_file: Some(TrustFile::Text(REPO, 0o666)),
            ..case("world-writable declaration")
        },
        Case {
            device_dir: Some((DEVICE, 0o777)),
            ..case("world-writable device directory")
        },
        Case {
            device_dir: Some((DEVICE, 0o2770)),
            trust_file: Some(TrustFile::Text(REPO, 0o660)),
            ..case("group-writable by this process's own group")
        },
        Case {
            trust_file: Some(TrustFile::Directory),
            ..case("declaration unreadable")
        },
        Case {
            env: &[],
            ..case("no key home at all")
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
            env: &[("XDG_CONFIG_HOME", "{base}/xdg"), ("HOME", "{base}/home")],
            device_dir: Some(("{base}/xdg/elohim/device", 0o700)),
            ..case("xdg config home")
        },
        Case {
            env: &[("HOME", "{base}/home")],
            device_dir: Some(("{base}/home/.config/elohim/device", 0o700)),
            ..case("home config")
        },
        Case {
            env: &[("XDG_CONFIG_HOME", "relative"), ("HOME", "{base}/home")],
            device_dir: Some(("{base}/home/.config/elohim/device", 0o700)),
            ..case("relative xdg falls through to home")
        },
    ]
}

/// The tree every case stands in: two checkouts, a nested one, a symlink to the first.
fn materialize(base: &Path, case: &Case) {
    let at =
        |template: &str| PathBuf::from(template.replace("{base}", &base.display().to_string()));
    for dir in [
        "repo/.git",
        "repo/src/deep",
        "repo/vendor/sub",
        "other/.git",
    ] {
        std::fs::create_dir_all(base.join(dir)).unwrap();
    }
    std::fs::write(
        base.join("repo/vendor/sub/.git"),
        "gitdir: ../../.git/modules/sub\n",
    )
    .unwrap();
    std::os::unix::fs::symlink(base.join("repo"), base.join("link")).unwrap();
    if let Some((dir, mode)) = case.device_dir {
        let dir = at(dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("trusted-checkouts");
        match &case.trust_file {
            Some(TrustFile::Text(text, file_mode)) => {
                std::fs::write(&file, text.replace("{base}", &base.display().to_string())).unwrap();
                std::fs::set_permissions(&file, std::fs::Permissions::from_mode(*file_mode))
                    .unwrap();
            }
            Some(TrustFile::Directory) => std::fs::create_dir(&file).unwrap(),
            None => {}
        }
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(mode)).unwrap();
    }
}

fn run(case: &Case) -> Value {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().canonicalize().unwrap();
    materialize(&base, case);
    let b = base.display().to_string();
    let sub = |s: &str| s.replace("{base}", &b);
    let unsub = |s: String| s.replace(&b, "{base}");
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
        .map(|file| unsub(file.display().to_string()));
    let (root, git_args, config_env, unapplied) = match declaration_with(&cwd, &lookup) {
        Declaration::Trusted(root) => (
            Some(unsub(root.display().to_string())),
            args_for(&root),
            config_env_os(count.as_deref(), &root),
            None,
        ),
        Declaration::Undeclared => (None, Vec::new(), Vec::new(), None),
        Declaration::Unapplied(why) => (None, Vec::new(), Vec::new(), Some(why.class())),
    };
    let git_args: Vec<String> = git_args.into_iter().map(unsub).collect();
    let config_env: Vec<[String; 2]> = config_env.into_iter().map(|(k, v)| [k, unsub(v)]).collect();

    let mut input = serde_json::Map::new();
    input.insert(
        "env".into(),
        Value::Object(
            case.env
                .iter()
                .map(|(k, v)| ((*k).to_owned(), json!(v)))
                .collect(),
        ),
    );
    input.insert(
        "deviceDir".into(),
        match case.device_dir {
            Some((dir, mode)) => json!({ "path": dir, "mode": format!("{mode:o}") }),
            None => Value::Null,
        },
    );
    input.insert(
        "trustFile".into(),
        match &case.trust_file {
            Some(TrustFile::Text(text, mode)) => {
                json!({ "text": text, "mode": format!("{mode:o}") })
            }
            Some(TrustFile::Directory) => json!({ "directory": true }),
            None => Value::Null,
        },
    );
    input.insert("cwd".into(), json!(case.cwd));
    input.insert(
        "inheritedCount".into(),
        match case.count {
            Count::Unset => Value::Null,
            Count::Utf8(text) => json!({ "utf8": text }),
            Count::Bytes(bytes) => json!({ "bytes": bytes }),
        },
    );
    json!({
        "name": case.name,
        "input": Value::Object(input),
        "expect": {
            "trustFile": trust_file,
            "root": root,
            "gitArgs": git_args,
            "configEnv": config_env,
            "unapplied": unapplied,
        },
    })
}

fn minted() -> String {
    let document = json!({
        "format": "elohim-declared-trust-vectors@1",
        "contract": "elohim/eprfs/epr-cli/src/git.rs (module doc: the checkout-trust declaration)",
        "mintedBy": "elohim/eprfs/epr-cli/src/git/trust_vectors.rs; re-mint with \
                     EPR_MINT_TRUST_VECTORS=1, never edit by hand",
        "tree": "{base}/repo/.git/, {base}/repo/src/deep/, {base}/repo/vendor/sub/.git (a file), \
                 {base}/other/.git/, {base}/link -> {base}/repo; deviceDir created (parents \
                 default) with trustFile written inside, file then directory modes applied",
        "cases": cases().iter().map(run).collect::<Vec<_>>(),
    });
    let mut text = serde_json::to_string_pretty(&document).unwrap();
    text.push('\n');
    text
}

#[test]
fn the_vector_file_is_what_this_reader_answers() {
    let text = minted();
    if std::env::var_os("EPR_MINT_TRUST_VECTORS").is_some() {
        std::fs::write(FIXTURE, &text).unwrap();
    }
    let on_disk = std::fs::read_to_string(FIXTURE).unwrap_or_default();
    assert!(
        on_disk == text,
        "{FIXTURE} differs from what epr-cli's reader answers; re-mint it with \
         EPR_MINT_TRUST_VECTORS=1 and bring ark's reader along"
    );
}
