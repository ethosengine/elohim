//! View-scoped walk and query over a verified tree.
//!
//! [`load_verified_tree`](crate::load_verified_tree) fetches a root's whole
//! closure. The functions here fetch only what a [`View`] names: the tree
//! nodes on the spine to each view path, and the nodes below a view path.
//! Every node is verified against the link in its parent through the same
//! checks the closure loader uses, and every traversal is bounded by
//! [`TreeLimits`] on the nodes it visits.
//!
//! A node the store does not hold is a named absence, not an error: the walk
//! yields [`WalkItem::Absent`] with [`AbsentReason::NotHeld`] and continues
//! with the rest of the view. A node a tier refused, could not reach, or could
//! not place under the root is [`AbsentReason::Withheld`]: absence is not
//! established, and the walk continues too. A node that is held but fails its
//! address check, or fails to decode, is still an error: integrity is never
//! degraded to absence. A single answer ([`read_path`], [`list_dir`],
//! [`diff_trees`]) cannot be partial, so a node it needs and no tier gave is
//! [`TreeError::Unanswered`] naming the path, with ruling 7's three parts.
//!
//! Every node and leaf fetch goes through [`EprfsStorage::fetch_at`] with the
//! root it lies under and its path, so the same functions walk a local store, a
//! [`TierChain`](crate::TierChain) or a remote tier through `dyn EprfsStorage`.

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::fmt;

use bytes::Bytes;
use seam_contracts::Unanswered;
use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::tier::{ItemAbsence, TierId};
use crate::tree::{check_root_codec, fetch_checked, fetch_node, valid_name, Budget};
use crate::{
    BatchItem, BatchOptions, BlobCid, EprfsStorage, ProjectionPath, TreeEntry, TreeEntryKind,
    TreeError, TreeLimits, TreeNode, TreeResult,
};

/// A path inside a tree as raw byte components. Tree names are bytes and may
/// be non-UTF-8, so matching never goes through an OS path. The empty path is
/// the root.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct TreePath(Vec<Vec<u8>>);

impl TreePath {
    pub fn root() -> Self {
        Self(Vec::new())
    }

    /// Each component must be a valid single tree name.
    pub fn from_components(components: Vec<Vec<u8>>) -> TreeResult<Self> {
        if let Some(bad) = components.iter().find(|c| !valid_name(c)) {
            return Err(TreeError::InvalidView(format!(
                "invalid path component {:?}",
                String::from_utf8_lossy(bad)
            )));
        }
        Ok(Self(components))
    }

    /// `b""` is the root; otherwise split on `b'/'`. Empty components are refused.
    pub fn from_slash_bytes(bytes: &[u8]) -> TreeResult<Self> {
        if bytes.is_empty() {
            return Ok(Self::root());
        }
        Self::from_components(bytes.split(|b| *b == b'/').map(<[u8]>::to_vec).collect())
    }

    pub fn components(&self) -> &[Vec<u8>] {
        &self.0
    }

    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    pub fn join(&self, name: &[u8]) -> Self {
        let mut components = self.0.clone();
        components.push(name.to_vec());
        Self(components)
    }

    /// Component-wise prefix: `src` is a prefix of `src/x`, never of `src2`.
    pub fn starts_with(&self, prefix: &TreePath) -> bool {
        self.0.starts_with(&prefix.0)
    }

    /// Sum of name lengths: the measure `TreeLimits::max_path_bytes` bounds.
    pub fn byte_len(&self) -> usize {
        self.0.iter().map(Vec::len).sum()
    }

    /// A path whose components were already validated (decoded tree names).
    pub(crate) fn from_trusted(components: Vec<Vec<u8>>) -> Self {
        Self(components)
    }

    /// The slash-joined UTF-8 form, the path's human-readable wire form. The root is `""`. A
    /// non-UTF-8 component refuses with [`TreeError::InvalidView`]: it is never decoded lossily.
    pub fn to_slash_string(&self) -> TreeResult<String> {
        let mut out = String::with_capacity(self.byte_len() + self.0.len());
        for (index, component) in self.0.iter().enumerate() {
            if index > 0 {
                out.push('/');
            }
            out.push_str(std::str::from_utf8(component).map_err(|_| {
                TreeError::InvalidView(format!(
                    "path component is not UTF-8: {:?}",
                    String::from_utf8_lossy(component)
                ))
            })?);
        }
        Ok(out)
    }
}

/// Two forms, the same rule as `BlobLink` (B5): DAG-CBOR and other binary formats carry an array
/// of byte strings, one per component, so non-UTF-8 names survive; JSON carries the slash-joined
/// UTF-8 string (`""` for the root), and a non-UTF-8 name refuses rather than decoding lossily.
impl Serialize for TreePath {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            let text = self.to_slash_string().map_err(serde::ser::Error::custom)?;
            serializer.serialize_str(&text)
        } else {
            use serde::ser::SerializeSeq;
            let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
            for component in &self.0 {
                seq.serialize_element(serde_bytes::Bytes::new(component))?;
            }
            seq.end()
        }
    }
}

impl<'de> Deserialize<'de> for TreePath {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            let text = String::deserialize(deserializer)?;
            TreePath::from_slash_bytes(text.as_bytes()).map_err(de::Error::custom)
        } else {
            struct Components;
            impl<'de> Visitor<'de> for Components {
                type Value = Vec<Vec<u8>>;
                fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    f.write_str("an array of byte-string path components")
                }
                fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                    let mut out = Vec::new();
                    while let Some(component) = seq.next_element::<serde_bytes::ByteBuf>()? {
                        out.push(component.into_vec());
                    }
                    Ok(out)
                }
            }
            let components = deserializer.deserialize_seq(Components)?;
            // The one reserved path a tree entry can never name (a derivation output, C2).
            if let Some(derived) = TreePath::parse_derived(&components) {
                return Ok(derived);
            }
            TreePath::from_components(components).map_err(de::Error::custom)
        }
    }
}

impl TryFrom<&ProjectionPath> for TreePath {
    type Error = TreeError;

    fn try_from(path: &ProjectionPath) -> TreeResult<Self> {
        let mut components = Vec::new();
        for component in path.as_path().components() {
            match component {
                std::path::Component::Normal(name) => {
                    #[cfg(unix)]
                    {
                        use std::os::unix::ffi::OsStrExt;
                        components.push(name.as_bytes().to_vec());
                    }
                    #[cfg(not(unix))]
                    {
                        let name = name.to_str().ok_or_else(|| {
                            TreeError::InvalidView("non-UTF-8 projection path".into())
                        })?;
                        components.push(name.as_bytes().to_vec());
                    }
                }
                _ => {
                    return Err(TreeError::InvalidView(format!(
                        "projection path is not relative and normal: {}",
                        path.as_path().display()
                    )))
                }
            }
        }
        Self::from_components(components)
    }
}

impl fmt::Display for TreePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return f.write_str("<root>");
        }
        for (index, component) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str("/")?;
            }
            f.write_str(&String::from_utf8_lossy(component))?;
        }
        Ok(())
    }
}

/// Where a path stands relative to a [`View`]. (Slice 1 called this `Placement`; decision 12
/// renamed it, because "placement" is the placement contract's word for who carries what.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coverage {
    /// Some view path is a prefix of this path, or equal to it.
    InView,
    /// This path is a strict prefix of some view path: an ancestor the walk
    /// must pass through. The root is on the spine of every non-empty view.
    Spine,
    /// Neither: never fetched and never descended. Its name is still visible
    /// in the verified spine node that holds it.
    Outside,
}

/// The set of paths a reader intends to act on.
///
/// - A directory named in the view brings in its whole subtree; a file brings in itself.
/// - Matching is component-wise: `src` covers `src/x` but not `src2`.
/// - The set is normalized to an antichain: duplicates are removed, and a path
///   covered by a shorter view path is dropped (`{a, a/b}` becomes `{a}`).
/// - [`View::whole`] is `{root}` and covers everything. An empty view fetches
///   nothing and yields nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View {
    paths: BTreeSet<TreePath>,
}

/// A view crosses a wire as its sequence of paths, each in [`TreePath`]'s form.
impl Serialize for View {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(&self.paths)
    }
}

/// Re-normalized on arrival ([`View::from_tree_paths`]): a sender cannot hand a reader a view
/// that is not an antichain.
impl<'de> Deserialize<'de> for View {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Vec::<TreePath>::deserialize(deserializer).map(Self::from_tree_paths)
    }
}

impl View {
    pub fn whole() -> Self {
        Self::from_tree_paths([TreePath::root()])
    }

    pub fn new(paths: impl IntoIterator<Item = ProjectionPath>) -> TreeResult<Self> {
        let paths = paths
            .into_iter()
            .map(|path| TreePath::try_from(&path))
            .collect::<TreeResult<Vec<_>>>()?;
        Ok(Self::from_tree_paths(paths))
    }

    pub fn from_tree_paths(paths: impl IntoIterator<Item = TreePath>) -> Self {
        let sorted: BTreeSet<TreePath> = paths.into_iter().collect();
        // Sorted component-wise, every path a kept path covers follows it
        // contiguously, so comparing with the last kept path is enough.
        let mut kept: BTreeSet<TreePath> = BTreeSet::new();
        let mut last: Option<TreePath> = None;
        for path in sorted {
            if last.as_ref().is_some_and(|prefix| path.starts_with(prefix)) {
                continue;
            }
            last = Some(path.clone());
            kept.insert(path);
        }
        Self { paths: kept }
    }

    pub fn paths(&self) -> impl Iterator<Item = &TreePath> {
        self.paths.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub fn coverage(&self, path: &TreePath) -> Coverage {
        if self.paths.iter().any(|view| path.starts_with(view)) {
            Coverage::InView
        } else if self.paths.iter().any(|view| view.starts_with(path)) {
            Coverage::Spine
        } else {
            Coverage::Outside
        }
    }

    pub(crate) fn check_path_bytes(&self, limits: &TreeLimits) -> TreeResult<()> {
        if self
            .paths
            .iter()
            .any(|path| path.byte_len() > limits.max_path_bytes)
        {
            return Err(TreeError::Budget("path bytes"));
        }
        Ok(())
    }

    pub(crate) fn root_scope(&self) -> Option<Scope<'_>> {
        let first = self.paths.iter().next()?;
        Some(if first.is_root() {
            Scope::InView(first)
        } else {
            Scope::Spine(self.paths.iter().collect())
        })
    }
}

/// What a frontier node carries: the view path covering it, or the view paths
/// still to be reached through it. Tails borrow from the [`View`].
#[derive(Debug, Clone)]
pub(crate) enum Scope<'v> {
    InView(&'v TreePath),
    Spine(Vec<&'v TreePath>),
}

impl Scope<'_> {
    fn coverage(&self) -> Coverage {
        match self {
            Scope::InView(_) => Coverage::InView,
            Scope::Spine(_) => Coverage::Spine,
        }
    }
}

/// One node's entries matched against its scope.
struct Classified<'v> {
    /// Parallel to the node's entries: `None` is Outside.
    entries: Vec<Option<Scope<'v>>>,
    /// View paths whose next component names no entry.
    unmatched: Vec<&'v TreePath>,
}

/// Match a node at `depth` against its scope. Entries and tails are both
/// byte-sorted, so one merge pass suffices.
fn classify<'v>(scope: &Scope<'v>, depth: usize, entries: &[TreeEntry]) -> Classified<'v> {
    let tails = match scope {
        Scope::InView(covering) => {
            return Classified {
                entries: entries
                    .iter()
                    .map(|_| Some(Scope::InView(covering)))
                    .collect(),
                unmatched: Vec::new(),
            }
        }
        Scope::Spine(tails) => tails,
    };
    let mut classified = Vec::with_capacity(entries.len());
    let mut unmatched = Vec::new();
    let mut next = 0;
    for entry in entries {
        while next < tails.len() && tails[next].components()[depth] < entry.name {
            unmatched.push(tails[next]);
            next += 1;
        }
        let start = next;
        while next < tails.len() && tails[next].components()[depth] == entry.name {
            next += 1;
        }
        classified.push(group_scope(&tails[start..next], depth));
    }
    unmatched.extend_from_slice(&tails[next..]);
    Classified {
        entries: classified,
        unmatched,
    }
}

/// The scope of a child whose name the view paths in `group` all share at
/// `depth`: InView when one of them ends there, else Spine; `None` when empty.
fn group_scope<'v>(group: &[&'v TreePath], depth: usize) -> Option<Scope<'v>> {
    if group.is_empty() {
        None
    } else if let Some(exact) = group
        .iter()
        .find(|tail| tail.components().len() == depth + 1)
    {
        // Normalization makes an exact match the only tail in its group.
        Some(Scope::InView(exact))
    } else {
        Some(Scope::Spine(group.to_vec()))
    }
}

fn coverage_of(scope: &Option<Scope<'_>>) -> Coverage {
    scope.as_ref().map_or(Coverage::Outside, Scope::coverage)
}

/// Why a view path yielded no node. Non-exhaustive, variants included: a
/// remote tier adds reasons without breaking a caller, and a crate outside
/// eprfs-core builds values through the public constructors (B12).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AbsentReason {
    /// The verified tree has no entry at `missing_at`.
    #[non_exhaustive]
    NotFound { missing_at: TreePath },
    /// The view path continues below a file or symlink at `leaf`.
    #[non_exhaustive]
    ThroughLeaf { leaf: TreePath },
    /// The tree names a node at `missing_at` that was observed absent at every
    /// tier in `tried` (R6). `tried` is empty for a leaf store (B2).
    #[non_exhaustive]
    NotHeld {
        missing_at: TreePath,
        cid: BlobCid,
        tried: Vec<TierId>,
    },
    /// A tier refused, was over budget, could not be reached, or could not
    /// establish the path's membership: absence is NOT established, and the
    /// walk continues.
    #[non_exhaustive]
    Withheld {
        missing_at: TreePath,
        cid: BlobCid,
        why: ItemAbsence,
    },
}

impl AbsentReason {
    pub fn not_found(missing_at: TreePath) -> Self {
        Self::NotFound { missing_at }
    }

    pub fn through_leaf(leaf: TreePath) -> Self {
        Self::ThroughLeaf { leaf }
    }

    pub fn not_held(missing_at: TreePath, cid: BlobCid, tried: Vec<TierId>) -> Self {
        Self::NotHeld {
            missing_at,
            cid,
            tried,
        }
    }

    pub fn withheld(missing_at: TreePath, cid: BlobCid, why: ItemAbsence) -> Self {
        Self::Withheld {
            missing_at,
            cid,
            why,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct WalkedNode {
    pub path: TreePath,
    pub cid: BlobCid,
    pub node: TreeNode,
    /// Spine or InView, never Outside.
    pub coverage: Coverage,
    /// Parallel to `node.entries`.
    pub entry_coverage: Vec<Coverage>,
}

impl WalkedNode {
    pub fn new(
        path: TreePath,
        cid: BlobCid,
        node: TreeNode,
        coverage: Coverage,
        entry_coverage: Vec<Coverage>,
    ) -> Self {
        Self {
            path,
            cid,
            node,
            coverage,
            entry_coverage,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum WalkItem {
    Node(WalkedNode),
    /// An External entry, never followed. Spine means a view path went through it.
    #[non_exhaustive]
    Boundary {
        path: TreePath,
        kind: String,
        reference: Vec<u8>,
        coverage: Coverage,
    },
    /// A view path that yielded no node. `NotHeld` and `Withheld` carry ruling
    /// 7's three parts in `unanswered` (when to come back, what is local
    /// meanwhile); `NotFound` and `ThroughLeaf` are observed answers about a
    /// held tree and carry `None`.
    #[non_exhaustive]
    Absent {
        view_path: TreePath,
        reason: AbsentReason,
        unanswered: Option<Unanswered<ItemAbsence>>,
    },
}

impl WalkItem {
    pub fn node(node: WalkedNode) -> Self {
        Self::Node(node)
    }

    pub fn boundary(path: TreePath, kind: String, reference: Vec<u8>, coverage: Coverage) -> Self {
        Self::Boundary {
            path,
            kind,
            reference,
            coverage,
        }
    }

    /// A view path that yielded no node, checked: `NotFound` and `ThroughLeaf` carry no
    /// unanswered parts; `NotHeld` carries `Unanswered { why: NotHeld { tried } }` with the same
    /// `tried`; `Withheld` carries `Unanswered { why }` with the same, unobserved, `why`. Any other
    /// combination is refused (`Invalid`), so a remote tier building values through this
    /// constructor (B12) cannot name one reason and carry another's come-back. Revision of
    /// interfaces spec §B: it returns `TreeResult`.
    pub fn absent(
        view_path: TreePath,
        reason: AbsentReason,
        unanswered: Option<Unanswered<ItemAbsence>>,
    ) -> TreeResult<Self> {
        let consistent = match (&reason, &unanswered) {
            (AbsentReason::NotFound { .. } | AbsentReason::ThroughLeaf { .. }, None) => true,
            (AbsentReason::NotHeld { tried, .. }, Some(u)) => {
                matches!(&u.why, ItemAbsence::NotHeld { tried: named } if named == tried)
            }
            (AbsentReason::Withheld { why, .. }, Some(u)) => !why.is_observed() && &u.why == why,
            _ => false,
        };
        if !consistent {
            return Err(TreeError::Invalid(
                "an absence's reason and its unanswered parts disagree".into(),
            ));
        }
        Ok(Self::Absent {
            view_path,
            reason,
            unanswered,
        })
    }

    /// The verified tree has no entry at `missing_at`.
    pub fn not_found(view_path: TreePath, missing_at: TreePath) -> Self {
        Self::Absent {
            view_path,
            reason: AbsentReason::not_found(missing_at),
            unanswered: None,
        }
    }

    /// The view path continues below the file or symlink at `leaf`.
    pub fn through_leaf(view_path: TreePath, leaf: TreePath) -> Self {
        Self::Absent {
            view_path,
            reason: AbsentReason::through_leaf(leaf),
            unanswered: None,
        }
    }

    /// The node at `missing_at` no tier gave, with the answer that says why: the reason is read
    /// from `unanswered`, so the two cannot disagree (`NotHeld` for an observed absence,
    /// `Withheld` for every other kind).
    pub fn unanswered(
        view_path: TreePath,
        missing_at: TreePath,
        cid: BlobCid,
        unanswered: Unanswered<ItemAbsence>,
    ) -> Self {
        let reason = match &unanswered.why {
            ItemAbsence::NotHeld { tried } => {
                AbsentReason::not_held(missing_at, cid, tried.clone())
            }
            other => AbsentReason::withheld(missing_at, cid, other.clone()),
        };
        Self::Absent {
            view_path,
            reason,
            unanswered: Some(unanswered),
        }
    }

    /// The bytes a gathered walk keeps for this item beyond what a fetched node accounts for: an
    /// absence's paths and every string its unanswered parts carry, which a remote tier supplies.
    /// Nodes are charged their path where they are kept; a boundary's path is charged as the walk
    /// meets it (`external path bytes`).
    pub(crate) fn retained_bytes(&self) -> usize {
        match self {
            WalkItem::Absent {
                view_path,
                reason,
                unanswered,
            } => {
                let reason_path = match reason {
                    AbsentReason::NotFound { missing_at }
                    | AbsentReason::NotHeld { missing_at, .. }
                    | AbsentReason::Withheld { missing_at, .. } => missing_at.byte_len(),
                    AbsentReason::ThroughLeaf { leaf } => leaf.byte_len(),
                };
                let reason_ids = match reason {
                    AbsentReason::NotHeld { tried, .. } => ids_bytes(tried),
                    AbsentReason::Withheld { why, .. } => absence_bytes(why),
                    AbsentReason::NotFound { .. } | AbsentReason::ThroughLeaf { .. } => 0,
                };
                view_path
                    .byte_len()
                    .saturating_add(reason_path)
                    .saturating_add(reason_ids)
                    .saturating_add(unanswered.as_ref().map_or(0, unanswered_bytes))
            }
            WalkItem::Node(node) => node.path.byte_len(),
            WalkItem::Boundary { .. } => 0,
        }
    }
}

fn ids_bytes(ids: &[TierId]) -> usize {
    ids.iter().map(|id| id.as_str().len()).sum()
}

fn absence_bytes(why: &ItemAbsence) -> usize {
    match why {
        ItemAbsence::NotHeld { tried } => ids_bytes(tried),
        ItemAbsence::Refused { by, refusal } => by
            .as_str()
            .len()
            .saturating_add(refusal.needs_standing.as_ref().map_or(0, String::len)),
        ItemAbsence::OverBudget { by, rule } => by.as_str().len().saturating_add(rule.len()),
        ItemAbsence::Unreachable { tier } => tier.as_str().len(),
        ItemAbsence::MembershipUnknown { by } => by.as_str().len(),
        ItemAbsence::Integrity { from } => from.as_str().len(),
    }
}

fn unanswered_bytes(u: &Unanswered<ItemAbsence>) -> usize {
    use seam_contracts::{ComeBack, Meanwhile};
    let come_back = match &u.come_back {
        ComeBack::At { .. } => 0,
        ComeBack::OnEvent { event } => event.len(),
    };
    let meanwhile = match &u.meanwhile {
        Meanwhile::UseHeld { what } => what.len(),
        Meanwhile::Nothing { because } => because.len(),
        Meanwhile::RunHere | Meanwhile::BuiltInInForce => 0,
    };
    let escalation = u.escalation.as_ref().map_or(0, |e| {
        e.who_can_be_asked
            .iter()
            .map(String::len)
            .fold(e.changed.len(), usize::saturating_add)
    });
    absence_bytes(&u.why)
        .saturating_add(come_back)
        .saturating_add(meanwhile)
        .saturating_add(escalation)
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct WalkStats {
    pub nodes_fetched: usize,
    pub nodes_reused: usize,
    pub node_bytes: usize,
    pub entries_visited: usize,
}

/// A queued directory. It carries no ancestor list: content addressing rules out a link cycle (a
/// node cannot name its own address), and `max_depth` bounds any chain of links, so the frame
/// retains its CID, its path and its scope, and nothing proportional to depth beyond the path.
pub(crate) struct Frame<'v> {
    pub(crate) cid: BlobCid,
    pub(crate) path: TreePath,
    pub(crate) depth: usize,
    pub(crate) path_bytes: usize,
    /// What queueing this frame charged to `pending path bytes`, given back when it is visited.
    pub(crate) charge: usize,
    pub(crate) scope: Scope<'v>,
}

/// What a level walk charges for each directory it queues, beyond the path's bytes: the frame
/// itself, and one component header per path component (the path is a `Vec` of components). A
/// level is held whole in memory between rounds, so its frames are metered by their retained
/// size against `max_total_bytes` (`"pending path bytes"`), not by count (B13). Fixed numbers,
/// not `size_of`, so the TypeScript twin charges the same and both refuse alike;
/// `queued_frame_charge_covers_the_frame` pins them above the real sizes.
pub(crate) const QUEUED_FRAME_BYTES: usize = 192;
pub(crate) const QUEUED_COMPONENT_BYTES: usize = 24;

pub(crate) fn queued_frame_charge(path_bytes: usize, components: usize) -> usize {
    components
        .saturating_mul(QUEUED_COMPONENT_BYTES)
        .saturating_add(path_bytes)
        .saturating_add(QUEUED_FRAME_BYTES)
}

impl<'v> Frame<'v> {
    /// The root frame of a walk under `view`, or `None` for an empty view.
    pub(crate) fn root(root: &BlobCid, view: &'v View) -> Option<Self> {
        view.root_scope().map(|scope| Frame {
            cid: root.clone(),
            path: TreePath::root(),
            depth: 0,
            path_bytes: 0,
            charge: 0,
            scope,
        })
    }

    pub(crate) fn item(&self) -> BatchItem {
        BatchItem::new(self.cid.clone(), self.path.clone())
    }

    /// The view paths this frame carries.
    fn view_paths(&self) -> Vec<&'v TreePath> {
        match &self.scope {
            Scope::InView(covering) => vec![*covering],
            Scope::Spine(tails) => tails.clone(),
        }
    }

    /// The absences a frame whose node could not be fetched yields, one per view
    /// path it carries. A leaf store's miss (`Incomplete`) and a tier's
    /// `NotHeld` are observed absences; any other tier answer is `Withheld`.
    /// Every other error is still an error.
    pub(crate) fn absences(&self, error: TreeError) -> TreeResult<Vec<WalkItem>> {
        let unanswered = match error {
            TreeError::Incomplete { .. } => local_miss(),
            TreeError::Unanswered { why, .. } => *why,
            other => return Err(other),
        };
        Ok(self
            .view_paths()
            .into_iter()
            .map(|view_path| {
                WalkItem::unanswered(
                    view_path.clone(),
                    self.path.clone(),
                    self.cid.clone(),
                    unanswered.clone(),
                )
            })
            .collect())
    }
}

/// The three parts of a leaf store's own miss: no tier was asked, so only a
/// tier becoming reachable can change the answer. The same value
/// [`ItemAbsence::from_error`] gives a `BlobNotFound`.
pub(crate) fn local_miss() -> Unanswered<ItemAbsence> {
    ItemAbsence::not_held(Vec::new())
}

/// What visiting one fetched node yields: its walk item, the items that follow
/// it (boundaries, then absences below it), and the directory frames to visit.
pub(crate) struct Expanded<'v> {
    pub(crate) node: WalkedNode,
    pub(crate) trailing: Vec<WalkItem>,
    pub(crate) children: Vec<Frame<'v>>,
}

/// Match a fetched node against its frame's scope and charge its entries:
/// shared by the depth-first walk and the level-batched walk, so both apply
/// the same view matching and budget. `pending_base` is the depth-first
/// stack's length, whose count `max_pending_entries` bounds; the level walk
/// passes `None`, and each frame it queues is charged its retained size
/// ([`queued_frame_charge`]) against `max_total_bytes` instead (B13).
pub(crate) fn expand<'v>(
    budget: &mut Budget<'_>,
    frame: Frame<'v>,
    node: TreeNode,
    pending_base: Option<usize>,
) -> TreeResult<Expanded<'v>> {
    budget.charge_entries(node.entries.len())?;
    let classified = classify(&frame.scope, frame.depth, &node.entries);
    let mut trailing = Vec::new();
    let mut children = Vec::new();
    for (entry, scope) in node.entries.iter().zip(&classified.entries) {
        let Some(scope) = scope else { continue };
        let child_path_bytes = budget.child_path_bytes(frame.path_bytes, entry.name.len())?;
        let child_path = frame.path.join(&entry.name);
        match (&entry.kind, scope) {
            (TreeEntryKind::Directory { tree }, _) => {
                budget.check_child_depth(frame.depth)?;
                let charge = match pending_base {
                    Some(base) => {
                        budget.push_pending(base + children.len(), child_path_bytes)?;
                        child_path_bytes
                    }
                    None => {
                        let charge = queued_frame_charge(child_path_bytes, frame.depth + 1);
                        budget.push_pending_bytes(charge)?;
                        charge
                    }
                };
                children.push(Frame {
                    cid: tree.as_blob_cid().clone(),
                    path: child_path,
                    depth: frame.depth + 1,
                    path_bytes: child_path_bytes,
                    charge,
                    scope: scope.clone(),
                });
            }
            (TreeEntryKind::External { kind, reference }, scope) => {
                budget.charge_external_path(child_path_bytes)?;
                trailing.push(WalkItem::boundary(
                    child_path,
                    kind.clone(),
                    reference.clone(),
                    scope.coverage(),
                ));
            }
            (TreeEntryKind::File { .. } | TreeEntryKind::Symlink { .. }, Scope::InView(_)) => {}
            (TreeEntryKind::File { .. } | TreeEntryKind::Symlink { .. }, Scope::Spine(tails)) => {
                for view_path in tails {
                    trailing.push(WalkItem::through_leaf(
                        (*view_path).clone(),
                        child_path.clone(),
                    ));
                }
            }
        }
    }
    for view_path in &classified.unmatched {
        trailing.push(WalkItem::not_found(
            (*view_path).clone(),
            frame.path.join(&view_path.components()[frame.depth]),
        ));
    }
    let entry_coverage = classified.entries.iter().map(coverage_of).collect();
    Ok(Expanded {
        node: WalkedNode::new(
            frame.path,
            frame.cid,
            node,
            frame.scope.coverage(),
            entry_coverage,
        ),
        trailing,
        children,
    })
}

/// A pull cursor over a view-scoped walk: depth-first pre-order in byte-sorted
/// name order. Memory is bounded by the frontier plus the nodes already
/// visited, which a shared subtree reuses instead of refetching. It fetches
/// tree nodes only, never a leaf. Items the caller keeps are the caller's to
/// bound; [`walk_collect`] keeps them all and meters their paths.
#[must_use]
pub struct TreeWalk<'a, S: ?Sized> {
    storage: &'a S,
    root: BlobCid,
    opts: BatchOptions,
    budget: Budget<'a>,
    stack: Vec<Frame<'a>>,
    out: VecDeque<WalkItem>,
    cache: HashMap<BlobCid, (TreeNode, usize)>,
    stats: WalkStats,
    done: bool,
}

/// Start a walk of `root` under `view`. Refuses a non-DAG-CBOR root and any
/// view path longer than `max_path_bytes` before any fetch.
pub fn walk<'a, S: EprfsStorage + ?Sized>(
    storage: &'a S,
    root: &BlobCid,
    view: &'a View,
    opts: &BatchOptions,
    limits: &'a TreeLimits,
) -> TreeResult<TreeWalk<'a, S>> {
    check_root_codec(root)?;
    view.check_path_bytes(limits)?;
    Ok(TreeWalk {
        storage,
        root: root.clone(),
        opts: opts.clone(),
        budget: Budget::new(limits),
        stack: Frame::root(root, view).into_iter().collect(),
        out: VecDeque::new(),
        cache: HashMap::new(),
        stats: WalkStats::default(),
        done: false,
    })
}

impl<'a, S: EprfsStorage + ?Sized> TreeWalk<'a, S> {
    /// The next item, `None` when the walk is finished. After an error the
    /// cursor returns `None`.
    pub async fn next(&mut self) -> Option<TreeResult<WalkItem>> {
        loop {
            if let Some(item) = self.out.pop_front() {
                return Some(Ok(item));
            }
            if self.done {
                return None;
            }
            let Some(frame) = self.stack.pop() else {
                self.done = true;
                return None;
            };
            if let Err(error) = self.visit(frame).await {
                self.done = true;
                self.out.clear();
                self.stack.clear();
                return Some(Err(error));
            }
        }
    }

    pub fn stats(&self) -> &WalkStats {
        &self.stats
    }

    async fn visit(&mut self, frame: Frame<'a>) -> TreeResult<()> {
        self.budget.pop_pending(frame.charge);
        self.budget.check_depth(frame.depth)?;
        let node = if let Some((node, len)) = self.cache.get(&frame.cid) {
            self.budget.charge_reused(*len)?;
            self.stats.nodes_reused += 1;
            node.clone()
        } else {
            match fetch_node(
                self.storage,
                &self.root,
                &frame.item(),
                &self.opts,
                &mut self.budget,
            )
            .await
            {
                Ok((node, len)) => {
                    self.stats.nodes_fetched += 1;
                    self.stats.node_bytes += len;
                    self.cache.insert(frame.cid.clone(), (node.clone(), len));
                    node
                }
                Err(error) => {
                    let absences = frame.absences(error)?;
                    self.out.extend(absences);
                    return Ok(());
                }
            }
        };
        self.stats.entries_visited += node.entries.len();
        let expanded = expand(&mut self.budget, frame, node, Some(self.stack.len()))?;
        self.out.push_back(WalkItem::Node(expanded.node));
        self.out.extend(expanded.trailing);
        self.stack.extend(expanded.children.into_iter().rev());
        Ok(())
    }
}

/// A finished walk, gathered.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct WalkedView {
    pub root: BlobCid,
    pub nodes: Vec<WalkedNode>,
    /// `WalkItem::Boundary` items, in walk order.
    pub boundaries: Vec<WalkItem>,
    /// `WalkItem::Absent` items, in walk order.
    pub absent: Vec<WalkItem>,
    pub stats: WalkStats,
}

impl WalkedView {
    /// Every file and symlink in view: what a sparse materializer would fetch.
    pub fn leaves_in_view(&self) -> impl Iterator<Item = (TreePath, &TreeEntryKind)> {
        self.nodes.iter().flat_map(|walked| {
            walked
                .node
                .entries
                .iter()
                .zip(&walked.entry_coverage)
                .filter(|(entry, coverage)| {
                    **coverage == Coverage::InView
                        && matches!(
                            entry.kind,
                            TreeEntryKind::File { .. } | TreeEntryKind::Symlink { .. }
                        )
                })
                .map(|(entry, _)| (walked.path.join(&entry.name), &entry.kind))
        })
    }
}

/// Run a walk to the end and keep every item. What each kept item retains is
/// charged against `max_total_bytes` (`"result path bytes"`) as it is kept:
/// a node's path, and an absence's paths and the strings its unanswered parts
/// carry ([`WalkItem::retained_bytes`]). A shared subtree reached under many
/// long paths costs its node bytes once per path, but each path it, or one of
/// its missing children, is kept under is new memory. A caller with a tree too
/// large to keep uses the [`TreeWalk`] cursor instead.
pub async fn walk_collect<S: EprfsStorage + ?Sized>(
    storage: &S,
    root: &BlobCid,
    view: &View,
    opts: &BatchOptions,
    limits: &TreeLimits,
) -> TreeResult<WalkedView> {
    let mut cursor = walk(storage, root, view, opts, limits)?;
    let mut retained = Budget::new(limits);
    let mut walked = WalkedView {
        root: root.clone(),
        nodes: Vec::new(),
        boundaries: Vec::new(),
        absent: Vec::new(),
        stats: WalkStats::default(),
    };
    while let Some(item) = cursor.next().await {
        match item? {
            WalkItem::Node(node) => {
                retained.charge_result(node.path.byte_len())?;
                walked.nodes.push(node);
            }
            boundary @ WalkItem::Boundary { .. } => walked.boundaries.push(boundary),
            absent @ WalkItem::Absent { .. } => {
                retained.charge_result(absent.retained_bytes())?;
                walked.absent.push(absent);
            }
        }
    }
    walked.stats = cursor.stats().clone();
    Ok(walked)
}

/// A single answer cannot be partial: a node or leaf no tier gave is
/// [`TreeError::Unanswered`] at the path where it is missing, with ruling 7's
/// three parts (B1). A leaf store's miss becomes the same error naming no tier.
fn at_path(error: TreeError, path: &TreePath) -> TreeError {
    match error {
        TreeError::Incomplete { cid, .. } => TreeError::Unanswered {
            cid,
            path: path.clone(),
            why: Box::new(local_miss()),
        },
        TreeError::Unanswered { cid, why, .. } => TreeError::Unanswered {
            cid,
            path: path.clone(),
            why,
        },
        other => other,
    }
}

async fn node_at<S: EprfsStorage + ?Sized>(
    storage: &S,
    root: &BlobCid,
    cid: &BlobCid,
    path: &TreePath,
    depth: usize,
    opts: &BatchOptions,
    budget: &mut Budget<'_>,
) -> TreeResult<TreeNode> {
    budget.check_depth(depth)?;
    let item = BatchItem::new(cid.clone(), path.clone());
    let (node, _) = fetch_node(storage, root, &item, opts, budget)
        .await
        .map_err(|error| at_path(error, path))?;
    budget.charge_entries(node.entries.len())?;
    Ok(node)
}

pub(crate) enum Resolved {
    Root,
    Entry(TreeEntry),
}

/// The one single-path step every resolver shares, for an entry that is not the path's last
/// component: a directory is descended into; a file or symlink is `NotADirectory`, and an
/// external entry is `ExternalBoundary`, both at `here`, the entry's own path.
pub(crate) fn descend(entry: &TreeEntry, here: TreePath) -> TreeResult<BlobCid> {
    match &entry.kind {
        TreeEntryKind::Directory { tree } => Ok(tree.as_blob_cid().clone()),
        TreeEntryKind::File { .. } | TreeEntryKind::Symlink { .. } => {
            Err(TreeError::NotADirectory { path: here })
        }
        TreeEntryKind::External { kind, .. } => Err(TreeError::ExternalBoundary {
            path: here,
            kind: kind.clone(),
        }),
    }
}

/// The link at a path's last entry: its object (a directory's tree included), or
/// `ExternalBoundary` for an external entry, which names no object here.
pub(crate) fn link_of(entry: &TreeEntry, here: TreePath) -> TreeResult<BlobCid> {
    match &entry.kind {
        TreeEntryKind::Directory { tree } => Ok(tree.as_blob_cid().clone()),
        TreeEntryKind::File { blob, .. } => Ok(blob.as_blob_cid().clone()),
        TreeEntryKind::Symlink { target } => Ok(target.as_blob_cid().clone()),
        TreeEntryKind::External { kind, .. } => Err(TreeError::ExternalBoundary {
            path: here,
            kind: kind.clone(),
        }),
    }
}

/// Walk the single spine to `path`, verifying each node, without descending
/// into the target itself.
pub(crate) async fn resolve_entry<S: EprfsStorage + ?Sized>(
    storage: &S,
    root: &BlobCid,
    path: &TreePath,
    opts: &BatchOptions,
    budget: &mut Budget<'_>,
) -> TreeResult<Resolved> {
    check_root_codec(root)?;
    if path.byte_len() > budget.limits().max_path_bytes {
        return Err(TreeError::Budget("path bytes"));
    }
    let Some((last, parents)) = path.components().split_last() else {
        return Ok(Resolved::Root);
    };
    let mut cid = root.clone();
    let mut here = TreePath::root();
    for (depth, name) in parents.iter().enumerate() {
        let node = node_at(storage, root, &cid, &here, depth, opts, budget).await?;
        here = here.join(name);
        let entry =
            find(&node, name).ok_or_else(|| TreeError::PathNotFound { path: path.clone() })?;
        let child = descend(entry, here.clone())?;
        budget.check_child_depth(depth)?;
        cid = child;
    }
    let node = node_at(storage, root, &cid, &here, parents.len(), opts, budget).await?;
    let entry = find(&node, last).ok_or_else(|| TreeError::PathNotFound { path: path.clone() })?;
    Ok(Resolved::Entry(entry.clone()))
}

pub(crate) fn find<'n>(node: &'n TreeNode, name: &[u8]) -> Option<&'n TreeEntry> {
    node.entries
        .binary_search_by(|entry| entry.name.as_slice().cmp(name))
        .ok()
        .map(|index| &node.entries[index])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", rename_all_fields = "camelCase")]
pub enum LeafKind {
    File { executable: bool },
    Symlink,
}

/// Serializable so a tier answers it over a transport (the device peer's tier ALPN).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ReadLeaf {
    pub path: TreePath,
    pub cid: BlobCid,
    pub kind: LeafKind,
    pub bytes: Bytes,
}

impl ReadLeaf {
    /// For a tier answering a read on the server side (B5).
    pub fn new(path: TreePath, cid: BlobCid, kind: LeafKind, bytes: Bytes) -> Self {
        Self {
            path,
            cid,
            kind,
            bytes,
        }
    }
}

/// Read one file or symlink: the spine nodes to it plus its one verified leaf.
pub async fn read_path<S: EprfsStorage + ?Sized>(
    storage: &S,
    root: &BlobCid,
    path: &TreePath,
    opts: &BatchOptions,
    limits: &TreeLimits,
) -> TreeResult<ReadLeaf> {
    let mut budget = Budget::new(limits);
    let entry = match resolve_entry(storage, root, path, opts, &mut budget).await? {
        Resolved::Root => return Err(TreeError::NotALeaf { path: path.clone() }),
        Resolved::Entry(entry) => entry,
    };
    let (cid, kind): (BlobCid, LeafKind) = match entry.kind {
        TreeEntryKind::File { blob, executable } => (blob.into(), LeafKind::File { executable }),
        TreeEntryKind::Symlink { target } => (target.into(), LeafKind::Symlink),
        TreeEntryKind::Directory { .. } => return Err(TreeError::NotALeaf { path: path.clone() }),
        TreeEntryKind::External { kind, .. } => {
            return Err(TreeError::ExternalBoundary {
                path: path.clone(),
                kind,
            })
        }
    };
    let item = BatchItem::new(cid.clone(), path.clone());
    let bytes = fetch_checked(storage, root, &item, opts)
        .await
        .map_err(|error| at_path(error, path))?;
    budget.charge_fetched(bytes.len())?;
    Ok(ReadLeaf {
        path: path.clone(),
        cid,
        kind,
        bytes,
    })
}

/// Serializable so a tier answers it over a transport (the device peer's tier ALPN).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct DirListing {
    pub path: TreePath,
    pub cid: BlobCid,
    /// The directory's entries; no child is fetched.
    pub entries: Vec<TreeEntry>,
}

impl DirListing {
    /// For a tier answering a listing on the server side (B5).
    pub fn new(path: TreePath, cid: BlobCid, entries: Vec<TreeEntry>) -> Self {
        Self { path, cid, entries }
    }
}

/// List one directory: the spine nodes to it plus the directory node itself.
pub async fn list_dir<S: EprfsStorage + ?Sized>(
    storage: &S,
    root: &BlobCid,
    path: &TreePath,
    opts: &BatchOptions,
    limits: &TreeLimits,
) -> TreeResult<DirListing> {
    let mut budget = Budget::new(limits);
    let cid = match resolve_entry(storage, root, path, opts, &mut budget).await? {
        Resolved::Root => root.clone(),
        Resolved::Entry(entry) => match entry.kind {
            TreeEntryKind::Directory { tree } => tree.into(),
            TreeEntryKind::File { .. } | TreeEntryKind::Symlink { .. } => {
                return Err(TreeError::NotADirectory { path: path.clone() })
            }
            TreeEntryKind::External { kind, .. } => {
                return Err(TreeError::ExternalBoundary {
                    path: path.clone(),
                    kind,
                })
            }
        },
    };
    let node = node_at(
        storage,
        root,
        &cid,
        path,
        path.components().len(),
        opts,
        &mut budget,
    )
    .await?;
    Ok(DirListing {
        path: path.clone(),
        cid,
        entries: node.entries,
    })
}

/// One difference between two roots. `None/Some` is added, `Some/None` is
/// removed, `Some/Some` is a changed kind, executable bit, leaf or reference.
/// Serializable so a tier answers it over a transport (the device peer's tier ALPN).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct TreeChange {
    pub path: TreePath,
    pub old: Option<TreeEntryKind>,
    pub new: Option<TreeEntryKind>,
}

impl TreeChange {
    /// For a tier answering a diff on the server side (B5).
    pub fn new(path: TreePath, old: Option<TreeEntryKind>, new: Option<TreeEntryKind>) -> Self {
        Self { path, old, new }
    }
}

/// One directory pair to compare. `None` on a side is an empty directory:
/// the path has no directory there, but the other side's directory still
/// carries view tails below it.
struct DiffFrame<'v> {
    old: Option<BlobCid>,
    new: Option<BlobCid>,
    path: TreePath,
    depth: usize,
    ancestors: Vec<(Option<BlobCid>, Option<BlobCid>)>,
    path_bytes: usize,
    scope: Scope<'v>,
}

/// Diff two roots under a view as one co-walk. Two directories with the same
/// CID are equal and never fetched, so the cost follows the change. A
/// directory added or removed in view is reported once at its path and not
/// expanded; a directory changed on both sides is descended, never reported
/// itself. A Spine entry (an ancestor of a view path, not itself in view) is
/// not reported when it is a file or symlink: such a side is read as an empty
/// directory, and only the changes at the view paths below are reported. An
/// External entry on the spine is the boundary the walk reports there, so a
/// change with an External on either side is reported once at that path and
/// not descended. Every change kept is charged its path and any External
/// bytes against `max_total_bytes` (`"result path bytes"`). Output is
/// pre-order in byte-sorted name order.
pub async fn diff_trees<S: EprfsStorage + ?Sized>(
    storage: &S,
    old: &BlobCid,
    new: &BlobCid,
    view: &View,
    opts: &BatchOptions,
    limits: &TreeLimits,
) -> TreeResult<Vec<TreeChange>> {
    check_root_codec(old)?;
    check_root_codec(new)?;
    view.check_path_bytes(limits)?;
    let Some(scope) = view.root_scope() else {
        return Ok(Vec::new());
    };
    if old == new {
        return Ok(Vec::new());
    }
    let mut budget = Budget::new(limits);
    let mut changes = Vec::new();
    let mut stack = vec![DiffFrame {
        old: Some(old.clone()),
        new: Some(new.clone()),
        path: TreePath::root(),
        depth: 0,
        ancestors: Vec::new(),
        path_bytes: 0,
        scope,
    }];
    while let Some(frame) = stack.pop() {
        budget.pop_pending(frame.path_bytes);
        let pair = (frame.old.clone(), frame.new.clone());
        if frame.ancestors.contains(&pair) {
            return Err(TreeError::Invalid("directory link cycle".into()));
        }
        // Each side's nodes are fetched under that side's own root (B6), so a
        // serving tier judges reach and membership per side.
        let old_entries = diff_side(
            storage,
            old,
            frame.old.as_ref(),
            &frame.path,
            frame.depth,
            opts,
            &mut budget,
        )
        .await?;
        let new_entries = diff_side(
            storage,
            new,
            frame.new.as_ref(),
            &frame.path,
            frame.depth,
            opts,
            &mut budget,
        )
        .await?;
        let mut lineage = frame.ancestors;
        lineage.push(pair);
        for (name, old_kind, new_kind) in merge_entries(&old_entries, &new_entries) {
            let Some(scope) = scope_for(&frame.scope, frame.depth, name) else {
                continue;
            };
            if old_kind == new_kind {
                continue;
            }
            let child_path_bytes = budget.child_path_bytes(frame.path_bytes, name.len())?;
            let child_path = frame.path.join(name);
            let (old_dir, new_dir) = (dir_cid(old_kind), dir_cid(new_kind));
            let boundary = is_external(old_kind) || is_external(new_kind);
            let (descend, report) = match scope {
                // An External on the spine is the boundary the walk reports:
                // the view path stops there, so the change is reported there.
                Scope::Spine(_) if boundary => (false, true),
                // Otherwise an ancestor of a view path is never itself a
                // change: a file or symlink on a side holds nothing under the
                // view there.
                Scope::Spine(_) => (old_dir.is_some() || new_dir.is_some(), false),
                Scope::InView(_) => {
                    let both = old_dir.is_some() && new_dir.is_some();
                    (both, !both)
                }
            };
            if descend {
                budget.check_child_depth(frame.depth)?;
                budget.push_pending(stack.len(), child_path_bytes)?;
                stack.push(DiffFrame {
                    old: old_dir,
                    new: new_dir,
                    path: child_path,
                    depth: frame.depth + 1,
                    ancestors: lineage.clone(),
                    path_bytes: child_path_bytes,
                    scope,
                });
            } else if report {
                budget.charge_result(
                    child_path_bytes + external_bytes(old_kind) + external_bytes(new_kind),
                )?;
                changes.push(TreeChange {
                    path: child_path,
                    old: old_kind.cloned(),
                    new: new_kind.cloned(),
                });
            }
        }
    }
    // Component-wise order on paths is pre-order in byte-sorted name order.
    changes.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(changes)
}

/// One side of a diff frame: the verified directory's entries, or none for
/// a side with no directory at this path (never fetched).
async fn diff_side<S: EprfsStorage + ?Sized>(
    storage: &S,
    side_root: &BlobCid,
    cid: Option<&BlobCid>,
    path: &TreePath,
    depth: usize,
    opts: &BatchOptions,
    budget: &mut Budget<'_>,
) -> TreeResult<Vec<TreeEntry>> {
    match cid {
        Some(cid) => Ok(node_at(storage, side_root, cid, path, depth, opts, budget)
            .await?
            .entries),
        None => {
            budget.check_depth(depth)?;
            Ok(Vec::new())
        }
    }
}

fn is_external(kind: Option<&TreeEntryKind>) -> bool {
    matches!(kind, Some(TreeEntryKind::External { .. }))
}

/// The bytes a kept External kind clones out of its node.
fn external_bytes(kind: Option<&TreeEntryKind>) -> usize {
    match kind {
        Some(TreeEntryKind::External { kind, reference }) => kind.len() + reference.len(),
        _ => 0,
    }
}

fn dir_cid(kind: Option<&TreeEntryKind>) -> Option<BlobCid> {
    match kind {
        Some(TreeEntryKind::Directory { tree }) => Some(tree.as_blob_cid().clone()),
        _ => None,
    }
}

/// The scope of one child name under a node, or `None` when it is Outside.
fn scope_for<'v>(scope: &Scope<'v>, depth: usize, name: &[u8]) -> Option<Scope<'v>> {
    match scope {
        Scope::InView(covering) => Some(Scope::InView(covering)),
        Scope::Spine(tails) => {
            let group: Vec<&'v TreePath> = tails
                .iter()
                .copied()
                .filter(|tail| tail.components()[depth] == name)
                .collect();
            group_scope(&group, depth)
        }
    }
}

type Merged<'n> = (
    &'n [u8],
    Option<&'n TreeEntryKind>,
    Option<&'n TreeEntryKind>,
);

/// Merge two byte-sorted entry lists by name.
fn merge_entries<'n>(old: &'n [TreeEntry], new: &'n [TreeEntry]) -> Vec<Merged<'n>> {
    let mut merged = Vec::with_capacity(old.len().max(new.len()));
    let (mut i, mut j) = (0, 0);
    while i < old.len() || j < new.len() {
        match (old.get(i), new.get(j)) {
            (Some(a), Some(b)) if a.name == b.name => {
                merged.push((a.name.as_slice(), Some(&a.kind), Some(&b.kind)));
                i += 1;
                j += 1;
            }
            (Some(a), Some(b)) if a.name < b.name => {
                merged.push((a.name.as_slice(), Some(&a.kind), None));
                i += 1;
            }
            (Some(_), Some(b)) | (None, Some(b)) => {
                merged.push((b.name.as_slice(), None, Some(&b.kind)));
                j += 1;
            }
            (Some(a), None) => {
                merged.push((a.name.as_slice(), Some(&a.kind), None));
                i += 1;
            }
            (None, None) => unreachable!(),
        }
    }
    merged
}

/// How much of a view a device holds (spec §12.2). No `Ord`: depth does not
/// order offline capability (B14).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Depth {
    /// Nothing held: every read is a query answered elsewhere.
    Query,
    /// The whole view's bytes held, offline.
    Sparse,
    /// Only what was read held.
    OnRead,
    /// Kept by a residency pin; read from the pin, never written by eprfs.
    Kept,
}

impl Depth {
    /// Spec §12.2's table: `Kept` satisfies every requirement; `Sparse` (the
    /// whole view offline) and `OnRead` (only what was read) are not comparable
    /// to each other; each satisfies itself and `Query`.
    pub fn satisfies_offline(self, required: Depth) -> bool {
        match (self, required) {
            (Depth::Kept, _) => true,
            (_, Depth::Query) => true,
            (held, required) => held == required,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{
        dir, external, file, ten_thousand, ten_thousand_content, tp, CountingStorage,
    };
    use crate::{decode_tree, load_verified_tree};
    use std::collections::HashSet;

    use crate::FetchPolicy;

    const LOCAL: FetchPolicy = FetchPolicy::LocalOnly;

    fn limits() -> TreeLimits {
        TreeLimits::default()
    }

    fn three_path_view() -> View {
        View::from_tree_paths([tp("a1/b2/f003"), tp("a4/b4/f099"), tp("a9/b0")])
    }

    const THREE_PATH_SPINE: [&str; 7] = ["", "a1", "a1/b2", "a4", "a4/b4", "a9", "a9/b0"];

    fn dagcbor_round_trip<T: Serialize + for<'de> Deserialize<'de>>(value: &T) -> T {
        let bytes = serde_ipld_dagcbor::to_vec(value).unwrap();
        serde_ipld_dagcbor::from_slice(&bytes).unwrap()
    }

    #[test]
    fn served_answers_round_trip_as_dag_cbor() {
        let leaf_cid = BlobCid::compute_raw(b"\xffbytes");
        let leaf = ReadLeaf::new(
            TreePath::from_components(vec![b"a".to_vec(), vec![0xff]]).unwrap(),
            leaf_cid.clone(),
            LeafKind::File { executable: true },
            Bytes::from_static(b"\xffbytes"),
        );
        assert_eq!(dagcbor_round_trip(&leaf), leaf, "non-UTF-8 names survive");
        let listing = DirListing::new(TreePath::root(), leaf_cid.clone(), vec![]);
        assert_eq!(dagcbor_round_trip(&listing), listing);
        let change = TreeChange::new(
            tp("a/b"),
            None,
            Some(TreeEntryKind::File {
                blob: leaf_cid.into(),
                executable: false,
            }),
        );
        assert_eq!(dagcbor_round_trip(&change), change);
    }

    #[test]
    fn a_view_from_the_wire_is_renormalized() {
        let sent = vec![tp("a/b"), tp("a"), tp("a"), tp("c")];
        let bytes = serde_ipld_dagcbor::to_vec(&sent).unwrap();
        let view: View = serde_ipld_dagcbor::from_slice(&bytes).unwrap();
        assert_eq!(view, View::from_tree_paths([tp("a"), tp("c")]));
        assert_eq!(dagcbor_round_trip(&view), view);
    }

    fn cids(nodes: &HashMap<TreePath, BlobCid>, paths: &[&str]) -> Vec<BlobCid> {
        paths.iter().map(|p| nodes[&tp(p)].clone()).collect()
    }

    fn paths(walked: &WalkedView) -> Vec<String> {
        walked.nodes.iter().map(|n| n.path.to_string()).collect()
    }

    #[tokio::test]
    async fn walk_ten_thousand_entries_three_path_view_fetches_only_spine() {
        let mut store = CountingStorage::new();
        let (root, nodes) = ten_thousand(&mut store, true);
        let view = three_path_view();
        let walked = walk_collect(&store, &root, &view, &LOCAL.into(), &limits())
            .await
            .unwrap();

        // Exactly the seven spine and in-view nodes, each once, in pre-order; no leaf.
        assert_eq!(store.fetched(), cids(&nodes, &THREE_PATH_SPINE));
        assert_eq!(
            walked
                .nodes
                .iter()
                .map(|n| n.path.clone())
                .collect::<Vec<_>>(),
            THREE_PATH_SPINE.map(tp).to_vec()
        );
        let coverages: Vec<Coverage> = walked.nodes.iter().map(|n| n.coverage).collect();
        let mut expected = vec![Coverage::Spine; 6];
        expected.push(Coverage::InView);
        assert_eq!(coverages, expected);
        assert_eq!(walked.stats.nodes_fetched, 7);
        assert_eq!(walked.stats.nodes_reused, 0);
        assert_eq!(walked.stats.entries_visited, 340);
        assert!(walked.absent.is_empty());
        assert!(walked.boundaries.is_empty());

        let b2 = &walked.nodes[2];
        assert_eq!(b2.entry_coverage.len(), 100);
        assert_eq!(
            b2.entry_coverage
                .iter()
                .filter(|p| **p == Coverage::InView)
                .count(),
            1
        );
        assert_eq!(b2.entry_coverage[3], Coverage::InView);
        assert_eq!(walked.leaves_in_view().count(), 102);

        // The contrast: the closure loader fetches every node and every leaf.
        store.reset();
        load_verified_tree(&store, &root, LOCAL, &limits())
            .await
            .unwrap();
        assert_eq!(store.fetched().len(), 10_111);
    }

    #[tokio::test]
    async fn walk_over_store_holding_only_the_spine_succeeds_local_only() {
        let mut full = CountingStorage::new();
        let (root, nodes) = ten_thousand(&mut full, false);
        let keep: HashSet<BlobCid> = cids(&nodes, &THREE_PATH_SPINE).into_iter().collect();
        let mut sparse = CountingStorage::new();
        sparse.blobs = full
            .blobs
            .into_iter()
            .filter(|(cid, _)| keep.contains(cid))
            .collect();
        assert_eq!(sparse.blobs.len(), 7);
        let view = three_path_view();
        let walked = walk_collect(&sparse, &root, &view, &LOCAL.into(), &limits())
            .await
            .unwrap();
        assert_eq!(walked.nodes.len(), 7);
        assert!(walked.absent.is_empty());
    }

    #[tokio::test]
    async fn view_prefix_is_component_wise() {
        let view = View::from_tree_paths([tp("src")]);
        assert_eq!(view.coverage(&tp("src")), Coverage::InView);
        assert_eq!(view.coverage(&tp("src/x")), Coverage::InView);
        assert_eq!(view.coverage(&tp("src2")), Coverage::Outside);
        assert_eq!(view.coverage(&TreePath::root()), Coverage::Spine);

        let mut store = CountingStorage::new();
        let src = store.put_tree(vec![file(b"x", b"x")]);
        let src2 = store.put_tree(vec![file(b"y", b"y")]);
        let root = store.put_tree(vec![dir(b"src", src.clone()), dir(b"src2", src2)]);
        let walked = walk_collect(&store, &root, &view, &LOCAL.into(), &limits())
            .await
            .unwrap();
        assert_eq!(store.fetched(), vec![root, src]);
        assert_eq!(
            walked.nodes[0].entry_coverage,
            vec![Coverage::InView, Coverage::Outside]
        );
    }

    #[test]
    fn view_normalizes_duplicates_and_covered_paths() {
        let view = View::from_tree_paths([tp("a/b"), tp("a"), tp("a"), tp("c/d"), tp("ab")]);
        assert_eq!(
            view.paths().cloned().collect::<Vec<_>>(),
            vec![tp("a"), tp("ab"), tp("c/d")]
        );
        assert_eq!(
            View::from_tree_paths([tp("x/y"), TreePath::root()]),
            View::whole()
        );
    }

    #[test]
    fn view_from_projection_paths_and_whole_view() {
        let view = View::new([
            ProjectionPath::new("a/b").unwrap(),
            ProjectionPath::new("c").unwrap(),
        ])
        .unwrap();
        assert_eq!(
            view.paths().cloned().collect::<Vec<_>>(),
            vec![tp("a/b"), tp("c")]
        );
        assert_eq!(View::whole().coverage(&tp("any/thing")), Coverage::InView);
        assert_eq!(View::whole().coverage(&TreePath::root()), Coverage::InView);
        assert!(View::from_tree_paths([]).is_empty());
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            let raw = ProjectionPath::new(std::ffi::OsStr::from_bytes(b"\xff/x")).unwrap();
            assert_eq!(
                TreePath::try_from(&raw).unwrap().components(),
                &[vec![0xff], b"x".to_vec()]
            );
        }
        assert!(matches!(
            TreePath::from_slash_bytes(b"a//b"),
            Err(TreeError::InvalidView(_))
        ));
        assert!(matches!(
            TreePath::from_components(vec![b"..".to_vec()]),
            Err(TreeError::InvalidView(_))
        ));
    }

    #[tokio::test]
    async fn empty_view_fetches_nothing() {
        let mut store = CountingStorage::new();
        let (root, _) = ten_thousand(&mut store, false);
        let view = View::from_tree_paths([]);
        let walked = walk_collect(&store, &root, &view, &LOCAL.into(), &limits())
            .await
            .unwrap();
        assert!(walked.nodes.is_empty() && walked.absent.is_empty());
        assert!(store.fetched().is_empty());
        // The root codec is still checked, with no fetch.
        assert!(matches!(
            walk(
                &store,
                &BlobCid::compute_raw(b"x"),
                &view,
                &LOCAL.into(),
                &limits()
            ),
            Err(TreeError::Invalid(_))
        ));
        assert!(store.fetched().is_empty());
    }

    #[tokio::test]
    async fn walk_refuses_oversized_view_path_before_any_fetch() {
        let mut store = CountingStorage::new();
        let (root, _) = ten_thousand(&mut store, false);
        let view = three_path_view();
        let tight = TreeLimits {
            max_path_bytes: 7,
            ..limits()
        };
        assert!(matches!(
            walk(&store, &root, &view, &LOCAL.into(), &tight),
            Err(TreeError::Budget("path bytes"))
        ));
        assert!(store.fetched().is_empty());
    }

    #[tokio::test]
    async fn walk_depth_budget_applies_to_spine() {
        let mut store = CountingStorage::new();
        let (root, _) = ten_thousand(&mut store, false);
        let view = View::from_tree_paths([tp("a1/b2/f003")]);
        let shallow = TreeLimits {
            max_depth: 1,
            ..limits()
        };
        assert!(matches!(
            walk_collect(&store, &root, &view, &LOCAL.into(), &shallow).await,
            Err(TreeError::Budget("tree depth"))
        ));
        let deep_enough = TreeLimits {
            max_depth: 2,
            ..limits()
        };
        walk_collect(&store, &root, &view, &LOCAL.into(), &deep_enough)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn walk_entry_budget_counts_only_visited_nodes() {
        let mut store = CountingStorage::new();
        let (root, _) = ten_thousand(&mut store, true);
        let view = three_path_view();
        let at = |n| TreeLimits {
            max_total_entries: n,
            ..limits()
        };
        walk_collect(&store, &root, &view, &LOCAL.into(), &at(340))
            .await
            .unwrap();
        assert!(matches!(
            walk_collect(&store, &root, &view, &LOCAL.into(), &at(339)).await,
            Err(TreeError::Budget("total entries"))
        ));
        assert!(matches!(
            load_verified_tree(&store, &root, LOCAL, &at(340)).await,
            Err(TreeError::Budget("total entries"))
        ));
    }

    #[tokio::test]
    async fn walk_verifies_each_node_against_parent_link() {
        let mut store = CountingStorage::new();
        let (root, nodes) = ten_thousand(&mut store, false);
        let b2 = nodes[&tp("a1/b2")].clone();
        let other = store.blobs[&nodes[&tp("a1/b3")]].clone();
        store.tamper(&b2, other);
        let view = three_path_view();
        match walk_collect(&store, &root, &view, &LOCAL.into(), &limits()).await {
            Err(TreeError::Integrity(cid)) => assert_eq!(cid, b2),
            other => panic!("expected integrity failure, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn walk_decode_failure_is_an_error_not_an_absence() {
        let mut store = CountingStorage::new();
        // Bytes that verify against their own DAG-CBOR CID but are not a tree.
        let garbage = Bytes::from_static(&[0xff]);
        let garbage_cid = BlobCid::compute(&garbage);
        store.blobs.insert(garbage_cid.clone(), garbage);
        let root = store.put_tree(vec![dir(b"g", garbage_cid)]);
        assert!(matches!(
            walk_collect(&store, &root, &View::whole(), &LOCAL.into(), &limits()).await,
            Err(TreeError::Invalid(_))
        ));
    }

    #[tokio::test]
    async fn walk_emits_external_boundary_and_does_not_follow() {
        let mut store = CountingStorage::new();
        let src = store.put_tree(vec![file(b"x", b"x")]);
        let root = store.put_tree(vec![dir(b"src", src), external(b"vendor")]);
        let boundary = |coverage| WalkItem::Boundary {
            path: tp("vendor"),
            kind: "opaque".into(),
            reference: vec![0, 0xff],
            coverage,
        };

        let named = View::from_tree_paths([tp("vendor")]);
        let walked = walk_collect(&store, &root, &named, &LOCAL.into(), &limits())
            .await
            .unwrap();
        assert_eq!(walked.boundaries, vec![boundary(Coverage::InView)]);
        assert_eq!(store.fetched(), vec![root.clone()]);

        store.reset();
        let through = View::from_tree_paths([tp("vendor/x")]);
        let walked = walk_collect(&store, &root, &through, &LOCAL.into(), &limits())
            .await
            .unwrap();
        assert_eq!(walked.boundaries, vec![boundary(Coverage::Spine)]);
        assert!(walked.absent.is_empty());
        assert_eq!(store.fetched(), vec![root]);
    }

    #[tokio::test]
    async fn walk_reports_absent_view_path_by_name() {
        let mut store = CountingStorage::new();
        let (root, nodes) = ten_thousand(&mut store, false);
        let view = View::from_tree_paths([tp("a1/nope"), tp("a1/b2/zzz"), tp("zz")]);
        let walked = walk_collect(&store, &root, &view, &LOCAL.into(), &limits())
            .await
            .unwrap();
        let not_found = |view_path: &str, missing_at: &str| WalkItem::Absent {
            view_path: tp(view_path),
            reason: AbsentReason::NotFound {
                missing_at: tp(missing_at),
            },
            unanswered: None,
        };
        assert_eq!(
            walked.absent,
            vec![
                not_found("zz", "zz"),
                not_found("a1/nope", "a1/nope"),
                not_found("a1/b2/zzz", "a1/b2/zzz"),
            ]
        );
        assert_eq!(store.fetched(), cids(&nodes, &["", "a1", "a1/b2"]));
    }

    #[tokio::test]
    async fn walk_view_through_a_file_is_absent_through_leaf() {
        let mut store = CountingStorage::new();
        let (root, _) = ten_thousand(&mut store, true);
        let view = View::from_tree_paths([tp("a1/b2/f003/x")]);
        let walked = walk_collect(&store, &root, &view, &LOCAL.into(), &limits())
            .await
            .unwrap();
        assert_eq!(
            walked.absent,
            vec![WalkItem::Absent {
                view_path: tp("a1/b2/f003/x"),
                reason: AbsentReason::ThroughLeaf {
                    leaf: tp("a1/b2/f003")
                },
                unanswered: None,
            }]
        );
        assert_eq!(store.fetched().len(), 3);
    }

    #[tokio::test]
    async fn walk_shared_subtree_fetched_once_charged_per_path() {
        let mut store = CountingStorage::new();
        let shared = store.put_tree(vec![file(b"f", b"x")]);
        let root = store.put_tree(vec![dir(b"x", shared.clone()), dir(b"y", shared.clone())]);
        let walked = walk_collect(&store, &root, &View::whole(), &LOCAL.into(), &limits())
            .await
            .unwrap();
        assert_eq!(store.fetched(), vec![root.clone(), shared.clone()]);
        assert_eq!(paths(&walked), vec!["<root>", "x", "y"]);
        assert_eq!(walked.stats.nodes_fetched, 2);
        assert_eq!(walked.stats.nodes_reused, 1);

        let charged = store.blobs[&root].len() + 2 * store.blobs[&shared].len();
        let tight = TreeLimits {
            max_total_bytes: charged - 1,
            ..limits()
        };
        assert!(matches!(
            walk_collect(&store, &root, &View::whole(), &LOCAL.into(), &tight).await,
            Err(TreeError::Budget("expanded bytes"))
        ));
        let exact = TreeLimits {
            max_total_bytes: charged,
            ..limits()
        };
        walk_collect(&store, &root, &View::whole(), &LOCAL.into(), &exact)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn walk_whole_view_visits_same_node_set_as_load_verified_tree() {
        let mut store = CountingStorage::new();
        let (root, _) = ten_thousand(&mut store, true);
        let walked = walk_collect(&store, &root, &View::whole(), &LOCAL.into(), &limits())
            .await
            .unwrap();
        // Tree nodes only: the walk never fetches a leaf.
        assert_eq!(store.fetched().len(), 111);
        assert_eq!(walked.leaves_in_view().count(), 10_000);
        let walked_set: HashSet<BlobCid> = walked.nodes.iter().map(|n| n.cid.clone()).collect();
        let loaded = load_verified_tree(&store, &root, LOCAL, &limits())
            .await
            .unwrap();
        let loaded_set: HashSet<BlobCid> = loaded.nodes.keys().cloned().collect();
        assert_eq!(walked_set, loaded_set);
        assert!(walked.nodes.iter().all(|n| n.coverage == Coverage::InView));
    }

    #[tokio::test]
    async fn walk_rejects_non_dag_cbor_root() {
        let store = CountingStorage::new();
        assert!(matches!(
            walk(
                &store,
                &BlobCid::compute_raw(b"x"),
                &View::whole(),
                &LOCAL.into(),
                &limits()
            ),
            Err(TreeError::Invalid(_))
        ));
        assert!(store.fetched().is_empty());
    }

    #[tokio::test]
    async fn walk_missing_subtree_is_named_not_held_and_walk_continues() {
        let mut store = CountingStorage::new();
        let (root, nodes) = ten_thousand(&mut store, false);
        let b4 = nodes[&tp("a4/b4")].clone();
        store.remove(&b4);
        let view = three_path_view();
        let walked = walk_collect(&store, &root, &view, &LOCAL.into(), &limits())
            .await
            .unwrap();
        assert_eq!(
            paths(&walked),
            vec!["<root>", "a1", "a1/b2", "a4", "a9", "a9/b0"]
        );
        assert_eq!(
            walked.absent,
            vec![WalkItem::Absent {
                view_path: tp("a4/b4/f099"),
                reason: AbsentReason::NotHeld {
                    missing_at: tp("a4/b4"),
                    cid: b4,
                    tried: vec![],
                },
                unanswered: Some(local_miss()),
            }]
        );

        // Under the whole view, the missing a3 subtree is one absence; the other
        // 100 nodes still stream.
        let a3 = nodes[&tp("a3")].clone();
        store.remove(&a3);
        let walked = walk_collect(&store, &root, &View::whole(), &LOCAL.into(), &limits())
            .await
            .unwrap();
        assert_eq!(walked.nodes.len(), 111 - 11 - 1);
        assert_eq!(
            walked.absent,
            vec![
                WalkItem::Absent {
                    view_path: TreePath::root(),
                    reason: AbsentReason::NotHeld {
                        missing_at: tp("a3"),
                        cid: a3,
                        tried: vec![],
                    },
                    unanswered: Some(local_miss()),
                },
                WalkItem::Absent {
                    view_path: TreePath::root(),
                    reason: AbsentReason::NotHeld {
                        missing_at: tp("a4/b4"),
                        cid: nodes[&tp("a4/b4")].clone(),
                        tried: vec![],
                    },
                    unanswered: Some(local_miss()),
                },
            ]
        );
    }

    #[tokio::test]
    async fn read_path_over_a_spine_not_held_is_unanswered_naming_path() {
        let mut store = CountingStorage::new();
        let (root, nodes) = ten_thousand(&mut store, true);
        let b2 = nodes[&tp("a1/b2")].clone();
        store.remove(&b2);
        // B1: a single answer names the missing node's path and carries R7's three parts.
        match read_path(&store, &root, &tp("a1/b2/f003"), &LOCAL.into(), &limits()).await {
            Err(TreeError::Unanswered { cid, path, why }) => {
                assert_eq!(cid, b2);
                assert_eq!(path, tp("a1/b2"));
                assert_eq!(*why, local_miss());
            }
            other => panic!("expected Unanswered, got {other:?}"),
        }
        assert!(matches!(
            list_dir(&store, &root, &tp("a1/b2"), &LOCAL.into(), &limits()).await,
            Err(TreeError::Unanswered { ref path, .. }) if *path == tp("a1/b2")
        ));
    }

    #[tokio::test]
    async fn read_path_fetches_spine_plus_one_leaf_and_verifies() {
        let mut store = CountingStorage::new();
        let (root, nodes) = ten_thousand(&mut store, true);
        let content = ten_thousand_content(1, 2, 3);
        let leaf = read_path(&store, &root, &tp("a1/b2/f003"), &LOCAL.into(), &limits())
            .await
            .unwrap();
        assert_eq!(leaf.bytes.as_ref(), content.as_slice());
        assert_eq!(leaf.kind, LeafKind::File { executable: false });
        assert_eq!(leaf.cid, BlobCid::compute_raw(&content));
        let mut expected = cids(&nodes, &["", "a1", "a1/b2"]);
        expected.push(leaf.cid.clone());
        assert_eq!(store.fetched(), expected);

        // A tampered leaf is an integrity failure.
        store.tamper(&leaf.cid, Bytes::from_static(b"forged"));
        assert!(matches!(
            read_path(&store, &root, &tp("a1/b2/f003"), &LOCAL.into(), &limits()).await,
            Err(TreeError::Integrity(_))
        ));
    }

    #[tokio::test]
    async fn read_path_missing_is_path_not_found_named() {
        let mut store = CountingStorage::new();
        let (root, _) = ten_thousand(&mut store, false);
        let error = read_path(&store, &root, &tp("a1/b2/nope"), &LOCAL.into(), &limits())
            .await
            .unwrap_err();
        assert!(matches!(error, TreeError::PathNotFound { .. }));
        assert!(error.to_string().contains("a1/b2/nope"), "{error}");
    }

    #[tokio::test]
    async fn read_path_on_directory_is_not_a_leaf_and_through_external_refuses() {
        let mut store = CountingStorage::new();
        let src = store.put_tree(vec![file(b"x", b"x")]);
        let root = store.put_tree(vec![dir(b"src", src), external(b"vendor")]);
        assert!(matches!(
            read_path(&store, &root, &tp("src"), &LOCAL.into(), &limits()).await,
            Err(TreeError::NotALeaf { .. })
        ));
        assert!(matches!(
            read_path(&store, &root, &TreePath::root(), &LOCAL.into(), &limits()).await,
            Err(TreeError::NotALeaf { .. })
        ));
        for path in ["vendor", "vendor/x"] {
            match read_path(&store, &root, &tp(path), &LOCAL.into(), &limits()).await {
                Err(TreeError::ExternalBoundary { path, kind }) => {
                    assert_eq!(path, tp("vendor"));
                    assert_eq!(kind, "opaque");
                }
                other => panic!("expected ExternalBoundary, got {other:?}"),
            }
        }
        match read_path(&store, &root, &tp("src/x/y"), &LOCAL.into(), &limits()).await {
            Err(TreeError::NotADirectory { path }) => assert_eq!(path, tp("src/x")),
            other => panic!("expected NotADirectory, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn list_dir_root_and_nested_fetch_no_children() {
        let mut store = CountingStorage::new();
        let (root, nodes) = ten_thousand(&mut store, true);
        let listing = list_dir(&store, &root, &TreePath::root(), &LOCAL.into(), &limits())
            .await
            .unwrap();
        assert_eq!(listing.entries.len(), 10);
        assert_eq!(listing.cid, root);
        assert_eq!(store.fetched(), vec![root.clone()]);

        store.reset();
        let listing = list_dir(&store, &root, &tp("a4/b4"), &LOCAL.into(), &limits())
            .await
            .unwrap();
        assert_eq!(listing.entries.len(), 100);
        assert_eq!(listing.cid, nodes[&tp("a4/b4")]);
        assert_eq!(store.fetched(), cids(&nodes, &["", "a4", "a4/b4"]));
    }

    #[tokio::test]
    async fn list_dir_on_file_is_not_a_directory() {
        let mut store = CountingStorage::new();
        let (root, _) = ten_thousand(&mut store, false);
        match list_dir(&store, &root, &tp("a1/b2/f003"), &LOCAL.into(), &limits()).await {
            Err(TreeError::NotADirectory { path }) => assert_eq!(path, tp("a1/b2/f003")),
            other => panic!("expected NotADirectory, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn diff_identical_roots_fetch_nothing() {
        let mut store = CountingStorage::new();
        let (root, _) = ten_thousand(&mut store, false);
        let changes = diff_trees(
            &store,
            &root,
            &root,
            &View::whole(),
            &LOCAL.into(),
            &limits(),
        )
        .await
        .unwrap();
        assert!(changes.is_empty());
        assert!(store.fetched().is_empty());
    }

    #[tokio::test]
    async fn diff_reports_added_removed_modified_under_view() {
        let mut store = CountingStorage::new();
        let d = store.put_tree(vec![file(b"x", b"x")]);
        let e_old = store.put_tree(vec![file(b"y", b"y")]);
        let e_new = store.put_tree(vec![file(b"y", b"y"), file(b"z", b"z")]);
        let n = store.put_tree(vec![file(b"q", b"q")]);
        let exec = |name: &[u8], executable| TreeEntry {
            name: name.to_vec(),
            kind: TreeEntryKind::File {
                blob: BlobCid::compute_raw(b"k").into(),
                executable,
            },
        };
        let old = store.put_tree(vec![
            file(b"a", b"1"),
            dir(b"d", d),
            dir(b"e", e_old),
            exec(b"k", false),
        ]);
        let new = store.put_tree(vec![
            file(b"a", b"2"),
            dir(b"e", e_new),
            exec(b"k", true),
            dir(b"n", n),
        ]);
        let changes = diff_trees(&store, &old, &new, &View::whole(), &LOCAL.into(), &limits())
            .await
            .unwrap();
        let shape: Vec<(String, bool, bool)> = changes
            .iter()
            .map(|c| (c.path.to_string(), c.old.is_some(), c.new.is_some()))
            .collect();
        assert_eq!(
            shape,
            vec![
                ("a".to_string(), true, true),
                ("d".to_string(), true, false),
                ("e/z".to_string(), false, true),
                ("k".to_string(), true, true),
                ("n".to_string(), false, true),
            ]
        );
        assert!(matches!(
            changes[3].new,
            Some(TreeEntryKind::File {
                executable: true,
                ..
            })
        ));

        // Under a view of `e` only, the rest of the root's changes are outside.
        let only_e = View::from_tree_paths([tp("e")]);
        let changes = diff_trees(&store, &old, &new, &only_e, &LOCAL.into(), &limits())
            .await
            .unwrap();
        assert_eq!(
            changes.iter().map(|c| c.path.clone()).collect::<Vec<_>>(),
            vec![tp("e/z")]
        );
    }

    /// The 10k tree with one file changed at a3/b7/f005. Returns (old, new,
    /// old nodes, new a3, new b7).
    fn ten_thousand_with_one_change(
        store: &mut CountingStorage,
    ) -> (
        BlobCid,
        BlobCid,
        HashMap<TreePath, BlobCid>,
        BlobCid,
        BlobCid,
    ) {
        let (old, nodes) = ten_thousand(store, false);
        let node = |store: &CountingStorage, path: &str| {
            decode_tree(&store.blobs[&nodes[&tp(path)]], &TreeLimits::default()).unwrap()
        };
        let mut b7 = node(store, "a3/b7");
        b7.entries[5] = file(b"f005", b"changed");
        let new_b7 = store.put_tree(b7.entries);
        let mut a3 = node(store, "a3");
        a3.entries[7] = dir(b"b7", new_b7.clone());
        let new_a3 = store.put_tree(a3.entries);
        let mut root = node(store, "");
        root.entries[3] = dir(b"a3", new_a3.clone());
        let new = store.put_tree(root.entries);
        (old, new, nodes, new_a3, new_b7)
    }

    #[tokio::test]
    async fn diff_skips_shared_subtrees_by_cid() {
        let mut store = CountingStorage::new();
        let (old, new, nodes, new_a3, new_b7) = ten_thousand_with_one_change(&mut store);
        let changes = diff_trees(&store, &old, &new, &View::whole(), &LOCAL.into(), &limits())
            .await
            .unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, tp("a3/b7/f005"));
        assert!(changes[0].old.is_some() && changes[0].new.is_some());
        let fetched: HashSet<BlobCid> = store.fetched().into_iter().collect();
        let mut expected: HashSet<BlobCid> =
            cids(&nodes, &["", "a3", "a3/b7"]).into_iter().collect();
        expected.extend([new.clone(), new_a3, new_b7]);
        assert_eq!(store.fetched().len(), 6);
        assert_eq!(fetched, expected);
    }

    #[tokio::test]
    async fn diff_ignores_changes_outside_view() {
        let mut store = CountingStorage::new();
        let (old, new, _, _, _) = ten_thousand_with_one_change(&mut store);
        let view = View::from_tree_paths([tp("a1"), tp("a3/b7/f004")]);
        let changes = diff_trees(&store, &old, &new, &view, &LOCAL.into(), &limits())
            .await
            .unwrap();
        assert!(changes.is_empty());
        // a1 is equal by CID and never fetched; only the spine to a3/b7 is.
        assert_eq!(store.fetched().len(), 6);
    }

    fn change_shape(changes: &[TreeChange]) -> Vec<(String, bool, bool)> {
        changes
            .iter()
            .map(|c| (c.path.to_string(), c.old.is_some(), c.new.is_some()))
            .collect()
    }

    /// A Spine ancestor added, removed or changing kind is never itself a
    /// change: only the view paths below it are reported, when they exist.
    #[tokio::test]
    async fn diff_reports_only_view_tails_below_a_spine_ancestor() {
        let mut store = CountingStorage::new();
        let view = View::from_tree_paths([tp("a/b")]);
        let empty = store.put_tree(vec![file(b"z", b"z")]);
        let c_only = store.put_tree(vec![file(b"c", b"c")]);
        let with_b = store.put_tree(vec![file(b"b", b"b"), file(b"c", b"c")]);
        let b_dir_inner = store.put_tree(vec![file(b"x", b"x")]);
        let b_dir = store.put_tree(vec![dir(b"b", b_dir_inner)]);
        let root_c_only = store.put_tree(vec![dir(b"a", c_only), file(b"z", b"z")]);
        let root_with_b = store.put_tree(vec![dir(b"a", with_b), file(b"z", b"z")]);
        let root_b_dir = store.put_tree(vec![dir(b"a", b_dir), file(b"z", b"z")]);
        let root_file_1 = store.put_tree(vec![file(b"a", b"1"), file(b"z", b"z")]);
        let root_file_2 = store.put_tree(vec![file(b"a", b"2"), file(b"z", b"z")]);
        let diff = |old: &BlobCid, new: &BlobCid| {
            let (store, view, old, new) = (&store, &view, old.clone(), new.clone());
            async move {
                change_shape(
                    &diff_trees(store, &old, &new, view, &LOCAL.into(), &limits())
                        .await
                        .unwrap(),
                )
            }
        };
        let added = vec![("a/b".to_string(), false, true)];
        let removed = vec![("a/b".to_string(), true, false)];

        // Added or removed ancestor with nothing at the view path: no change.
        assert!(diff(&empty, &root_c_only).await.is_empty());
        assert!(diff(&root_c_only, &empty).await.is_empty());
        // Added or removed ancestor holding the view path: the tail only.
        assert_eq!(diff(&empty, &root_with_b).await, added);
        assert_eq!(diff(&root_with_b, &empty).await, removed);
        // An in-view directory added under a new ancestor is reported once.
        let b_added = diff(&empty, &root_b_dir).await;
        assert_eq!(b_added, added);
        // File <-> directory at the ancestor.
        assert!(diff(&root_file_1, &root_c_only).await.is_empty());
        assert!(diff(&root_c_only, &root_file_1).await.is_empty());
        assert_eq!(diff(&root_file_1, &root_with_b).await, added);
        assert_eq!(diff(&root_with_b, &root_file_1).await, removed);
        // A file changed at the ancestor holds nothing under the view.
        assert!(diff(&root_file_1, &root_file_2).await.is_empty());
        // Directory on both sides still descends to the tail.
        assert_eq!(diff(&root_c_only, &root_with_b).await, added);
    }

    /// Under a whole view the ancestor is in view, so the same add, remove
    /// and kind change are reported once at the ancestor, as before.
    #[tokio::test]
    async fn diff_in_view_kind_change_is_reported_at_its_path() {
        let mut store = CountingStorage::new();
        let with_b = store.put_tree(vec![file(b"b", b"b")]);
        let empty = store.put_tree(vec![file(b"z", b"z")]);
        let root_dir = store.put_tree(vec![dir(b"a", with_b), file(b"z", b"z")]);
        let root_file = store.put_tree(vec![file(b"a", b"1"), file(b"z", b"z")]);
        let whole = View::whole();
        for (old, new, expected) in [
            (&empty, &root_dir, ("a", false, true)),
            (&root_dir, &empty, ("a", true, false)),
            (&root_file, &root_dir, ("a", true, true)),
        ] {
            let changes = diff_trees(&store, old, new, &whole, &LOCAL.into(), &limits())
                .await
                .unwrap();
            assert_eq!(
                change_shape(&changes),
                vec![(expected.0.to_string(), expected.1, expected.2)]
            );
        }
    }

    fn external_ref(name: &[u8], reference: u8) -> TreeEntry {
        TreeEntry {
            name: name.to_vec(),
            kind: TreeEntryKind::External {
                kind: "git".into(),
                reference: vec![reference],
            },
        }
    }

    /// A view path through an External stops at the boundary in the walk, so
    /// the diff reports a change there: a changed reference, External to
    /// directory, directory to External, and an External added.
    #[tokio::test]
    async fn diff_external_on_the_spine_is_reported_at_the_boundary() {
        let mut store = CountingStorage::new();
        let inner = store.put_tree(vec![file(b"x", b"x")]);
        let root_e1 = store.put_tree(vec![external_ref(b"vendor", 1), file(b"z", b"z")]);
        let root_e2 = store.put_tree(vec![external_ref(b"vendor", 2), file(b"z", b"z")]);
        let root_dir = store.put_tree(vec![dir(b"vendor", inner.clone()), file(b"z", b"z")]);
        let root_none = store.put_tree(vec![file(b"z", b"z")]);
        let through = View::from_tree_paths([tp("vendor/x")]);
        let at = View::from_tree_paths([tp("vendor")]);
        let kinds = |changes: &[TreeChange]| -> Vec<(String, Option<TreeEntryKind>, Option<TreeEntryKind>)> {
            changes
                .iter()
                .map(|c| (c.path.to_string(), c.old.clone(), c.new.clone()))
                .collect()
        };
        let ext = |reference: u8| TreeEntryKind::External {
            kind: "git".into(),
            reference: vec![reference],
        };
        let dir_kind = TreeEntryKind::Directory {
            tree: inner.clone().into(),
        };
        for (old, new, expected) in [
            (&root_e1, &root_e2, (Some(ext(1)), Some(ext(2)))),
            (&root_e1, &root_dir, (Some(ext(1)), Some(dir_kind.clone()))),
            (&root_dir, &root_e1, (Some(dir_kind.clone()), Some(ext(1)))),
            (&root_none, &root_e1, (None, Some(ext(1)))),
        ] {
            let expected = vec![("vendor".to_string(), expected.0, expected.1)];
            for view in [&through, &at] {
                store.reset();
                let changes = diff_trees(&store, old, new, view, &LOCAL.into(), &limits())
                    .await
                    .unwrap();
                assert_eq!(kinds(&changes), expected);
                // The directory below the boundary is never descended.
                assert!(!store.fetched().contains(&inner));
            }
        }

        // The walk agrees: the same view path is a Boundary through the spine.
        let walked = walk_collect(&store, &root_e2, &through, &LOCAL.into(), &limits())
            .await
            .unwrap();
        assert_eq!(
            walked.boundaries,
            vec![WalkItem::Boundary {
                path: tp("vendor"),
                kind: "git".into(),
                reference: vec![2],
                coverage: Coverage::Spine,
            }]
        );
    }

    /// root -> `depth` directories, each named with 100 bytes, -> a bottom
    /// node holding `bottom`. Paths grow by 100 bytes per level, so the paths
    /// a gathered answer keeps grow faster than the node bytes fetched.
    fn long_chain(store: &mut CountingStorage, depth: usize, bottom: Vec<TreeEntry>) -> BlobCid {
        let mut cid = store.put_tree(bottom);
        for level in (0..depth).rev() {
            let name = format!("{level:x<100}");
            cid = store.put_tree(vec![dir(name.as_bytes(), cid)]);
        }
        cid
    }

    fn fetched_bytes(store: &CountingStorage) -> usize {
        store
            .fetched()
            .iter()
            .map(|cid| store.blobs[cid].len())
            .sum()
    }

    #[tokio::test]
    async fn walk_collect_meters_the_paths_it_keeps() {
        let mut store = CountingStorage::new();
        let root = long_chain(&mut store, 10, vec![file(b"z", b"z")]);
        let whole = View::whole();
        let walked = walk_collect(&store, &root, &whole, &LOCAL.into(), &limits())
            .await
            .unwrap();
        let node_bytes = walked.stats.node_bytes;
        let kept: usize = walked.nodes.iter().map(|n| n.path.byte_len()).sum();
        assert_eq!(kept, 100 * (1..=10).sum::<usize>());
        // The deepest pending path (1,000 bytes) and the node bytes fit; the
        // kept paths do not.
        assert!((1000..kept).contains(&node_bytes), "{node_bytes} {kept}");
        let tight = TreeLimits {
            max_total_bytes: node_bytes,
            ..TreeLimits::default()
        };

        // The cursor retains nothing, so it passes under the tight budget.
        let mut cursor = walk(&store, &root, &whole, &LOCAL.into(), &tight).unwrap();
        let mut count = 0;
        while let Some(item) = cursor.next().await {
            item.unwrap();
            count += 1;
        }
        assert_eq!(count, 11);
        // Gathering keeps every path, and is refused.
        assert!(matches!(
            walk_collect(&store, &root, &whole, &LOCAL.into(), &tight).await,
            Err(TreeError::Budget("result path bytes"))
        ));
    }

    /// The level walk meters what it keeps while it runs, as `walk_collect` does: under a budget
    /// the kept paths pass mid-walk, it refuses before fetching the rest of the chain.
    #[tokio::test]
    async fn the_level_walk_meters_the_paths_it_keeps_as_it_keeps_them() {
        let mut store = CountingStorage::new();
        let root = long_chain(&mut store, 10, vec![file(b"z", b"z")]);
        let kept = 100 * (1..=10).sum::<usize>();
        let tight = TreeLimits {
            max_total_bytes: kept / 2,
            ..TreeLimits::default()
        };
        assert!(matches!(
            crate::walk_batched(&store, &root, &View::whole(), &LOCAL.into(), &tight).await,
            Err(TreeError::Budget("result path bytes"))
        ));
        assert!(
            store.fetched().len() < 11,
            "refused only after fetching all {} nodes",
            store.fetched().len()
        );
    }

    /// Absences are metered too: a shared subtree whose missing children are reached through many
    /// long paths yields one absence per path, each keeping its own full-length `missing_at` and
    /// its unanswered parts. Gathering them is refused (`walk_collect`), and the level walk, which
    /// queues every such child as a frame, is refused by its queued frames' retained size.
    #[tokio::test]
    async fn absences_of_a_shared_subtrees_missing_children_are_metered() {
        let long = |prefix: &str, i: usize| format!("{prefix}{i:03}{}", "x".repeat(196));
        let mut store = CountingStorage::new();
        let mut missing = Vec::new();
        let mut children = Vec::new();
        for c in 0..10 {
            let child = store.put_tree(vec![file(format!("f{c}").as_bytes(), b"leaf")]);
            missing.push(child.clone());
            children.push(dir(long("c", c).as_bytes(), child));
        }
        let shared = store.put_tree(children);
        let root = store.put_tree(
            (0..50)
                .map(|p| dir(long("p", p).as_bytes(), shared.clone()))
                .collect(),
        );
        for child in &missing {
            store.remove(child);
        }
        let whole = View::whole();
        let walked = walk_collect(&store, &root, &whole, &LOCAL.into(), &limits())
            .await
            .unwrap();
        assert_eq!(walked.absent.len(), 500);
        let retained: usize = walked
            .absent
            .iter()
            .map(WalkItem::retained_bytes)
            .chain(walked.nodes.iter().map(|n| n.path.byte_len()))
            .sum();
        let node_paths: usize = walked.nodes.iter().map(|n| n.path.byte_len()).sum();
        let traversed =
            walked.stats.node_bytes + walked.stats.nodes_reused * store.blobs[&shared].len();
        // Every line but the kept absences fits: fetched and reused node bytes, the depth-first
        // pending paths (50 × 200), the kept node paths.
        let tight = traversed.max(node_paths).max(50 * 200) + 1;
        assert!(tight < retained, "{tight} {retained}");
        let limits = TreeLimits {
            max_total_bytes: tight,
            ..TreeLimits::default()
        };
        assert!(matches!(
            walk_collect(&store, &root, &whole, &LOCAL.into(), &limits).await,
            Err(TreeError::Budget("result path bytes"))
        ));
        assert!(matches!(
            crate::walk_batched(&store, &root, &whole, &LOCAL.into(), &limits).await,
            Err(TreeError::Budget(
                "result path bytes" | "pending path bytes"
            ))
        ));
    }

    /// The fixed per-frame charge covers what a queued frame really holds, so the TypeScript twin
    /// can charge the same numbers.
    #[test]
    fn queued_frame_charge_covers_the_frame() {
        assert!(std::mem::size_of::<Frame<'static>>() <= QUEUED_FRAME_BYTES);
        assert!(std::mem::size_of::<Vec<u8>>() <= QUEUED_COMPONENT_BYTES);
        assert_eq!(queued_frame_charge(10, 2), 10 + 2 * 24 + 192);
    }

    /// `WalkItem::absent` refuses a reason and unanswered parts that disagree, and the derived
    /// constructor cannot build one.
    #[test]
    fn an_absence_cannot_name_one_reason_and_carry_another() {
        let cid = BlobCid::compute(b"node");
        let hub = TierId::agent("uhCAkhub");
        let refused = ItemAbsence::refused(
            &hub,
            crate::ReachRefusal::new(crate::ReachReason::OutsideStanding, None),
        );
        let not_held = ItemAbsence::not_held(vec![hub.clone()]);
        let at = tp("a");
        let bad = [
            (
                AbsentReason::not_held(at.clone(), cid.clone(), vec![hub.clone()]),
                Some(refused.clone()),
            ),
            (
                AbsentReason::not_held(at.clone(), cid.clone(), vec![]),
                Some(not_held.clone()),
            ),
            (
                AbsentReason::withheld(at.clone(), cid.clone(), refused.why.clone()),
                None,
            ),
            (
                AbsentReason::withheld(at.clone(), cid.clone(), not_held.why.clone()),
                Some(not_held.clone()),
            ),
            (AbsentReason::not_found(at.clone()), Some(not_held.clone())),
        ];
        for (reason, unanswered) in bad {
            assert!(
                matches!(
                    WalkItem::absent(at.clone(), reason.clone(), unanswered),
                    Err(TreeError::Invalid(_))
                ),
                "{reason:?}"
            );
        }
        let good = [
            (
                AbsentReason::not_held(at.clone(), cid.clone(), vec![hub.clone()]),
                Some(not_held.clone()),
            ),
            (
                AbsentReason::withheld(at.clone(), cid.clone(), refused.why.clone()),
                Some(refused.clone()),
            ),
            (AbsentReason::through_leaf(at.clone()), None),
        ];
        for (reason, unanswered) in good {
            assert!(WalkItem::absent(at.clone(), reason, unanswered).is_ok());
        }
        match WalkItem::unanswered(at.clone(), at.clone(), cid, refused.clone()) {
            WalkItem::Absent {
                reason: AbsentReason::Withheld { why, .. },
                unanswered: Some(u),
                ..
            } => assert_eq!((why, u), (refused.why.clone(), refused)),
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn diff_meters_the_paths_it_keeps() {
        let mut store = CountingStorage::new();
        let files = |tag: &str| -> Vec<TreeEntry> {
            (0..20)
                .map(|k| {
                    file(
                        format!("f{k:02}").as_bytes(),
                        format!("{tag}{k}").as_bytes(),
                    )
                })
                .collect()
        };
        let old = long_chain(&mut store, 10, files("old"));
        let new = long_chain(&mut store, 10, files("new"));
        let whole = View::whole();
        let changes = diff_trees(&store, &old, &new, &whole, &LOCAL.into(), &limits())
            .await
            .unwrap();
        assert_eq!(changes.len(), 20);
        let kept: usize = changes.iter().map(|c| c.path.byte_len()).sum();
        let fetched = fetched_bytes(&store);
        assert!((1000..kept).contains(&fetched), "{fetched} {kept}");
        let tight = TreeLimits {
            max_total_bytes: fetched,
            ..TreeLimits::default()
        };
        assert!(matches!(
            diff_trees(&store, &old, &new, &whole, &LOCAL.into(), &tight).await,
            Err(TreeError::Budget("result path bytes"))
        ));
    }

    #[test]
    fn tree_path_display_is_lossy_and_names_root() {
        assert_eq!(TreePath::root().to_string(), "<root>");
        assert_eq!(tp("a/b").to_string(), "a/b");
        let raw = TreePath::from_components(vec![vec![0xff], b"x".to_vec()]).unwrap();
        assert_eq!(raw.to_string(), "\u{fffd}/x");
        assert_eq!(raw.byte_len(), 2);
        assert!(tp("a/b").starts_with(&tp("a")));
        assert!(!tp("ab").starts_with(&tp("a")));
        assert_eq!(tp("a").join(b"b"), tp("a/b"));
    }
}
