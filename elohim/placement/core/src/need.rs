//! D.2 — needs. A need is an intent to receive (spec §7.1).

use std::collections::BTreeSet;

use cid::Cid;
use elohim_epr::Reach;
use elohim_epr_rea::{atom_cid, AgentRef};
use seam_contracts::Answer;
use serde::{Deserialize, Serialize};

use crate::kind::{GrantScope, KindId};
use crate::offer::CidString;
use crate::params::PlaceError;

/// What a need is keyed on: R1, per tree root and view, never per blob, once the custody rows
/// migrate.
///
/// The wire form is the spec's adjacently tagged `{ "form": …, "id": … }`, the CID forms carrying
/// `cid_serde`. `Serialize` is derived; `Deserialize` is written by hand, because canonical DAG-CBOR
/// sorts `id` before `form`, and serde's derived adjacently tagged decoder then buffers `id` before it
/// knows the form, which loses the tag-42 CID (`tests::need_key_forms_are_strings_in_json_and_bytes_in_dag_cbor`
/// failed on the derived form). The hand-written decoder reads the two fields in either order.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case", tag = "form", content = "id")]
pub enum NeedKey {
    /// Legacy custody marker (`sha256-<hex>`), until the custody rows migrate.
    Blob(String),
    /// `TreeView::cid()`.
    TreeView(#[serde(with = "elohim_epr_rea::index::cid_serde")] Cid),
    /// `TaskSpec` CID (§F).
    Task(#[serde(with = "elohim_epr_rea::index::cid_serde")] Cid),
    Session(#[serde(with = "elohim_epr_rea::index::cid_serde")] Cid),
}

/// The `id` field before its form is known: text in either format, or a CID link in a binary one.
enum KeyId {
    Text(String),
    Link(Cid),
}

impl<'de> Deserialize<'de> for KeyId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = KeyId;
            fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str("a need key id: a string or a CID")
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<KeyId, E> {
                Ok(KeyId::Text(v.to_string()))
            }
            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<KeyId, E> {
                Ok(KeyId::Text(v))
            }
            /// A CID link arrives as `cid`'s private newtype around its bytes (cid 0.11 `serde.rs`).
            fn visit_newtype_struct<D: serde::Deserializer<'de>>(
                self,
                d: D,
            ) -> Result<KeyId, D::Error> {
                struct Bytes;
                impl serde::de::Visitor<'_> for Bytes {
                    type Value = Cid;
                    fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                        f.write_str("a CID's bytes")
                    }
                    fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<Cid, E> {
                        Cid::try_from(v).map_err(E::custom)
                    }
                }
                d.deserialize_bytes(Bytes).map(KeyId::Link)
            }
        }
        if d.is_human_readable() {
            String::deserialize(d).map(KeyId::Text)
        } else {
            d.deserialize_any(V)
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum KeyForm {
    Blob,
    TreeView,
    Task,
    Session,
}

impl<'de> Deserialize<'de> for NeedKey {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::{Error, MapAccess, Visitor};
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = NeedKey;
            fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str("a need key { form, id }")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<NeedKey, A::Error> {
                let (mut form, mut id) = (None, None);
                while let Some(k) = map.next_key::<String>()? {
                    match k.as_str() {
                        "form" if form.is_none() => form = Some(map.next_value::<KeyForm>()?),
                        "id" if id.is_none() => id = Some(map.next_value::<KeyId>()?),
                        other => {
                            return Err(A::Error::custom(format!(
                                "unexpected need key field `{other}`"
                            )))
                        }
                    }
                }
                let form = form.ok_or_else(|| A::Error::missing_field("form"))?;
                let id = id.ok_or_else(|| A::Error::missing_field("id"))?;
                let cid = |id: KeyId| match id {
                    KeyId::Link(c) => Ok(c),
                    KeyId::Text(t) => Cid::try_from(t.as_str()).map_err(A::Error::custom),
                };
                Ok(match form {
                    KeyForm::Blob => match id {
                        KeyId::Text(t) => NeedKey::Blob(t),
                        KeyId::Link(_) => {
                            return Err(A::Error::custom("a blob marker is text, not a CID"))
                        }
                    },
                    KeyForm::TreeView => NeedKey::TreeView(cid(id)?),
                    KeyForm::Task => NeedKey::Task(cid(id)?),
                    KeyForm::Session => NeedKey::Session(cid(id)?),
                })
            }
        }
        d.deserialize_map(V)
    }
}

impl NeedKey {
    /// Ranking bytes. `Blob` → the marker's UTF-8 bytes, so the XOR key is byte-identical to
    /// `placement.rs::hash_into_keyspace` (pinned by the golden test); the others → the CID's bytes.
    pub fn rank_bytes(&self) -> Vec<u8> {
        match self {
            NeedKey::Blob(marker) => marker.as_bytes().to_vec(),
            NeedKey::TreeView(c) | NeedKey::Task(c) | NeedKey::Session(c) => c.to_bytes(),
        }
    }

    /// The CID this key names, when it names one.
    pub fn cid(&self) -> Option<&Cid> {
        match self {
            NeedKey::Blob(_) => None,
            NeedKey::TreeView(c) | NeedKey::Task(c) | NeedKey::Session(c) => Some(c),
        }
    }

    /// The key's string form, as commitment ids hash it: the marker, or the CID's string.
    pub fn as_id_string(&self) -> String {
        match self {
            NeedKey::Blob(marker) => marker.clone(),
            NeedKey::TreeView(c) | NeedKey::Task(c) | NeedKey::Session(c) => c.to_string(),
        }
    }
}

/// How many holders, across how many failure domains. `domains <= holders`; resolved by the charter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Demand {
    pub holders: u16,
    pub domains: u16,
}

/// What a carrier must offer.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Requirements {
    pub always_on: bool,
    /// For a task: `disk_bytes` plus the materialized view's size (F16).
    pub bytes: Option<u64>,
    pub memory_bytes: Option<u64>,
    pub cpu_millis: Option<u32>,
    pub pids: Option<u32>,
    /// The weakest isolation the need accepts (F6).
    pub isolation: Option<Isolation>,
    /// A narrowing every peer's admit sees (F14); `None`: any carrier.
    pub carriers: Option<BTreeSet<AgentRefKey>>,
}

/// placement-core's own copy of ark's isolation ladder (placement-core does not depend on ark-core);
/// ark-core maps `passport::EffectiveTier` onto it 1:1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Isolation {
    None,
    Intrinsic,
    Delegated,
    Bounded,
    Enforced,
}

/// An agent key as an ordered string, for sets and maps (`AgentRef` has no `Ord`, finding 5).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AgentRefKey(String);

impl AgentRefKey {
    pub fn of(agent: &AgentRef) -> Self {
        Self(agent.0.clone())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&AgentRef> for AgentRefKey {
    fn from(agent: &AgentRef) -> Self {
        Self::of(agent)
    }
}

/// Who expressed a need. Deliberately no `Device` variant: a device's own needs exist only as
/// [`PrivateNeed`]. A struct, not an internally tagged enum, so the id's form stays honest in both
/// formats (G1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Expresser {
    pub by: ExpresserKind,
    /// The manifest or charter CID string, or the agent key.
    pub id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExpresserKind {
    App,
    Person,
    Charter,
    Agent,
}

/// The charter's serve-order classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum NeedClass {
    Care,
    Member,
    Agent,
    Commons,
}

impl NeedClass {
    /// The built-in serve order, used when a charter names none.
    pub const BUILTIN_ORDER: &'static [NeedClass] = &[
        NeedClass::Care,
        NeedClass::Member,
        NeedClass::Agent,
        NeedClass::Commons,
    ];

    pub fn label(self) -> &'static str {
        match self {
            NeedClass::Care => "care",
            NeedClass::Member => "member",
            NeedClass::Agent => "agent",
            NeedClass::Commons => "commons",
        }
    }
}

/// Kind-specific terms inside the need's canonical encoding (F4).
///
/// `input_root` is a [`CidString`] rather than a `Cid` under `cid_serde`, revising the interfaces
/// spec's `#[serde(with = "cid_serde")] input_root: Cid`: this enum is internally tagged, and G1's
/// own rule is that no CID sits inside an internally tagged enum, because its deserializer buffers
/// the content and reports human-readable whatever the format (`need::tests::task_terms_round_trip_dag_cbor`
/// pins the fixed form). `base_root` was already a `CidString` in the spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    tag = "terms"
)]
pub enum NeedTerms {
    None,
    Task {
        input_root: CidString,
        scope: GrantScope,
        base_root: Option<CidString>,
    },
}

/// An intent to receive. Every CID field carries `cid_serde`: a `bafy…` string in JSON, bytes in
/// DAG-CBOR (G1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Need {
    pub kind: KindId,
    pub key: NeedKey,
    pub demand: Demand,
    pub requires: Requirements,
    pub terms: NeedTerms,
    /// The reach it was expressed at; it crosses no wider (spec §7.2).
    pub reach: Reach,
    /// Carried for whom; one receiver per need.
    pub receiver: AgentRef,
    pub expressed_by: Expresser,
    pub class: NeedClass,
    /// The space whose membrane it was expressed in (VF `in_scope_of`).
    #[serde(with = "elohim_epr_rea::index::cid_serde")]
    pub in_scope_of: Cid,
}

impl Need {
    /// `atom_cid` of the canonical encoding: the identity `explain` and `Carrying` key on (D10).
    pub fn id(&self) -> Result<Cid, PlaceError> {
        atom_cid(self).map_err(|e| PlaceError(format!("need encoding: {e}")))
    }

    /// The task's input root, when the terms carry one.
    pub fn input_root(&self) -> Option<Cid> {
        match &self.terms {
            NeedTerms::Task { input_root, .. } => input_root.to_cid().ok(),
            NeedTerms::None => None,
        }
    }
}

/// `eprfs_core::Depth`'s variants, mirrored because placement-core does not depend on eprfs-core;
/// ark-core maps the two 1:1. No `Ord`: depth does not order offline capability (B14).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Depth {
    Query,
    Sparse,
    OnRead,
    Kept,
}

/// Device-membrane needs ("keep this offline", "spare my battery", a budget). Not `Serialize` (R6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateNeed {
    pub key: NeedKey,
    pub depth: Depth,
    pub budget_bytes: Option<u64>,
}

/// Implemented by each source of needs: the manifest reader (§G), a person's keep intents, a spool.
/// The answer is R7's: `Absent` when the scope expresses nothing, `Unreachable` when the source
/// could not read it. Only a `Present` answer (an empty list included) may withdraw a carrying, so
/// a failed read never drains the scope (fix round 2; the spec returned a bare `Vec<Need>`).
pub trait NeedSource {
    fn needs(&self, scope: &Cid) -> Answer<Vec<Need>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn need(terms: NeedTerms) -> Need {
        Need {
            kind: KindId::Task,
            key: NeedKey::Task(elohim_epr::cid::compute_cid(b"task")),
            demand: Demand {
                holders: 1,
                domains: 1,
            },
            requires: Requirements::default(),
            terms,
            reach: Reach::Intimate,
            receiver: AgentRef("uhCAk-r".into()),
            expressed_by: Expresser {
                by: ExpresserKind::Person,
                id: "uhCAk-r".into(),
            },
            class: NeedClass::Member,
            in_scope_of: elohim_epr::cid::compute_cid(b"space"),
        }
    }

    #[test]
    fn need_key_forms_are_strings_in_json_and_bytes_in_dag_cbor() {
        let c = elohim_epr::cid::compute_cid(b"tree");
        let k = NeedKey::TreeView(c);
        let json = serde_json::to_string(&k).unwrap();
        assert_eq!(json, format!(r#"{{"form":"tree-view","id":"{c}"}}"#));
        assert_eq!(serde_json::from_str::<NeedKey>(&json).unwrap(), k);
        let cbor = serde_ipld_dagcbor::to_vec(&k).unwrap();
        // tag 42 (0xd8 0x2a) marks a CID link: bytes, not text.
        assert!(cbor.windows(2).any(|w| w == [0xd8, 0x2a]), "{cbor:x?}");
        assert_eq!(serde_ipld_dagcbor::from_slice::<NeedKey>(&cbor).unwrap(), k);
    }

    #[test]
    fn blob_rank_bytes_are_the_marker_bytes() {
        assert_eq!(
            NeedKey::Blob("sha256-ab".into()).rank_bytes(),
            b"sha256-ab".to_vec()
        );
    }

    #[test]
    fn task_terms_round_trip_dag_cbor() {
        let input = elohim_epr::cid::compute_cid(b"input");
        let n = need(NeedTerms::Task {
            input_root: CidString::from(&input),
            scope: GrantScope::MeasureStage,
            base_root: None,
        });
        let cbor = serde_ipld_dagcbor::to_vec(&n).unwrap();
        let back: Need = serde_ipld_dagcbor::from_slice(&cbor).unwrap();
        assert_eq!(back, n);
        assert_eq!(back.input_root(), Some(input));
        assert_eq!(back.id().unwrap(), n.id().unwrap());
    }

    /// Why `NeedTerms::Task.input_root` is a `CidString`: the spec's form, a `Cid` under `cid_serde`
    /// inside this internally tagged enum, encodes to DAG-CBOR and then does not decode. If serde
    /// ever preserves the link through its buffered content, this test fails and the spec form can
    /// return.
    #[test]
    fn a_cid_inside_an_internally_tagged_enum_does_not_decode_from_dag_cbor() {
        #[derive(Debug, Serialize, Deserialize)]
        #[serde(
            rename_all = "kebab-case",
            rename_all_fields = "camelCase",
            tag = "terms"
        )]
        enum SpecTerms {
            Task {
                #[serde(with = "elohim_epr_rea::index::cid_serde")]
                input_root: Cid,
            },
        }
        let t = SpecTerms::Task {
            input_root: elohim_epr::cid::compute_cid(b"input"),
        };
        let cbor = serde_ipld_dagcbor::to_vec(&t).unwrap();
        assert!(serde_ipld_dagcbor::from_slice::<SpecTerms>(&cbor).is_err());
    }

    #[test]
    fn need_key_decodes_in_either_field_order() {
        let c = elohim_epr::cid::compute_cid(b"tree");
        let json = format!(r#"{{"id":"{c}","form":"task"}}"#);
        assert_eq!(
            serde_json::from_str::<NeedKey>(&json).unwrap(),
            NeedKey::Task(c)
        );
        assert!(serde_json::from_str::<NeedKey>(r#"{"form":"tree-view","id":"nope"}"#).is_err());
        assert!(serde_json::from_str::<NeedKey>(r#"{"form":"blob"}"#).is_err());
        let blob = NeedKey::Blob("sha256-ab".into());
        let cbor = serde_ipld_dagcbor::to_vec(&blob).unwrap();
        assert_eq!(
            serde_ipld_dagcbor::from_slice::<NeedKey>(&cbor).unwrap(),
            blob
        );
    }

    #[test]
    fn json_to_need_to_dag_cbor_keeps_the_id() {
        // G1's contract vector: a JSON fixture deserializes, re-encodes to DAG-CBOR, and gives the
        // same Need::id as one built in Rust.
        let n = need(NeedTerms::None);
        let json = serde_json::to_string(&n).unwrap();
        let from_json: Need = serde_json::from_str(&json).unwrap();
        assert_eq!(from_json.id().unwrap(), n.id().unwrap());
        assert!(json.contains("\"inScopeOf\":\"bafy"), "{json}");
    }

    #[test]
    fn need_refuses_unknown_fields() {
        let n = need(NeedTerms::None);
        let mut v: serde_json::Value = serde_json::to_value(&n).unwrap();
        v["device"] = serde_json::json!("phone");
        assert!(serde_json::from_value::<Need>(v).is_err());
    }
}
