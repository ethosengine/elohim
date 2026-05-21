//! Manifest Hygiene — schema contract test for the 5 elohim DNA manifests
//! and the elohim.happ workdir happ.yaml. Enforces wave-1 §7 conventions.
//!
//! Pure YAML parsing. No conductor. No DNA bundle artifacts. Runs in
//! milliseconds and gates on husky pre-push.
//!
//! Why a test and not a lint: the assertions are about *coherence across
//! files* (e.g., the network_seed in `dna/imagodei/dna.yaml` must match the
//! network_seed in the imagodei role of `workdir/happ.yaml`) — cheap to
//! express as a test, painful to express as a linter rule.

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// The 5 DNAs that get hygiene enforcement. The dna.yaml `name` field may
/// differ from the directory name for historical reasons (`elohim/` holds
/// the DNA named `lamad`; `node-registry/` holds `node_registry`).
const DNAS: &[(&str, &str)] = &[
    ("elohim", "lamad"),
    ("imagodei", "imagodei"),
    ("mishpat", "mishpat"),
    ("node-registry", "node_registry"),
    ("infrastructure", "infrastructure"),
];

/// DNAs that carry a bootstrap-steward modifier in happ.yaml. Infrastructure
/// is federation-native and does not (see wave-1 plan §1.2 Q2 outcome).
const BOOTSTRAP_STEWARD_DNAS: &[&str] = &["lamad", "imagodei", "mishpat", "node_registry"];

#[derive(Debug, Deserialize)]
struct DnaManifest {
    manifest_version: String,
    name: String,
    integrity: IntegritySection,
    #[serde(default)]
    #[allow(dead_code)]
    coordinator: Option<serde_yaml::Value>,
}

#[derive(Debug, Deserialize)]
struct IntegritySection {
    network_seed: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    properties: Option<serde_yaml::Value>,
    #[serde(default)]
    #[allow(dead_code)]
    zomes: Option<serde_yaml::Value>,
}

#[derive(Debug, Deserialize)]
struct HappManifest {
    manifest_version: String,
    #[allow(dead_code)]
    name: String,
    roles: Vec<HappRole>,
}

#[derive(Debug, Deserialize)]
struct HappRole {
    name: String,
    #[serde(default)]
    provisioning: Option<RoleProvisioning>,
    dna: HappRoleDna,
}

#[derive(Debug, Deserialize)]
struct RoleProvisioning {
    #[serde(default)]
    deferred: bool,
}

#[derive(Debug, Deserialize)]
struct HappRoleDna {
    #[serde(default)]
    modifiers: Option<Modifiers>,
    #[serde(default)]
    clone_limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct Modifiers {
    network_seed: Option<String>,
    #[serde(default)]
    properties: Option<serde_yaml::Value>,
}

fn holochain_root() -> PathBuf {
    // manifest-hygiene dir: elohim/holochain/tests/manifest-hygiene
    // target: elohim/holochain
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .expect("manifest-hygiene dir must be two levels deep under elohim/holochain")
}

fn load_dna_manifest(dir_name: &str) -> Result<DnaManifest> {
    let path = holochain_root().join("dna").join(dir_name).join("dna.yaml");
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("reading dna.yaml at {path:?}"))?;
    serde_yaml::from_str(&text).with_context(|| format!("parsing dna.yaml at {path:?}"))
}

fn load_happ_manifest() -> Result<HappManifest> {
    let path = holochain_root()
        .join("dna")
        .join("elohim")
        .join("workdir")
        .join("happ.yaml");
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("reading happ.yaml at {path:?}"))?;
    serde_yaml::from_str(&text).with_context(|| format!("parsing happ.yaml at {path:?}"))
}

fn expected_network_seed(dna_name: &str) -> String {
    format!("elohim_{dna_name}_alpha")
}

// -----------------------------------------------------------------------------

#[test]
fn every_dna_manifest_is_version_0() -> Result<()> {
    for (dir, name) in DNAS {
        let m = load_dna_manifest(dir)?;
        if m.manifest_version != "0" {
            bail!(
                "dna/{dir}/dna.yaml manifest_version must be \"0\" (Holochain 0.6 DnaManifest tag), got {:?}",
                m.manifest_version
            );
        }
        if m.name != *name {
            bail!(
                "dna/{dir}/dna.yaml name field must be {:?} (matches happ.yaml role), got {:?}",
                name, m.name
            );
        }
    }
    Ok(())
}

#[test]
fn every_dna_has_stable_alpha_network_seed() -> Result<()> {
    for (dir, name) in DNAS {
        let m = load_dna_manifest(dir)?;
        let seed = m
            .integrity
            .network_seed
            .as_ref()
            .ok_or_else(|| anyhow!("dna/{dir}/dna.yaml integrity.network_seed missing"))?;
        let expected = expected_network_seed(name);
        if seed != &expected {
            bail!(
                "dna/{dir}/dna.yaml integrity.network_seed must be {:?} (elohim_<dna>_alpha stability contract); got {:?}",
                expected, seed
            );
        }
    }
    Ok(())
}

// Note: a `every_dna_declares_lineage_field` test existed here and asserted
// `lineage: []` on every DNA manifest. Holochain 0.6 gates the `lineage` field
// behind the `unstable-migration` cargo feature; the stable `hc dna pack`
// rejects the field as unknown. The test (and the field) were removed pending
// upstream stabilization. Design intent — recoverable DNA upgrade chains —
// is tracked separately (rna module, NETWORK_UPGRADES.md, future brainstorm).

#[test]
fn happ_is_version_0_and_has_required_elohim_roles() -> Result<()> {
    let h = load_happ_manifest()?;
    if h.manifest_version != "0" {
        bail!(
            "elohim/workdir/happ.yaml manifest_version must be \"0\" (HC 0.6 AppManifest tag), got {:?}",
            h.manifest_version
        );
    }
    // The `hrea` role is intentionally absent until a real hREA `version_pin`
    // lands in elohim/holochain/dna/elohim/workdir/happ.yaml AND the Jenkins
    // pipeline fetches the upstream bundle into ../hrea/workdir/hrea.dna.
    // See the commented-out role block in happ.yaml and
    // elohim/holochain/dna/hrea/workdir/README.md. When that lands,
    // re-add "hrea" to expected_roles below and restore the deferred-role
    // assertion that follows.
    let expected_roles = [
        "lamad",
        "infrastructure",
        "imagodei",
        "mishpat",
        "node_registry",
    ];
    let got: Vec<&str> = h.roles.iter().map(|r| r.name.as_str()).collect();
    for role in expected_roles.iter() {
        if !got.contains(role) {
            bail!("happ.yaml is missing role {:?}; got {:?}", role, got);
        }
    }

    // Wave 3 M1: hrea was the only deferred role; other roles eagerly
    // provision at conductor startup. With the hrea role temporarily
    // commented out (pending real version_pin + Jenkinsfile fetch), no
    // role should be deferred at the moment. When hrea returns, this
    // becomes `assert_eq!(deferred_roles, vec!["hrea"])` again.
    let deferred_roles: Vec<&str> = h
        .roles
        .iter()
        .filter(|r| {
            r.provisioning
                .as_ref()
                .map(|p| p.deferred)
                .unwrap_or(false)
        })
        .map(|r| r.name.as_str())
        .collect();
    let expected_deferred: Vec<&str> = vec![];
    assert_eq!(
        deferred_roles, expected_deferred,
        "no role should be deferred while hrea is commented out; got: {deferred_roles:?}"
    );

    Ok(())
}

#[test]
fn every_role_has_clone_limit_zero() -> Result<()> {
    let h = load_happ_manifest()?;
    for role in &h.roles {
        let cl = role.dna.clone_limit.ok_or_else(|| {
            anyhow!(
                "happ.yaml role {:?} must declare `clone_limit:` (default-deny 0)",
                role.name
            )
        })?;
        if cl != 0 {
            bail!(
                "happ.yaml role {:?} clone_limit must be 0 (default-deny); got {}",
                role.name, cl
            );
        }
    }
    Ok(())
}

#[test]
fn happ_role_seeds_match_dna_manifest_seeds() -> Result<()> {
    let h = load_happ_manifest()?;
    let role_seeds: BTreeMap<&str, Option<&str>> = h
        .roles
        .iter()
        .map(|r| {
            (
                r.name.as_str(),
                r.dna.modifiers.as_ref().and_then(|m| m.network_seed.as_deref()),
            )
        })
        .collect();

    for (dir, dna_name) in DNAS {
        let dna = load_dna_manifest(dir)?;
        let dna_seed = dna.integrity.network_seed.as_deref();
        let happ_seed = role_seeds.get(dna_name).copied().flatten();
        match (dna_seed, happ_seed) {
            (Some(ds), Some(hs)) if ds == hs => {}
            (Some(ds), Some(hs)) => bail!(
                "network_seed mismatch for {dna_name}: dna.yaml says {:?}, happ.yaml role says {:?}",
                ds, hs
            ),
            (ds, hs) => bail!(
                "network_seed missing for {dna_name}: dna.yaml={:?}, happ.yaml={:?}",
                ds, hs
            ),
        }
    }
    Ok(())
}

#[test]
fn bootstrap_steward_dnas_declare_progenitor_property() -> Result<()> {
    let h = load_happ_manifest()?;
    for role in &h.roles {
        if !BOOTSTRAP_STEWARD_DNAS.contains(&role.name.as_str()) {
            continue;
        }
        let props = role
            .dna
            .modifiers
            .as_ref()
            .and_then(|m| m.properties.as_ref())
            .ok_or_else(|| {
                anyhow!(
                    "happ.yaml role {:?} is a bootstrap-steward DNA but has no modifiers.properties block",
                    role.name
                )
            })?;
        let map = match props {
            serde_yaml::Value::Mapping(m) => m,
            _ => bail!(
                "happ.yaml role {:?} modifiers.properties must be a mapping",
                role.name
            ),
        };
        let has_progenitor = map
            .keys()
            .any(|k| matches!(k, serde_yaml::Value::String(s) if s == "progenitor_pubkey"));
        if !has_progenitor {
            bail!(
                "happ.yaml role {:?} modifiers.properties must declare `progenitor_pubkey` \
                 (the Holochain primitive field — surface language is \"bootstrap steward\")",
                role.name
            );
        }
    }
    Ok(())
}

#[test]
fn infrastructure_role_is_federation_native() -> Result<()> {
    // Infrastructure must NOT declare a progenitor_pubkey — it is
    // federation-native per wave-1 plan §1.2 Q2 resolution.
    let h = load_happ_manifest()?;
    let role = h
        .roles
        .iter()
        .find(|r| r.name == "infrastructure")
        .ok_or_else(|| anyhow!("happ.yaml missing infrastructure role"))?;
    if let Some(props) = role.dna.modifiers.as_ref().and_then(|m| m.properties.as_ref()) {
        if let serde_yaml::Value::Mapping(map) = props {
            let has_progenitor = map
                .keys()
                .any(|k| matches!(k, serde_yaml::Value::String(s) if s == "progenitor_pubkey"));
            if has_progenitor {
                bail!(
                    "infrastructure role must NOT declare progenitor_pubkey — it is \
                     federation-native (doorways self-register via their own agent key). \
                     See wave-1 plan §1.2 Q2."
                );
            }
        }
    }
    Ok(())
}

#[test]
fn no_bare_progenitor_vocabulary_in_dna_yaml() -> Result<()> {
    // `progenitor_pubkey` IS allowed — it's the Holochain primitive field
    // name. Bare "progenitor" occurrences without the "_pubkey" suffix are
    // surface-language leaks we want to catch.
    for (dir, _) in DNAS {
        let path = holochain_root().join("dna").join(dir).join("dna.yaml");
        let text = std::fs::read_to_string(&path)?;
        if has_bare_progenitor(&text) {
            bail!(
                "dna/{dir}/dna.yaml uses \"progenitor\" outside the schema field name \
                 `progenitor_pubkey`. Surface language must be \"bootstrap steward\".\n\
                 File: {:?}",
                path
            );
        }
    }
    Ok(())
}

/// Hand-rolled matcher for `progenitor` not followed by `_pubkey`.
fn has_bare_progenitor(haystack: &str) -> bool {
    let needle = "progenitor";
    let suffix = "_pubkey";
    let mut cursor = 0;
    while let Some(idx) = haystack[cursor..].find(needle) {
        let hit = cursor + idx;
        let after = hit + needle.len();
        if !haystack[after..].starts_with(suffix) {
            return true;
        }
        cursor = after;
    }
    false
}

// -----------------------------------------------------------------------------
// self-tests for the hand-rolled matcher — kept in-file so the assertion is
// close to the code it guards.

#[test]
fn bare_progenitor_matcher_distinguishes_surface_from_schema() {
    // Schema-field usage (allowed).
    assert!(!has_bare_progenitor("progenitor_pubkey: ~"));
    assert!(!has_bare_progenitor(
        "modifiers.properties.progenitor_pubkey in happ.yaml"
    ));
    // Bare surface-language use (banned).
    assert!(has_bare_progenitor("// progenitor pattern"));
    assert!(has_bare_progenitor("R&O calls this the progenitor"));
    // Mixed — one bad occurrence is enough.
    assert!(has_bare_progenitor(
        "progenitor_pubkey field, and the progenitor too"
    ));
}
