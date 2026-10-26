//! A gate's closure material as a task input: one eprfs tree, built here so CID math lives only in
//! Rust.
//!
//! The material file is the canonical JSON `genesis/orchestrator/gate-evidence.mjs::closureKey`
//! hashes (sorted keys, no whitespace), so its sha256 is brit's `--inputs-hash` and nothing else
//! is: the dedup key stays brit's, and the content address is the tree's. Each top-level field is
//! one leaf named by its key, holding that field's canonical JSON as raw bytes, so a method's view
//! can name the fields it reads. The gate method's view is every field.

use std::{collections::BTreeMap, fs, path::Path};

use eprfs_core::{
    encode_tree, BlobCid, ProjectedRoot, TreeEntry, TreeEntryKind, TreeLimits, TreeNode, TreePath,
    TreeView, View,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// A read, canonical closure material and the tree it addresses.
#[derive(Debug)]
pub struct Material {
    bytes: Vec<u8>,
    fields: BTreeMap<String, Value>,
    root: BlobCid,
}

impl Material {
    /// Reads `path`, refusing anything but a JSON object whose bytes are already canonical: the
    /// same bytes serde writes back, so the inputs hash and the tree name one value.
    pub fn read(path: &Path) -> Result<Self, String> {
        let bytes =
            fs::read(path).map_err(|error| format!("reading {}: {error}", path.display()))?;
        Self::from_bytes(bytes)
    }

    fn from_bytes(bytes: Vec<u8>) -> Result<Self, String> {
        let Value::Object(object) = serde_json::from_slice::<Value>(&bytes)
            .map_err(|error| format!("closure material is not JSON: {error}"))?
        else {
            return Err("closure material is not a JSON object".into());
        };
        let fields: BTreeMap<String, Value> = object.into_iter().collect();
        let canonical = serde_json::to_vec(&fields).map_err(|error| error.to_string())?;
        if canonical != bytes {
            return Err(
                "closure material is not canonical JSON (sorted keys, no whitespace)".into(),
            );
        }
        if fields.is_empty() {
            return Err("closure material names no field".into());
        }
        let mut entries = Vec::with_capacity(fields.len());
        for (name, value) in &fields {
            let leaf = serde_json::to_vec(value).map_err(|error| error.to_string())?;
            entries.push(TreeEntry {
                name: name.as_bytes().to_vec(),
                kind: TreeEntryKind::File {
                    blob: BlobCid::compute_raw(&leaf).into(),
                    executable: false,
                },
            });
        }
        let node = TreeNode::new(entries).map_err(|error| format!("closure tree: {error}"))?;
        let (root, _) = encode_tree(&node, &TreeLimits::default())
            .map_err(|error| format!("closure tree: {error}"))?;
        Ok(Self {
            bytes,
            fields,
            root,
        })
    }

    /// The material's exact bytes.
    pub fn byte_len(&self) -> u64 {
        self.bytes.len() as u64
    }

    /// Lowercase hex sha256 of the bytes: brit's `--inputs-hash`, a dedup key and not an address.
    pub fn inputs_hash(&self) -> String {
        hex::encode(Sha256::digest(&self.bytes))
    }

    /// The tree root: the task's input root.
    pub fn root(&self) -> &BlobCid {
        &self.root
    }

    /// Every field, as the method's declared view.
    pub fn view(&self) -> Result<View, String> {
        let paths = self
            .fields
            .keys()
            .map(|name| {
                TreePath::from_components(vec![name.as_bytes().to_vec()])
                    .map_err(|error| format!("closure field name: {error}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(View::from_tree_paths(paths))
    }

    /// The input projected under `view`. `project_view` has not landed in eprfs-core, so this is
    /// the one projection decidable without it: a view naming every field of this one-level tree
    /// projects to the tree itself. Any other view is refused rather than guessed.
    pub fn projected(&self, view: &TreeView) -> Result<ProjectedRoot, String> {
        if view.root() != &self.root {
            return Err("the view is not over this closure's root".into());
        }
        if !view.paths().iter().eq(self.view()?.paths()) {
            return Err("only a view naming every closure field projects here".into());
        }
        // A ProjectedRoot decodes from its CID string, as a record's writer leaves it.
        serde_json::from_value(Value::String(self.root.to_string()))
            .map_err(|error| format!("projected root: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn material(text: &str) -> Result<Material, String> {
        Material::from_bytes(text.as_bytes().to_vec())
    }

    #[test]
    fn the_inputs_hash_is_the_sha256_of_the_canonical_bytes() {
        let text = r#"{"manifest":"abc","project":"orchestrator","v":3}"#;
        let m = material(text).unwrap();
        assert_eq!(
            m.inputs_hash(),
            hex::encode(Sha256::digest(text.as_bytes()))
        );
        assert_eq!(m.byte_len(), text.len() as u64);
    }

    #[test]
    fn a_non_canonical_material_is_refused() {
        for text in [
            r#"{"v":3,"project":"orchestrator"}"#,
            r#"{"project": "orchestrator"}"#,
            "{\"v\":3}\n",
            "[1]",
            "{}",
        ] {
            assert!(material(text).is_err(), "{text}");
        }
    }

    /// One field moves the root; the same fields give the same root; the whole-field view
    /// projects to the root and a narrower view is refused.
    #[test]
    fn the_root_follows_the_fields_and_projects_only_under_the_whole_view() {
        let a = material(r#"{"manifest":"abc","v":3}"#).unwrap();
        let b = material(r#"{"manifest":"abd","v":3}"#).unwrap();
        assert_ne!(a.root(), b.root());
        assert_eq!(
            a.root(),
            material(r#"{"manifest":"abc","v":3}"#).unwrap().root()
        );
        assert_eq!(a.root().as_cid().codec(), 0x71);
        let whole = TreeView::new(a.root().clone(), &a.view().unwrap());
        assert_eq!(a.projected(&whole).unwrap().cid(), a.root());
        let narrow = TreeView::new(
            a.root().clone(),
            &View::from_tree_paths([TreePath::from_slash_bytes(b"v").unwrap()]),
        );
        assert!(a.projected(&narrow).is_err());
        let other = TreeView::new(b.root().clone(), &b.view().unwrap());
        assert!(a.projected(&other).is_err());
    }
}
