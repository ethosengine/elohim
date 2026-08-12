//! Containment — the recursion this fabric asserts in doctrine and has never held in a type.
//!
//! Every [`crate::model::Intent`], [`crate::model::Commitment`], [`crate::model::FlowEvent`]
//! and [`crate::model::Process`] already carries `in_scope_of: Cid` — "the container EPR
//! accountable for this flow". The README states the recursion outright ("a container is
//! itself a resource in its parent's scope; whether it's the right boundary at the right
//! instrumentation level is judged one level up, recursively"), but nothing in the crate
//! could walk it: `in_scope_of` was a flat pointer with no declared parent, so scope matching
//! was exact equality and a commitment in a child scope was invisible to a query on its
//! parent.
//!
//! A [`Scopes`] is that missing edge, and only that edge. It is **Ephemeral (C)** — a
//! read-side index over containment that is *already notarized twice*: `Place.parent_place_id`
//! plus the `ParentToChildPlace` link type on the mishpat DNA, and `Collective` + `Membership`
//! on imagodei. Rebuild it by walking those links. **It mints no entry type and no head**;
//! minting a `Holon` entry would be a second home for an edge the DHT already witnesses.
//!
//! The canon calls a container that is itself contained a **holon**
//! (`commons-holonic-stewardship-backlog`, "the elohim ceiling is itself holonic/sociocratic
//! — every holon has its own elohim and its own ceiling… this is the VSM recursion"). This
//! type is that idea, named for the field it indexes rather than for the theory, so the join
//! is visible at every call site: a `FlowEvent`'s `in_scope_of` is a scope, and a
//! [`Scopes`] says which scopes contain it. Note "node" is *not* available as a synonym —
//! in this repo a node is a peer/device (`node-registry`, `elohim-node`, `nodeTypes`), and
//! borrowing it here would be the misrouting the seam map warns about.
//!
//! # One tree per axis — containment is single-parent, membership is not
//!
//! A farm sits in a watershed (ecological), a county (political), and a co-op (economic) at
//! the same time. That is **not** a multi-parent scope: it is three different containments,
//! and they answer different questions. Modelling it as one graph with three parents would
//! make `contains` ambiguous exactly where it must not be — "is this flow inside the limit?"
//! has one answer per axis and no answer at all across axes, because a watershed's yield and
//! a co-op's budget are not commensurable (spec Q16).
//!
//! So: hold a `Scopes` **per axis** and fold each separately. The forest invariant is
//! what keeps each axis's answer single-valued; plurality lives in the number of trees, not
//! in the number of parents. This is the same shape as the plural-lens position the canon
//! already takes — *plurality is the feature* — and it is why a DAG would be the wrong
//! generalization rather than a more permissive one.
//!
//! Why this is the load-bearing piece rather than a convenience: the aggregation rule that
//! composes a capacity upward is not uniform across a scope tree. A leaf agent has nothing
//! beneath it to aggregate; a factory takes the *minimum* over its inputs (Liebig — summing
//! machine throughputs overstates the whole); a community takes the *harmonic* mean so one
//! badly-served member cannot be averaged away; an ecosystem's limit is *declared at the
//! scope*, not folded from parts. Those are properties of the container, so a container had
//! to become expressible before the rule could be declared on one. See
//! [`crate::model::LimitSource`].
//!
//! # Invariant
//!
//! A scope tree is a **forest**: every scope has at most one parent, and there are no cycles.
//! [`Scopes::from_tree`] is the only constructor and refuses a cyclic or multi-parent
//! edge set, so every walk here terminates by construction. The map is private for exactly
//! that reason — this crate's older types keep `pub` fields and a re-checkable `validate()`,
//! which is why a struct literal can bypass [`crate::model::Bound::new`] and
//! [`crate::stock::Stock::new`] today (recorded as spec Q8). A new type does not need to
//! inherit that hole.

use crate::error::{FabricError, Result};
use cid::Cid;
use std::collections::{BTreeMap, BTreeSet};

/// A containment forest over scope CIDs. Derived, never stored.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scopes {
    /// child -> parent. A scope absent as a key is a root.
    parent: BTreeMap<Cid, Cid>,
}

impl Scopes {
    /// Build from `(child, parent)` edges, refusing anything that is not a forest.
    ///
    /// Two refusals, both structural rather than advisory:
    /// - **multi-parent** — a scope offered two different parents. Containment with two
    ///   answers is not containment; the caller's link walk is wrong and silently picking
    ///   one would bury it. (Re-offering the *same* edge is idempotent, not an error — link
    ///   walks legitimately visit an edge twice.)
    /// - **cycle** — a scope contained in itself, transitively. Refused rather than
    ///   depth-capped, because a cap turns a malformed input into a quietly truncated
    ///   answer, and this crate's discipline is to refuse rather than guess.
    pub fn from_tree<I>(edges: I) -> Result<Self>
    where
        I: IntoIterator<Item = (Cid, Cid)>,
    {
        let mut parent: BTreeMap<Cid, Cid> = BTreeMap::new();
        for (child, up) in edges {
            if child == up {
                return Err(FabricError::InvalidScopeRelation(format!(
                    "scope {child} is declared its own parent"
                )));
            }
            if let Some(existing) = parent.get(&child) {
                if existing != &up {
                    return Err(FabricError::InvalidScopeRelation(format!(
                        "scope {child} has two parents ({existing} and {up}) — containment \
                         with two answers is not containment"
                    )));
                }
                continue;
            }
            parent.insert(child, up);
        }

        // Cycle check over a functional graph (at most one parent each), O(n) total: walk
        // each unresolved chain once, colouring nodes as we go. A node already proven
        // acyclic is never re-walked, so a deep forest costs one pass, not depth-squared.
        let mut settled: BTreeSet<Cid> = BTreeSet::new();
        let mut path: Vec<Cid> = Vec::new();
        for start in parent.keys() {
            if settled.contains(start) {
                continue;
            }
            path.clear();
            let mut on_path: BTreeSet<Cid> = BTreeSet::new();
            let mut cursor = *start;
            loop {
                if settled.contains(&cursor) {
                    break;
                }
                if !on_path.insert(cursor) {
                    return Err(FabricError::InvalidScopeRelation(format!(
                        "containment cycle through scope {cursor}"
                    )));
                }
                path.push(cursor);
                match parent.get(&cursor) {
                    Some(next) => cursor = *next,
                    None => break,
                }
            }
            settled.extend(path.iter().copied());
        }

        Ok(Self { parent })
    }

    /// The declared parent of `scope`, or `None` when it is a root.
    pub fn parent_of(&self, scope: &Cid) -> Option<&Cid> {
        self.parent.get(scope)
    }

    /// Walk from `scope` up to its root, **excluding** `scope` itself.
    pub fn containers(&self, scope: &Cid) -> Containers<'_> {
        Containers {
            scopes: self,
            cursor: self.parent.get(scope).copied(),
        }
    }

    /// How many containment steps sit between `scope` and its root. A root is `0`.
    pub fn depth(&self, scope: &Cid) -> usize {
        self.containers(scope).count()
    }

    /// Does `ancestor` contain `scope`?
    ///
    /// **Reflexive**: a scope contains itself, so `contains(x, x)` is `true` even for a
    /// scope this index has never heard of. That is the semantics a scope query wants —
    /// "this scope and everything under it" must include the scope's own flows, and a leaf
    /// agent (§3's first row, the one with no aggregation) has only its own.
    pub fn contains(&self, ancestor: &Cid, scope: &Cid) -> bool {
        scope == ancestor || self.containers(scope).any(|up| up == *ancestor)
    }

    /// Every scope contained by `root`, including `root` itself.
    ///
    /// Prefer this over repeated [`Self::contains`] when folding: it pays one pass and then
    /// answers membership in log time, instead of walking the chain once per event.
    pub fn descendants(&self, root: &Cid) -> BTreeSet<Cid> {
        let mut out = BTreeSet::new();
        out.insert(*root);
        // One pass per node; each walk stops early at the first already-known ancestor.
        for scope in self.parent.keys() {
            if out.contains(scope) {
                continue;
            }
            let mut chain = Vec::new();
            let mut cursor = *scope;
            loop {
                if out.contains(&cursor) {
                    out.extend(chain.iter().copied());
                    break;
                }
                chain.push(cursor);
                match self.parent.get(&cursor) {
                    Some(next) => cursor = *next,
                    None => break,
                }
            }
        }
        out
    }

    /// Number of declared containment edges.
    pub fn len(&self) -> usize {
        self.parent.len()
    }

    /// Whether any containment is declared at all. An empty scopes is honest: it means
    /// nothing has told us about containment, and every [`Self::contains`] then answers
    /// only reflexively rather than pretending everything is unrelated *or* related.
    pub fn is_empty(&self) -> bool {
        self.parent.is_empty()
    }
}

/// Iterator over a scope's containers, nearest first. Terminates by construction — see the
/// forest invariant on [`Scopes`].
pub struct Containers<'a> {
    scopes: &'a Scopes,
    cursor: Option<Cid>,
}

impl Iterator for Containers<'_> {
    type Item = Cid;

    fn next(&mut self) -> Option<Cid> {
        let current = self.cursor?;
        self.cursor = self.scopes.parent.get(&current).copied();
        Some(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // The canonical mint, never a second one — `Cid::new_v1(0x71, …)` open-coded here would
    // be exactly the BritCid/BlobCid divergence the consolidation spec records.
    use elohim_epr::cid::compute_cid;

    fn cid(tag: &str) -> Cid {
        compute_cid(tag.as_bytes())
    }

    /// watershed > farm > field, plus a sibling farm.
    fn nested() -> Scopes {
        Scopes::from_tree([
            (cid("field"), cid("farm")),
            (cid("farm"), cid("watershed")),
            (cid("orchard"), cid("watershed")),
        ])
        .expect("a forest")
    }

    #[test]
    fn a_scope_is_contained_by_every_container_not_only_its_parent() {
        let h = nested();
        assert!(h.contains(&cid("farm"), &cid("field")));
        assert!(
            h.contains(&cid("watershed"), &cid("field")),
            "transitive containment is the whole point — exact-equality scope matching is \
             what made a child scope invisible to a query on its parent"
        );
    }

    #[test]
    fn containment_is_reflexive_even_for_an_unknown_scope() {
        let h = nested();
        assert!(h.contains(&cid("field"), &cid("field")));
        assert!(
            h.contains(&cid("stranger"), &cid("stranger")),
            "a leaf with no declared containment still contains its own flows"
        );
    }

    #[test]
    fn containment_does_not_run_downward_or_sideways() {
        let h = nested();
        assert!(!h.contains(&cid("field"), &cid("watershed")));
        assert!(!h.contains(&cid("orchard"), &cid("field")));
    }

    #[test]
    fn ancestors_are_nearest_first_and_exclude_self() {
        let h = nested();
        let up: Vec<Cid> = h.containers(&cid("field")).collect();
        assert_eq!(up, vec![cid("farm"), cid("watershed")]);
        assert_eq!(h.depth(&cid("field")), 2);
        assert_eq!(h.depth(&cid("watershed")), 0, "a root is depth 0");
    }

    #[test]
    fn descendants_agree_with_contains() {
        let h = nested();
        let under = h.descendants(&cid("watershed"));
        assert_eq!(
            under,
            [cid("watershed"), cid("farm"), cid("field"), cid("orchard")]
                .into_iter()
                .collect::<BTreeSet<_>>()
        );
        for scope in [cid("watershed"), cid("farm"), cid("field"), cid("orchard")] {
            assert_eq!(
                under.contains(&scope),
                h.contains(&cid("watershed"), &scope),
                "the precomputed set and the walk must never disagree"
            );
        }
    }

    #[test]
    fn a_cycle_is_refused_rather_than_depth_capped() {
        let err = Scopes::from_tree([
            (cid("a"), cid("b")),
            (cid("b"), cid("c")),
            (cid("c"), cid("a")),
        ])
        .expect_err("a cycle is not a forest");
        assert!(
            matches!(err, FabricError::InvalidScopeRelation(ref m) if m.contains("cycle")),
            "got {err:?} — a depth cap would turn malformed containment into a quietly \
             truncated answer"
        );
    }

    #[test]
    fn a_scope_cannot_be_its_own_parent() {
        assert!(matches!(
            Scopes::from_tree([(cid("a"), cid("a"))]),
            Err(FabricError::InvalidScopeRelation(_))
        ));
    }

    #[test]
    fn two_different_parents_are_refused_but_a_repeated_edge_is_not() {
        assert!(
            matches!(
                Scopes::from_tree([(cid("a"), cid("b")), (cid("a"), cid("c"))]),
                Err(FabricError::InvalidScopeRelation(_))
            ),
            "containment with two answers is not containment"
        );
        let h = Scopes::from_tree([(cid("a"), cid("b")), (cid("a"), cid("b"))])
            .expect("re-offering the same edge is idempotent — link walks revisit edges");
        assert_eq!(h.len(), 1);
    }

    #[test]
    fn an_empty_scope_tree_answers_only_reflexively() {
        let h = Scopes::default();
        assert!(h.is_empty());
        assert!(h.contains(&cid("a"), &cid("a")));
        assert!(!h.contains(&cid("a"), &cid("b")));
        assert_eq!(h.containers(&cid("a")).count(), 0);
    }
}
