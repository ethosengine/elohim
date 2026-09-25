//! The governance card — governance legible where the consumer is.
//!
//! `elohim-governance.json` is what a stranger who finds the Jenkins pipeline reads to DISCOVER who
//! governs the gift, EXPLORE its terms and two-sided disclosure, VERIFY every claim with commands
//! they can run, and GIVE FEEDBACK as a note against the offer's CID. It renders whatever the offer's
//! standing is — legibility needs no permission — and prints the stakes verbatim, so an approval
//! resting on a fixture co-steward can never read as network validation.

use serde_json::{json, Value};

use crate::drift::DriftReport;
use crate::observe::{Context, Result, Standing};
use crate::translate::Translation;

/// The last observed build, when the card is rendered with one.
pub struct LastObserved<'a> {
    pub translation: &'a Translation,
    pub drift: &'a DriftReport,
}

/// Render the card. `stewards` is the collective's steward report (`epr flow memory collective`).
pub fn render(ctx: &Context, stewards: Value, last: Option<LastObserved<'_>>) -> Result<Value> {
    let offer = ctx.offer.declaration();
    let cid = ctx.offer_cid()?.to_string();
    let standing = ctx.standing()?;
    let (state, validated_at, approval, missing) = match &standing {
        Standing::Unminted { .. } => (
            "unminted".to_string(),
            Value::Null,
            Value::Null,
            json!(
                "the offer is not minted in this sidecar; `jenkins-bridge offer mint` proposes it"
            ),
        ),
        Standing::Minted(s) => (
            serde_json::to_value(s.state)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default(),
            s.approval
                .as_ref()
                .map_or(Value::Null, |a| json!(a.validated_at)),
            serde_json::to_value(&s.approval).unwrap_or(Value::Null),
            json!(s.missing),
        ),
    };
    let governed_by = format!(
        "governed by {} · offer {} · {}",
        offer.provider,
        cid,
        match &standing {
            Standing::Minted(s) if s.is_active() => s
                .approval
                .as_ref()
                .map_or("unapproved", |a| a.validated_at)
                .to_string(),
            Standing::Minted(s) => format!("{:?}", s.state).to_lowercase(),
            Standing::Unminted { .. } => "unminted".to_string(),
        }
    );
    let last_gap = last.as_ref().map(|l| {
        json!({
            "build": l.translation.build,
            "url": l.translation.url,
            "sha": l.translation.sha,
            "orchestratorRun": l.translation.orchestrator_run,
            "basis": l.translation.basis,
            "process": l.translation.process_cid.to_string(),
            "clean": l.drift.is_clean(),
            "findings": l.drift.findings.iter().map(|f| f.line()).collect::<Vec<_>>(),
        })
    });
    let mut verify = vec![
        format!("epr flow walk {cid}"),
        format!("epr flow walk {}", ctx.offer.path()),
        format!("epr flow memory collective --input {}", ctx.offer.path()),
    ];
    if let Some(l) = &last {
        verify.push(format!("epr flow walk {}", l.translation.process_cid));
        verify.push(format!(
            "jenkins-bridge drift --stages <edge-{}.wfapi.json>{}",
            l.translation.build,
            l.translation
                .orchestrator_run
                .as_ref()
                .map_or(String::new(), |r| format!(
                    " --graph <orchestrator-{r}.actual-build-graph.json>"
                ))
        ));
    }
    Ok(json!({
        "cardVersion": 1,
        "governedBy": offer.provider,
        "buildDescription": governed_by,
        "stewards": stewards,
        "offer": {
            "id": offer.id,
            "cid": cid,
            "path": ctx.offer.path(),
            "provider": offer.provider,
            "receiver": offer.receiver,
            "author": offer.author,
            "state": state,
            "validatedAt": validated_at,
            "approval": approval,
            "missing": missing,
            "stakes": standing.line(),
            "offered": offer.offered,
            "terms": offer.terms,
            "crossesNetwork": offer.crosses_network,
        },
        "disclosure": offer.disclosure,
        "recipe": {
            "id": ctx.recipe.id,
            "version": ctx.recipe.version,
            "stages": ctx.recipe.stages.len(),
        },
        "lastObservedGap": last_gap,
        "verify": verify,
        "feedback": {
            "route": format!(
                "epr flow note {cid} --kind observation --reason \"<what you saw>\" --session <your session>"
            ),
            "contest": format!(
                "a Steward withdraws the gift by the same process it was approved: epr flow note {cid} --kind verdict --verdict changes-requested --reason \"<why>\" --session <steward session>"
            ),
            "note": "Feedback is an observation against the offer CID. Feedback without an identity is a low-reach unattested observation: counted and visible, and it can never withdraw an offer on its own.",
        },
        "rules": [
            "Gifts flow out: refusing any gift is free.",
            "Observations flow in only as Process + FlowEvent, provider service:jenkins (provenance, never payee).",
            "The offer creates no claim and implies no settlement.",
            "Authority is never the consumer's: an offer is active only on a distinct Steward's verdict.",
        ],
    }))
}
