//! How the code gets back to the terminal that asked.
//!
//! Two paths, chosen by the terminal when it asks:
//!
//! - **Loopback**: the portal redirects the browser to a listener the terminal
//!   opened on `127.0.0.1`. Used when the browser and the terminal share a
//!   machine.
//! - **Paste**: the portal shows the code and the person carries it across.
//!   Used when the terminal is remote (a workspace, an SSH session) and the
//!   browser cannot reach its loopback.
//!
//! The portal never redirects anywhere else for this grant, so there is no
//! redirect address for an attacker to register or spoof.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ReturnPath {
    Loopback { port: u16 },
    Paste,
}

/// What the portal does once the controller has approved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReturnTarget {
    /// Send the browser here.
    Redirect(String),
    /// Show this for the person to paste into their terminal.
    Display(String),
}

/// Separates code from state in the pasted form. Neither side may contain it:
/// both are held to the URL-safe alphabet by [`is_token`].
const PASTE_SEPARATOR: char = '#';

/// Codes and states are opaque tokens over the URL-safe alphabet, so they need
/// no escaping in a redirect and cannot smuggle a separator.
pub fn is_token(text: &str, min_len: usize, max_len: usize) -> bool {
    (min_len..=max_len).contains(&text.len())
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

pub fn return_target(path: ReturnPath, code: &str, state: &str) -> ReturnTarget {
    match path {
        ReturnPath::Loopback { port } => ReturnTarget::Redirect(format!(
            "http://127.0.0.1:{port}/callback?code={code}&state={state}"
        )),
        ReturnPath::Paste => ReturnTarget::Display(format!("{code}{PASTE_SEPARATOR}{state}")),
    }
}

/// Split what a person pasted back into `(code, state)`. Surrounding
/// whitespace is forgiven; anything else malformed is `None`.
pub fn parse_pasted(text: &str) -> Option<(&str, &str)> {
    let (code, state) = text.trim().split_once(PASTE_SEPARATOR)?;
    (!code.is_empty() && !state.is_empty() && !state.contains(PASTE_SEPARATOR))
        .then_some((code, state))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_redirects_to_the_terminals_listener() {
        assert_eq!(
            return_target(ReturnPath::Loopback { port: 49152 }, "c0de", "st8"),
            ReturnTarget::Redirect("http://127.0.0.1:49152/callback?code=c0de&state=st8".into())
        );
    }

    #[test]
    fn paste_round_trips_through_a_person() {
        let ReturnTarget::Display(shown) = return_target(ReturnPath::Paste, "c0de", "st8") else {
            panic!("paste shows, it does not redirect");
        };
        assert_eq!(parse_pasted(&format!("  {shown}\n")), Some(("c0de", "st8")));
    }

    #[test]
    fn malformed_pastes_are_refused() {
        for bad in ["", "c0de", "#st8", "c0de#", "a#b#c"] {
            assert_eq!(parse_pasted(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn tokens_hold_to_the_url_safe_alphabet() {
        assert!(is_token("abc-DEF_123", 1, 64));
        for bad in ["a#b", "a b", "a&b", "a=b", "a/b", ""] {
            assert!(!is_token(bad, 1, 64), "{bad:?}");
        }
        assert!(!is_token("abc", 4, 64));
    }

    #[test]
    fn the_wire_form_is_tagged() {
        assert_eq!(
            serde_json::to_string(&ReturnPath::Loopback { port: 8123 }).unwrap(),
            r#"{"kind":"loopback","port":8123}"#
        );
        assert_eq!(
            serde_json::to_string(&ReturnPath::Paste).unwrap(),
            r#"{"kind":"paste"}"#
        );
    }
}
