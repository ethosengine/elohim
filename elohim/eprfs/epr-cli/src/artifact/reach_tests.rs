//! The put-time reach classifier (K12 packet 3, design §2, threats T8–T10).
use super::*;
use eprfs_core::TreePath;

const OID: &str = "0123456789abcdef0123456789abcdef01234567";
const PROJECT: &str = "elohim-storage";

fn cid(seed: &str) -> String {
    BlobCid::compute_bytes(seed.as_bytes()).to_string()
}

fn mesh(view: &[&str]) -> MethodDescriptor {
    MethodDescriptor {
        name: MESH_BUILD.into(),
        tools: [("rustc".to_string(), "rustc 1.100.0".to_string())].into(),
        flags: vec!["--bin=elohim-storage".into()],
        params: None,
        exec: None,
        step: Some(format!("{MESH_BUILD}:{PROJECT}")),
        embedder: None,
        view: view
            .iter()
            .map(|p| TreePath::from_slash_bytes(p.as_bytes()).unwrap())
            .collect(),
        non_path_inputs: [
            ("RUSTFLAGS".to_string(), String::new()),
            ("PATH".to_string(), "/usr/bin:/bin".to_string()),
            ("target.isa".to_string(), "x86-64-v3".to_string()),
        ]
        .into(),
        deterministic: false,
    }
}

const VIEW: [&str; 4] = ["crate/src", "README.md", "vendored/lib", "absent"];

/// Every line grammar the mesh-build writer emits (`closure_selection`, `cargo_config_lines`,
/// `host::tool_lines`).
fn document() -> String {
    [
        format!("crate/src\t040000 tree {OID}"),
        format!("README.md\t100644 blob {OID}"),
        format!("vendored/lib\tgitlink:vendored@{OID}"),
        "absent\t-".to_string(),
        format!("cargo-config:0:config.toml\t{}", cid("a")),
        "cargo-home:config\t-".to_string(),
        format!("cargo-home:config.toml\t{}", cid("b")),
        format!("tool:link-driver\t/usr/bin/cc\t{}", cid("c")),
        format!("tool:link-driver:target\t/usr/bin/cc\t{}", cid("d")),
        format!("tool:CC\t/usr/bin/gcc\t{}", cid("e")),
        format!("link:crt1.o\t/usr/lib/crt1.o\t{}", cid("f")),
        format!("link:libc.so>/lib/libc.so.6\t/lib/libc.so.6\t{}", cid("g")),
        "link:libfoo\t-".to_string(),
        "tool:link-driver\t-".to_string(),
    ]
    .iter()
    .map(|line| format!("{line}\n"))
    .collect()
}

fn mesh_reach(document: &str, descriptor: &MethodDescriptor, gaps: &[String]) -> Derived {
    let input = BlobCid::compute_bytes(document.as_bytes());
    derive_reach(
        Hashed::Document(document),
        &input,
        descriptor,
        PROJECT,
        gaps,
        &[],
    )
}

fn floor(members: bool) -> (sysroot::Manifest, MethodDescriptor) {
    let root = tempfile::tempdir().unwrap();
    if members {
        std::fs::write(root.path().join("header.h"), b"/* pinned */").unwrap();
    }
    let (_, manifest) = sysroot::pack(root.path(), "x86_64-unknown-linux-gnu").unwrap();
    let descriptor = MethodDescriptor {
        name: SYSROOT_PACK.into(),
        tools: Default::default(),
        flags: Vec::new(),
        params: None,
        exec: None,
        step: Some(format!("{SYSROOT_PACK}:sysroot")),
        embedder: None,
        view: Vec::new(),
        non_path_inputs: [
            (
                "sysroot.manifest".to_string(),
                manifest.cid().unwrap().to_string(),
            ),
            ("sysroot.recipe".to_string(), cid("recipe")),
            ("sysroot.triple".to_string(), manifest.triple.clone()),
        ]
        .into(),
        deterministic: true,
    };
    (manifest, descriptor)
}

fn floor_reach(manifest: &sysroot::Manifest, descriptor: &MethodDescriptor) -> Derived {
    derive_reach(
        Hashed::Floor(manifest),
        &manifest.cid().unwrap(),
        descriptor,
        "sysroot",
        &[],
        &[],
    )
}

fn private_because(derived: &Derived, reason: &str) {
    assert_eq!(derived.reach, Reach::Private, "{derived:?}");
    assert!(
        derived.narrowed_by.iter().any(|why| why.contains(reason)),
        "{reason} in {derived:?}"
    );
}

#[test]
fn every_writer_grammar_classifies_as_self() {
    let derived = mesh_reach(&document(), &mesh(&VIEW), &[]);
    assert_eq!(derived.reach, Reach::SelfScope, "{derived:?}");
    assert!(derived.narrowed_by.is_empty());
    assert_eq!(derived.slot(), "env:reach=self");
    let (manifest, descriptor) = floor(true);
    assert_eq!(floor_reach(&manifest, &descriptor).reach, Reach::SelfScope);
}

#[test]
fn the_level_names_are_elohim_eprs_serde_names() {
    for openness in 1..=8 {
        let reach = at_openness(openness);
        assert_eq!(reach.openness(), openness);
        assert_eq!(
            serde_json::to_value(reach).unwrap(),
            serde_json::Value::from(level_name(reach))
        );
        assert!(eprfs_peer::REACH_LEVELS.contains(&(level_name(reach), openness)));
    }
}

#[test]
fn an_unknown_input_line_makes_the_record_private() {
    let descriptor = mesh(&VIEW);
    for (line, why) in [
        (format!("secret:key\t{}", cid("s")), "secret:key"),
        // An empty line is no grammar.
        (String::new(), "input line ``"),
        // A known prefix whose shape is wrong falls through to no class.
        (format!("tool:CC\tgcc\t{}", cid("e")), "tool:CC"),
        (
            "link:crt1.o\t/usr/lib/crt1.o\tnot-a-cid".into(),
            "link:crt1.o",
        ),
        (
            format!("cargo-config:x:config\t{}", cid("a")),
            "cargo-config:x",
        ),
        (
            format!("cargo-home:credentials\t{}", cid("a")),
            "cargo-home:credentials",
        ),
        (format!("README.md\t100644 commit {OID}"), "README.md"),
        (format!("README.md\t10064 blob {OID}"), "README.md"),
        ("README.md\t100644 blob short".into(), "README.md"),
        (format!("vendored/lib\tgitlink:other@{OID}"), "vendored/lib"),
    ] {
        let mut document = document();
        document.push_str(&format!("{line}\n"));
        private_because(&mesh_reach(&document, &descriptor, &[]), why);
    }
}

#[test]
fn a_view_shaped_line_outside_the_view_is_private() {
    let line = format!("elsewhere/file.rs\t100644 blob {OID}\n");
    let mut document = document();
    document.push_str(&line);
    private_because(
        &mesh_reach(&document, &mesh(&VIEW), &[]),
        "elsewhere/file.rs",
    );
    // The same line is source once the descriptor's own view names the path.
    let mut view = VIEW.to_vec();
    view.push("elsewhere/file.rs");
    assert_eq!(
        mesh_reach(&document, &mesh(&view), &[]).reach,
        Reach::SelfScope
    );
    // `cargo-home:config\t-` and an absent view path share a shape; one outside the view is not
    // source.
    private_because(
        &mesh_reach("not-in-view\t-\n", &mesh(&VIEW), &[]),
        "not-in-view",
    );
}

#[test]
fn an_unknown_recipe_kind_is_private() {
    let document = document();
    let mut unknown = mesh(&VIEW);
    unknown.name = "cargo-build".into();
    unknown.step = Some(format!("cargo-build:{PROJECT}"));
    private_because(&mesh_reach(&document, &unknown, &[]), "cargo-build");
    let mut wrong_step = mesh(&VIEW);
    wrong_step.step = Some("mesh-build:another-project".into());
    private_because(&mesh_reach(&document, &wrong_step, &[]), "mesh-build step");
    wrong_step.step = None;
    private_because(&mesh_reach(&document, &wrong_step, &[]), "mesh-build step");
    // A known kind is known only for its own input.
    let mut floor_named = mesh(&VIEW);
    floor_named.name = SYSROOT_PACK.into();
    private_because(&mesh_reach(&document, &floor_named, &[]), SYSROOT_PACK);
    let (manifest, mut descriptor) = floor(true);
    descriptor.name = MESH_BUILD.into();
    private_because(&floor_reach(&manifest, &descriptor), MESH_BUILD);
}

#[test]
fn a_descriptor_field_with_no_class_is_private() {
    let document = document();
    let blob = BlobCid::compute_bytes(b"params");
    type Edit = fn(&mut MethodDescriptor, &BlobCid);
    let edits: [(Edit, &str); 4] = [
        (|d, c| d.params = Some(c.clone()), "params"),
        (|d, c| d.exec = Some(c.clone()), "exec"),
        (|d, _| d.embedder = Some("fixture".into()), "embedder"),
        (
            |d, _| {
                d.non_path_inputs
                    .insert("target.unknown".into(), "x".into());
            },
            "target.unknown",
        ),
    ];
    for (edit, why) in edits {
        let mut descriptor = mesh(&VIEW);
        edit(&mut descriptor, &blob);
        private_because(&mesh_reach(&document, &descriptor, &[]), why);
    }
    let floor_edits: [(Edit, &str); 6] = [
        (
            |d, _| {
                d.non_path_inputs.insert("PATH".into(), "/bin".into());
            },
            "non-path inputs",
        ),
        (
            |d, _| {
                d.tools.insert("cc".into(), "1".into());
            },
            "tools",
        ),
        (|d, _| d.flags.push("-O2".into()), "flags"),
        (
            |d, _| d.view.push(TreePath::from_slash_bytes(b"src").unwrap()),
            "view",
        ),
        (|d, c| d.exec = Some(c.clone()), "exec"),
        (
            |d, c| {
                d.non_path_inputs
                    .insert("sysroot.manifest".into(), c.to_string());
            },
            "another manifest",
        ),
    ];
    for (edit, why) in floor_edits {
        let (manifest, mut descriptor) = floor(true);
        edit(&mut descriptor, &blob);
        private_because(&floor_reach(&manifest, &descriptor), why);
    }
    // A floor recorded under a key whose input is not its manifest is not classified either.
    let (manifest, descriptor) = floor(true);
    private_because(
        &derive_reach(
            Hashed::Floor(&manifest),
            &blob,
            &descriptor,
            "sysroot",
            &[],
            &[],
        ),
        "verified floor manifest",
    );
}

#[test]
fn no_inputs_is_private() {
    assert_eq!(narrowest([]), Reach::Private);
    private_because(&mesh_reach("", &mesh(&VIEW), &[]), "empty input set");
    let (manifest, descriptor) = floor(false);
    assert!(manifest.files.is_empty());
    private_because(&floor_reach(&manifest, &descriptor), "empty input set");
    // No gap widens an empty set.
    let gaps = vec![cargo_project_gap()];
    private_because(&mesh_reach("", &mesh(&VIEW), &gaps), "empty input set");
}

fn cargo_project_gap() -> String {
    super::super::cargo_project::COMPILER_GAP.to_string()
}

fn every_class() -> Vec<Class> {
    let mut classes = vec![
        Class::Source,
        Class::CargoConfig,
        Class::CargoHome,
        Class::HostTool,
        Class::LinkInput,
        Class::BuildEnvironment,
        Class::FloorMember,
        Class::Unclassified("x".into()),
    ];
    classes.extend((1..=8).map(|openness| Class::Recorded {
        cid: cid("floor"),
        reach: at_openness(openness),
    }));
    classes
}

#[test]
fn derive_reach_never_exceeds_self() {
    let classes = every_class();
    let gap_sets = [Vec::new(), vec![cargo_project_gap()]];
    for gaps in &gap_sets {
        for a in &classes {
            for listed in [0, 1, 2] {
                let one = fold(std::slice::from_ref(a), listed, gaps);
                assert!(one.reach.openness() <= CEILING.openness(), "{a:?}: {one:?}");
            }
            for b in &classes {
                let two = fold(&[a.clone(), b.clone()], 2, gaps);
                assert!(two.reach.openness() <= CEILING.openness(), "{a:?} {b:?}");
                // The minimum is taken on openness, not declaration order.
                assert_eq!(
                    two.reach,
                    narrowest([a.reach(), b.reach(), CEILING]),
                    "{a:?} {b:?}"
                );
            }
        }
    }
    // A named floor a sibling declared at every level never derives above self.
    for openness in 1..=8 {
        let floor_cid = cid("named-floor");
        let event = custody_event(&floor_cid, Some(level_name(at_openness(openness))));
        let mut descriptor = mesh(&VIEW);
        descriptor
            .non_path_inputs
            .insert("target.sysroot".into(), floor_cid);
        let document = document();
        let derived = derive_reach(
            Hashed::Document(&document),
            &BlobCid::compute_bytes(document.as_bytes()),
            &descriptor,
            PROJECT,
            &[],
            &[&event],
        );
        let expected = if openness == 1 {
            Reach::Private
        } else {
            Reach::SelfScope
        };
        assert_eq!(derived.reach, expected, "openness {openness}");
    }
}

#[test]
fn an_open_build_input_gap_caps_reach_at_self() {
    for gap in [
        super::super::cargo_project::COMPILER_GAP,
        super::super::cargo_project::SCRIPTS_GAP,
        super::super::cargo_project::SOURCES_GAP,
    ] {
        let gaps = vec![gap.to_string()];
        assert_eq!(gap_cap(&gaps), Some(Reach::SelfScope), "{gap}");
        // An open gap never makes the record private (ruling Q4) ...
        let derived = mesh_reach(&document(), &mesh(&VIEW), &gaps);
        assert_eq!(derived.reach, Reach::SelfScope, "{gap}: {derived:?}");
        assert!(derived.narrowed_by.is_empty());
        // ... and nothing derives wider than self while it is open, even an input claimed wider.
        let wide = Class::Recorded {
            cid: cid("wide"),
            reach: Reach::Commons,
        };
        assert_eq!(fold(&[wide], 1, &gaps).reach, Reach::SelfScope, "{gap}");
    }
    // A gap that names no build input caps nothing.
    assert_eq!(
        gap_cap(&[super::super::preflight::PARTIAL_DECODE_GAP.into()]),
        None
    );
    assert_eq!(gap_cap(&[]), None);
}

fn custody_event(output: &str, reach: Option<&str>) -> FlowEvent {
    let mut classified_as = vec![
        "custody-blob".to_string(),
        output.to_string(),
        "copy:self".into(),
    ];
    if let Some(level) = reach {
        classified_as.push(format!("{REACH_SLOT}{level}"));
    }
    FlowEvent {
        action: ReaVerb::Produce,
        provider: elohim_epr_rea::AgentRef("agent".into()),
        receiver: elohim_epr_rea::AgentRef("did:key:zSelf".into()),
        resource: *BlobCid::parse(output).unwrap().as_cid(),
        quantity: elohim_epr_rea::Magnitude::Count {
            value: 1.0,
            unit: "bytes".into(),
        },
        process: None,
        in_scope_of: elohim_epr_rea::atom_cid(&"repo-scope".to_string()).unwrap(),
        fulfills: Vec::new(),
        satisfies: Vec::new(),
        occurred_at: "2026-11-15T00:00:00Z".into(),
        classified_as,
    }
}

#[test]
fn a_named_floor_takes_the_narrowest_claim_this_device_holds() {
    let floor_cid = cid("named-floor");
    let other = cid("another-output");
    let own_self = custody_event(&floor_cid, Some("self"));
    let sibling_private = custody_event(&floor_cid, Some("private"));
    let slotless = custody_event(&floor_cid, None);
    let unreadable = custody_event(&floor_cid, Some("SELF"));
    let unrelated = custody_event(&other, Some("self"));
    for (held, expected) in [
        (vec![], Reach::Private),
        (vec![&unrelated], Reach::Private),
        (vec![&own_self], Reach::SelfScope),
        (vec![&own_self, &unrelated], Reach::SelfScope),
        (vec![&own_self, &sibling_private], Reach::Private),
        (vec![&own_self, &slotless], Reach::Private),
        (vec![&own_self, &unreadable], Reach::Private),
    ] {
        assert_eq!(
            recorded(&floor_cid, &held),
            Class::Recorded {
                cid: floor_cid.clone(),
                reach: expected
            }
        );
    }
    assert!(matches!(
        recorded("not-a-cid", &[&own_self]),
        Class::Unclassified(_)
    ));
}
