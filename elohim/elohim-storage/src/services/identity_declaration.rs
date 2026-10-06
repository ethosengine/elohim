//! The node's identity, declared in a file, and the reconcile that applies it.
//!
//! `ELOHIM_IDENTITY_DECLARATION_PATH` names a TOML file holding a
//! [`consent_grant::Declaration`]: the person this node begins an identity for,
//! the devices that person expects, and, on an asking node, the node that approves it. It
//! sits beside the runtime config (`ELOHIM_RUNTIME_CONFIG_PATH`) and follows its
//! convention: a path named by the environment, read on the same ten-second
//! cadence, re-read when its (mtime, length) changes, nothing at all when unset.
//! It is a sibling rather than a section of that file because the runtime
//! config is a flat `KEY = value` list of registered operator flags whose
//! scanner ignores `[section]` headers, and a declaration has structure (a list
//! of devices) and is not a flag.
//!
//! WHY A FILE MAY MAKE THE NODE SIGN. The node signs as its person only for a
//! caller on its own machine. Writing a file on the node's own disk is such an
//! act: whoever can write it could equally run `epr identity begin` there. So
//! reconciling the declaration begins an identity exactly as that verb would,
//! and does nothing a person at the machine could not. The file holds no
//! secret, and a declaration never approves a device on its own: a device must
//! still ask, and the code stays bound to its request and PKCE verifier.
//!
//! What the reconcile does, at start and whenever the file changes:
//! - no identity on the node → create the Human and record its authority
//!   (`begin_with`), as `POST /auth/identity/begin` would;
//! - an identity that agrees with the declaration → nothing;
//! - an identity that disagrees → nothing, and a WARN naming
//!   `identity_declaration_disagrees`. An existing identity is never
//!   overwritten or re-keyed.

use std::path::{Path, PathBuf};

use consent_grant::{Declaration, DeclarationConflict};

use super::device_consent::{
    begin_with, BeginInput, Begun, CellFailure, CellStanding, ControllerCell,
};

/// The environment variable naming the declaration file.
pub const PATH_ENV: &str = "ELOHIM_IDENTITY_DECLARATION_PATH";

/// The declaration file, when one is named.
pub fn path() -> Option<PathBuf> {
    std::env::var_os(PATH_ENV)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// Read and parse the declaration at `path`.
pub fn read(path: &Path) -> Result<Declaration, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse(&text)
}

/// Parse a declaration's text. Unknown keys are refused, so a misspelt key is
/// an error rather than a silently ignored wish.
pub fn parse(text: &str) -> Result<Declaration, String> {
    toml::from_str(text).map_err(|e| format!("identity_declaration_unreadable: {e}"))
}

/// What one reconcile did.
#[derive(Debug)]
pub enum Reconciled {
    /// The declaration names no identity; nothing to do.
    Undeclared,
    /// The identity is what the declaration says; `begun` reports what, if
    /// anything, had to be created.
    Applied(Box<Begun>),
    /// An identity exists and is not the declared one. Kept as it is.
    Disagrees(DeclarationConflict),
    /// The node could not be asked, or refused; tried again on the next tick.
    Failed(String),
}

impl Reconciled {
    /// Stable code for the log and the declaration route.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Undeclared => "identity_undeclared",
            Self::Applied(b) if b.human_created || b.authority_created => "identity_begun",
            Self::Applied(_) => "identity_as_declared",
            Self::Disagrees(c) => c.code(),
            Self::Failed(_) => "identity_reconcile_failed",
        }
    }

    /// Whether the same file should be tried again without a change.
    pub fn retry(&self) -> bool {
        matches!(self, Self::Failed(_))
    }
}

fn failure_text(failure: CellFailure) -> String {
    match failure {
        CellFailure::Unavailable(d) | CellFailure::Refused(d) => d,
    }
}

/// Bring the node's identity to what `declaration` says, without ever
/// changing an identity that exists.
pub async fn reconcile(cell: &dyn ControllerCell, declaration: &Declaration) -> Reconciled {
    let Some(declared) = &declaration.identity else {
        return Reconciled::Undeclared;
    };
    // An existing Human must be the declared person before anything is done
    // for it, including recording its authority.
    match cell.standing().await {
        Ok(CellStanding::NoPerson) => {}
        Ok(_) => match cell.my_human().await {
            Ok(Some(existing)) => {
                if let Err(conflict) = declared.agrees_with(&existing) {
                    return Reconciled::Disagrees(conflict);
                }
            }
            Ok(None) => {}
            Err(f) => return Reconciled::Failed(failure_text(f)),
        },
        Err(f) => return Reconciled::Failed(failure_text(f)),
    }
    let input = BeginInput {
        display_name: declared.display_name.clone(),
        human_id: declared.human_id.clone(),
        identifier: declared.identifier.clone(),
        profile_reach: Some(declared.reach().to_string()),
        secret: None,
    };
    match begin_with(Some(cell), input).await {
        Ok(begun) => Reconciled::Applied(Box::new(begun)),
        Err(refused) => Reconciled::Failed(format!("the node refused: HTTP {}", refused.status())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::device_consent::tests::FakeCell;
    use crate::services::device_consent::NewHuman;

    fn declaration(name: &str) -> Declaration {
        parse(&format!(
            "[identity]\ndisplayName = \"{name}\"\nidentifier = \"matthew\"\n\n\
             [[devices]]\nlabel = \"home\"\ndeviceKey = \"uhCAkiqczpYdyymsibsOjupr1Fx31_cPHLgzxqwv9-3FOtATGMzzl\"\n\
             acts = [\"device.enroll\"]\n"
        ))
        .unwrap()
    }

    #[tokio::test]
    async fn an_absent_identity_is_begun_as_declared() {
        let cell = FakeCell::new(Ok(CellStanding::NoPerson));
        let done = reconcile(&cell, &declaration("Matthew")).await;
        assert_eq!(done.code(), "identity_begun", "{done:?}");
        let Reconciled::Applied(begun) = done else {
            unreachable!()
        };
        assert_eq!(begun.identifier, "matthew");
        let humans = cell.humans.lock().unwrap().clone();
        assert_eq!(humans[0].display_name, "Matthew");
        assert_eq!(humans[0].profile_reach, "private");
        assert_eq!(cell.bootstraps.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn a_present_identity_that_agrees_is_left_untouched() {
        let cell = FakeCell::new(Ok(CellStanding::NoPerson));
        reconcile(&cell, &declaration("Matthew")).await;
        let again = reconcile(&cell, &declaration("Matthew")).await;
        assert_eq!(again.code(), "identity_as_declared", "{again:?}");
        assert_eq!(cell.humans.lock().unwrap().len(), 1);
        assert_eq!(cell.bootstraps.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn a_declaration_that_disagrees_is_refused_and_changes_nothing() {
        let cell = FakeCell::new(Ok(CellStanding::Unbootstrapped {
            identity_root: "uhCkkRrENFlI2RlXCelrj6C6ttNq8qTI_wSh0fFmsrSvBgdkkM39j".into(),
        }));
        cell.humans.lock().unwrap().push(NewHuman {
            id: "someone-else".into(),
            display_name: "Someone".into(),
            profile_reach: "private".into(),
        });
        let refused = reconcile(&cell, &declaration("Matthew")).await;
        assert_eq!(refused.code(), "identity_declaration_disagrees");
        assert!(!refused.retry());
        // Not even the existing Human's authority is recorded for it.
        assert!(cell.bootstraps.lock().unwrap().is_empty());
        assert_eq!(cell.humans.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn no_declared_identity_means_nothing_to_do() {
        let cell = FakeCell::new(Ok(CellStanding::NoPerson));
        let undeclared =
            parse("[asking]\nlabel = \"home\"\nportal = \"http://127.0.0.1:8191\"\n").unwrap();
        assert_eq!(
            reconcile(&cell, &undeclared).await.code(),
            "identity_undeclared"
        );
        assert!(cell.humans.lock().unwrap().is_empty());
    }

    #[test]
    fn a_misspelt_key_is_refused_not_ignored() {
        let refused = parse("[identity]\ndisplayname = \"Matthew\"\n").unwrap_err();
        assert!(
            refused.starts_with("identity_declaration_unreadable"),
            "{refused}"
        );
        assert!(parse("[identity]\ndisplayName = \"M\"\npassword = \"x\"\n").is_err());
    }
}
