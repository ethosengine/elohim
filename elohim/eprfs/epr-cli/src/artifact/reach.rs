//! Put-time reach (K12, design §2): the narrowest reach among everything that entered a record's
//! derivation, written once as `env:reach=<level>` on its custody Produce.
//!
//! The classifier reads only what was hashed into the derivation key: the input document's exact
//! bytes (or the verified sysroot manifest) and the method descriptor. No flag, environment
//! variable or descriptor value names a level (T8). Every input with no class is `private` (T10),
//! an empty input set is `private` and levels are compared on [`Reach::openness`], never on
//! declaration order (T9). Nothing above `self` is ever derived (ruling Q1, 2026-11-15), and an
//! open `build-inputs.*` gap caps the record at `self` without making it `private` (Q4).

use std::collections::BTreeSet;

use elohim_epr::Reach;
use elohim_epr_rea::{FlowEvent, ReaVerb};
use eprfs_core::{BlobCid, MethodDescriptor};
use eprfs_peer::{event_reach, DeclaredReach, REACH_SLOT};

use super::{sysroot, MESH_BUILD};

/// The method name of a sysroot floor (`epr artifact sysroot put`).
pub(super) const SYSROOT_PACK: &str = "sysroot-pack";

/// The widest level this writer derives. The gate refuses every level above `self` until the
/// ladder is built (ruling Q1), so the writer never produces one.
pub(super) const CEILING: Reach = Reach::SelfScope;

/// The gap-name prefix of inputs whose kind is known and whose members are not listed (§2.4).
const BUILD_INPUTS_GAP: &str = "build-inputs.";

/// The non-path inputs of a floor, exactly (`sysroot_cli::put`).
const FLOOR_INPUTS: [&str; 3] = ["sysroot.manifest", "sysroot.recipe", "sysroot.triple"];

/// Declared-target inputs that describe the target and read nothing (`validate_target_inputs`).
const TARGET_DESCRIPTIONS: [&str; 5] = ["arch", "isa", "os", "libc", "cflags"];

/// What was hashed into the key's input half.
#[derive(Debug, Clone, Copy)]
pub(super) enum Hashed<'a> {
    /// A mesh build's input document: the exact bytes whose CID is the key's input.
    Document(&'a str),
    /// A floor's manifest, as `sysroot::verify` reproduced it from the payload.
    Floor(&'a sysroot::Manifest),
}

/// The class of one input, and through it the input's reach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Class {
    /// Committed source of the repository at one of the descriptor's view paths.
    Source,
    /// Cargo configuration on the path from the crate to the filesystem root.
    CargoConfig,
    /// This account's Cargo configuration.
    CargoHome,
    /// Host tool identity and defaults: `tool:` lines, the descriptor's tools and flags.
    HostTool,
    /// A host link input (`link:` lines), not attributed to a package.
    LinkInput,
    /// The declared build environment: non-path inputs other than `target.*`.
    BuildEnvironment,
    /// A member of a floor whose manifest was verified against its payload.
    FloorMember,
    /// A recorded artifact named by CID (`target.sysroot`, §2.5), at the narrowest claim this
    /// device holds for it.
    Recorded { cid: String, reach: Reach },
    /// Something that entered the key or the method with no class here.
    Unclassified(String),
}

impl Class {
    /// The reach of an input of this class.
    pub(super) fn reach(&self) -> Reach {
        match self {
            Class::Source
            | Class::CargoConfig
            | Class::CargoHome
            | Class::HostTool
            | Class::LinkInput
            | Class::BuildEnvironment
            | Class::FloorMember => Reach::SelfScope,
            Class::Recorded { reach, .. } => *reach,
            Class::Unclassified(_) => Reach::Private,
        }
    }
}

/// The level with the smallest openness; an empty set is `private`, never the widest level.
pub(super) fn narrowest(levels: impl IntoIterator<Item = Reach>) -> Reach {
    levels
        .into_iter()
        .reduce(|a, b| if b.openness() < a.openness() { b } else { a })
        .unwrap_or(Reach::Private)
}

/// The cap an open gap places on a record: `self` while any `build-inputs.*` gap is open. It never
/// makes a record `private` (ruling Q4).
pub(super) fn gap_cap(gaps: &[String]) -> Option<Reach> {
    gaps.iter()
        .any(|gap| gap.starts_with(BUILD_INPUTS_GAP))
        .then_some(Reach::SelfScope)
}

/// The serde name of a level, as `elohim_epr::Reach` spells it and the gate reads it.
pub(super) fn level_name(reach: Reach) -> &'static str {
    match reach {
        Reach::Private => "private",
        Reach::SelfScope => "self",
        Reach::Intimate => "intimate",
        Reach::Trusted => "trusted",
        Reach::Familiar => "familiar",
        Reach::Community => "community",
        Reach::Public => "public",
        Reach::Commons => "commons",
    }
}

/// One record's derived reach, with what made it `private`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Derived {
    pub reach: Reach,
    /// Each input that narrowed the record to `private`, for the put-time printout.
    pub narrowed_by: Vec<String>,
}

impl Derived {
    /// The one `env:reach=` slot of the record's custody Produce.
    pub(super) fn slot(&self) -> String {
        format!("{REACH_SLOT}{}", level_name(self.reach))
    }
}

/// Fold classes into a record's reach: the narrowest class, an empty input set `private`, then the
/// gap cap and the writer's ceiling.
pub(super) fn fold(classes: &[Class], listed: usize, gaps: &[String]) -> Derived {
    let mut narrowed_by = Vec::new();
    for class in classes {
        match class {
            Class::Unclassified(what) => narrowed_by.push(what.clone()),
            Class::Recorded { cid, reach } if *reach == Reach::Private => narrowed_by.push(
                format!("the named artifact {cid} has no reach this device holds above private"),
            ),
            _ => {}
        }
    }
    let mut reach = if listed == 0 {
        narrowed_by.push("the key's input lists nothing (an empty input set)".into());
        Reach::Private
    } else {
        narrowest(classes.iter().map(Class::reach))
    };
    if let Some(cap) = gap_cap(gaps) {
        reach = narrowest([reach, cap]);
    }
    Derived {
        reach: narrowest([reach, CEILING]),
        narrowed_by,
    }
}

/// The narrowest reach of a record from what was hashed into its key and its method descriptor.
/// `project` is the step's project; `key_input` the key's input CID; `held` this device's flow
/// events, read only for a recorded artifact the method names (§2.5).
pub(super) fn derive_reach(
    hashed: Hashed<'_>,
    key_input: &BlobCid,
    descriptor: &MethodDescriptor,
    project: &str,
    gaps: &[String],
    held: &[&FlowEvent],
) -> Derived {
    let mut classes = Vec::new();
    let listed = match hashed {
        Hashed::Document(document) => document_classes(document, descriptor, &mut classes),
        Hashed::Floor(manifest) => floor_classes(manifest, key_input, &mut classes),
    };
    method_classes(descriptor, project, hashed, held, &mut classes);
    fold(&classes, listed, gaps)
}

/// Classify the input document line by line; the number of lines it lists.
fn document_classes(
    document: &str,
    descriptor: &MethodDescriptor,
    classes: &mut Vec<Class>,
) -> usize {
    let view: BTreeSet<String> = descriptor
        .view()
        .paths()
        .filter_map(|path| path.to_slash_string().ok())
        .collect();
    let mut listed = 0;
    for line in document.split_terminator('\n') {
        listed += 1;
        classes.push(line_class(line, &view));
    }
    listed
}

fn line_class(line: &str, view: &BTreeSet<String>) -> Class {
    let prefixed = [
        (
            "cargo-config:",
            cargo_config_line as fn(&str) -> bool,
            Class::CargoConfig,
        ),
        ("cargo-home:", cargo_home_line, Class::CargoHome),
        ("tool:", host_line, Class::HostTool),
        ("link:", host_line, Class::LinkInput),
    ];
    for (prefix, grammar, class) in prefixed {
        if line.strip_prefix(prefix).is_some_and(grammar) {
            return class;
        }
    }
    if view
        .iter()
        .any(|path| view_entry(line, path).is_some_and(|entry| entry_grammar(entry, path)))
    {
        return Class::Source;
    }
    Class::Unclassified(format!("input line `{}`", shown(line)))
}

/// `cargo-config:<n>:<name>\t<cid>`.
fn cargo_config_line(rest: &str) -> bool {
    let Some((head, cid)) = rest.split_once('\t') else {
        return false;
    };
    let Some((index, name)) = head.split_once(':') else {
        return false;
    };
    !index.is_empty()
        && index.bytes().all(|b| b.is_ascii_digit())
        && cargo_config_name(name)
        && BlobCid::parse(cid).is_ok()
}

/// `cargo-home:<name>\t<cid or ->`.
fn cargo_home_line(rest: &str) -> bool {
    rest.split_once('\t').is_some_and(|(name, cid)| {
        cargo_config_name(name) && (cid == "-" || BlobCid::parse(cid).is_ok())
    })
}

fn cargo_config_name(name: &str) -> bool {
    matches!(name, "config" | "config.toml")
}

/// `<label>\t-` or `<label>\t<absolute path>\t<cid>` (host tool and link lines).
fn host_line(rest: &str) -> bool {
    let Some((label, value)) = rest.split_once('\t') else {
        return false;
    };
    !label.is_empty()
        && (value == "-"
            || value
                .split_once('\t')
                .is_some_and(|(path, cid)| path.starts_with('/') && BlobCid::parse(cid).is_ok()))
}

/// The entry of a view-shaped line for one of the descriptor's view paths.
fn view_entry<'l>(line: &'l str, path: &str) -> Option<&'l str> {
    line.strip_prefix(path)?.strip_prefix('\t')
}

/// `<mode> <blob|tree> <oid>`, `gitlink:<prefix>@<sha>` for a prefix at or above `path`, or `-`.
fn entry_grammar(entry: &str, path: &str) -> bool {
    if entry == "-" {
        return true;
    }
    if let Some(gitlink) = entry.strip_prefix("gitlink:") {
        return gitlink.rsplit_once('@').is_some_and(|(prefix, sha)| {
            (path == prefix || path.starts_with(&format!("{prefix}/"))) && object_id(sha)
        });
    }
    let mut parts = entry.split(' ');
    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(mode), Some("blob" | "tree"), Some(oid), None) => {
            mode.len() == 6 && mode.bytes().all(|b| (b'0'..=b'7').contains(&b)) && object_id(oid)
        }
        _ => false,
    }
}

/// A git object id: SHA-1 or SHA-256 hex.
fn object_id(id: &str) -> bool {
    matches!(id.len(), 40 | 64) && id.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Classify the members of a verified floor; the number of members.
fn floor_classes(
    manifest: &sysroot::Manifest,
    key_input: &BlobCid,
    classes: &mut Vec<Class>,
) -> usize {
    match manifest.cid() {
        Ok(cid) if &cid == key_input => {}
        _ => classes.push(Class::Unclassified(
            "the key's input is not the verified floor manifest".into(),
        )),
    }
    // `scripts`, `isa_notes` and `libc_version` are observations derived from the members; the
    // version and triple are the manifest's own framing. None of them narrows.
    classes.extend(manifest.files.iter().map(|_| Class::FloorMember));
    manifest.files.len()
}

/// Classify the method descriptor. Destructured whole, so a new field cannot pass unclassified.
fn method_classes(
    descriptor: &MethodDescriptor,
    project: &str,
    hashed: Hashed<'_>,
    held: &[&FlowEvent],
    classes: &mut Vec<Class>,
) {
    let MethodDescriptor {
        name,
        tools,
        flags,
        params,
        exec,
        step,
        embedder,
        view,
        non_path_inputs,
        // Bound to the method CID; a claim about the method, not an input.
        deterministic: _,
    } = descriptor;
    let step_is = |kind: &str| step.as_deref() == Some(format!("{kind}:{project}").as_str());
    match (name.as_str(), hashed) {
        (MESH_BUILD, Hashed::Document(_)) => {
            if !step_is(MESH_BUILD) {
                classes.push(Class::Unclassified(format!(
                    "the mesh-build step {step:?} is not {MESH_BUILD}:{project}"
                )));
            }
            if !tools.is_empty() || !flags.is_empty() {
                classes.push(Class::HostTool);
            }
            // The view is covered by the document's view lines.
            for (input, value) in non_path_inputs {
                match input.strip_prefix("target.") {
                    None => classes.push(Class::BuildEnvironment),
                    Some(target) if TARGET_DESCRIPTIONS.contains(&target) => {}
                    Some("sysroot") => classes.push(recorded(value, held)),
                    Some(_) => classes.push(Class::Unclassified(format!(
                        "the declared target input `{input}`"
                    ))),
                }
            }
        }
        (SYSROOT_PACK, Hashed::Floor(manifest)) => {
            if !step_is(SYSROOT_PACK) {
                classes.push(Class::Unclassified(format!(
                    "the sysroot-pack step {step:?} is not {SYSROOT_PACK}:{project}"
                )));
            }
            for (field, empty) in [
                ("tools", tools.is_empty()),
                ("flags", flags.is_empty()),
                ("view", view.is_empty()),
            ] {
                if !empty {
                    classes.push(Class::Unclassified(format!("a floor method's {field}")));
                }
            }
            if !non_path_inputs.keys().map(String::as_str).eq(FLOOR_INPUTS) {
                classes.push(Class::Unclassified(format!(
                    "a floor method's non-path inputs {:?} are not exactly {FLOOR_INPUTS:?}",
                    non_path_inputs.keys().collect::<Vec<_>>()
                )));
            }
            let names_manifest = manifest
                .cid()
                .is_ok_and(|cid| non_path_inputs.get(FLOOR_INPUTS[0]) == Some(&cid.to_string()));
            if !names_manifest || non_path_inputs.get(FLOOR_INPUTS[2]) != Some(&manifest.triple) {
                classes.push(Class::Unclassified(
                    "a floor method that names another manifest or triple than the verified one"
                        .into(),
                ));
            }
        }
        (other, _) => classes.push(Class::Unclassified(format!(
            "the recipe kind `{other}` has no classifier for this input"
        ))),
    }
    for (field, set) in [
        ("params", params.is_some()),
        ("exec", exec.is_some()),
        ("embedder", embedder.is_some()),
    ] {
        if set {
            classes.push(Class::Unclassified(format!("the method's {field}")));
        }
    }
}

/// A recorded artifact named by CID: the narrowest `env:reach` among the custody records this
/// device holds for that output (its own and those folded from a sibling). As in the gate's fold
/// (design §3.3 rule 2), once any of those records carries a slot the slotless ones are ignored;
/// no record, or only slotless ones, is `private`, and an unreadable slot counts as `private`. A
/// sibling's claim enters only this minimum: it can narrow a record, never widen one, and it never
/// decides what this device serves.
fn recorded(value: &str, held: &[&FlowEvent]) -> Class {
    let Ok(cid) = BlobCid::parse(value) else {
        return Class::Unclassified(format!("the named artifact `{}`", shown(value)));
    };
    let cid = cid.to_string();
    // Slotless records contribute nothing: with no slotted record the minimum is over nothing,
    // which is `private`.
    let levels: Vec<Reach> = held
        .iter()
        .filter(|event| {
            event.action == ReaVerb::Produce
                && event.classified_as.first().map(String::as_str) == Some("custody-blob")
                && event.classified_as.get(1) == Some(&cid)
        })
        .filter_map(|event| event_reach(event))
        .map(|declared| match declared {
            DeclaredReach::OwnDevices => Reach::SelfScope,
            DeclaredReach::Wider(openness) => at_openness(openness),
            DeclaredReach::Private | DeclaredReach::Unreadable => Reach::Private,
        })
        .collect();
    Class::Recorded {
        cid,
        reach: narrowest(levels),
    }
}

/// The level with this openness; `private` for a number outside the scale.
fn at_openness(openness: u8) -> Reach {
    const ALL: [Reach; 8] = [
        Reach::Private,
        Reach::SelfScope,
        Reach::Intimate,
        Reach::Trusted,
        Reach::Familiar,
        Reach::Community,
        Reach::Public,
        Reach::Commons,
    ];
    ALL.into_iter()
        .find(|reach| reach.openness() == openness)
        .unwrap_or(Reach::Private)
}

/// At most 120 characters of an input, for a printed reason.
fn shown(text: &str) -> String {
    let mut shown: String = text.chars().take(120).collect();
    if shown.len() < text.len() {
        shown.push('…');
    }
    shown.escape_debug().to_string()
}

#[cfg(test)]
#[path = "reach_tests.rs"]
mod tests;
