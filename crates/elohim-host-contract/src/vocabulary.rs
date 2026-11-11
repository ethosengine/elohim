use std::cmp::Ordering;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// An open axis name (for example `isa.x86`, `abi.libc`, or the root axis `arch`).
/// Names have no implicit meaning: the observer must explicitly supply each axis.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AxisId(pub String);

/// The four closed comparison operations; adding axes requires no new operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ComparisonKind {
    /// Exact scalar equality.
    Equal,
    /// Every required feature belongs to the observed feature set.
    Subset,
    /// A numeric version floor within the same named line.
    AtLeast,
    /// Every required name was resolved by the observer.
    Present,
}

/// A numeric version in one named line; `glibc` and `musl` never compare across lines.
/// Components are compared numerically, with omitted trailing components equal to zero.
/// An empty component list denotes version zero, not an unknown observation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Version {
    /// Version namespace, for example `glibc`.
    pub line: String,
    /// Major, minor and further numeric components, in significance order.
    pub components: Vec<u32>,
}

impl Version {
    pub(crate) fn compare(&self, other: &Self) -> Option<Ordering> {
        if self.line != other.line {
            return None;
        }
        let length = self.components.len().max(other.components.len());
        Some(
            self.components
                .iter()
                .copied()
                .chain(std::iter::repeat(0))
                .take(length)
                .cmp(
                    other
                        .components
                        .iter()
                        .copied()
                        .chain(std::iter::repeat(0))
                        .take(length),
                ),
        )
    }
}

/// Required value carried together with its comparison kind, preventing mismatches.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
pub enum Predicate {
    /// A scalar the host must equal.
    Equal(String),
    /// The minimum feature set.
    Subset(BTreeSet<String>),
    /// The minimum version within a named line.
    AtLeast(Version),
    /// Names which must all resolve on the host.
    Present(BTreeSet<String>),
}

impl Predicate {
    /// The closed comparison kind of this predicate.
    pub fn kind(&self) -> ComparisonKind {
        match self {
            Self::Equal(_) => ComparisonKind::Equal,
            Self::Subset(_) => ComparisonKind::Subset,
            Self::AtLeast(_) => ComparisonKind::AtLeast,
            Self::Present(_) => ComparisonKind::Present,
        }
    }

    pub(crate) fn satisfied_by(&self, host: &AxisValue) -> bool {
        match (self, host) {
            (Self::Equal(required), AxisValue::Scalar(value)) => required == value,
            (Self::Subset(required) | Self::Present(required), AxisValue::Names(values)) => {
                required.is_subset(values)
            }
            (Self::AtLeast(required), AxisValue::Version(value)) => {
                matches!(
                    value.compare(required),
                    Some(Ordering::Equal | Ordering::Greater)
                )
            }
            _ => false,
        }
    }

    /// Logical conjunction when expressible as one predicate. Incomparable kinds,
    /// scalar values or version lines cannot silently replace one another.
    pub(crate) fn stricter(&self, other: &Self) -> Option<Self> {
        match (self, other) {
            (Self::Equal(left), Self::Equal(right)) if left == right => Some(self.clone()),
            (Self::Subset(left), Self::Subset(right)) => {
                Some(Self::Subset(left.union(right).cloned().collect()))
            }
            (Self::Present(left), Self::Present(right)) => {
                Some(Self::Present(left.union(right).cloned().collect()))
            }
            (Self::AtLeast(left), Self::AtLeast(right)) => match left.compare(right)? {
                Ordering::Less => Some(other.clone()),
                Ordering::Greater => Some(self.clone()),
                // Deterministic even for equivalent spellings such as 2.34 and 2.34.0.
                Ordering::Equal => Some(self.min(other).clone()),
            },
            _ => None,
        }
    }
}

/// One explicitly supplied host value. Serializable alone for local diagnostics;
/// the host inventory [`crate::Provides`] deliberately has no serialization impl.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
pub enum AxisValue {
    /// Architecture, OS, byte order, or another scalar observation.
    Scalar(String),
    /// Observed features or successfully resolved names (an empty set is observed).
    Names(BTreeSet<String>),
    /// An observed version in an explicit line.
    Version(Version),
}
