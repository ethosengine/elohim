//! Ordered relay resolution for the device endpoint.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde_json::Value;
use url::Url;

use super::{LEGACY_RELAY_ENV, RELAY_ENV};
use crate::error::PeerError;

const POLICY_REL: &str = "genesis/agentic/pool-policy.json";
const DEFAULT_RELAYS: [&str; 2] = [
    "https://relay.alpha.elohim.host",
    "https://relay.elohim.host",
];
const DEPRECATION: &str = "deprecated: EPR_PEER_RELAY_URL → EPR_PEER_RELAYS";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RelaySource {
    Env,
    LegacyEnv,
    Rulebook,
    Default,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RelayPlan {
    relays: Vec<Url>,
    source: RelaySource,
    third_party: bool,
    disabled: bool,
}

impl RelayPlan {
    fn disabled(source: RelaySource) -> RelayPlan {
        RelayPlan {
            relays: Vec::new(),
            source,
            third_party: false,
            disabled: true,
        }
    }

    fn first(&self) -> Option<Url> {
        if self.disabled {
            None
        } else {
            self.relays.first().cloned()
        }
    }
}

/// Resolve the endpoint's single relay from process overrides and the selected Rulebook.
pub(super) fn first_from_env() -> Result<Option<Url>, PeerError> {
    let primary = std::env::var(RELAY_ENV).ok();
    if non_blank(primary.as_deref()) == Some("off") {
        return Ok(None);
    }
    let legacy = std::env::var(LEGACY_RELAY_ENV).ok();
    let policy = load_rulebook()?;
    let plan = resolve(
        primary.as_deref(),
        legacy.as_deref(),
        policy.as_ref(),
        &mut |line| eprintln!("epr peer: {line}"),
    )?;
    if plan.source == RelaySource::Default {
        eprintln!("epr peer: relay Rulebook absent; using the compiled two-premise default");
    }
    if plan.third_party {
        eprintln!("epr peer: relays include THIRD-PARTY n0 infrastructure");
    }
    Ok(plan.first())
}

fn resolve(
    primary: Option<&str>,
    legacy: Option<&str>,
    policy: Option<&Value>,
    log: &mut impl FnMut(&str),
) -> Result<RelayPlan, PeerError> {
    let primary = non_blank(primary);
    let legacy = non_blank(legacy);
    let (prefix, source) = if let Some(value) = primary {
        if value == "off" {
            return Ok(RelayPlan::disabled(RelaySource::Env));
        }
        (Some(value), RelaySource::Env)
    } else if let Some(value) = legacy {
        log(DEPRECATION);
        (Some(value), RelaySource::LegacyEnv)
    } else if has_rulebook_relays(policy) {
        (None, RelaySource::Rulebook)
    } else {
        (None, RelaySource::Default)
    };

    let mut relays = Vec::new();
    let mut third_party = false;
    if let Some(prefix) = prefix {
        if source == RelaySource::Env {
            append_entries(prefix.split(','), &mut relays, &mut third_party)?;
        } else {
            append_entries([prefix], &mut relays, &mut third_party)?;
        }
    }
    append_rulebook(policy, &mut relays, &mut third_party)?;

    if relays.is_empty() {
        return Err(policy_error(
            "relay resolution produced no relay; use EPR_PEER_RELAYS=off to disable relaying",
        ));
    }

    Ok(RelayPlan {
        relays,
        source,
        third_party,
        disabled: false,
    })
}

fn non_blank(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn has_rulebook_relays(policy: Option<&Value>) -> bool {
    policy
        .and_then(|value| value.pointer("/berth/channel/relays"))
        .is_some()
}

fn append_rulebook(
    policy: Option<&Value>,
    relays: &mut Vec<Url>,
    third_party: &mut bool,
) -> Result<(), PeerError> {
    let Some(value) = policy.and_then(|value| value.pointer("/berth/channel/relays")) else {
        return append_entries(DEFAULT_RELAYS, relays, third_party);
    };
    let entries = value
        .as_array()
        .ok_or_else(|| policy_error("berth.channel.relays must be an array"))?;
    if entries.is_empty() {
        return Err(policy_error("berth.channel.relays must not be empty"));
    }
    let entries = entries
        .iter()
        .map(|entry| {
            let entry = entry
                .as_str()
                .ok_or_else(|| policy_error("every berth.channel.relays entry must be a string"))?;
            if entry.trim().is_empty() {
                return Err(policy_error(
                    "every berth.channel.relays entry must be a non-blank string",
                ));
            }
            Ok(entry)
        })
        .collect::<Result<Vec<_>, _>>()?;
    append_entries(entries, relays, third_party)
}

fn append_entries<'a>(
    entries: impl IntoIterator<Item = &'a str>,
    relays: &mut Vec<Url>,
    third_party: &mut bool,
) -> Result<(), PeerError> {
    for entry in entries {
        let entry = entry.trim();
        if entry.is_empty() {
            return Err(invalid_url(entry));
        }
        if entry == "off" {
            return Err(invalid_url(entry));
        }
        if entry == "n0" {
            *third_party = true;
            for relay in iroh::defaults::prod::default_relay_map().urls() {
                push_unique(relays, Url::from(relay.clone()));
            }
            continue;
        }
        let relay = Url::parse(entry).map_err(|source| PeerError::InvalidRelayUrl {
            value: entry.to_string(),
            source,
        })?;
        push_unique(relays, relay);
    }
    Ok(())
}

fn push_unique(relays: &mut Vec<Url>, relay: Url) {
    if !relays.contains(&relay) {
        relays.push(relay);
    }
}

fn invalid_url(value: &str) -> PeerError {
    let source = Url::parse(value).expect_err("relay control tokens are not URLs");
    PeerError::InvalidRelayUrl {
        value: value.to_string(),
        source,
    }
}

fn policy_error(reason: impl Into<String>) -> PeerError {
    PeerError::Policy {
        path: PathBuf::from(POLICY_REL),
        reason: reason.into(),
    }
}

fn load_rulebook() -> Result<Option<Value>, PeerError> {
    let Some(path) = selected_rulebook().or_else(find_rulebook) else {
        return Ok(None);
    };
    let invalid = |reason: String| PeerError::Policy {
        path: path.clone(),
        reason,
    };
    let text = std::fs::read_to_string(&path).map_err(|error| invalid(error.to_string()))?;
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|error| invalid(error.to_string()))
}

fn selected_rulebook() -> Option<PathBuf> {
    selected_rulebook_from_args(std::env::args_os())
}

fn selected_rulebook_from_args(args: impl IntoIterator<Item = OsString>) -> Option<PathBuf> {
    // The public resolver keeps its no-argument compatibility surface. These are the same CLI
    // flags that select the Rules loaded immediately before the endpoint is configured.
    let args = args.into_iter().collect::<Vec<_>>();
    let (options, root_flag) = match (
        args.get(1).and_then(|arg| arg.to_str()),
        args.get(2).and_then(|arg| arg.to_str()),
    ) {
        (Some("peer"), Some("serve")) => (&args[3..], "--root"),
        (Some("flow"), Some("fold")) => (&args[3..], "--repo"),
        _ => return None,
    };
    if let Some(policy) = last_option(options, "--policy") {
        return Some(policy);
    }
    let root = last_option(options, root_flag).or_else(|| std::env::current_dir().ok())?;
    Some(root.join(POLICY_REL))
}

fn last_option(args: &[OsString], name: &str) -> Option<PathBuf> {
    args.windows(2)
        .filter(|pair| pair[0].to_str() == Some(name))
        .next_back()
        .map(|pair| PathBuf::from(&pair[1]))
}

fn find_rulebook() -> Option<PathBuf> {
    std::env::var_os("EPR_CHECKOUT")
        .map(PathBuf::from)
        .and_then(|root| rulebook_below(&root))
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .and_then(|cwd| cwd.ancestors().find_map(rulebook_below))
        })
}

fn rulebook_below(root: &Path) -> Option<PathBuf> {
    let path = root.join(POLICY_REL);
    path.is_file().then_some(path)
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use serde_json::json;

    use super::*;

    fn policy(relays: Value) -> Value {
        json!({"berth": {"channel": {"relays": relays, "relay_mode": "single"}}})
    }

    fn resolve_for(
        primary: Option<&str>,
        legacy: Option<&str>,
        policy: Option<&Value>,
    ) -> (RelayPlan, Vec<String>) {
        let mut logs = Vec::new();
        let plan = resolve(primary, legacy, policy, &mut |line| {
            logs.push(line.to_string())
        })
        .expect("relay plan");
        (plan, logs)
    }

    fn strings(plan: &RelayPlan) -> Vec<&str> {
        plan.relays.iter().map(Url::as_str).collect()
    }

    #[test]
    fn resolved_unset_or_blank_env_never_disables() {
        let policy = policy(json!([
            "https://relay.alpha.elohim.host",
            "https://relay.elohim.host"
        ]));
        for primary in [None, Some(""), Some("   ")] {
            let (plan, _) = resolve_for(primary, None, Some(&policy));
            assert!(!plan.disabled);
            assert_eq!(
                strings(&plan),
                [
                    "https://relay.alpha.elohim.host/",
                    "https://relay.elohim.host/"
                ]
            );
        }
    }

    #[test]
    fn unset_or_blank_env_never_disables() {
        if std::env::var_os("EPR_RELAY_PUBLIC_RESOLVER_CHILD").is_some() {
            let relay = crate::relay_from_env()
                .expect("public relay resolver")
                .expect("unset and blank overrides retain a relay");
            assert_eq!(relay.as_str(), "https://relay.alpha.elohim.host/");
            let receipt =
                std::env::var_os("EPR_RELAY_PUBLIC_RESOLVER_RECEIPT").expect("child receipt path");
            std::fs::write(receipt, "resolved").expect("child receipt");
            return;
        }

        let checkout = tempfile::tempdir().expect("checkout");
        let policy_path = checkout.path().join(POLICY_REL);
        std::fs::create_dir_all(policy_path.parent().expect("policy parent"))
            .expect("policy parent exists");
        std::fs::write(
            &policy_path,
            serde_json::to_vec(&policy(json!([
                "https://relay.alpha.elohim.host",
                "https://relay.elohim.host"
            ])))
            .expect("policy JSON"),
        )
        .expect("policy file");

        for primary in [None, Some(""), Some("   ")] {
            let receipt = checkout.path().join(format!(
                "receipt-{}",
                primary.unwrap_or("unset").replace(' ', "blank")
            ));
            let mut child = Command::new(std::env::current_exe().expect("test binary"));
            child
                .arg("endpoint::relays::tests::unset_or_blank_env_never_disables")
                .arg("--exact")
                .arg("--nocapture")
                .env("EPR_RELAY_PUBLIC_RESOLVER_CHILD", "1")
                .env("EPR_RELAY_PUBLIC_RESOLVER_RECEIPT", &receipt)
                .env("EPR_CHECKOUT", checkout.path())
                .env_remove(RELAY_ENV)
                .env_remove(LEGACY_RELAY_ENV);
            if let Some(primary) = primary {
                child.env(RELAY_ENV, primary);
            }
            let output = child.output().expect("public resolver child runs");
            assert!(
                output.status.success(),
                "public resolver failed for {primary:?}: stdout={} stderr={}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(receipt.is_file(), "public resolver child did not run");
        }
    }

    #[test]
    fn env_entries_reorder_but_never_shrink_the_rulebook_list() {
        let policy = policy(json!([
            "https://relay.alpha.elohim.host",
            "https://relay.elohim.host"
        ]));
        let (plan, _) = resolve_for(Some("https://relay.elohim.host"), None, Some(&policy));
        assert_eq!(plan.source, RelaySource::Env);
        assert_eq!(
            strings(&plan),
            [
                "https://relay.elohim.host/",
                "https://relay.alpha.elohim.host/"
            ]
        );
    }

    #[test]
    fn off_is_the_only_disable() {
        let policy = policy(json!(["https://relay.alpha.elohim.host"]));
        let (off, _) = resolve_for(Some("off"), None, Some(&policy));
        assert!(off.disabled);
        assert!(off.first().is_none());

        for value in [None, Some(""), Some("n0"), Some("https://relay.example")] {
            let (plan, _) = resolve_for(value, None, Some(&policy));
            assert!(!plan.disabled, "{value:?} must retain a relay");
            assert!(plan.first().is_some());
        }
    }

    #[test]
    fn n0_marks_third_party() {
        let policy = policy(json!(["https://relay.alpha.elohim.host"]));
        let (plan, _) = resolve_for(Some("n0"), None, Some(&policy));
        assert!(plan.third_party);
        assert!(strings(&plan)
            .iter()
            .any(|url| url.contains("relay.n0.iroh.iroh.link")));
        assert!(strings(&plan).contains(&"https://relay.alpha.elohim.host/"));
    }

    #[test]
    fn legacy_url_prefix_logs_deprecation() {
        let policy = policy(json!(["https://relay.alpha.elohim.host"]));
        let (plan, logs) = resolve_for(None, Some("https://legacy-relay.example"), Some(&policy));
        assert_eq!(plan.source, RelaySource::LegacyEnv);
        assert_eq!(strings(&plan)[0], "https://legacy-relay.example/");
        assert_eq!(logs, vec![DEPRECATION.to_string()]);
    }

    #[test]
    fn legacy_url_with_comma_is_one_prefix_entry() {
        let policy = policy(json!(["https://relay.alpha.elohim.host"]));
        let legacy = "https://legacy-relay.example/relay?tag=a,b";
        let (plan, logs) = resolve_for(None, Some(legacy), Some(&policy));
        assert_eq!(plan.source, RelaySource::LegacyEnv);
        assert_eq!(strings(&plan)[0], legacy);
        assert_eq!(logs, vec![DEPRECATION.to_string()]);
    }

    #[test]
    fn missing_rulebook_field_uses_the_compiled_two_premise_default() {
        let (plan, _) = resolve_for(None, None, Some(&json!({"berth": {"channel": {}}})));
        assert_eq!(plan.source, RelaySource::Default);
        assert_eq!(
            strings(&plan),
            [
                "https://relay.alpha.elohim.host/",
                "https://relay.elohim.host/"
            ]
        );
    }

    #[test]
    fn blank_rulebook_entries_are_refused() {
        for entries in [
            json!([""]),
            json!(["", "   "]),
            json!(["https://relay.alpha.elohim.host", " "]),
        ] {
            let error = resolve(None, None, Some(&policy(entries)), &mut |_| {})
                .expect_err("blank Rulebook relay");
            assert!(
                matches!(error, PeerError::Policy { reason, .. } if reason.contains("non-blank"))
            );
        }
    }

    #[test]
    fn cli_rulebook_selection_honors_policy_and_root() {
        let explicit = PathBuf::from("/selected/pool-policy.json");
        assert_eq!(
            selected_rulebook_from_args(
                [
                    "epr",
                    "peer",
                    "serve",
                    "--root",
                    "/checkout-b",
                    "--policy",
                    explicit.to_str().expect("UTF-8 path"),
                ]
                .into_iter()
                .map(OsString::from)
            ),
            Some(explicit)
        );
        assert_eq!(
            selected_rulebook_from_args(
                ["epr", "peer", "serve", "--root", "/checkout-b"]
                    .into_iter()
                    .map(OsString::from)
            ),
            Some(PathBuf::from("/checkout-b").join(POLICY_REL))
        );
        assert_eq!(
            selected_rulebook_from_args(
                ["epr", "flow", "fold", "--repo", "/checkout-c"]
                    .into_iter()
                    .map(OsString::from)
            ),
            Some(PathBuf::from("/checkout-c").join(POLICY_REL))
        );
    }

    #[test]
    fn relay_fields_stay_outside_the_rules_digest_regression_guard() {
        let base = json!({
            "berth": {
                "resources": {
                    "dev-integration": {"capacity": 1, "scope": "steward"}
                },
                "channel": {
                    "crosses": {"dev-berth-events@1": ["claim", "release"]}
                }
            }
        });
        let mut with_relays = base.clone();
        with_relays["berth"]["channel"]["relays"] = json!([
            "https://relay.alpha.elohim.host",
            "https://relay.elohim.host"
        ]);
        with_relays["berth"]["channel"]["relay_mode"] = json!("single");

        let before = crate::Rules::from_policy(&base).expect("base rules");
        let after = crate::Rules::from_policy(&with_relays).expect("rules with relay fields");
        assert_eq!(before.digest(), after.digest());
    }
}
