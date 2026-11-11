use std::collections::BTreeSet;

use elohim_epr::{cbor, cid, EprError};
use serde::{Deserialize, Serialize};

use crate::AxisId;

/// An immutable alias document. Its identity includes name, axis and sorted,
/// deduplicated features, encoded by EPR's canonical DAG-CBOR implementation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    /// Human-readable alias; changing it changes the document CID.
    pub name: String,
    /// Axis whose features the bundle names.
    pub axis: AxisId,
    /// Features represented by this bundle, independent of insertion order.
    pub features: BTreeSet<String>,
}

impl Bundle {
    /// Canonical DAG-CBOR map with `name`, `axis`, and sorted `features` fields.
    pub fn canonical_bytes(&self) -> elohim_epr::Result<Vec<u8>> {
        // EPR accepts Ipld but does not re-export its type. Serde supplies the
        // inferred string/list/map value without adding another direct dependency
        // or implementing a parallel CBOR encoder.
        let document = serde_json::from_value(serde_json::json!({
            "name": self.name, "axis": self.axis, "features": self.features,
        }))
        .map_err(|error| EprError::Encode(error.to_string()))?;
        cbor::encode(&document)
    }

    /// CIDv1 DAG-CBOR / SHA2-256 identity, using EPR's existing record CID helper.
    pub fn cid(&self) -> elohim_epr::Result<String> {
        Ok(cid::compute_cid(&self.canonical_bytes()?).to_string())
    }
}

/// Allocation-free specification of a shipped bundle; materialize with `to_bundle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BundleSpec {
    /// Fixed alias.
    pub name: &'static str,
    /// Fixed axis name.
    pub axis: &'static str,
    /// Fixed psABI feature list, relative to the x86-64 architectural baseline.
    pub features: &'static [&'static str],
}

impl BundleSpec {
    /// Materialize the same document shape as a caller-supplied bundle.
    pub fn to_bundle(&self) -> Bundle {
        Bundle {
            name: self.name.to_owned(),
            axis: AxisId(self.axis.to_owned()),
            features: self
                .features
                .iter()
                .map(|feature| (*feature).to_owned())
                .collect(),
        }
    }
}

/// psABI x86-64-v2 extensions above the base x86-64 instruction set.
///
/// Feature names are psABI spellings. An observer reading Linux `/proc/cpuinfo` must map
/// its spellings onto them (`pni` is `sse3`; `abm` is reported for `lzcnt`) rather than
/// compare raw flags, or a capable host is refused as `unmet`.
pub const X86_64_V2: BundleSpec = BundleSpec {
    name: "x86-64-v2",
    axis: "isa.x86",
    features: &[
        "cx16", "lahf_lm", "popcnt", "sse3", "sse4_1", "sse4_2", "ssse3",
    ],
};

/// psABI x86-64-v3 extensions, including all v2 requirements and OSXSAVE.
/// Observers must establish usable AVX state; CPU hardware flags alone are not proof.
pub const X86_64_V3: BundleSpec = BundleSpec {
    name: "x86-64-v3",
    axis: "isa.x86",
    features: &[
        "cx16", "lahf_lm", "popcnt", "sse3", "sse4_1", "sse4_2", "ssse3", "avx", "avx2", "bmi1",
        "bmi2", "f16c", "fma", "lzcnt", "movbe", "osxsave",
    ],
};
