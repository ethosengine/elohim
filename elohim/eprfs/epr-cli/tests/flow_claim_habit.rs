//! Qualified habit references retain parent intent/actor ownership and pinned owner evidence.
#![cfg(unix)]
use elohim_epr_cli::flow;
use elohim_epr_rea::{FlowRecord, FlowStore, SidecarFlowStore};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

fn git(root: &Path, args: &[&str]) -> String {
    let mut command = elohim_epr_cli::process::build_command("git", args, root, &[]);
    let out = command
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.test")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.test")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn habit(id: &str) -> String {
    format!("---\nepr-habit-version: 1\nid: {id}\nstatus: red\nactive: false\ninvariant: Local source is retained\nretire-when: The predecessor retires\nchecks: [echo fixture]\n---\n")
}

struct Fixture {
    temporary: TempDir,
    root: PathBuf,
    owner: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temporary = TempDir::new().unwrap();
        let root = temporary.path().join("parent");
        let source = temporary.path().join("source");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&source).unwrap();
        git(&source, &["init", "-q"]);
        write(
            &source,
            ".epr-meta/repository.yaml",
            "version: 1\nagent: repo:fixture/tool\n",
        );
        write(&source, ".epr-meta/native.habit.md", &habit("native"));
        write(
            &source,
            "fixtures/.epr-meta/native.habit.md",
            &habit("native"),
        );
        git(&source, &["add", ".epr-meta", "fixtures"]);
        git(&source, &["commit", "-qm", "owner"]);
        git(&root, &["init", "-q"]);
        write(
            &root,
            ".epr-meta/repository.yaml",
            "version: 1\nagent: repo:fixture/parent\n",
        );
        write(&root, ".epr-meta/local.habit.md", &habit("local"));
        write(
            &root,
            "plans/epic.md",
            "---\nid: fixture-epic\n---\nOriginal parent intent.\n",
        );
        write(
            &root,
            "gap-items/epic.json",
            r#"{"doc":"plans/epic.md","items":[{"id":"epic#1","state":"OPEN"}]}"#,
        );
        write(&root, ".claude/epr-meta/recipes.yaml", "version: 1\nrecipes:\n  - id: fixture\n    version: 1\n    description: Parent intent\n    stages:\n      - {name: plan, artifactKind: 'doc:plan', paths: ['plans/*.md']}\n      - {name: intent, artifactKind: 'gap:item', paths: ['gap-items/*.json']}\n    edges: []\n");
        git(
            &root,
            &[
                "-c",
                "protocol.file.allow=always",
                "submodule",
                "add",
                "-q",
                source.to_str().unwrap(),
                "modules/tool",
            ],
        );
        git(
            &root,
            &[
                "add",
                ".epr-meta",
                ".claude",
                "plans",
                "gap-items",
                ".gitmodules",
                "modules/tool",
            ],
        );
        git(&root, &["commit", "-qm", "parent"]);
        let owner = root.join("modules/tool");
        let fixture = Self {
            temporary,
            root,
            owner,
        };
        fixture.ok(&["flow", "project", "--json"]);
        fixture.ok(&[
            "actor",
            "claim",
            "--as",
            "agent:implementer@gpt-6",
            "--session",
            "habit-fixture",
            "--json",
        ]);
        fixture
    }
    fn epr(&self, args: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_epr"));
        for (name, _) in std::env::vars().filter(|(name, _)| {
            name.starts_with("ELOHIM_") || name.starts_with("EPR_") || name.starts_with("GIT_")
        }) {
            command.env_remove(name);
        }
        command
            .args(args)
            .current_dir(&self.root)
            .env("EPR_UNDER_TEST", "1")
            .env("XDG_CONFIG_HOME", self.temporary.path().join("config"))
            .env(
                "ELOHIM_DEVICE_KEY_FILE",
                self.temporary.path().join("absent-device"),
            )
            .output()
            .unwrap()
    }
    fn ok(&self, args: &[&str]) -> Value {
        let out = self.epr(args);
        assert!(
            out.status.success(),
            "{args:?}: {} {}",
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out.stdout)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    }
    fn claim(&self, selector: &str) -> Output {
        self.epr(&[
            "flow",
            "claim",
            "--on",
            "epic#1",
            "--serves",
            selector,
            "--session",
            "habit-fixture",
            "--json",
        ])
    }
    fn refused(&self, selector: &str) {
        let path = self.root.join(".eprfs/status/flows.jsonl");
        let before = std::fs::read(&path).unwrap();
        let out = self.claim(selector);
        assert!(
            !out.status.success(),
            "{selector}: {}",
            String::from_utf8_lossy(&out.stdout)
        );
        assert_eq!(
            std::fs::read(path).unwrap(),
            before,
            "refusal appended a record"
        );
    }
}

#[test]
fn qualified_claim_preserves_parent_intent_actor_and_replay() {
    let fixture = Fixture::new();
    let actor = fixture.ok(&[
        "actor",
        "claim",
        "--as",
        "agent:implementer@gpt-6",
        "--session",
        "habit-fixture",
        "--json",
    ]);
    fixture.refused("native");
    let records = SidecarFlowStore::open(&fixture.root)
        .unwrap()
        .records()
        .unwrap();
    let (intent_cid, original_scope) = records
        .iter()
        .find_map(|(cid, record)| match record {
            FlowRecord::Intent(intent)
                if intent
                    .resource_spec
                    .classified_as
                    .iter()
                    .any(|c| c == "epic#1") =>
            {
                Some((*cid, intent.in_scope_of))
            }
            _ => None,
        })
        .unwrap();
    let out = fixture.claim("modules/tool#native");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let result: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(result["habit"], "modules/tool#native");
    assert_eq!(result["intent"], intent_cid.to_string());
    let records = SidecarFlowStore::open(&fixture.root)
        .unwrap()
        .records()
        .unwrap();
    let commitment = records
        .iter()
        .find_map(|(cid, record)| {
            if cid.to_string() != result["commitment_cid"].as_str().unwrap() {
                return None;
            }
            match record {
                FlowRecord::Commitment(c) => Some(c),
                _ => None,
            }
        })
        .unwrap();
    assert_eq!(commitment.in_scope_of, original_scope);
    assert_eq!(commitment.satisfies, vec![intent_cid]);
    assert_eq!(commitment.provider.0, "agent:implementer@gpt-6");
    assert_eq!(commitment.receiver.0, "repo:fixture/parent");
    let slots = &commitment.resource_spec.classified_as;
    assert!(slots.contains(&format!(
        "actor-claim:{}",
        actor["recordCid"].as_str().unwrap()
    )));
    assert!(slots.contains(&"habit:modules/tool#native".to_string()));
    assert!(slots.contains(&"habit-owner:repo:fixture/tool".to_string()));
    assert!(slots.contains(&format!(
        "habit-owner-commit:{}",
        git(&fixture.owner, &["rev-parse", "HEAD"])
    )));
    let raw = std::fs::read(fixture.owner.join(".epr-meta/native.habit.md")).unwrap();
    assert!(slots.contains(&format!(
        "habit-declaration:{}",
        eprfs_core::BlobCid::compute_raw(&raw)
    )));
    assert!(!fixture.owner.join(".eprfs/status/flows.jsonl").exists());
    assert!(!flow::registers::read_habits(&fixture.root)
        .unwrap()
        .iter()
        .any(|h| h.id == "native"));
    let before = std::fs::read(fixture.root.join(".eprfs/status/flows.jsonl")).unwrap();
    let again: Value =
        serde_json::from_slice(&fixture.claim("modules/tool#native").stdout).unwrap();
    assert_eq!(again["appended"], false);
    assert_eq!(
        before,
        std::fs::read(fixture.root.join(".eprfs/status/flows.jsonl")).unwrap()
    );
}

#[test]
fn unsafe_unknown_ambiguous_and_stale_owners_refuse_without_append() {
    let fixture = Fixture::new();
    let untracked = fixture.root.join("untracked");
    std::fs::create_dir(&untracked).unwrap();
    git(&untracked, &["init", "-q"]);
    write(&untracked, ".epr-meta/native.habit.md", &habit("native"));
    git(&untracked, &["add", ".epr-meta"]);
    git(&untracked, &["commit", "-qm", "untracked owner"]);
    for selector in [
        "../source#native",
        "/source#native",
        "modules/./tool#native",
        "modules//tool#native",
        "modules/*#native",
        "modules/tool#*",
        "modules/tool#native#extra",
        "modules/tool#missing",
        "modules#native",
        "absent#native",
        "untracked#native",
    ] {
        fixture.refused(selector);
    }
    let atom = fixture.owner.join(".epr-meta/native.habit.md");
    let owner_declaration = fixture.owner.join(".epr-meta/repository.yaml");
    let owner_bytes = std::fs::read(&owner_declaration).unwrap();
    std::fs::write(
        &owner_declaration,
        "version: 1\nagent: repo:fixture/impostor\n",
    )
    .unwrap();
    fixture.refused("modules/tool#native");
    std::fs::write(&owner_declaration, owner_bytes).unwrap();
    let original = std::fs::read(&atom).unwrap();
    std::fs::write(&atom, habit("native") + "edited").unwrap();
    fixture.refused("modules/tool#native");
    git(&fixture.owner, &["add", ".epr-meta/native.habit.md"]);
    std::fs::write(&atom, &original).unwrap();
    fixture.refused("modules/tool#native");
    git(
        &fixture.owner,
        &["reset", "-q", "HEAD", "--", ".epr-meta/native.habit.md"],
    );
    write(
        &fixture.owner,
        "nested/.epr-meta/native.habit.md",
        &habit("native"),
    );
    git(&fixture.owner, &["add", "nested"]);
    fixture.refused("modules/tool#native");
    git(&fixture.owner, &["commit", "-qm", "duplicate"]);
    fixture.refused("modules/tool#native");
    git(&fixture.root, &["add", "modules/tool"]);
    fixture.refused("modules/tool#native");
    git(&fixture.root, &["commit", "-qm", "pin duplicate"]);
    fixture.refused("modules/tool#native");
}

#[test]
fn oversized_pinned_declaration_refuses_without_append() {
    let fixture = Fixture::new();
    write(
        &fixture.owner,
        ".epr-meta/native.habit.md",
        &(habit("native") + &"x".repeat(65537)),
    );
    git(&fixture.owner, &["add", ".epr-meta/native.habit.md"]);
    git(&fixture.owner, &["commit", "-qm", "oversized declaration"]);
    git(&fixture.root, &["add", "modules/tool"]);
    git(&fixture.root, &["commit", "-qm", "pin oversized owner"]);
    fixture.refused("modules/tool#native");
}

#[cfg(unix)]
#[test]
fn symlinked_owner_and_declaration_ancestors_never_escape() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let meta = fixture.owner.join(".epr-meta");
    let moved = fixture.owner.join("retained-meta");
    std::fs::rename(&meta, &moved).unwrap();
    symlink(&moved, &meta).unwrap();
    fixture.refused("modules/tool#native");
    std::fs::remove_file(&meta).unwrap();
    std::fs::rename(&moved, &meta).unwrap();
    let modules = fixture.root.join("modules");
    let moved = fixture.root.join("retained-modules");
    std::fs::rename(&modules, &moved).unwrap();
    symlink(&moved, &modules).unwrap();
    fixture.refused("modules/tool#native");
}
