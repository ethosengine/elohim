//! Discoverable request contracts; examples require caller-authored evidence and judgment.
use eprfs_agent::memory::{Collective, Contribution, FileRef, Reach};
use serde_json::{json, Value};

pub fn input_guide(reference: &FileRef, collective: &Collective) -> Value {
    let example = Contribution {
        version: 1,
        collective: reference.clone(),
        author: "REQUIRED: current registered session identity".into(),
        steward: collective.steward.clone(),
        scope: "workspace".into(),
        reach: Reach::Workspace,
        concern: "REQUIRED: named concern".into(),
        claim: "REQUIRED: qualified assertion".into(),
        uncertainty: vec!["REQUIRED: limitations or evidenced absence of uncertainty".into()],
        sources: vec![],
        supersedes: vec![],
        contradicts: vec![],
        imported: None,
    };
    json!({
        "transport":"Author a UTF-8 JSON request in an allowed repository path; use --input PATH. Examples below are input guidance, not executable actions.",
        "fileRef":{"path":"Repository-relative normalized path; no symlinks", "cid":"Exact raw BlobCid from memory pin --input PATH"},
        "source":{"resource":"FileRef", "reach":"private|workspace|repository, checked against most-specific source policy"},
        "common":{"version":1,"collective":reference,"unknownFields":"refused"},
        "contribute":{"readOnly":false,"needs":"Registered --session; author must match claim. Local honor-system attribution, not authentication.","example":example,"sources":"Required 1..8 Source records"},
        "project":{"readOnly":true,"fields":{"purpose":"nonempty string","audience":"private|workspace|repository","inputs":"1..16 FileRefs to recorded contributions","omissions":"array of strings"}},
        "feedback":{"readOnly":false,"needs":"Registered --session; target bytes must remain saved and retrievable.","fields":{"target":"FileRef","kind":"stale-source|misleading-projection|omitted-contradiction|poor-selection","passage":"exact substring of target bytes","reason":"nonempty explanation"}},
        "graduate":{"readOnly":true,"fields":{"contribution":"FileRef","review":"exact independent native approved verdict event CID on contribution","audience":"repository"},"meaning":"Local rehearsal only, never publication or experiential acceptance"},
        "policyDefaults":{"sourceRule":"Longest component-prefix match; unknown roots and duplicate rule paths refused","selection":"Only explicit pins, no latest-version selection","limits":{"sourceFiles":32,"sourceBytes":262144,"projectionBytes":24576,"sidecarBytes":33554432}},
        "retention":"No projection output is saved by this CLI. Save exact output explicitly, then pin saved bytes before feedback or a consequential decision. ReceiptCid addresses only canonical receipt JSON, not saved full output."
    })
}
