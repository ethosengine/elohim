//! The statement contract: which signed authority statements this coordinator
//! issues, accepts for a new act, and honors on a historical read.
//!
//! A delegation rule in a coordinator is the verifier's own diligence, in force
//! wherever this wasm runs. When a release stops accepting a form an older
//! coordinator still issues, that is a network-visible change shipped as a local
//! one. This table makes it declared: the artifact states its own contract, and
//! the code that decides consults the table rather than mirroring it.
//!
//! Scope: authority statements only (delegations, grants, acceptances).
//!
//! Each `statement` lists its forms OLDEST FIRST. The order is load-bearing:
//! "lowest accepted" in a refusal is the first form, in table order, that is
//! still accepted for a new act.
//!
//! Column meanings (each derived from the code path, not from intent):
//! - `issues`: a code path in this coordinator signs this form.
//! - `accepts_new`: this form can authorize an act this coordinator performs
//!   now (a new publication, a new acceptance witness, a new declaration).
//! - `honors_historical`: an existing instance still verifies on a read of
//!   past state (election reads, recovery of an already-accepted version).

use hdk::prelude::*;

/// Bumped whenever a row changes. A tightening records the version it landed in.
pub const CONTRACT_VERSION: u32 = 1;

/// The root author's content-scoped grant to a delegate (`HeadDelegation`).
pub const HEAD_DELEGATION: &str = "head-delegation";
/// The root author's signed acceptance of one exact delegated version. The
/// signed domain is `elohim:accepted-content-head:<form>`.
pub const ACCEPTED_CONTENT_HEAD: &str = "accepted-content-head";

/// Head-delegation grant forms, by field presence on `HeadDelegationPayload`.
pub const GRANT_LEGACY: &str = "legacy";
pub const GRANT_ISSUED: &str = "issued";
pub const GRANT_DEVICE_BOUND: &str = "device-bound";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatementRule {
    pub statement: &'static str,
    pub form: &'static str,
    pub issues: bool,
    pub accepts_new: bool,
    pub honors_historical: bool,
}

/// The contract. Release rule for any row whose `accepts_new` goes false:
///
/// - Default order is expand-before-contract: release N issues the new form and
///   accepts both; release N+1 refuses the old form for new acts.
/// - A one-step tightening (issue and refuse in the same release) is allowed
///   only when `statement-contract.lock.json` marks it `"mode": "immediate"`
///   with a reason (for example, a security fix).
/// - The reviewed lockfile line is the approval. No second signature is needed.
///
/// The test `lockfile_pins_the_table` fails when this table and the lockfile
/// differ, or when a refused row carries no valid tightening record.
///
/// Evidence for each bool (head_delegation.rs unless noted):
/// - head-delegation/legacy: issues=false — `grant_head_delegation` always sets
///   `issuance_action_hash: Some(..)`. accepts_new=false — `verify_head_delegation`
///   refuses it through `new_act_issuance`. honors_historical=true —
///   `verify_accepted_head` keeps the legacy branches (`acceptance_history_anchor`
///   falls back to the root, the legacy root stop in
///   `verify_acceptance_author_history`, the indexed `is_revoked` check).
/// - head-delegation/issued: issues=true — `grant_head_delegation` with no
///   `device_binding`. accepts_new=true — `verify_head_delegation` verifies its
///   native issuance record. honors_historical=true — `verify_accepted_head`.
/// - head-delegation/device-bound: issues=true — `grant_head_delegation` with a
///   `device_binding`. accepts_new=true — `verify_head_delegation` plus
///   `invocation::verify_device`. honors_historical=true — `verify_accepted_head`
///   with the controller-publication witness branch.
/// - accepted-content-head/v2: issues=true — `acceptance_from_witness` signs the
///   v2 domain for a legacy grant, reachable only over a pre-existing root-author
///   witness (`accepted_delegated_head_inner`); a new legacy witness cannot be
///   created because `stage_acceptance` runs `verify_head_delegation`.
///   accepts_new=true — `verify_accepted_head` admits it for
///   `declare_earned_canonical_head` recovery and `preflight_head_publication`
///   with `accepted_head` (lib.rs). honors_historical=true — election reads in
///   lib.rs call `verify_accepted_head`.
/// - accepted-content-head/v3, v4: issues=true — `acceptance_from_witness`
///   (`accept_delegated_head`, `get_accepted_delegated_head`). accepts_new and
///   honors_historical as for v2; v4 additionally requires the device witness.
pub const STATEMENT_CONTRACT: &[StatementRule] = &[
    StatementRule {
        statement: HEAD_DELEGATION,
        form: GRANT_LEGACY,
        issues: false,
        accepts_new: false,
        honors_historical: true,
    },
    StatementRule {
        statement: HEAD_DELEGATION,
        form: GRANT_ISSUED,
        issues: true,
        accepts_new: true,
        honors_historical: true,
    },
    StatementRule {
        statement: HEAD_DELEGATION,
        form: GRANT_DEVICE_BOUND,
        issues: true,
        accepts_new: true,
        honors_historical: true,
    },
    StatementRule {
        statement: ACCEPTED_CONTENT_HEAD,
        form: "v2",
        issues: true,
        accepts_new: true,
        honors_historical: true,
    },
    StatementRule {
        statement: ACCEPTED_CONTENT_HEAD,
        form: "v3",
        issues: true,
        accepts_new: true,
        honors_historical: true,
    },
    StatementRule {
        statement: ACCEPTED_CONTENT_HEAD,
        form: "v4",
        issues: true,
        accepts_new: true,
        honors_historical: true,
    },
];

fn rule(statement: &str, form: &str) -> Option<&'static StatementRule> {
    STATEMENT_CONTRACT
        .iter()
        .find(|r| r.statement == statement && r.form == form)
}

/// Fail-closed: a form absent from the table is not accepted for a new act.
pub fn accepts_new(statement: &str, form: &str) -> bool {
    rule(statement, form).is_some_and(|r| r.accepts_new)
}

/// The oldest form of `statement` still accepted for a new act.
pub fn lowest_accepted_new(statement: &str) -> Option<&'static str> {
    STATEMENT_CONTRACT
        .iter()
        .find(|r| r.statement == statement && r.accepts_new)
        .map(|r| r.form)
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StatementContractRow {
    pub statement: String,
    pub form: String,
    pub issues: bool,
    pub accepts_new: bool,
    pub honors_historical: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StatementContract {
    pub contract_version: u32,
    pub statements: Vec<StatementContractRow>,
}

pub fn contract() -> StatementContract {
    StatementContract {
        contract_version: CONTRACT_VERSION,
        statements: STATEMENT_CONTRACT
            .iter()
            .map(|r| StatementContractRow {
                statement: r.statement.into(),
                form: r.form.into(),
                issues: r.issues,
                accepts_new: r.accepts_new,
                honors_historical: r.honors_historical,
            })
            .collect(),
    }
}

/// This coordinator's own statement contract. Pure constant: no host calls.
/// A reader asks it once per distinct coordinator wasm hash, never per app.
#[hdk_extern]
pub fn statement_contract(_: ()) -> ExternResult<StatementContract> {
    Ok(contract())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCKFILE: &str = include_str!("../statement-contract.lock.json");

    #[derive(Deserialize)]
    struct Lock {
        contract_version: u32,
        statements: Vec<LockRow>,
    }

    #[derive(Deserialize)]
    struct LockRow {
        #[serde(flatten)]
        row: StatementContractRow,
        #[serde(default)]
        tightening: Option<Tightening>,
    }

    #[derive(Deserialize)]
    struct Tightening {
        mode: String,
        reason: String,
        #[serde(rename = "contractVersion")]
        contract_version: u32,
    }

    /// The lockfile check, pure so its refusals can be exercised directly.
    fn check_lock(table: &StatementContract, lock_json: &str) -> Result<(), String> {
        let lock: Lock =
            serde_json::from_str(lock_json).map_err(|e| format!("lockfile malformed: {e}"))?;
        if lock.contract_version != table.contract_version {
            return Err(format!(
                "contract_version differs: table={} lock={}",
                table.contract_version, lock.contract_version
            ));
        }
        let locked: Vec<&StatementContractRow> = lock.statements.iter().map(|r| &r.row).collect();
        let current: Vec<&StatementContractRow> = table.statements.iter().collect();
        if locked != current {
            return Err(format!(
                "table and lockfile rows differ:\n table={current:#?}\n lock={locked:#?}"
            ));
        }
        for entry in &lock.statements {
            let id = format!("{}/{}", entry.row.statement, entry.row.form);
            match (&entry.tightening, entry.row.accepts_new) {
                (None, false) => {
                    return Err(format!(
                        "{id}: refused for new acts without a tightening record"
                    ))
                }
                (Some(_), true) => {
                    return Err(format!(
                        "{id}: tightening recorded on a form still accepted for new acts"
                    ))
                }
                (Some(t), false) => {
                    if t.mode != "staged" && t.mode != "immediate" {
                        return Err(format!("{id}: unknown tightening mode {:?}", t.mode));
                    }
                    if t.reason.trim().is_empty() {
                        return Err(format!("{id}: tightening has an empty reason"));
                    }
                    if t.contract_version == 0 || t.contract_version > table.contract_version {
                        return Err(format!(
                            "{id}: tightening contractVersion {} outside 1..={}",
                            t.contract_version, table.contract_version
                        ));
                    }
                }
                (None, true) => {}
            }
        }
        Ok(())
    }

    #[test]
    fn lockfile_pins_the_table() {
        if let Err(why) = check_lock(&contract(), LOCKFILE) {
            panic!(
                "statement-contract.lock.json does not match STATEMENT_CONTRACT: {why}\n\
                 A form leaving accepts_new needs a reviewed tightening line \
                 (mode staged|immediate, a reason, the contractVersion it landed in)."
            );
        }
    }

    fn lock_value() -> serde_json::Value {
        serde_json::from_str(LOCKFILE).unwrap()
    }

    fn row_mut<'a>(v: &'a mut serde_json::Value, form: &str) -> &'a mut serde_json::Value {
        v["statements"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["form"] == form)
            .unwrap()
    }

    #[test]
    fn lockfile_check_refuses_drift_and_undeclared_tightening() {
        let table = contract();
        let check = |v: &serde_json::Value| check_lock(&table, &v.to_string());

        // A table row moving without the lockfile line moving with it.
        let mut moved = table.clone();
        moved.statements[1].accepts_new = false;
        assert!(check_lock(&moved, LOCKFILE).unwrap_err().contains("differ"));

        // The lockfile disagreeing with the table on any field.
        for field in ["issues", "accepts_new", "honors_historical"] {
            let mut v = lock_value();
            let flipped = !row_mut(&mut v, GRANT_ISSUED)[field].as_bool().unwrap();
            row_mut(&mut v, GRANT_ISSUED)[field] = flipped.into();
            assert!(check(&v).unwrap_err().contains("differ"), "{field}");
        }
        let mut v = lock_value();
        v["contract_version"] = (CONTRACT_VERSION + 1).into();
        assert!(check(&v).unwrap_err().contains("contract_version"));

        // A refused row with no tightening record.
        let mut v = lock_value();
        row_mut(&mut v, GRANT_LEGACY)
            .as_object_mut()
            .unwrap()
            .remove("tightening");
        assert!(check(&v).unwrap_err().contains("without a tightening"));

        // Empty reason, unknown mode, out-of-range version.
        let mut v = lock_value();
        row_mut(&mut v, GRANT_LEGACY)["tightening"]["reason"] = "  ".into();
        assert!(check(&v).unwrap_err().contains("empty reason"));
        let mut v = lock_value();
        row_mut(&mut v, GRANT_LEGACY)["tightening"]["mode"] = "quietly".into();
        assert!(check(&v).unwrap_err().contains("unknown tightening mode"));
        let mut v = lock_value();
        row_mut(&mut v, GRANT_LEGACY)["tightening"]["contractVersion"] = 0.into();
        assert!(check(&v).unwrap_err().contains("contractVersion"));

        // A tightening on a still-accepted form is a stale or premature line.
        let mut v = lock_value();
        let t = row_mut(&mut v, GRANT_LEGACY)["tightening"].clone();
        row_mut(&mut v, GRANT_ISSUED)["tightening"] = t;
        assert!(check(&v).unwrap_err().contains("still accepted"));

        assert_eq!(check(&lock_value()), Ok(()));
    }

    #[test]
    fn extern_shape_is_snake_case_and_ordered_oldest_first() {
        let json = serde_json::to_value(contract()).unwrap();
        assert_eq!(json["contract_version"], CONTRACT_VERSION);
        let first = &json["statements"][0];
        for key in [
            "statement",
            "form",
            "issues",
            "accepts_new",
            "honors_historical",
        ] {
            assert!(first.get(key).is_some(), "missing {key}");
        }
        assert_eq!(first.as_object().unwrap().len(), 5);
        assert_eq!(lowest_accepted_new(HEAD_DELEGATION), Some(GRANT_ISSUED));
        assert_eq!(lowest_accepted_new(ACCEPTED_CONTENT_HEAD), Some("v2"));
        assert!(!accepts_new(HEAD_DELEGATION, "unknown-form"));
        assert!(!accepts_new("unknown-statement", GRANT_ISSUED));
    }

    #[test]
    fn rows_are_unique() {
        for (i, a) in STATEMENT_CONTRACT.iter().enumerate() {
            for b in &STATEMENT_CONTRACT[i + 1..] {
                assert!(
                    (a.statement, a.form) != (b.statement, b.form),
                    "duplicate row {}/{}",
                    a.statement,
                    a.form
                );
            }
        }
    }
}
