//! Read open landing-gate misses from local and admitted crossing flow records.

use std::collections::{BTreeMap, BTreeSet};

use cid::Cid;
use elohim_epr_rea::{FlowEvent, FlowRecord};
use serde::Serialize;

use super::via_device;

const LANDING_MEASURE: &str = "dev-landing@1";
const GATE_VERDICT_MEASURE: &str = "gate-verdict@1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenMiss {
    pub intent: String,
    pub event: String,
    pub announcer: Option<String>,
    pub change: String,
    pub commit: String,
    pub git_ref: String,
    pub project: String,
    pub closure: String,
    pub p90_s: Option<u64>,
    pub wait_until: Option<u64>,
    pub at: Option<String>,
}

type MissKey = (Option<String>, String, String, String);
type IntentKey = (Option<String>, String);

/// Project every miss that remains open after folding the supplied records.
pub fn open_misses(
    records: &[(Cid, FlowRecord)],
    admits: &dyn Fn(&str) -> bool,
    me: Option<&str>,
) -> Vec<OpenMiss> {
    let mut misses = BTreeMap::<MissKey, OpenMiss>::new();
    let mut seen_events = BTreeSet::new();
    let mut verdicts = BTreeSet::<(String, String)>::new();
    let mut settled = BTreeSet::<IntentKey>::new();
    let mut intents = BTreeMap::<IntentKey, Option<String>>::new();

    for (cid, record) in records {
        match record {
            FlowRecord::Intent(intent) => {
                let slots = &intent.resource_spec.classified_as;
                if slots.first().map(String::as_str) == Some("dev-landing:intent") {
                    intents.insert(
                        (None, cid.to_string()),
                        one_value(slots.iter().map(String::as_str), "supersedes:")
                            .map(str::to_string),
                    );
                }
            }
            FlowRecord::Event(event) => {
                let Some(announcer) = eligible_announcer(event, admits, me) else {
                    continue;
                };
                let measure = one_slot(event, "measure:");
                let kind = one_slot(event, "reason:");
                if measure == Some(GATE_VERDICT_MEASURE) && kind == Some("verdict") {
                    let env = event_env(event);
                    if let (Some(project), Some(closure)) = (env.get("project"), env.get("closure"))
                    {
                        verdicts.insert(((*project).to_string(), (*closure).to_string()));
                    }
                    continue;
                }
                if measure != Some(LANDING_MEASURE) {
                    continue;
                }
                let intent = event.resource.to_string();
                match kind {
                    Some("misses") if seen_events.insert(*cid) => {
                        collect_misses(&mut misses, cid, event, announcer, &intent);
                    }
                    Some("landed" | "withdraw" | "dismiss") => {
                        settled.insert((announcer, intent));
                    }
                    Some("intent") => {
                        let env = event_env(event);
                        intents.insert(
                            (announcer, intent),
                            env.get("supersedes").map(|value| (*value).to_string()),
                        );
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    let superseded = live_superseded(&intents, &settled);
    misses.retain(|(announcer, intent, project, closure), _| {
        !verdicts.contains(&(project.clone(), closure.clone()))
            && !settled.contains(&(announcer.clone(), intent.clone()))
            && !superseded.contains(&(announcer.clone(), intent.clone()))
    });
    misses.into_values().collect()
}

fn eligible_announcer(
    event: &FlowEvent,
    admits: &dyn Fn(&str) -> bool,
    me: Option<&str>,
) -> Option<Option<String>> {
    match via_device(event) {
        Some(writer) if me == Some(writer) || !admits(writer) => None,
        Some(writer) => Some(Some(writer.to_string())),
        None => Some(None),
    }
}

fn collect_misses(
    misses: &mut BTreeMap<MissKey, OpenMiss>,
    cid: &Cid,
    event: &FlowEvent,
    announcer: Option<String>,
    intent: &str,
) {
    let env = event_env(event);
    let (Some(change), Some(commit), Some(git_ref), Some(raw_misses)) = (
        env.get("change"),
        env.get("commit"),
        env.get("ref"),
        env.get("miss"),
    ) else {
        return;
    };
    let p90 = joined_pairs(env.get("p90").copied().unwrap_or_default());
    let wait_until = env.get("wait_until").and_then(|value| value.parse().ok());
    let at = env.get("at").map(|value| (*value).to_string());
    for (project, closure) in joined_pairs(raw_misses) {
        let key = (
            announcer.clone(),
            intent.to_string(),
            project.clone(),
            closure.clone(),
        );
        let p90_s = p90.get(&project).and_then(|value| value.parse().ok());
        if let Some(existing) = misses.get_mut(&key) {
            if p90_s.is_some() {
                existing.p90_s = p90_s;
            }
            if wait_until.is_some() {
                existing.wait_until = wait_until;
            }
            continue;
        }
        misses.insert(
            key,
            OpenMiss {
                intent: intent.to_string(),
                event: cid.to_string(),
                announcer: announcer.clone(),
                change: (*change).to_string(),
                commit: (*commit).to_string(),
                git_ref: (*git_ref).to_string(),
                project,
                closure,
                p90_s,
                wait_until,
                at: at.clone(),
            },
        );
    }
}

fn live_superseded(
    intents: &BTreeMap<IntentKey, Option<String>>,
    settled: &BTreeSet<IntentKey>,
) -> BTreeSet<IntentKey> {
    let directly_superseded: BTreeSet<_> = intents
        .iter()
        .filter_map(|((announcer, _), prior)| Some((announcer.clone(), prior.as_ref()?.clone())))
        .collect();
    intents
        .iter()
        .filter(|(key, _)| !settled.contains(*key) && !directly_superseded.contains(*key))
        .filter_map(|((announcer, _), prior)| Some((announcer.clone(), prior.as_ref()?.clone())))
        .collect()
}

fn event_env(event: &FlowEvent) -> BTreeMap<&str, &str> {
    event
        .classified_as
        .iter()
        .filter_map(|slot| slot.strip_prefix("env:")?.split_once('='))
        .collect()
}

fn one_slot<'a>(event: &'a FlowEvent, prefix: &str) -> Option<&'a str> {
    one_value(event.classified_as.iter().map(String::as_str), prefix)
}

fn one_value<'a>(values: impl Iterator<Item = &'a str>, prefix: &str) -> Option<&'a str> {
    let mut matching = values.filter_map(|value| value.strip_prefix(prefix));
    let value = matching.next()?;
    matching.next().is_none().then_some(value)
}

fn joined_pairs(raw: &str) -> BTreeMap<String, String> {
    raw.split(',')
        .filter_map(|pair| pair.split_once('='))
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use elohim_epr_rea::{atom_cid, AgentRef, FlowEvent, Magnitude, ReaVerb};

    use super::*;

    const CHANGE: &str = "I0123456789abcdef0123456789abcdef01234567";
    const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
    const GIT_REF: &str = "refs/heads/self/f4-ask";
    const SIBLING: &str = "did:key:sibling";
    const ME: &str = "did:key:me";

    fn cid(seed: &str) -> Cid {
        atom_cid(&seed.to_string()).unwrap()
    }

    fn event(
        measure: &str,
        kind: &str,
        resource: Cid,
        env: &[(&str, &str)],
        via: Option<&str>,
    ) -> FlowRecord {
        let mut classified_as = vec![
            format!("test:{kind}"),
            CHANGE.to_string(),
            format!("reason:{kind}"),
            format!("measure:{measure}"),
            "value:1".to_string(),
        ];
        classified_as.extend(env.iter().map(|(key, value)| format!("env:{key}={value}")));
        classified_as.extend(via.map(|writer| format!("via:{writer}")));
        FlowRecord::Event(FlowEvent {
            action: ReaVerb::Cite,
            provider: AgentRef("agent:test".to_string()),
            receiver: AgentRef("repo:test".to_string()),
            resource,
            quantity: Magnitude::Count {
                value: 1.0,
                unit: "landings".to_string(),
            },
            process: None,
            in_scope_of: cid("scope"),
            fulfills: Vec::new(),
            satisfies: Vec::new(),
            occurred_at: "2026-11-23T00:00:00Z".to_string(),
            classified_as,
        })
    }

    fn misses(
        intent: Cid,
        miss: &str,
        p90: Option<&str>,
        wait_until: Option<&str>,
        at: &str,
        via: Option<&str>,
    ) -> FlowRecord {
        let mut env = vec![
            ("change", CHANGE),
            ("commit", COMMIT),
            ("ref", GIT_REF),
            ("miss", miss),
            ("at", at),
        ];
        if let Some(p90) = p90 {
            env.push(("p90", p90));
        }
        if let Some(wait_until) = wait_until {
            env.push(("wait_until", wait_until));
        }
        event(LANDING_MEASURE, "misses", intent, &env, via)
    }

    fn verdict(project: &str, closure: &str, success: &str, via: Option<&str>) -> FlowRecord {
        event(
            GATE_VERDICT_MEASURE,
            "verdict",
            cid("verdict-resource"),
            &[
                ("project", project),
                ("closure", closure),
                ("success", success),
            ],
            via,
        )
    }

    fn admitted(_: &str) -> bool {
        true
    }

    #[test]
    fn replayed_misses_event_yields_one_row() {
        let intent = cid("replay-intent");
        let event = misses(
            intent,
            "eprfs=abcdef",
            Some("eprfs=42"),
            Some("1792769289"),
            "1792769200.123",
            None,
        );
        let event_cid = cid("replay-event");
        let records = vec![(event_cid, event.clone()), (event_cid, event)];

        let rows = open_misses(&records, &admitted, None);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].intent, intent.to_string());
        assert_eq!(rows[0].event, event_cid.to_string());
        assert_eq!(rows[0].announcer, None);
        assert_eq!(rows[0].change, CHANGE);
        assert_eq!(rows[0].commit, COMMIT);
        assert_eq!(rows[0].git_ref, GIT_REF);
        assert_eq!(rows[0].project, "eprfs");
        assert_eq!(rows[0].closure, "abcdef");
        assert_eq!(rows[0].p90_s, Some(42));
        assert_eq!(rows[0].wait_until, Some(1_792_769_289));
        assert_eq!(rows[0].at.as_deref(), Some("1792769200.123"));
    }

    #[test]
    fn re_raised_miss_keeps_first_event_and_latest_optional_values() {
        let intent = cid("reraised-intent");
        let first_cid = cid("reraised-first");
        let records = vec![
            (
                first_cid,
                misses(
                    intent,
                    "eprfs=abcdef",
                    Some("eprfs=10"),
                    Some("100"),
                    "1.000",
                    Some(SIBLING),
                ),
            ),
            (
                cid("reraised-second"),
                misses(
                    intent,
                    "eprfs=abcdef",
                    Some("eprfs=20"),
                    Some("200"),
                    "2.000",
                    Some(SIBLING),
                ),
            ),
            (
                cid("reraised-third"),
                misses(intent, "eprfs=abcdef", None, None, "3.000", Some(SIBLING)),
            ),
        ];

        let rows = open_misses(&records, &admitted, Some(ME));

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].event, first_cid.to_string());
        assert_eq!(rows[0].at.as_deref(), Some("1.000"));
        assert_eq!(rows[0].p90_s, Some(20));
        assert_eq!(rows[0].wait_until, Some(200));
    }

    #[test]
    fn green_verdict_settles_a_miss() {
        assert!(rows_with_verdict("true", "abcdef").is_empty());
    }

    #[test]
    fn red_verdict_settles_a_miss() {
        assert!(rows_with_verdict("false", "abcdef").is_empty());
    }

    #[test]
    fn verdict_for_another_closure_does_not_settle() {
        assert_eq!(rows_with_verdict("true", "fedcba").len(), 1);
    }

    fn rows_with_verdict(success: &str, closure: &str) -> Vec<OpenMiss> {
        let intent = cid("verdict-intent");
        open_misses(
            &[
                (
                    cid("verdict-miss"),
                    misses(intent, "eprfs=abcdef", None, None, "1", None),
                ),
                (
                    cid("verdict"),
                    verdict("eprfs", closure, success, Some(SIBLING)),
                ),
            ],
            &admitted,
            Some(ME),
        )
    }

    #[test]
    fn landed_withdraw_and_dismiss_settle_only_the_same_announcer() {
        for kind in ["landed", "withdraw", "dismiss"] {
            let intent = cid(&format!("settled-{kind}"));
            let miss = misses(intent, "eprfs=abcdef", None, None, "1", Some(SIBLING));
            let settlement = event(LANDING_MEASURE, kind, intent, &[], Some(SIBLING));
            assert!(open_misses(
                &[(cid("settled-miss"), miss.clone()), (cid(kind), settlement)],
                &admitted,
                Some(ME),
            )
            .is_empty());

            let other = event(LANDING_MEASURE, kind, intent, &[], Some("did:key:other"));
            assert_eq!(
                open_misses(
                    &[
                        (cid("unsettled-miss"), miss),
                        (cid("other-settlement"), other)
                    ],
                    &admitted,
                    Some(ME),
                )
                .len(),
                1
            );
        }
    }

    #[test]
    fn live_successor_from_the_same_announcer_settles_a_miss() {
        let intent = cid("superseded-intent");
        let successor = cid("successor-intent");
        let successor_event = event(
            LANDING_MEASURE,
            "intent",
            successor,
            &[("supersedes", &intent.to_string())],
            Some(SIBLING),
        );
        let records = vec![
            (
                cid("superseded-miss"),
                misses(intent, "eprfs=abcdef", None, None, "1", Some(SIBLING)),
            ),
            (cid("successor-event"), successor_event),
        ];

        assert!(open_misses(&records, &admitted, Some(ME)).is_empty());
    }

    #[test]
    fn wait_until_in_the_past_does_not_settle() {
        let intent = cid("past-wait-intent");
        let records = vec![(
            cid("past-wait-event"),
            misses(intent, "eprfs=abcdef", None, Some("1"), "1", None),
        )];

        let rows = open_misses(&records, &admitted, None);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].wait_until, Some(1));
    }

    #[test]
    fn unadmitted_via_writer_is_ignored() {
        let intent = cid("unadmitted-intent");
        let records = vec![(
            cid("unadmitted-event"),
            misses(intent, "eprfs=abcdef", None, None, "1", Some(SIBLING)),
        )];

        assert!(open_misses(&records, &|_| false, Some(ME)).is_empty());
    }

    #[test]
    fn unadmitted_verdict_does_not_settle() {
        let intent = cid("unadmitted-verdict-intent");
        let records = vec![
            (
                cid("unadmitted-verdict-miss"),
                misses(intent, "eprfs=abcdef", None, None, "1", None),
            ),
            (
                cid("unadmitted-verdict"),
                verdict("eprfs", "abcdef", "true", Some(SIBLING)),
            ),
        ];

        assert_eq!(open_misses(&records, &|_| false, Some(ME)).len(), 1);
    }

    #[test]
    fn via_equal_to_me_is_ignored() {
        let intent = cid("loopback-intent");
        let records = vec![(
            cid("loopback-event"),
            misses(intent, "eprfs=abcdef", None, None, "1", Some(ME)),
        )];

        assert!(open_misses(&records, &admitted, Some(ME)).is_empty());
    }
}
