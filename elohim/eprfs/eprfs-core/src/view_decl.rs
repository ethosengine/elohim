//! The serializable carrier of a root and a view (interfaces spec §B.3): the Rust twin of the SDK
//! `tree-view` object.
//!
//! Canonical form (DAG-CBOR): a tag-42 root and byte-string path components, so non-UTF-8 names
//! survive. Human-readable form (JSON): the CID string and slash-joined UTF-8 paths; a non-UTF-8
//! path refuses with [`TreeError::InvalidView`]. The root path is `""` in JSON and the empty
//! component list in DAG-CBOR, so `View::whole()` is `[""]`.
//!
//! Decoding refuses a path list that is empty, not sorted component-wise (not by the slash-joined
//! string: `-` 0x2d sorts before `/` 0x2f), or not an antichain (a path covered by another), as
//! `decode_tree` refuses noncanonical bytes. So [`TreeView::cid`] is a function of the [`View`]
//! (B7).

use serde::de::{self, Deserializer};
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};

use crate::address::{BlobCid, BlobLink};
use crate::tree::{TreeError, TreeResult};
use crate::walk::{TreePath, View};

/// A tree root and the view of it a reader declares: what `needs.closure`, a task's input and a
/// derivation key's input name (R1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeView {
    root: BlobLink,
    paths: Vec<TreePath>,
}

impl TreeView {
    /// Normalized by [`View`]'s own rule: sorted component-wise, an antichain.
    pub fn new(root: BlobCid, view: &View) -> Self {
        Self {
            root: root.into(),
            paths: view.paths().cloned().collect(),
        }
    }

    pub fn root(&self) -> &BlobCid {
        self.root.as_blob_cid()
    }

    pub fn view(&self) -> View {
        View::from_tree_paths(self.paths.iter().cloned())
    }

    /// The paths, in canonical order.
    pub fn paths(&self) -> &[TreePath] {
        &self.paths
    }

    /// The DAG-CBOR CID of the canonical form: the need key of R1. An empty view has no
    /// canonical form (decoding refuses it), so it has no CID either.
    pub fn cid(&self) -> TreeResult<BlobCid> {
        check_canonical(&self.paths)?;
        let bytes = serde_ipld_dagcbor::to_vec(self)
            .map_err(|error| TreeError::InvalidView(format!("tree-view encoding: {error}")))?;
        Ok(BlobCid::compute(&bytes))
    }
}

/// The decode-side rule, also applied before a CID is computed.
fn check_canonical(paths: &[TreePath]) -> TreeResult<()> {
    if paths.is_empty() {
        return Err(TreeError::InvalidView("a tree view names no path".into()));
    }
    // Sorted component-wise and adjacent pairs never prefix one another: for a sorted list that
    // is the whole antichain check, because anything between a path and a path it covers is
    // covered by it too.
    for pair in paths.windows(2) {
        if pair[0] >= pair[1] {
            return Err(TreeError::InvalidView(
                "tree-view paths are not sorted component-wise".into(),
            ));
        }
        if pair[1].starts_with(&pair[0]) {
            return Err(TreeError::InvalidView(format!(
                "tree-view path {} is covered by {}",
                pair[1], pair[0]
            )));
        }
    }
    Ok(())
}

impl Serialize for TreeView {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        check_canonical(&self.paths).map_err(serde::ser::Error::custom)?;
        let mut state = serializer.serialize_struct("TreeView", 2)?;
        state.serialize_field("paths", &self.paths)?;
        state.serialize_field("root", &self.root)?;
        state.end()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TreeViewWire {
    root: BlobLink,
    paths: Vec<TreePath>,
}

impl<'de> Deserialize<'de> for TreeView {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = TreeViewWire::deserialize(deserializer)?;
        check_canonical(&wire.paths).map_err(de::Error::custom)?;
        if wire.root.as_blob_cid().as_cid().codec() != 0x71 {
            return Err(de::Error::custom(
                "tree-view root must use DAG-CBOR CID codec",
            ));
        }
        Ok(TreeView {
            root: wire.root,
            paths: wire.paths,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::tp;

    fn root() -> BlobCid {
        BlobCid::parse("bafyreia7fco57n2u6rg7m4hym6hx7jdg3c7ga7bimukcoz5cxhizsvbble").unwrap()
    }

    #[test]
    fn new_normalizes_and_json_spells_the_root_path_empty() {
        let view = View::from_tree_paths([tp("a/b"), tp("a")]);
        let tv = TreeView::new(root(), &view);
        assert_eq!(tv.paths(), &[tp("a")]);
        let whole = TreeView::new(root(), &View::whole());
        assert_eq!(
            serde_json::to_value(&whole).unwrap(),
            serde_json::json!({"paths": [""], "root": root().to_string()})
        );
        assert_eq!(whole.view(), View::whole());
    }

    #[test]
    fn decode_refuses_unsorted_covered_and_empty_path_lists() {
        let r = root().to_string();
        for paths in [
            serde_json::json!(["a", "a/b"]),
            serde_json::json!(["b", "a"]),
            serde_json::json!([]),
        ] {
            let value = serde_json::json!({"root": r, "paths": paths});
            assert!(serde_json::from_value::<TreeView>(value).is_err());
        }
        // Component-wise order, not string order: "a/b" sorts before "a-b" as paths.
        let ok = serde_json::json!({"root": r, "paths": ["a/b", "a-b"]});
        assert!(serde_json::from_value::<TreeView>(ok).is_ok());
        let unknown = serde_json::json!({"root": r, "paths": ["a"], "extra": 1});
        assert!(serde_json::from_value::<TreeView>(unknown).is_err());
    }

    #[test]
    fn canonical_form_keeps_non_utf8_names_and_cid_is_a_function_of_the_view() {
        let odd = TreePath::from_components(vec![vec![0xff], b"x".to_vec()]).unwrap();
        let tv = TreeView::new(root(), &View::from_tree_paths([odd.clone()]));
        let bytes = serde_ipld_dagcbor::to_vec(&tv).unwrap();
        let back: TreeView = serde_ipld_dagcbor::from_slice(&bytes).unwrap();
        assert_eq!(back, tv);
        assert!(
            serde_json::to_string(&tv).is_err(),
            "JSON refuses a non-UTF-8 name"
        );
        let same = TreeView::new(
            root(),
            &View::from_tree_paths([odd.clone(), odd.join(b"deeper")]),
        );
        assert_eq!(same.cid().unwrap(), tv.cid().unwrap());
        assert!(TreeView::new(root(), &View::from_tree_paths([]))
            .cid()
            .is_err());
    }
}
