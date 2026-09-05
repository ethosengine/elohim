//! @dna-scope: node_registry
//! @concern:happ-lineage-migration — de-risk probes A and B.
//!
//! Spec: `genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md`
//! §2 (the notarization-carrying record), §3 (design gate), §11.1 (probes).
//!
//! **Probe A** — the core claim. Two versions of node_registry (v1 as it stood
//! before the integrity change; v2 with the `NotarizationWitness` entry type)
//! are installed as two apps in ONE conductor under the SAME agent key. A real
//! node_registry record is authored on v1; the identical entry is re-created
//! natively on v2 and keeps its EntryHash; the v1 action + signature are then
//! carried into v2 as a `NotarizationWitness` and accepted by v2's own
//! validation, which re-verifies the v1 signature with no access to v1.
//! Two negatives prove the validation is real: a flipped signature byte and a
//! `lineage_dna_hash` that is not in v2's declared lineage are both refused.
//!
//! **Probe B** — a LATE `open_chain`. v2 has already authored several actions
//! when v1 closes its chain toward v2 and v2 records the crossing. Station 8's
//! only unmeasured Holochain rule.
//!
//! **Probe B2** — the REMOTE agent-activity authority. Probe B found that
//! `close_chain` is not a source-chain guard and that the author's OWN
//! authority (a single conductor) accepts the post-close write. B2 puts a
//! second conductor on the same v1 DHT and reads that peer's verdict.
//!
//! Artifacts. The `NotarizationWitness` entry type (+ `EntryToWitness` link,
//! `commit_witness` / `get_witnesses_for` externs) is gated behind the
//! `lineage-witness` cargo feature (off by default) in
//! `elohim/holochain/dna/node-registry` — the DEFAULT build packs
//! byte-identical to pristine node-registry (no DNA-hash move on `dev`),
//! and only `--features lineage-witness` packs v2. Build both from
//! `elohim/holochain/dna/node-registry`:
//!
//! ```sh
//! just build && hc dna pack . -o node-registry-v1.dna       # v1 (pristine, default)
//! just build-witness                                        # v2 DEPLOYABLE (node-registry-v2.dna)
//! just build-witness-test                                   # v2 TEST (node-registry-v2-test.dna)
//! ```
//!
//! The two v2 bundles are DNA-hash-IDENTICAL — the difference is coordinator
//! wasm only. `build-witness-test` additionally passes `lineage-test`, which
//! fences `carry_page_for_test`: a hand-built-page injection point the G10
//! refusals need and a running conductor must never answer. THIS FILE reads the
//! test bundle by default, because those refusals cannot be reached otherwise.
//!
//! v1 is the pristine artifact (predecessor); v2 is the `lineage-witness`
//! artifact (successor). Their paths come from `NODE_REGISTRY_V1_DNA` /
//! `NODE_REGISTRY_V2_DNA` when set, else the in-repo defaults below
//! (`node-registry-v1.dna` / `node-registry-v2-test.dna`). The test is
//! `#[ignore]`d only in the sense of skipping cleanly when v1 is absent —
//! CI runs sweettests with `--run-ignored all`, so no `#[ignore]` is used.

use std::path::PathBuf;

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{
        load_dna_from_path, single_agent_conductor, two_agent_conductors,
        two_agent_conductors_isolated,
    },
    fixtures::{network_seed, node_registration, NodeRegistration},
};
use holochain::sweettest::{await_consistency_s, SweetConductor, SweetZome};
use holochain_types::prelude::*;

const DNA: &str = "node_registry";
const ZOME: &str = "node_registry_coordinator";

// ============================================================================
// Wire mirrors of the integrity types (see `node_registry_integrity`).
//
// Field names must match exactly; the conductor round-trips msgpack, so the
// test never links the WASM crate.
// ============================================================================

/// Mirror of `node_registry_integrity::CarriedProof`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CarriedProof {
    action: Action,
    signature: Signature,
    entry: Option<Entry>,
}

/// Mirror of `node_registry_integrity::NotarizationWitness`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct NotarizationWitness {
    lineage_dna_hash: DnaHash,
    proofs: Vec<CarriedProof>,
}

/// Mirror of `node_registry_coordinator::ExportInput` (Task 1, v1 bounded export).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ExportInput {
    cursor: Option<u32>,
    limit: u32,
}

/// Mirror of `node_registry_coordinator::ExportResume` (Task 24, G8) — the pin
/// a multi-page walk hands back so the predecessor stops re-walking its whole
/// chain per page.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct ExportResume {
    head: String,
    digest: String,
    total: u32,
    #[serde(default)]
    observed_head: Option<u32>,
    /// Task 24 fix round 1 (additive): `(app-entry ordinal, action_seq)` — what
    /// lets a resumed page scan forward from where the last one stopped instead
    /// of rebuilding the ordinal index over the whole chain.
    #[serde(default)]
    cursor_seq: Option<(u32, u32)>,
}

/// Mirror of `ExportInput` WITH the Task 24 `resume` field.
///
/// Kept separate from [`ExportInput`] on purpose: every other test in this file
/// keeps sending the two-field shape, which is exactly the byte shape the
/// LANDED storage driver sends. Those call sites are therefore a standing
/// old-caller compatibility assertion — if `resume` ever stopped being
/// `#[serde(default)]` on the zome side, they would all fail to decode.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ExportInputResumed {
    cursor: Option<u32>,
    limit: u32,
    resume: Option<ExportResume>,
}

/// Mirror of `node_registry_coordinator::ExportPage`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ExportPage {
    records: Vec<SignedActionHashed>,
    entries: Vec<Option<Entry>>,
    /// Task 26 (additive): the entry-def NAME the exporting DNA publishes for
    /// each record, paired POSITIONALLY with `records`. `#[serde(default)]`
    /// because a bundle packed before Task 26 does not emit it — and because
    /// this mirror is also SERIALIZED, into `carry_page_for_test`, where an
    /// empty vector is exactly how a pre-Task-26 predecessor's page looks.
    #[serde(default)]
    type_names: Vec<String>,
    next_cursor: Option<u32>,
    digest: String,
    /// Task 9 (additive): the app-record count the export walks — the WHOLE
    /// chain on the own-chain path, and only the COURIER'S OWN INTEGRATED VIEW
    /// of a neighbour's chain on the held path.
    /// `#[serde(default)]` because a v1 bundle packed before Task 9 does not
    /// emit the field — the carry receipt then reports `v1_total: None` rather
    /// than a fabricated number.
    #[serde(default)]
    total: Option<u32>,
    /// Task 18 fix round 1 (additive): the highest action SEQUENCE observed for
    /// that chain — the one field that reaches past the courier's view, so a
    /// held page can be checked for truncation from inside itself. A sequence,
    /// not a count, so `observed_head >= total - 1` always.
    #[serde(default)]
    observed_head: Option<u32>,
    /// Task 24 (additive): the walk pin. `#[serde(default)]` because a bundle
    /// packed before Task 24 does not emit it.
    #[serde(default)]
    resume: Option<ExportResume>,
    /// Task 24 fix round 1 (additive): action rows the page's POSITION scan
    /// read. The metric risk row R1 watches — see the zome-side field doc.
    #[serde(default)]
    scanned: Option<u32>,
    /// Task 29 (additive): WHICH read answered the page — `"authority"` when
    /// the agent-activity authorities served it over the network,
    /// `"local-only"` when they answered empty and the conductor fell back to
    /// its own store, `None` on the own-chain path. Station 6's root cause was
    /// that a partial-arc local read is indistinguishable from a short chain
    /// unless the page says so. `#[serde(default)]` because a bundle packed
    /// before Task 29 does not emit it.
    #[serde(default)]
    view: Option<String>,
}

/// Mirror of `node_registry_coordinator::ExportHeldInput` (Task 18, v1 held view).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ExportHeldInput {
    agent: AgentPubKey,
    cursor: Option<u32>,
    limit: u32,
}

/// Mirror of `node_registry_coordinator::CarrySource` (Task 18).
///
/// Externally tagged, exactly as serde renders the zome-side enum: `Own` is the
/// bare string, `Held(agent)` a single-key map.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum CarrySource {
    #[allow(dead_code)]
    Own,
    Held(AgentPubKey),
}

/// Mirror of `node_registry_coordinator::CarryInput` (Task 9, v2 cross-cell carry).
///
/// Task 18 added a `source` field to the zome-side struct. This mirror
/// DELIBERATELY keeps the pre-Task-18 shape (no `source`): the landed storage
/// decoder (`elohim-storage/.../release_adoption/apply.rs`) emits exactly these
/// bytes, so every existing carry test doubles as the byte-compatibility check
/// that `#[serde(default)] source: CarrySource` still decodes an old page
/// request as `CarrySource::Own`. Use [`CarryInputHeld`] to exercise the new
/// discriminator.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CarryInput {
    v1_cell: CellId,
    cursor: Option<u32>,
    limit: u32,
}

/// The Task 18 input shape: [`CarryInput`] plus the explicit source.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CarryInputHeld {
    v1_cell: CellId,
    cursor: Option<u32>,
    limit: u32,
    source: CarrySource,
}

/// Mirror of `node_registry_coordinator::CarryReceipt`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CarryReceipt {
    carried: u32,
    next_cursor: Option<u32>,
    v1_digest: String,
    /// Base64, NOT a native `ActionHash` — the landed consumer
    /// (`elohim-storage/.../release_adoption/apply.rs`) decodes a `String`, and
    /// a `HoloHash` on the wire is a msgpack byte array. Empty for a page that
    /// authored no witness.
    witness_hash: String,
    /// Station 3's equality `carried == v1_count` is only falsifiable if this
    /// is READ from v1's export page, never derived from `carried`. On the held
    /// path it counts the COURIER'S VIEW of the neighbour's chain, so the
    /// equality means "everything the courier had", never "everything the
    /// neighbour had".
    v1_total: Option<u32>,
    /// Additive: how many of `carried` were re-created NATIVELY here
    /// (held-carries excluded).
    #[serde(default)]
    self_carried: u32,
    /// Task 18 fix round 1 (additive): the predecessor's `observed_head`,
    /// carried through so a held receipt is checkable for truncation.
    #[serde(default)]
    v1_observed_head: Option<u32>,
    /// Task 20 (additive): how many of `carried` were ALREADY carried before
    /// this page ran (a self-carry entry hash already on this chain, or a
    /// held-carry entry hash already witnessed from this lineage) — a retry
    /// re-creates nothing and authors no proof for these.
    #[serde(default)]
    already_carried: u32,
    /// Task 29 (additive): the export page's `view`, threaded through verbatim
    /// — which read answered the walk this page carried. A held receipt
    /// reporting `local-only` is scoped to one peer's arc and must not be
    /// recorded as the neighbour's chain.
    #[serde(default)]
    view: Option<String>,
    /// Task 29 (additive): the export page's `scanned`, threaded through
    /// verbatim — the producer half of the mirror storage landed at Task 28,
    /// which decodes `None` as "not reported".
    #[serde(default)]
    scanned: Option<u32>,
}

/// Mirror of `node_registry_coordinator::ReadoptInput` (Task 13b, Station 7 —
/// the revert's re-authoring, driven ON the v1 cell).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ReadoptInput {
    v2_cell: CellId,
    cursor: Option<u32>,
    limit: u32,
}

/// Mirror of `node_registry_coordinator::ReadoptReceipt`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ReadoptReceipt {
    /// Own v2 records re-created natively on v1 by THIS page — new actions
    /// over the SAME entry hashes.
    readopted: u32,
    /// Already on v1 before the page ran. Non-zero on a first sweep is
    /// CORRECT: the carried re-creations of v1's own records are on both
    /// chains by construction.
    already_present: u32,
    /// App entry types v1 does not know — v2's `NotarizationWitness`. Skipped,
    /// counted, never an error.
    #[serde(default)]
    foreign: u32,
    next_cursor: Option<u32>,
    v2_digest: String,
    /// READ from v2's export page, never derived from `readopted`, so the
    /// driver's completeness check can actually fail.
    v2_total: Option<u32>,
}

/// The properties both node_registry versions read.
///
/// `progenitor_pubkey` is the pre-existing bootstrap-steward property;
/// `lineage` / `constitution_root` are what the evolution epic adds. They share
/// one properties map, and each reader ignores the other's keys.
#[derive(Debug, serde::Serialize, serde::Deserialize, SerializedBytes)]
struct LineageProperties {
    progenitor_pubkey: Option<String>,
    lineage: Vec<DnaHash>,
    constitution_root: Option<String>,
}

fn dna_dir() -> PathBuf {
    // src/tests/ -> sweettest/ -> tests/ -> holochain/
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("elohim/holochain root")
        .join("dna")
        .join("node-registry")
}

/// The predecessor artifact: node_registry packed BEFORE the integrity change.
fn v1_path() -> PathBuf {
    std::env::var("NODE_REGISTRY_V1_DNA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dna_dir().join("node-registry-v1.dna"))
}

/// The successor artifact: the in-tree bundle packed WITH the `lineage-witness`
/// cargo feature (see the module doc comment for the two build commands).
fn v2_path() -> PathBuf {
    std::env::var("NODE_REGISTRY_V2_DNA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dna_dir().join("node-registry-v2-test.dna"))
}

/// The v2 bundle is a FEATURE build (`just build-witness-test`) that CI's DNA pipeline does not
/// produce yet. Until it does, a missing bundle is a loud SKIP, never a silent pass and never
/// a red for a bundle nobody built: the probe's evidence is its own log line.
fn v2_bundle_or_skip() -> Option<PathBuf> {
    let p = v2_path();
    if p.exists() {
        Some(p)
    } else {
        eprintln!(
            "SKIPPED @concern:happ-lineage-migration — v2 bundle absent at {} \
             (build it: cd elohim/holochain/dna/node-registry && just build-witness-test)",
            p.display()
        );
        None
    }
}

/// One conductor, one agent, two apps: v1 and v2 of node_registry.
struct Crossing {
    conductor: SweetConductor,
    alice: AgentPubKey,
    v1_hash: DnaHash,
    v2_hash: DnaHash,
    z1: SweetZome,
    z2: SweetZome,
}

async fn install_crossing() -> Result<Crossing> {
    let (v1_path, v2_path) = (v1_path(), v2_path());
    assert!(
        v1_path.exists(),
        "predecessor DNA not found at {v1_path:?} — pack node_registry BEFORE the integrity \
         change and copy it there (or set NODE_REGISTRY_V1_DNA)"
    );
    assert!(
        v2_path.exists(),
        "successor DNA not found at {v2_path:?} — run `just build && hc dna pack .` in \
         elohim/holochain/dna/node-registry"
    );

    let (mut conductor, alice) = single_agent_conductor().await?;
    let seed = network_seed(DNA);

    // v1 carries only the property it already knew about.
    let v1_props = SerializedBytes::try_from(LineageProperties {
        progenitor_pubkey: Some(alice.to_string()),
        lineage: vec![],
        constitution_root: None,
    })?;
    let v1_dna = load_dna_from_path(&v1_path, &seed, Some(v1_props)).await?;
    let v1_hash = v1_dna.dna_hash().clone();

    // v2 DECLARES v1 as its parent. The lineage folds into v2's DNA hash, so
    // v2's identity commits to the crossing and every peer agrees on it.
    let v2_props = SerializedBytes::try_from(LineageProperties {
        progenitor_pubkey: Some(alice.to_string()),
        lineage: vec![v1_hash.clone()],
        constitution_root: None,
    })?;
    let v2_dna = load_dna_from_path(&v2_path, &seed, Some(v2_props)).await?;
    let v2_hash = v2_dna.dna_hash().clone();

    assert_ne!(
        v1_hash, v2_hash,
        "an integrity change MUST move the DNA hash — v1 and v2 are the same DNA"
    );

    // Two apps, ONE agent key: `setup_app_for_agent` installs with
    // `InstallAppPayload.agent_key = Some(alice)`, so there is no re-key.
    let app1 = conductor
        .setup_app_for_agent("node-registry-v1", alice.clone(), &[v1_dna])
        .await?;
    let app2 = conductor
        .setup_app_for_agent("node-registry-v2", alice.clone(), &[v2_dna])
        .await?;

    let z1 = app1.cells().first().unwrap().zome(ZOME);
    let z2 = app2.cells().first().unwrap().zome(ZOME);

    println!("[lineage] v1 dna hash = {v1_hash}");
    println!("[lineage] v2 dna hash = {v2_hash}");
    println!("[lineage] agent       = {alice}");

    Ok(Crossing {
        conductor,
        alice,
        v1_hash,
        v2_hash,
        z1,
        z2,
    })
}

/// Author a node_registry record on `zome` and return its signed action.
async fn author_and_read_back(
    conductor: &SweetConductor,
    zome: &SweetZome,
    node_id: &str,
    agent: &AgentPubKey,
) -> (ActionHash, SignedActionHashed) {
    let registration = node_registration(node_id, agent);
    let action_hash: ActionHash = conductor.call(zome, "register_node", registration).await;
    let signed: Option<SignedActionHashed> = conductor
        .call(zome, "get_signed_action", action_hash.clone())
        .await;
    (
        action_hash,
        signed.expect("the action just authored must be on this chain"),
    )
}

// ============================================================================
// PROBE A
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn probe_a_v1_notarization_carried_into_v2() -> Result<()> {
    let Some(_v2) = v2_bundle_or_skip() else { return Ok(()); };
    let Crossing {
        conductor,
        alice,
        v1_hash,
        v2_hash: _,
        z1,
        z2,
    } = install_crossing().await?;

    // --- 1. a real v1 record, and its notarization -------------------------
    let (_ah1, sah1) = author_and_read_back(&conductor, &z1, "lineage-probe", &alice).await;
    let eh1 = sah1
        .action()
        .entry_hash()
        .cloned()
        .expect("a Create action commits to an entry hash");
    println!("[probe A] v1 action  = {}", sah1.action_address());
    println!("[probe A] v1 entry   = {eh1}");

    // --- 2. the SAME content, re-created natively on v2 ---------------------
    let (_ah2, sah2) = author_and_read_back(&conductor, &z2, "lineage-probe", &alice).await;
    let eh2 = sah2
        .action()
        .entry_hash()
        .cloned()
        .expect("a Create action commits to an entry hash");
    println!("[probe A] v2 action  = {}", sah2.action_address());
    println!("[probe A] v2 entry   = {eh2}");

    assert_eq!(
        eh1, eh2,
        "CID continuity: byte-identical content must keep its EntryHash across the DNA line"
    );
    assert_ne!(
        sah1.action_address(),
        sah2.action_address(),
        "the v2 notarization is a NEW action — only the entry hash is shared"
    );

    // --- 3. carry the v1 notarization into v2 -------------------------------
    let witness = NotarizationWitness {
        lineage_dna_hash: v1_hash.clone(),
        proofs: vec![CarriedProof {
            action: sah1.action().clone(),
            signature: sah1.signature.clone(),
            // self-carry (§2.1): the author re-created the entry natively above,
            // so the witness carries the proof, not the bytes.
            entry: None,
        }],
    };

    let witness_hash: ActionHash = conductor.call(&z2, "commit_witness", witness.clone()).await;
    println!("[probe A] witness    = {witness_hash} ACCEPTED");

    // --- 4. and it is readable through the witness index --------------------
    let links: Vec<Link> = conductor.call(&z2, "get_witnesses_for", eh1.clone()).await;
    assert_eq!(
        links.len(),
        1,
        "expected exactly one witness link for the carried entry hash, got {}",
        links.len()
    );
    assert_eq!(
        links[0].target.clone().into_action_hash().as_ref(),
        Some(&witness_hash),
        "the witness link must target the committed witness"
    );

    // --- 5. NEGATIVE (a): one byte of the signature flipped -----------------
    let mut tampered_bytes = sah1.signature.0;
    tampered_bytes[0] ^= 0x01;
    let tampered = NotarizationWitness {
        lineage_dna_hash: v1_hash.clone(),
        proofs: vec![CarriedProof {
            action: sah1.action().clone(),
            signature: Signature(tampered_bytes),
            entry: None,
        }],
    };
    let err = conductor
        .call_fallible::<_, ActionHash>(&z2, "commit_witness", tampered)
        .await
        .expect_err("a tampered signature MUST be refused");
    let msg = format!("{err:?}");
    println!("[probe A] NEGATIVE tampered-signature refusal:\n{msg}");
    assert!(
        msg.contains("does not verify"),
        "expected the signature-verification refusal, got: {msg}"
    );

    // --- 6. NEGATIVE (b): a DNA hash that is not in the declared lineage ----
    let foreign = DnaHash::from_raw_32(vec![7u8; 32]);
    let off_lineage = NotarizationWitness {
        lineage_dna_hash: foreign.clone(),
        proofs: vec![CarriedProof {
            action: sah1.action().clone(),
            signature: sah1.signature.clone(),
            entry: None,
        }],
    };
    let err = conductor
        .call_fallible::<_, ActionHash>(&z2, "commit_witness", off_lineage)
        .await
        .expect_err("a witness naming a DNA outside the lineage MUST be refused");
    let msg = format!("{err:?}");
    println!("[probe A] NEGATIVE off-lineage refusal:\n{msg}");
    assert!(
        msg.contains("is not declared in this DNA's lineage property"),
        "expected the lineage refusal, got: {msg}"
    );

    Ok(())
}

// ============================================================================
// PROBE A (carry drive) — Task 9: `carry_from` pulls ONE bounded page from the
// v1 cell across the cell boundary and does, in the zome, exactly what probe A
// above does by hand: re-create the agent's own record natively (same
// EntryHash) and commit ONE witness for the page.
//
// It is a SEPARATE test rather than more steps appended to probe A because
// probe A's `get_witnesses_for` assertion counts witnesses for the carried
// entry hash; driving `carry_from` inside probe A would commit a SECOND
// witness for the same entry hash and turn that measured "exactly one" into a
// weaker "two, one of which". Both tests share `install_crossing()`, so the
// crossing under measurement is identical.
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn probe_a_carry_from_pulls_one_bounded_page_across_cells() -> Result<()> {
    let Some(_v2) = v2_bundle_or_skip() else { return Ok(()); };
    let Crossing {
        conductor,
        alice,
        v1_hash: _,
        v2_hash: _,
        z1,
        z2,
    } = install_crossing().await?;

    // --- 1. one real v1 record — the fact to carry --------------------------
    let (_ah1, sah1) = author_and_read_back(&conductor, &z1, "carry-probe", &alice).await;
    let eh1 = sah1
        .action()
        .entry_hash()
        .cloned()
        .expect("a Create action commits to an entry hash");
    println!("[probe A/carry] v1 action = {}", sah1.action_address());
    println!("[probe A/carry] v1 entry  = {eh1}");

    // --- 2. v1's own view of its export, for the receipt comparison ---------
    let v1_page: ExportPage = conductor
        .call(&z1, "export_records", ExportInput { cursor: None, limit: 16 })
        .await;
    assert_eq!(
        v1_page.records.len(),
        1,
        "one register_node call commits exactly one app-entry record"
    );
    assert_eq!(
        v1_page.total,
        Some(1),
        "ExportPage.total is the WHOLE-chain app-record count, not the page length"
    );

    // --- 3. the carry, driven across the cell boundary ----------------------
    let receipt: CarryReceipt = conductor
        .call(
            &z2,
            "carry_from",
            CarryInput {
                v1_cell: z1.cell_id().clone(),
                cursor: None,
                limit: 16,
            },
        )
        .await;
    println!("[probe A/carry] receipt = {receipt:?}");

    assert_eq!(receipt.carried, 1, "the page held exactly one record to carry");
    assert_eq!(
        receipt.next_cursor, None,
        "a partial page must not offer a further cursor"
    );
    assert_eq!(
        receipt.v1_digest, v1_page.digest,
        "the receipt must report v1's OWN chain digest, unmodified"
    );
    assert_eq!(
        receipt.v1_total,
        Some(1),
        "v1_total is read from v1's export page — Station 3's `carried == v1_count` \
         is only falsifiable when it is not derived from `carried`"
    );
    assert_eq!(
        receipt.carried,
        receipt.v1_total.expect("v1 reported a total"),
        "Station 3: everything v1 had was carried"
    );
    assert_eq!(
        receipt.self_carried, 1,
        "the one record is the agent's OWN, so it is a self-carry — re-created natively on \
         this chain, not held as bytes inside the witness"
    );
    assert!(
        !receipt.witness_hash.is_empty(),
        "a non-empty page commits exactly one witness"
    );

    // --- 4. ONE witness per page, reachable through the witness index -------
    let links: Vec<Link> = conductor.call(&z2, "get_witnesses_for", eh1.clone()).await;
    assert_eq!(
        links.len(),
        1,
        "carry_from commits exactly ONE witness per page, got {} links",
        links.len()
    );
    let linked = links[0]
        .target
        .clone()
        .into_action_hash()
        .expect("the witness link targets an action hash");
    assert_eq!(
        linked.to_string(),
        receipt.witness_hash,
        "the witness link must target the witness the receipt names — and the receipt must \
         render it as the canonical base64 the storage-side consumer decodes"
    );

    // --- 5. the self-carried record was re-created NATIVELY on v2 -----------
    // Same EntryHash, new ActionHash: v2 holds the content as its own commit,
    // not merely as bytes inside a witness.
    let v2_page: ExportPage = conductor
        .call(&z2, "export_records", ExportInput { cursor: None, limit: 64 })
        .await;
    let recreated = v2_page
        .records
        .iter()
        .find(|r| r.action().entry_hash() == Some(&eh1))
        .expect("carry_from must re-create the agent's own entry natively on v2");
    println!(
        "[probe A/carry] v2 re-created action = {}",
        recreated.action_address()
    );
    assert_ne!(
        recreated.action_address(),
        sah1.action_address(),
        "the v2 re-creation is a NEW action — only the entry hash is shared"
    );

    Ok(())
}

// ============================================================================
// PROBE A (carry idempotency) — Task 20, epic §7 C6b: a retried page commits
// no duplicate entries and no second witness. Same crossing shape as the
// carry-drive probe above (`install_crossing()`, one real v1 record,
// `CarryInput` with no `source` — the pre-Task-18 self-carry shape), but the
// SAME page is carried twice from the same cursor. This is the retry a driver
// takes after a crash, a timeout, or simply re-running a page defensively —
// it must re-create nothing and author no second witness for content this
// chain already holds.
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn probe_a_carry_from_retry_is_idempotent() -> Result<()> {
    let Some(_v2) = v2_bundle_or_skip() else { return Ok(()); };
    let Crossing {
        conductor,
        alice,
        v1_hash: _,
        v2_hash: _,
        z1,
        z2,
    } = install_crossing().await?;

    // --- 1. one real v1 record — the fact to carry --------------------------
    let (_ah1, sah1) = author_and_read_back(&conductor, &z1, "carry-retry-probe", &alice).await;
    let eh1 = sah1
        .action()
        .entry_hash()
        .cloned()
        .expect("a Create action commits to an entry hash");
    println!("[probe A/retry] v1 action = {}", sah1.action_address());
    println!("[probe A/retry] v1 entry  = {eh1}");

    let carry_input = || CarryInput {
        v1_cell: z1.cell_id().clone(),
        cursor: None,
        limit: 16,
    };

    // --- 2. the first, full carry (Own) — establishes the baseline ----------
    let first: CarryReceipt = conductor.call(&z2, "carry_from", carry_input()).await;
    println!("[probe A/retry] first receipt = {first:?}");
    assert_eq!(first.carried, 1, "the page held exactly one record to carry");
    assert_eq!(
        first.self_carried, 1,
        "the one record is the agent's own — re-created natively on the first pass"
    );
    assert_eq!(
        first.already_carried, 0,
        "nothing was already carried before the first pass ran"
    );
    assert!(
        !first.witness_hash.is_empty(),
        "a non-empty first page commits exactly one witness"
    );

    // --- 3. the SAME page, retried at cursor 0 -------------------------------
    let retry: CarryReceipt = conductor.call(&z2, "carry_from", carry_input()).await;
    println!("[probe A/retry] retry receipt = {retry:?}");

    assert_eq!(
        retry.carried, first.carried,
        "a retry reports the same `carried` count — the content IS carried, retry or not"
    );
    assert_eq!(
        retry.self_carried, 0,
        "the retry re-creates NOTHING natively — the entry is already on this chain"
    );
    assert_eq!(
        retry.already_carried, retry.carried,
        "on a retry every record on the page was already carried"
    );
    assert_eq!(
        retry.witness_hash, "",
        "a page with zero new proofs authors NO witness — the empty string is the landed \
         storage decoder's \"no witness this page\" signal"
    );

    // --- 4. exactly ONE witness link survives the retry ----------------------
    let links: Vec<Link> = conductor.call(&z2, "get_witnesses_for", eh1.clone()).await;
    assert_eq!(
        links.len(),
        1,
        "a retried carry_from must not commit a second witness or a second EntryToWitness \
         link — got {} links",
        links.len()
    );

    // --- 5. still exactly ONE re-created record on v2, not two --------------
    let v2_page: ExportPage = conductor
        .call(&z2, "export_records", ExportInput { cursor: None, limit: 64 })
        .await;
    let recreated_count = v2_page
        .records
        .iter()
        .filter(|r| r.action().entry_hash() == Some(&eh1))
        .count();
    assert_eq!(
        recreated_count, 1,
        "a retried self-carry must not mint a second action over the same entry hash"
    );

    Ok(())
}

// ============================================================================
// PROBE B — a LATE open_chain
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn probe_b_late_open_chain_after_v2_has_authored() -> Result<()> {
    let Some(_v2) = v2_bundle_or_skip() else { return Ok(()); };
    let Crossing {
        conductor,
        alice,
        v1_hash,
        v2_hash,
        z1,
        z2,
    } = install_crossing().await?;

    // Put real history on BOTH chains first: this is the whole point — v2 has
    // already been authoring when the crossing is finally recorded.
    let (_ah1, sah1) = author_and_read_back(&conductor, &z1, "late-open-1", &alice).await;
    let _ = author_and_read_back(&conductor, &z2, "late-open-1", &alice).await;
    let _ = author_and_read_back(&conductor, &z2, "late-open-2", &alice).await;

    let witness = NotarizationWitness {
        lineage_dna_hash: v1_hash.clone(),
        proofs: vec![CarriedProof {
            action: sah1.action().clone(),
            signature: sah1.signature.clone(),
            entry: None,
        }],
    };
    let _: ActionHash = conductor.call(&z2, "commit_witness", witness).await;

    // --- close v1 toward v2 --------------------------------------------------
    let close_hash: ActionHash = conductor
        .call_fallible(&z1, "close_chain_for", v2_hash.clone())
        .await
        .map_err(|e| anyhow::anyhow!("close_chain on v1 was refused: {e:?}"))?;
    println!("[probe B] v1 CloseChain = {close_hash}");

    // --- and open v2 from it, LATE ------------------------------------------
    let open_result: Result<ActionHash, _> = conductor
        .call_fallible(
            &z2,
            "open_chain_from",
            (v1_hash.clone(), close_hash.clone()),
        )
        .await;
    match &open_result {
        Ok(open_hash) => println!("[probe B] v2 OpenChain  = {open_hash} ACCEPTED (late)"),
        Err(e) => println!("[probe B] v2 OpenChain REFUSED (late):\n{e:?}"),
    }
    let open_hash = open_result
        .map_err(|e| anyhow::anyhow!("late open_chain refused — FINDING, quote verbatim: {e:?}"))?;
    assert_ne!(open_hash, close_hash);

    // --- what "the chain is closed" actually means on 0.7 -------------------
    //
    // FINDING (probe B). `close_chain` is NOT an authoring-time guard.
    // `PrevActionErrorKind::ActionAfterChainClose` is raised only in the
    // sys-validation workflow's `register_agent_activity`
    // (sys_validation_workflow.rs:1355-1362), i.e. by the agent-activity
    // AUTHORITY when it validates the RegisterAgentActivity op. The source
    // chain itself has no close guard (holochain_state/src/source_chain.rs
    // mentions CloseChain only in tests). So a post-close commit still returns
    // an ActionHash to its author; the refusal lands on the authority side.
    let after_close: Result<ActionHash, _> = conductor
        .call_fallible(
            &z1,
            "register_node",
            node_registration("after-close", &alice),
        )
        .await;
    match &after_close {
        Ok(hash) => println!(
            "[probe B] FINDING: post-close create ACCEPTED BY THE AUTHOR at {hash} \
             — close_chain is an authority-side rule, not a source-chain guard"
        ),
        Err(e) => println!("[probe B] post-close create refused at author time:\n{e:?}"),
    }

    // Then watch the authority. In a single conductor the agent is its own
    // agent-activity authority, so the rejection is observable locally.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let mut last = None;
    loop {
        let activity: AgentActivityStatus = conductor.call(&z1, "my_chain_activity", ()).await;
        let rejected = !activity.rejected_activity.is_empty();
        let warranted = !activity.warrants.is_empty();
        let invalid_status = matches!(activity.status, ChainStatus::Invalid(_));
        last = Some(activity);
        if rejected || warranted || invalid_status || std::time::Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    let activity = last.unwrap();
    println!("[probe B] v1 authority status      = {:?}", activity.status);
    println!(
        "[probe B] v1 valid_activity        = {:?}",
        activity.valid_activity
    );
    println!(
        "[probe B] v1 rejected_activity     = {:?}",
        activity.rejected_activity
    );
    println!(
        "[probe B] v1 warrants              = {}",
        activity.warrants.len()
    );

    // These two assertions LOCK IN what was measured on holochain 0.7.0 so a
    // future toolchain that starts enforcing the close is loud, not silent.
    // The epic's §3 "Design Constraints Discovered" assumes a post-close create
    // is refused; on 0.7, in a single conductor, it is not.
    assert!(
        after_close.is_ok(),
        "MEASURED CHANGE: close_chain now guards authoring at the source chain — \
         update the epic's §3 constraint and Station 8"
    );
    assert!(
        activity.rejected_activity.is_empty() && activity.warrants.is_empty(),
        "MEASURED CHANGE: the local agent-activity authority now rejects post-close \
         activity (ActionAfterChainClose fires without a remote authority) — \
         update the epic's Station 8"
    );

    Ok(())
}

// ============================================================================
// PROBE B2 — the REMOTE agent-activity authority, after a close
//
// Probe B measured ONE conductor: the author is its own agent-activity
// authority there, and it accepted the post-close create (`rejected_activity`
// empty, warrants 0). `ActionAfterChainClose` lives only in
// `sys_validation_workflow::register_agent_activity`, i.e. on the authority
// side — so the open question is whether a DIFFERENT conductor, holding
// alice's v1 agent-activity, refuses it.
//
// Shape: alice on conductor A holds v1 AND v2 (the crossing). bob on conductor
// B holds the SAME v1 DNA (same bundle, same seed, same properties => same DNA
// hash => same DHT), so he is an authority for alice's v1 activity. Alice
// closes v1 toward v2, then writes on v1 anyway. We then read bob's verdict.
//
// Nothing is asserted that was not measured; the probe's evidence is its
// `[probe B2]` log lines.
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn probe_b2_remote_authority_after_close() -> Result<()> {
    let Some(_v2) = v2_bundle_or_skip() else { return Ok(()); };

    let (v1_path, v2_path) = (v1_path(), v2_path());
    assert!(v1_path.exists(), "predecessor DNA not found at {v1_path:?}");

    // --- two conductors on ONE rendezvous, discovery isolated until exchange -
    let [(mut ca, alice), (mut cb, bob)] = two_agent_conductors_isolated().await?;
    let seed = network_seed(DNA);

    let v1_props = SerializedBytes::try_from(LineageProperties {
        progenitor_pubkey: Some(alice.to_string()),
        lineage: vec![],
        constitution_root: None,
    })?;
    let v1_dna = load_dna_from_path(&v1_path, &seed, Some(v1_props)).await?;
    let v1_hash = v1_dna.dna_hash().clone();

    let v2_props = SerializedBytes::try_from(LineageProperties {
        progenitor_pubkey: Some(alice.to_string()),
        lineage: vec![v1_hash.clone()],
        constitution_root: None,
    })?;
    let v2_hash = load_dna_from_path(&v2_path, &seed, Some(v2_props))
        .await?
        .dna_hash()
        .clone();
    assert_ne!(v1_hash, v2_hash);

    // BOTH conductors hold v1 and ONLY v1. v2 is loaded for its DNA hash alone
    // — `close_chain(MigrationTarget::Dna(h))` names a successor, it does not
    // require the successor to be installed, and B2's whole question lives in
    // the v1 DHT (does the REMOTE v1 agent-activity authority refuse?).
    //
    // Installing v2 on alice only would also break the harness:
    // `SweetConductor::exchange_peer_info` returns true only when EVERY space
    // across the batch holds an agent info per conductor, so a space that
    // exists on one conductor alone can never satisfy it (measured: the poll
    // times out at 30 s with peer info already correctly injected for v1).
    let app_a1 = ca
        .setup_app_for_agent("node-registry-v1", alice.clone(), &[v1_dna.clone()])
        .await?;
    let app_b1 = cb
        .setup_app_for_agent("node-registry-v1", bob.clone(), &[v1_dna])
        .await?;

    let cell_a1 = app_a1.cells().first().expect("alice v1 cell").clone();
    let cell_b1 = app_b1.cells().first().expect("bob v1 cell").clone();
    let za1 = cell_a1.zome(ZOME);
    let zb1 = cell_b1.zome(ZOME);

    println!("[probe B2] v1 dna hash = {v1_hash}");
    println!("[probe B2] v2 dna hash = {v2_hash}");
    println!("[probe B2] alice       = {alice}");
    println!("[probe B2] bob         = {bob}");

    // --- connect the pair ----------------------------------------------------
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        while !SweetConductor::exchange_peer_info([&ca, &cb]).await {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("timeout exchanging peer info"))?;

    // Pay bob's one-time wasm instantiation before any bounded window.
    let _: Vec<NodeRegistration> = cb.call(&zb1, "get_my_nodes", ()).await;

    // --- real history on v1, gossiped to bob ---------------------------------
    let (pre_hash, _pre) = author_and_read_back(&ca, &za1, "b2-pre-close", &alice).await;
    println!("[probe B2] pre-close action  = {pre_hash}");

    await_consistency_s(90, [&cell_a1, &cell_b1])
        .await
        .map_err(|e| anyhow::anyhow!("DHT consistency timeout before the close: {e}"))?;

    // --- alice closes v1 toward v2 -------------------------------------------
    let close_hash: ActionHash = ca
        .call_fallible(&za1, "close_chain_for", v2_hash.clone())
        .await
        .map_err(|e| anyhow::anyhow!("close_chain on v1 was refused: {e:?}"))?;
    println!("[probe B2] v1 CloseChain     = {close_hash}");

    await_consistency_s(90, [&cell_a1, &cell_b1])
        .await
        .map_err(|e| anyhow::anyhow!("DHT consistency timeout after the close: {e}"))?;

    // --- and writes on v1 anyway ---------------------------------------------
    let after_close: Result<ActionHash, _> = ca
        .call_fallible(
            &za1,
            "register_node",
            node_registration("b2-after-close", &alice),
        )
        .await;
    match &after_close {
        Ok(h) => println!("[probe B2] post-close create ACCEPTED BY THE AUTHOR at {h}"),
        Err(e) => println!("[probe B2] post-close create refused at author time:\n{e:?}"),
    }
    let after_hash = after_close
        .as_ref()
        .ok()
        .cloned()
        .expect("probe B measured the author accepts a post-close create");

    // --- watch the REMOTE authority (bob), bounded ---------------------------
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
    let mut bob_view: Option<AgentActivityStatus>;
    let mut saw_after_action;
    loop {
        let activity: AgentActivityStatus = cb
            .call(&zb1, "agent_activity_of", (alice.clone(), true))
            .await;
        let rejected = !activity.rejected_activity.is_empty();
        let warranted = !activity.warrants.is_empty();
        saw_after_action = activity
            .valid_activity
            .iter()
            .any(|(_, h)| h == &after_hash)
            || activity
                .rejected_activity
                .iter()
                .any(|(_, h)| h == &after_hash);
        bob_view = Some(activity);
        // Wait for BOTH the rejection and the warrant: the warrant is authored
        // asynchronously after the op is rejected, so breaking on the rejection
        // alone would race the assertions below.
        if (rejected && warranted) || std::time::Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    let bob_activity = bob_view.expect("polled at least once");

    println!("[probe B2] BOB (remote authority, local read)");
    println!("[probe B2]   status            = {:?}", bob_activity.status);
    println!(
        "[probe B2]   valid_activity    = {:?}",
        bob_activity.valid_activity
    );
    println!(
        "[probe B2]   rejected_activity = {:?}",
        bob_activity.rejected_activity
    );
    println!(
        "[probe B2]   warrants          = {}",
        bob_activity.warrants.len()
    );
    println!("[probe B2]   post-close action present in bob's activity = {saw_after_action}");

    // bob's network view, for contrast with his own store
    let bob_net: Result<AgentActivityStatus, _> = cb
        .call_fallible(&zb1, "agent_activity_of", (alice.clone(), false))
        .await;
    match &bob_net {
        Ok(a) => println!(
            "[probe B2]   NETWORK view: status={:?} valid={} rejected={} warrants={}",
            a.status,
            a.valid_activity.len(),
            a.rejected_activity.len(),
            a.warrants.len()
        ),
        Err(e) => println!("[probe B2]   NETWORK view errored:\n{e:?}"),
    }

    // alice's own authority view, for contrast (this is what probe B measured)
    let alice_activity: AgentActivityStatus = ca.call(&za1, "my_chain_activity", ()).await;
    println!(
        "[probe B2] ALICE (self authority): status={:?} valid={} rejected={} warrants={}",
        alice_activity.status,
        alice_activity.valid_activity.len(),
        alice_activity.rejected_activity.len(),
        alice_activity.warrants.len()
    );

    // --- bob's `get` of the post-close entry ---------------------------------
    let bob_get_local: Result<Option<Record>, _> = cb
        .call_fallible(&zb1, "get_record_at", (after_hash.clone(), true))
        .await;
    match &bob_get_local {
        Ok(Some(r)) => println!(
            "[probe B2] bob get(post-close) LOCAL  = SOME (entry present: {})",
            r.entry().as_option().is_some()
        ),
        Ok(None) => println!("[probe B2] bob get(post-close) LOCAL  = NONE"),
        Err(e) => println!("[probe B2] bob get(post-close) LOCAL  = ERROR\n{e:?}"),
    }
    let bob_get_net: Result<Option<Record>, _> = cb
        .call_fallible(&zb1, "get_record_at", (after_hash.clone(), false))
        .await;
    match &bob_get_net {
        Ok(Some(r)) => println!(
            "[probe B2] bob get(post-close) NETWORK = SOME (entry present: {})",
            r.entry().as_option().is_some()
        ),
        Ok(None) => println!("[probe B2] bob get(post-close) NETWORK = NONE"),
        Err(e) => println!("[probe B2] bob get(post-close) NETWORK = ERROR\n{e:?}"),
    }

    // ------------------------------------------------------------------------
    // MEASURED (holochain 0.7.0, 2026-09-04). These assertions lock in what the
    // probe saw, so a toolchain that changes any of it is LOUD, not silent.
    //
    //   * the author still accepts the post-close create (probe B, unchanged);
    //   * the REMOTE agent-activity authority REFUSES it: exactly the action
    //     whose `prev_action` is the CloseChain lands in `rejected_activity`,
    //     the chain status flips to `Invalid` at that seq, and a warrant is
    //     issued. Actions AFTER that one are still `valid_activity` — the rule
    //     in `sys_validation_workflow::register_agent_activity` fires only on
    //     the immediate successor of the CloseChain, not on the whole tail.
    //   * and the record is STILL RETRIEVABLE by `get`: the refusal is an
    //     agent-activity verdict, not a fetch fence.
    // ------------------------------------------------------------------------
    assert!(
        matches!(bob_activity.status, ChainStatus::Invalid(_)),
        "MEASURED CHANGE: the remote authority no longer marks alice's chain Invalid \
         after a post-close write — got {:?}",
        bob_activity.status
    );
    assert!(
        bob_activity
            .rejected_activity
            .iter()
            .any(|(_, h)| h == &after_hash),
        "MEASURED CHANGE: the remote authority no longer rejects the post-close action \
         (ActionAfterChainClose) — rejected_activity = {:?}",
        bob_activity.rejected_activity
    );
    assert!(
        !bob_activity.warrants.is_empty(),
        "MEASURED CHANGE: the remote authority no longer warrants post-close activity"
    );
    assert!(
        bob_activity
            .valid_activity
            .iter()
            .all(|(_, h)| h != &after_hash),
        "the post-close action must not be in BOTH valid and rejected activity"
    );
    assert!(
        matches!(bob_get_local, Ok(Some(_))),
        "MEASURED CHANGE: the post-close record is no longer retrievable from the \
         rejecting authority's own store — got {bob_get_local:?}"
    );
    assert!(
        alice_activity.rejected_activity.is_empty() && alice_activity.warrants.is_empty(),
        "MEASURED CHANGE: the AUTHOR's own authority now rejects its post-close activity \
         (probe B measured that it does not) — rejected={:?} warrants={}",
        alice_activity.rejected_activity,
        alice_activity.warrants.len()
    );

    Ok(())
}

// ============================================================================
// Task 1 (v1 bounded export): `export_records` returns bounded, cursor-
// resumable pages of the agent's own signed actions with a page-independent
// chain digest. v1-only — no v2 bundle needed.
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn export_records_is_bounded_and_resumable() -> Result<()> {
    let (mut conductor, alice) = single_agent_conductor().await?;
    let seed = network_seed(DNA);
    let v1 = load_dna_from_path(&v1_path(), &seed, None).await?;
    let app = conductor
        .setup_app_for_agent("node-registry-v1", alice.clone(), &[v1])
        .await?;
    let cell = app.cells().first().unwrap().clone();
    let zome = cell.zome(ZOME);

    for i in 0..5u32 {
        let _ah: ActionHash = conductor
            .call(&zome, "register_node", node_registration(&format!("n{i}"), &alice))
            .await;
    }

    let p1: ExportPage = conductor
        .call(&zome, "export_records", ExportInput { cursor: None, limit: 2 })
        .await;
    assert_eq!(p1.records.len(), 2, "page 1 must be bounded to the requested limit");
    assert_eq!(p1.entries.len(), 2);
    assert!(p1.next_cursor.is_some(), "a full page must offer a cursor to resume from");

    let p2: ExportPage = conductor
        .call(
            &zome,
            "export_records",
            ExportInput { cursor: p1.next_cursor, limit: 64 },
        )
        .await;

    // Measured (holochain 0.7.0, 2026-09-04): register_node commits exactly
    // ONE app entry per call (the NodeRegistration Create) — the region/
    // status/tier/node_id/custodian anchors are `hash_entry`-only bases for
    // `create_link`, never `create_entry`d, so they never appear on the
    // source chain. No coordinator `init` creates an app entry at genesis.
    // Five `register_node` calls therefore produce exactly 5 app-entry
    // records on the chain — no hidden genesis-era app records.
    assert_eq!(
        p1.records.len() + p2.records.len(),
        5,
        "all 5 registered app-entry records must be reachable across the two pages"
    );
    assert_eq!(p1.digest, p2.digest, "digest is page-independent — computed once over the whole chain");
    assert!(!p1.digest.is_empty());
    assert!(p2.next_cursor.is_none(), "the last page must not offer a further cursor");

    // --- Task 18: the HELD view of the same chain, like-for-like ------------
    //
    // `export_held_records` reads an agent's chain through the agent-activity
    // authority instead of the local `query()`. Pointed at the caller's OWN
    // key it must agree with `export_records` exactly — same digest recipe over
    // the same app-entry set, same whole-chain total. If the two ever disagree
    // the sweep's `v1_digest` comparison stops being like-for-like, which is
    // the only reason the carry receipt's digest is falsifiable at all.
    //
    // Bounded poll: agent activity is read from the INTEGRATED store, and
    // self-authored ops reach it a beat after the zome call returns. Polling to
    // the expected total keeps the equality honest instead of racing it.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let mut held: ExportPage;
    loop {
        held = conductor
            .call(
                &zome,
                "export_held_records",
                ExportHeldInput { agent: alice.clone(), cursor: None, limit: 64 },
            )
            .await;
        if held.total == Some(5) || std::time::Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    println!(
        "[export/held] total={:?} observed_head={:?} digest={} records={} view={:?} scanned={:?}",
        held.total,
        held.observed_head,
        held.digest,
        held.records.len(),
        held.view,
        held.scanned
    );
    assert_eq!(
        held.total,
        Some(5),
        "the held view of alice's own chain must see the same 5 app-entry records"
    );
    // Task 29: the page SAYS which store answered, and on a lone conductor the
    // answer is the fallback. MEASURED (holochain 0.7.0, 2026-09-04): reading
    // one's OWN key with `GetOptions::network()` returns an EMPTY activity list
    // here — there is no second peer to be an authority — so the network-first
    // read finds nothing, the local store holds all 5, and the page is honestly
    // labelled `local-only`. That label is the whole point: the same five
    // records served without it are indistinguishable from an authority answer,
    // which is how Station 6's frozen partial-arc view read as a total.
    assert_eq!(
        held.view.as_deref(),
        Some("local-only"),
        "a lone conductor gets an empty network answer and falls back to its own \
         store — the page must SAY so, got {:?}",
        held.view
    );
    assert_eq!(
        held.scanned,
        Some(5),
        "on the held path `scanned` is the size of the activity list the answering \
         view returned — here the 5 app entries the local store holds"
    );
    assert_eq!(
        held.digest, p1.digest,
        "LIKE-FOR-LIKE: the held export must use the SAME digest recipe over the same \
         app-entry set as export_records, or a carry receipt's v1_digest compares nothing"
    );
    assert_eq!(held.records.len(), 5, "limit 64 takes the whole chain in one page");
    assert!(held.next_cursor.is_none(), "a partial page must not offer a further cursor");
    assert_eq!(
        held.entries.len(),
        held.records.len(),
        "records and entries are paired POSITIONALLY"
    );

    // `observed_head` is a chain SEQUENCE (genesis and links included), `total`
    // a count of app entries, so the invariant is an inequality — and it is what
    // lets a driver notice a courier that has not caught up.
    let observed_head = held
        .observed_head
        .expect("the authority observed this chain, so it must report a head");
    assert!(
        observed_head >= held.total.unwrap() - 1,
        "observed_head is a sequence spanning EVERY action, so it can never sit \
         below the app-entry count minus one — got head {observed_head}, total {:?}",
        held.total
    );
    // MEASURED (holochain 0.7.0, 2026-09-04) and asserted so a toolchain or a
    // `register_node` change is LOUD rather than silent. The head is 33, not 5:
    //   seq 0..2   3 genesis actions (Dna, AgentValidationPkg, agent-key Create)
    //   seq 3      InitZomesComplete
    //   seq 4..33  5 × register_node, each SIX actions — one NodeRegistration
    //              Create plus five CreateLinks (region, status, tier, node_id,
    //              custodian; the fixture opts in to custodianship)
    // This is precisely why observed_head is not comparable to `total`: 33 vs 5
    // on a chain with no gap at all. Only the inequality above is an invariant.
    assert_eq!(
        observed_head, 33,
        "MEASURED (holochain 0.7.0): 3 genesis + InitZomesComplete + 5×6 register_node \
         actions — the head is a chain SEQUENCE spanning links and bookkeeping, not an \
         app-entry count"
    );
    assert_eq!(
        p1.observed_head,
        Some(observed_head),
        "the own-chain export reports the SAME head as the held view of the same chain"
    );

    // and the sweep's agent enumeration sees the author of those registrations
    let agents: Vec<AgentPubKey> = conductor.call(&zome, "known_agents", ()).await;
    println!("[export/held] known_agents = {agents:?}");
    assert!(
        agents.contains(&alice),
        "known_agents must list the author of the NodeRegistration entries this cell can read"
    );

    Ok(())
}

// ============================================================================
// Task 24 (G8) — THE EXPORT WALK IS LINEAR, THE DIGEST COMPUTED ONCE
//
// Before Task 24 every page of `export_records` re-walked and re-hashed the
// WHOLE chain to report its page-independent `digest`/`total`, so carrying N
// records cost N/EXPORT_CAP whole-chain walks with every entry blob loaded on
// each — quadratic on the one path a migration must run to completion.
//
// Two things fix it, and this test holds both honest:
//
//   1. The walk itself is now HEADERS for the ordinal index plus a
//      sequence-bounded query for the page's entries. That is unconditional —
//      no caller change — so it cannot be asserted from the wire. What CAN be
//      asserted is that the pages it produces are unchanged: same records, same
//      order, same page-independent digest, same total.
//   2. The `resume` pin removes the remaining per-page digest walk, and — the
//      part with teeth — REFUSES a page whose chain moved underneath it. That
//      refusal is the reason the shortcut is safe: a resumed page can never be
//      served against a digest it no longer describes.
//
// 200 records on purpose: four pages at the 64-record EXPORT_CAP, so the pin is
// exercised across three resumed pages rather than one.
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn export_walk_pins_the_head_and_digests_once() -> Result<()> {
    let (mut conductor, alice) = single_agent_conductor().await?;
    let seed = network_seed(DNA);
    let v1 = load_dna_from_path(&v1_path(), &seed, None).await?;
    let app = conductor
        .setup_app_for_agent("node-registry-v1", alice.clone(), &[v1])
        .await?;
    let cell = app.cells().first().unwrap().clone();
    let zome = cell.zome(ZOME);

    const N: u32 = 200;
    for i in 0..N {
        let _ah: ActionHash = conductor
            .call(
                &zome,
                "register_node",
                node_registration(&format!("g8-{i}"), &alice),
            )
            .await;
    }

    // --- page 1: unpinned, establishes the walk ----------------------------
    let p1: ExportPage = conductor
        .call(
            &zome,
            "export_records",
            ExportInput {
                cursor: None,
                limit: 64,
            },
        )
        .await;
    assert_eq!(p1.records.len(), 64, "page 1 is bounded by EXPORT_CAP");
    assert_eq!(p1.entries.len(), p1.records.len());
    assert_eq!(
        p1.total,
        Some(N),
        "one app entry per register_node — see the measured note in          export_records_is_bounded_and_resumable"
    );

    // Every page mints a pin, including the first — that is what makes the
    // walk resumable without the caller having to know how it was computed.
    let pin = p1
        .resume
        .clone()
        .expect("every page returns a resume token");
    assert_eq!(
        pin.digest, p1.digest,
        "the pin carries the SAME digest the page reports — it is a pin, not a second opinion"
    );
    assert_eq!(pin.total, N);
    assert!(
        !pin.head.is_empty(),
        "the pin names the chain head it is pinned to"
    );
    assert_eq!(
        pin.observed_head, p1.observed_head,
        "the pin carries the head sequence forward for a later blind page"
    );
    let (pin_ordinal, _pin_seq) = pin
        .cursor_seq
        .expect("a full page names where the NEXT page starts — that is the whole fix");
    assert_eq!(
        pin_ordinal,
        p1.next_cursor.unwrap(),
        "the pin's ordinal IS the cursor it is paired with; a mismatch is what makes a resumed \
         page fall back to the full walk"
    );

    // MEASURED (holochain 0.7.0): 3 genesis + InitZomesComplete + 200 x 6
    // register_node actions = 1204. An UNPINNED page has no choice but to read
    // all of them: the cursor is an ordinal into the app-entry subsequence and
    // nothing in it says where that lands.
    let first_scanned = p1
        .scanned
        .expect("every page reports what its position scan read");
    assert_eq!(
        first_scanned, 1204,
        "an unpinned page rebuilds the whole ordinal index — 3 genesis + InitZomesComplete + \
         200x6 register_node actions"
    );

    // --- pages 2..N: pinned. The digest is not recomputed; the WALK-IDENTITY
    //     half of the pin (head, digest, total, observed_head) never moves, and
    //     only `cursor_seq` advances — which is the whole mechanism: each page
    //     tells the next one where to start so it never rebuilds the index.
    let first_pin = pin.clone();
    let mut pin = pin;
    let mut cursor = p1.next_cursor;
    let mut walked = p1.records.len();
    let mut pages = 1;
    let mut resumed_scans: Vec<u32> = Vec::new();
    while let Some(next) = cursor {
        let page: ExportPage = conductor
            .call(
                &zome,
                "export_records",
                ExportInputResumed {
                    cursor: Some(next),
                    limit: 64,
                    resume: Some(pin.clone()),
                },
            )
            .await;
        pages += 1;
        assert_eq!(
            page.digest, p1.digest,
            "the digest is a property of the WALK, computed once — a resumed page reports the \
             pinned value, never a fresh one"
        );
        assert_eq!(page.total, Some(N), "so is the total");
        assert_eq!(page.entries.len(), page.records.len());

        let returned = page
            .resume
            .clone()
            .expect("every page returns a resume token");
        assert_eq!(
            (
                &returned.head,
                &returned.digest,
                returned.total,
                returned.observed_head
            ),
            (&pin.head, &pin.digest, pin.total, pin.observed_head),
            "the walk-identity half of the pin is what says THIS IS THE SAME WALK — it must come \
             back untouched, or the refusal it licenses means nothing"
        );
        if let Some(next_cursor) = page.next_cursor {
            let (ordinal, _seq) = returned
                .cursor_seq
                .expect("a full page names where the NEXT page starts");
            assert_eq!(
                ordinal, next_cursor,
                "the pin's ordinal must equal the cursor it accompanies, or the next page \
                 silently falls back to the full ordinal walk"
            );
        }

        // THE PROPERTY (risk row R1). A resumed page reads only its own probe
        // span, so its scan cost is bounded by `limit * SCAN_SPAN_FACTOR` and
        // — the part that actually matters — does NOT grow with how far into
        // the chain the page sits. Before fix round 1 every one of these read
        // all 1204 rows and the pin bought only the sha256.
        let scanned = page
            .scanned
            .expect("every page reports what its position scan read");
        resumed_scans.push(scanned);
        assert!(
            scanned <= 64 * 8,
            "a resumed page must stay inside its probe span (limit 64 x SCAN_SPAN_FACTOR 8), \
             got {scanned}"
        );
        assert!(
            scanned < first_scanned,
            "a resumed page must read strictly less than the whole-chain walk it replaces \
             ({scanned} vs {first_scanned})"
        );

        walked += page.records.len();
        cursor = page.next_cursor;
        pin = returned;
    }
    assert_eq!(
        walked as u32, N,
        "the pinned walk reaches every app record exactly once"
    );
    assert_eq!(pages, 4, "200 records at EXPORT_CAP 64 is 4 pages");
    println!("[G8] scanned: first(unpinned)={first_scanned}, resumed={resumed_scans:?}");
    // Flat, not merely bounded: the LAST resumed page is no more expensive than
    // the first. That is the difference between "linear walk" and "quadratic
    // walk with a smaller constant".
    assert!(
        resumed_scans.last().unwrap() <= resumed_scans.first().unwrap(),
        "scan cost must not grow with chain position — got {resumed_scans:?}"
    );
    assert!(
        resumed_scans.iter().sum::<u32>() < first_scanned,
        "EVERY resumed page together must read less than ONE unpinned walk — got \
         {resumed_scans:?} against {first_scanned}"
    );

    // --- the pin has teeth: a mid-walk write refuses the next resumed page --
    let _mid: ActionHash = conductor
        .call(
            &zome,
            "register_node",
            node_registration("g8-mid-walk", &alice),
        )
        .await;

    let err = conductor
        .call_fallible::<_, ExportPage>(
            &zome,
            "export_records",
            ExportInputResumed {
                cursor: Some(64),
                limit: 64,
                resume: Some(first_pin.clone()),
            },
        )
        .await
        .expect_err("a resumed page against a moved chain MUST be refused, not served");
    let msg = format!("{err:?}");
    println!("[G8] mid-walk refusal:\n{msg}");
    assert!(
        msg.contains("chain moved") && msg.contains("restart at 0"),
        "expected the NAMED chain-moved refusal (a driver matches on it to restart), got: {msg}"
    );

    // --- and the named remedy actually works -------------------------------
    let fresh: ExportPage = conductor
        .call(
            &zome,
            "export_records",
            ExportInput {
                cursor: None,
                limit: 64,
            },
        )
        .await;
    assert_eq!(
        fresh.total,
        Some(N + 1),
        "restarting at 0 sees the record the mid-walk write added"
    );
    assert_ne!(
        fresh.digest, p1.digest,
        "a chain that grew has a different digest — which is the fact the refusal was protecting"
    );
    let fresh_pin = fresh.resume.expect("the restarted walk mints its own pin");
    assert_ne!(
        fresh_pin.head, first_pin.head,
        "the new pin names the NEW chain head"
    );

    // An UNPINNED page against the same moved chain is never refused — the pin
    // is opt-in, and the landed storage driver (which sends none) keeps working
    // exactly as it did.
    let unpinned: ExportPage = conductor
        .call(
            &zome,
            "export_records",
            ExportInput {
                cursor: Some(64),
                limit: 64,
            },
        )
        .await;
    assert_eq!(
        unpinned.digest, fresh.digest,
        "an unpinned page recomputes against the chain as it is now — today's behaviour, unchanged"
    );
    assert_eq!(
        unpinned.scanned,
        Some(1210),
        "and it pays the full walk for it: 1204 + one more register_node's six actions. The \
         pin is what buys the bounded scan; without one, nothing changed"
    );

    Ok(())
}

// ============================================================================
// Task 18 — A NEIGHBOUR'S RECORD, HELD-CARRIED
//
// Station 5's live measurement found the held-carry branch of `carry_from`
// unreachable: `export_records` is a local `query()`, so a v1 cell can only
// ever hand the sweep its OWN chain, and every record it returns is therefore
// a self-carry. The gap is not in v2's carrying — it is that v1 had no held
// VIEW to offer.
//
// Task 18 adds one, unconditionally and coordinator-only (hash-neutral):
// `export_held_records(agent, cursor, limit)` reads a NEIGHBOUR's chain through
// the agent-activity authority and `get`s each record from the DHT, in the same
// `ExportPage` shape and with the same digest recipe. `known_agents()` names
// whom to ask. On the v2 side `CarryInput.source` selects between them.
//
// The shape here: alice and bob are DIFFERENT agents on the SAME v1 DHT; only
// alice holds v2. Bob authors a registration; alice's v2 carries it through her
// OWN v1 cell — courier, not author. The record must land as a HELD-carry
// (`entry: Some`, bytes included so v2's validator can check the entry hash),
// never re-created natively, with `self_carried == 0` and one witness authored
// by alice.
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn held_carry_pulls_a_neighbours_v1_record_into_v2() -> Result<()> {
    let Some(_v2) = v2_bundle_or_skip() else { return Ok(()); };

    // Bootstrap ON (the `two_agent_conductors` default): this test needs the
    // pair to find each other on the v1 space WITHOUT `exchange_peer_info`,
    // which cannot be used here — it only returns true when every space across
    // the batch holds an agent info per conductor, and v2 exists on alice
    // alone (measured in probe B2).
    let [(mut ca, alice), (mut cb, bob)] = two_agent_conductors().await?;
    let seed = network_seed(DNA);

    let v1_props = SerializedBytes::try_from(LineageProperties {
        progenitor_pubkey: Some(alice.to_string()),
        lineage: vec![],
        constitution_root: None,
    })?;
    let v1_dna = load_dna_from_path(&v1_path(), &seed, Some(v1_props)).await?;
    let v1_hash = v1_dna.dna_hash().clone();

    let v2_props = SerializedBytes::try_from(LineageProperties {
        progenitor_pubkey: Some(alice.to_string()),
        lineage: vec![v1_hash.clone()],
        constitution_root: None,
    })?;
    let v2_dna = load_dna_from_path(&v2_path(), &seed, Some(v2_props)).await?;
    let v2_hash = v2_dna.dna_hash().clone();
    assert_ne!(v1_hash, v2_hash);

    // alice: v1 AND v2 (she is the courier). bob: v1 only.
    let app_a1 = ca
        .setup_app_for_agent("node-registry-v1", alice.clone(), &[v1_dna.clone()])
        .await?;
    let app_a2 = ca
        .setup_app_for_agent("node-registry-v2", alice.clone(), &[v2_dna])
        .await?;
    let app_b1 = cb
        .setup_app_for_agent("node-registry-v1", bob.clone(), &[v1_dna])
        .await?;

    let cell_a1 = app_a1.cells().first().expect("alice v1 cell").clone();
    let cell_b1 = app_b1.cells().first().expect("bob v1 cell").clone();
    let za1 = cell_a1.zome(ZOME);
    let za2 = app_a2.cells().first().expect("alice v2 cell").zome(ZOME);
    let zb1 = cell_b1.zome(ZOME);

    println!("[task18] v1 dna hash = {v1_hash}");
    println!("[task18] v2 dna hash = {v2_hash}");
    println!("[task18] alice       = {alice}");
    println!("[task18] bob         = {bob}");

    // Pay bob's one-time wasm instantiation before any bounded window.
    let _: Vec<NodeRegistration> = cb.call(&zb1, "get_my_nodes", ()).await;

    // --- 1. BOB authors the fact ------------------------------------------
    let (_bob_ah, bob_sah) = author_and_read_back(&cb, &zb1, "neighbour-node", &bob).await;
    let bob_eh = bob_sah
        .action()
        .entry_hash()
        .cloned()
        .expect("a Create action commits to an entry hash");
    println!("[task18] bob action = {}", bob_sah.action_address());
    println!("[task18] bob entry  = {bob_eh}");

    // bob's own local view, for the like-for-like comparison below
    let bob_page: ExportPage = cb
        .call(&zb1, "export_records", ExportInput { cursor: None, limit: 16 })
        .await;
    assert_eq!(bob_page.total, Some(1), "bob committed exactly one app-entry record");

    await_consistency_s(120, [&cell_a1, &cell_b1])
        .await
        .map_err(|e| anyhow::anyhow!("DHT consistency timeout on the v1 space: {e}"))?;

    // --- 2. ALICE's v1 cell can SEE bob's chain (Task 18's held view) ------
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    let mut held: ExportPage;
    loop {
        held = ca
            .call(
                &za1,
                "export_held_records",
                ExportHeldInput { agent: bob.clone(), cursor: None, limit: 16 },
            )
            .await;
        if held.total == Some(1) || std::time::Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    println!(
        "[task18] alice's held view of bob: total={:?} observed_head={:?} digest={} records={}",
        held.total,
        held.observed_head,
        held.digest,
        held.records.len()
    );
    assert_eq!(
        held.total,
        Some(1),
        "alice's v1 cell must see bob's one app-entry record through the agent-activity \
         authority — this is exactly what `export_records` (a local query) cannot do"
    );
    assert_eq!(
        held.digest, bob_page.digest,
        "LIKE-FOR-LIKE across peers: alice's held digest of bob's chain must equal bob's own \
         — the two views have converged at this point, which is what await_consistency bought"
    );
    // Length first: `records[0]` below is only meaningful once the page is known
    // to hold exactly the one record, and a bare index would panic opaquely on
    // an empty page rather than naming what went wrong.
    assert_eq!(
        held.records.len(),
        1,
        "the held page must hold exactly bob's one record, got {}",
        held.records.len()
    );
    assert_eq!(
        held.entries.len(),
        held.records.len(),
        "records and entries are paired POSITIONALLY"
    );
    assert_eq!(
        held.records[0].action_address(),
        bob_sah.action_address(),
        "the held page must carry bob's real signed action"
    );
    assert!(
        held.entries[0].is_some(),
        "a held page ships the entry BYTES — v2's validator checks them against the \
         carried action's entry hash"
    );
    // The truncation check a held page CAN make about itself. MEASURED: bob's
    // chain head is 9 — 3 genesis actions, InitZomesComplete, then his one
    // register_node as SIX actions (the Create plus five CreateLinks). Against a
    // `total` of 1 that is a distance of 8 which is ALL bookkeeping, not a chain
    // alice is behind on: the distance alone proves nothing, which is why the
    // real completeness check is the agreement with bob's own head below.
    let held_head = held
        .observed_head
        .expect("alice's authority observed bob's chain, so it must report a head");
    assert!(
        held_head >= held.total.unwrap() - 1,
        "observed_head is a sequence over EVERY action and can never sit below the \
         app-entry count minus one — got head {held_head}, total {:?}",
        held.total
    );
    assert_eq!(
        held_head, 9,
        "MEASURED (holochain 0.7.0): 3 genesis + InitZomesComplete + 6 register_node \
         actions on bob's chain"
    );
    assert_eq!(
        held.observed_head, bob_page.observed_head,
        "alice's view of bob's chain HEAD must agree with bob's own — if it did not, \
         alice would be carrying a chain she has not caught up with"
    );

    // --- 3. and knows WHOM to ask ------------------------------------------
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let mut agents: Vec<AgentPubKey>;
    loop {
        agents = ca.call(&za1, "known_agents", ()).await;
        if agents.contains(&bob) || std::time::Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    println!("[task18] alice's known_agents = {agents:?}");
    assert!(
        agents.contains(&bob),
        "known_agents is how a sweep enumerates whose chains to carry — bob registered a \
         node on this DHT and must appear, got {agents:?}"
    );

    // --- 4. the CARRY: alice's v2 pulls bob's record through her own v1 -----
    let receipt: CarryReceipt = ca
        .call(
            &za2,
            "carry_from",
            CarryInputHeld {
                v1_cell: cell_a1.cell_id().clone(),
                cursor: None,
                limit: 16,
                source: CarrySource::Held(bob.clone()),
            },
        )
        .await;
    println!("[task18] receipt = {receipt:?}");

    assert_eq!(
        receipt.carried,
        held.total.expect("alice's held view reported a total"),
        "everything ALICE HAD OF BOB was carried — a held receipt describes the courier's \
         own integrated view, never a claim about bob's whole chain. (Here the two happen \
         to agree, because await_consistency ran first.)"
    );
    assert_eq!(receipt.carried, 1);
    assert_eq!(
        receipt.self_carried, 0,
        "bob's record is NOT alice's to re-create — a held-carry is never a self-carry"
    );
    assert_eq!(
        receipt.v1_digest, held.digest,
        "the receipt reports the digest of the walk it actually carried — alice's view \
         of bob's chain"
    );
    assert_eq!(receipt.v1_total, Some(1));
    assert_eq!(
        receipt.v1_observed_head, held.observed_head,
        "observed_head threads through the receipt, so a driver can tell a complete \
         held sweep from a courier that is behind"
    );
    assert_eq!(
        receipt.next_cursor, None,
        "a partial page offers no further cursor — on the held path that means \
         end-of-LOCAL-VIEW, not end-of-bob's-chain"
    );
    assert!(!receipt.witness_hash.is_empty(), "a non-empty page commits exactly one witness");

    // Task 29: the receipt SAYS which read answered, and what it cost.
    //
    // Deliberately NOT pinned to one label here. Alice and bob are two peers on
    // one DHT, so bob's agent-activity authority may be alice herself or the
    // remote — and which one serves a given call is a network fact, not a
    // property of the carry. What the receipt must never do is stay silent: a
    // held page whose view is unknown is exactly the shape that let Station 6
    // record a frozen partial-arc read as a completed crossing.
    let view = receipt
        .view
        .as_deref()
        .expect("a HELD receipt must name the view that answered — silence is the Station 6 bug");
    assert!(
        view == "authority" || view == "local-only",
        "the view label is one of the two the zome defines, got {view:?}"
    );
    println!(
        "[task18] receipt view = {view}, scanned = {:?}",
        receipt.scanned
    );
    assert_eq!(
        receipt.scanned,
        Some(1),
        "`scanned` threads through from the export page — on the held path it is the \
         size of the activity list the answering view returned, and bob has one app entry"
    );

    // --- 5. ONE witness, at bob's entry hash, carrying the BYTES ------------
    let links: Vec<Link> = ca.call(&za2, "get_witnesses_for", bob_eh.clone()).await;
    assert_eq!(
        links.len(),
        1,
        "carry_from commits exactly ONE witness per page, got {} links",
        links.len()
    );
    let witness_action = links[0]
        .target
        .clone()
        .into_action_hash()
        .expect("the witness link targets an action hash");
    assert_eq!(
        witness_action.to_string(),
        receipt.witness_hash,
        "the witness link must target the witness the receipt names"
    );

    let witness_record: Option<Record> = ca
        .call(&za2, "get_record_at", (witness_action.clone(), true))
        .await;
    let witness_record = witness_record.expect("the witness alice just authored is on her chain");
    assert_eq!(
        witness_record.action().author(),
        &alice,
        "COURIER semantics: the carrying agent authors the witness, not the original author"
    );
    let Some(Entry::App(app_bytes)) = witness_record.entry().as_option() else {
        panic!("the witness record must carry an app entry");
    };
    let witness: NotarizationWitness = holochain_serialized_bytes::decode(app_bytes.bytes())
        .expect("the witness entry decodes to NotarizationWitness");
    assert_eq!(witness.lineage_dna_hash, v1_hash);
    assert_eq!(witness.proofs.len(), 1, "one carried record, one proof");
    assert!(
        witness.proofs[0].entry.is_some(),
        "HELD-CARRY (§2.2): the bytes ride inside the witness, because the courier cannot \
         re-create another agent's entry natively"
    );
    assert_eq!(
        witness.proofs[0].action.author(),
        &bob,
        "the carried notarization is BOB's — alice only witnessed it"
    );

    // --- 6. and v2 did NOT re-create bob's entry as its own commit ----------
    let v2_page: ExportPage = ca
        .call(&za2, "export_records", ExportInput { cursor: None, limit: 64 })
        .await;
    assert!(
        v2_page
            .records
            .iter()
            .all(|r| r.action().entry_hash() != Some(&bob_eh)),
        "a held-carry must NEVER be re-created natively — alice's chain would then claim \
         authorship of bob's record"
    );

    // --- 7. NEGATIVE: Held(self) is a refusal, not a silent self-carry ------
    let err = ca
        .call_fallible::<_, CarryReceipt>(
            &za2,
            "carry_from",
            CarryInputHeld {
                v1_cell: cell_a1.cell_id().clone(),
                cursor: None,
                limit: 16,
                source: CarrySource::Held(alice.clone()),
            },
        )
        .await
        .expect_err("Held(self) MUST be refused — use CarrySource::Own");
    let msg = format!("{err:?}");
    println!("[task18] NEGATIVE Held(self) refusal:\n{msg}");
    assert!(
        msg.contains("Held(self)"),
        "expected the Held(self) refusal, got: {msg}"
    );

    Ok(())
}

// ============================================================================
// HELD-CARRY RETRY — Task 20, epic §7 C6b: a retried held-carry page must not
// commit a second witness either. Same two-agent fixture as
// `held_carry_pulls_a_neighbours_v1_record_into_v2` above (alice holds v1 AND
// v2 and is the courier; bob holds v1 only and is the author) — the SAME held
// page is carried twice from the same cursor.
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn held_carry_retry_is_idempotent() -> Result<()> {
    let Some(_v2) = v2_bundle_or_skip() else { return Ok(()); };

    let [(mut ca, alice), (mut cb, bob)] = two_agent_conductors().await?;
    let seed = network_seed(DNA);

    let v1_props = SerializedBytes::try_from(LineageProperties {
        progenitor_pubkey: Some(alice.to_string()),
        lineage: vec![],
        constitution_root: None,
    })?;
    let v1_dna = load_dna_from_path(&v1_path(), &seed, Some(v1_props)).await?;
    let v1_hash = v1_dna.dna_hash().clone();

    let v2_props = SerializedBytes::try_from(LineageProperties {
        progenitor_pubkey: Some(alice.to_string()),
        lineage: vec![v1_hash.clone()],
        constitution_root: None,
    })?;
    let v2_dna = load_dna_from_path(&v2_path(), &seed, Some(v2_props)).await?;
    let v2_hash = v2_dna.dna_hash().clone();
    assert_ne!(v1_hash, v2_hash);

    // alice: v1 AND v2 (she is the courier). bob: v1 only.
    let app_a1 = ca
        .setup_app_for_agent("node-registry-v1", alice.clone(), &[v1_dna.clone()])
        .await?;
    let app_a2 = ca
        .setup_app_for_agent("node-registry-v2", alice.clone(), &[v2_dna])
        .await?;
    let app_b1 = cb
        .setup_app_for_agent("node-registry-v1", bob.clone(), &[v1_dna])
        .await?;

    let cell_a1 = app_a1.cells().first().expect("alice v1 cell").clone();
    let cell_b1 = app_b1.cells().first().expect("bob v1 cell").clone();
    let za1 = cell_a1.zome(ZOME);
    let za2 = app_a2.cells().first().expect("alice v2 cell").zome(ZOME);
    let zb1 = cell_b1.zome(ZOME);

    println!("[task20] v1 dna hash = {v1_hash}");
    println!("[task20] v2 dna hash = {v2_hash}");
    println!("[task20] alice       = {alice}");
    println!("[task20] bob         = {bob}");

    // Pay bob's one-time wasm instantiation before any bounded window.
    let _: Vec<NodeRegistration> = cb.call(&zb1, "get_my_nodes", ()).await;

    // --- 1. BOB authors the fact --------------------------------------------
    let (_bob_ah, bob_sah) =
        author_and_read_back(&cb, &zb1, "neighbour-retry-node", &bob).await;
    let bob_eh = bob_sah
        .action()
        .entry_hash()
        .cloned()
        .expect("a Create action commits to an entry hash");
    println!("[task20] bob action = {}", bob_sah.action_address());
    println!("[task20] bob entry  = {bob_eh}");

    await_consistency_s(120, [&cell_a1, &cell_b1])
        .await
        .map_err(|e| anyhow::anyhow!("DHT consistency timeout on the v1 space: {e}"))?;

    // --- 2. ALICE's v1 cell can SEE bob's chain (Task 18's held view) ------
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    let mut held: ExportPage;
    loop {
        held = ca
            .call(
                &za1,
                "export_held_records",
                ExportHeldInput { agent: bob.clone(), cursor: None, limit: 16 },
            )
            .await;
        if held.total == Some(1) || std::time::Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    assert_eq!(
        held.total,
        Some(1),
        "alice's v1 cell must see bob's one app-entry record before the carry can run, got \
         {:?}",
        held.total
    );

    // --- 3. and knows WHOM to ask -------------------------------------------
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let mut agents: Vec<AgentPubKey>;
    loop {
        agents = ca.call(&za1, "known_agents", ()).await;
        if agents.contains(&bob) || std::time::Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    assert!(
        agents.contains(&bob),
        "bob must be known before he can be carried, got {agents:?}"
    );

    let carry_input = || CarryInputHeld {
        v1_cell: cell_a1.cell_id().clone(),
        cursor: None,
        limit: 16,
        source: CarrySource::Held(bob.clone()),
    };

    // --- 4. the FIRST held carry: alice's v2 pulls bob's record -------------
    let first: CarryReceipt = ca.call(&za2, "carry_from", carry_input()).await;
    println!("[task20] first held receipt = {first:?}");
    assert_eq!(first.carried, 1, "the page held exactly bob's one record");
    assert_eq!(first.self_carried, 0, "bob's record is never a self-carry");
    assert_eq!(
        first.already_carried, 0,
        "nothing was already carried before the first pass ran"
    );
    assert!(
        !first.witness_hash.is_empty(),
        "a non-empty first page commits exactly one witness"
    );

    // --- 5. the SAME held page, retried at cursor 0 --------------------------
    let retry: CarryReceipt = ca.call(&za2, "carry_from", carry_input()).await;
    println!("[task20] retry held receipt = {retry:?}");

    assert_eq!(
        retry.carried, first.carried,
        "a held retry reports the same `carried` count — alice already carried what she had \
         of bob"
    );
    assert_eq!(
        retry.self_carried, 0,
        "a held-carry is never a self-carry, retried or not"
    );
    assert_eq!(
        retry.already_carried, retry.carried,
        "on a retry every record on the held page was already witnessed from this lineage"
    );
    assert_eq!(
        retry.witness_hash, "",
        "a held page with zero new proofs authors NO witness — the empty string is the \
         landed storage decoder's \"no witness this page\" signal"
    );

    // --- 6. exactly ONE witness link survives the retry ----------------------
    let links: Vec<Link> = ca.call(&za2, "get_witnesses_for", bob_eh.clone()).await;
    assert_eq!(
        links.len(),
        1,
        "a retried held-carry must not commit a second witness or a second EntryToWitness \
         link — got {} links",
        links.len()
    );

    Ok(())
}

// ============================================================================
// STATION 8 — the sunset's fence (epic §3 (ii), §4 step 5, §8)
//
// Probe B measured that `close_chain` is not a source-chain guard; Probe B2
// measured that the REMOTE agent-activity authority refuses exactly the
// CloseChain's immediate successor and issues a warrant, while the tail
// validates again and the bytes stay fetchable. So Holochain's contribution to
// the sunset is EVIDENCE, and the fence is ours:
//
//   (i)  the storage controller disables the v1 cell (Task 14b), and
//   (ii) v2's witness validation refuses a carried v1 proof that sits AFTER
//        that chain's close — the two tests below.
//
// (ii) has two deterministic halves, and each gets its own test because they
// reach the close by different routes and must be able to fail separately:
//   * INTRA-WITNESS — the close and a post-close fact in ONE batch;
//   * ACROSS WITNESSES — the close was carried by an EARLIER witness on this
//     carrier's own v2 chain, which is what `seal_close` commits.
//
// A third fact both tests rest on: **absence of a close is not a rule.** Every
// probe above carries proofs with no close anywhere and is accepted; the
// pre-close proof carried AFTER the seal in the second test below is the
// positive control that the rule does not over-refuse.
// ============================================================================

/// Mirror of `node_registry_coordinator::SealReceipt` (Task 14a).
///
/// Every hash is a base64 `String`, not a native `HoloHash` — the same wire
/// discipline `CarryReceipt::witness_hash` documents, so the storage-side
/// vehicle (Task 14b) decodes strings and never re-derives a hash.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SealReceipt {
    close_hash: String,
    open_hash: String,
    witness_hash: String,
    already_sealed: bool,
    /// Fix round 1 (additive): the seal found v1 ALREADY closed toward this DNA
    /// with no matching `OpenChain` here — a half-seal — and resumed at the open
    /// step instead of closing v1 twice.
    #[serde(default)]
    resumed: bool,
}

/// Every `CloseChain` on `zome`'s chain, as `(action_seq, action_hash)`.
///
/// Reads through the same two pristine-v1 externs the zome's half-seal probe
/// uses, so the test measures what the zome can actually see.
async fn close_chain_actions(
    conductor: &SweetConductor,
    zome: &SweetZome,
) -> Vec<(u32, ActionHash)> {
    let activity: AgentActivityStatus = conductor.call(zome, "my_chain_activity", ()).await;
    let mut found = Vec::new();
    for (seq, hash) in activity.valid_activity {
        let record: Option<Record> = conductor
            .call(zome, "get_record_at", (hash.clone(), true))
            .await;
        if let Some(record) = record {
            if matches!(record.action().data, ActionData::CloseChain(_)) {
                found.push((seq, hash));
            }
        }
    }
    found.sort_by_key(|(seq, _)| *seq);
    found
}

/// INTRA-WITNESS half: one batch may not carry a close AND a fact authored
/// after it.
#[tokio::test(flavor = "multi_thread")]
async fn station_8_close_and_a_post_close_proof_in_one_witness_is_refused() -> Result<()> {
    let Some(_v2) = v2_bundle_or_skip() else { return Ok(()); };
    let Crossing {
        conductor,
        alice,
        v1_hash,
        v2_hash,
        z1,
        z2,
    } = install_crossing().await?;

    // A fact authored BEFORE the close — this one is always carryable.
    let (_pre_ah, pre) = author_and_read_back(&conductor, &z1, "s8-pre-close", &alice).await;

    // v1 closes toward v2. Called directly (not through `seal_close`) so this
    // test measures the intra-witness rule alone: nothing is committed on v2's
    // chain, so the across-witness half cannot fire and mask it.
    let close_hash: ActionHash = conductor
        .call_fallible(&z1, "close_chain_for", v2_hash.clone())
        .await
        .map_err(|e| anyhow::anyhow!("close_chain on v1 was refused: {e:?}"))?;
    let signed_close: SignedActionHashed = conductor
        .call::<_, Option<SignedActionHashed>>(&z1, "get_signed_action", close_hash.clone())
        .await
        .expect("the CloseChain just authored must be on v1's chain");
    println!(
        "[station 8] v1 CloseChain = {close_hash} at action_seq {}",
        signed_close.action().header.action_seq
    );

    // And writes on v1 anyway — accepted by the author's own conductor, as
    // probes B and B2 measured. That acceptance is precisely why v2 needs a
    // rule of its own.
    let (_post_ah, post) = author_and_read_back(&conductor, &z1, "s8-post-close", &alice).await;
    assert!(
        post.action().header.action_seq > signed_close.action().header.action_seq,
        "the post-close write must sit after the close on v1's chain"
    );

    let close_proof = CarriedProof {
        action: signed_close.action().clone(),
        signature: signed_close.signature.clone(),
        entry: None,
    };

    // POSITIVE control: the close plus a PRE-close fact is fine — the rule
    // fences what comes after the close, not the whole batch.
    let ok = NotarizationWitness {
        lineage_dna_hash: v1_hash.clone(),
        proofs: vec![
            close_proof.clone(),
            CarriedProof {
                action: pre.action().clone(),
                signature: pre.signature.clone(),
                entry: None,
            },
        ],
    };
    let ok_hash: ActionHash = conductor.call(&z2, "commit_witness", ok).await;
    println!("[station 8] close + pre-close proof ACCEPTED at {ok_hash}");

    // NEGATIVE: the close plus a POST-close fact, same batch.
    let refused = NotarizationWitness {
        lineage_dna_hash: v1_hash.clone(),
        proofs: vec![
            close_proof,
            CarriedProof {
                action: post.action().clone(),
                signature: post.signature.clone(),
                entry: None,
            },
        ],
    };
    let err = conductor
        .call_fallible::<_, ActionHash>(&z2, "commit_witness", refused)
        .await
        .expect_err("a proof authored after the carried close MUST be refused");
    let msg = format!("{err:?}");
    println!("[station 8] NEGATIVE intra-witness refusal:\n{msg}");
    assert!(
        msg.contains("after close"),
        "expected the after-close refusal BY NAME, got: {msg}"
    );

    Ok(())
}

/// ACROSS-WITNESSES half, and the seal itself: `seal_close` closes v1, opens v2
/// from that close and witnesses the close here — in that order — after which
/// v1 stays readable, a pre-close fact still carries, and a post-close fact is
/// refused on every later witness by name.
#[tokio::test(flavor = "multi_thread")]
async fn station_8_seal_close_then_post_close_carry_is_refused_by_v2() -> Result<()> {
    let Some(_v2) = v2_bundle_or_skip() else { return Ok(()); };
    let Crossing {
        conductor,
        alice,
        v1_hash,
        v2_hash,
        z1,
        z2,
    } = install_crossing().await?;

    // Two pre-close facts: one carried before the seal, one held back as the
    // positive control that the rule does not fence what came before the close.
    let (_a_ah, pre_a) = author_and_read_back(&conductor, &z1, "s8-seal-pre-a", &alice).await;
    let (_b_ah, pre_b) = author_and_read_back(&conductor, &z1, "s8-seal-pre-b", &alice).await;

    let carried_before: ActionHash = conductor
        .call(
            &z2,
            "commit_witness",
            NotarizationWitness {
                lineage_dna_hash: v1_hash.clone(),
                proofs: vec![CarriedProof {
                    action: pre_a.action().clone(),
                    signature: pre_a.signature.clone(),
                    entry: None,
                }],
            },
        )
        .await;
    println!("[station 8] pre-seal carry ACCEPTED at {carried_before}");

    // --- the seal -----------------------------------------------------------
    let seal: SealReceipt = conductor
        .call(&z2, "seal_close", z1.cell_id().clone())
        .await;
    println!("[station 8] seal = {seal:?}");
    assert!(!seal.already_sealed, "the first seal authors the crossing");
    assert!(!seal.close_hash.is_empty() && !seal.open_hash.is_empty());
    assert!(
        !seal.witness_hash.is_empty(),
        "the seal must carry the close into v2 as a proof — that is what lets \
         every later witness be validated against it"
    );

    // close BEFORE open, and the open NAMES the close.
    let close_hash = ActionHash::try_from(seal.close_hash.as_str())
        .map_err(|e| anyhow::anyhow!("close_hash is not an ActionHash: {e:?}"))?;
    let open_hash = ActionHash::try_from(seal.open_hash.as_str())
        .map_err(|e| anyhow::anyhow!("open_hash is not an ActionHash: {e:?}"))?;
    let signed_close: SignedActionHashed = conductor
        .call::<_, Option<SignedActionHashed>>(&z1, "get_signed_action", close_hash.clone())
        .await
        .expect("the CloseChain must be on v1's chain");
    let signed_open: SignedActionHashed = conductor
        .call::<_, Option<SignedActionHashed>>(&z2, "get_signed_action", open_hash.clone())
        .await
        .expect("the OpenChain must be on v2's chain");
    match &signed_close.action().data {
        ActionData::CloseChain(d) => assert_eq!(
            d.new_target,
            Some(MigrationTarget::Dna(v2_hash.clone())),
            "v1 must close TOWARD v2"
        ),
        other => panic!("close_hash does not name a CloseChain: {other:?}"),
    }
    match &signed_open.action().data {
        ActionData::OpenChain(d) => {
            assert_eq!(d.close_hash, close_hash, "the open must name the close");
            assert_eq!(
                d.prev_target,
                MigrationTarget::Dna(v1_hash.clone()),
                "v2 must open FROM v1"
            );
        }
        other => panic!("open_hash does not name an OpenChain: {other:?}"),
    }

    // --- v1 stays readable forever ------------------------------------------
    let still_there: Option<SignedActionHashed> = conductor
        .call(&z1, "get_signed_action", pre_a.as_hash().clone())
        .await;
    assert!(
        still_there.is_some(),
        "a closed chain must stay READABLE — the sunset closes it, it does not erase it"
    );
    let v1_page: ExportPage = conductor
        .call(&z1, "export_records", ExportInput { cursor: None, limit: 16 })
        .await;
    assert_eq!(
        v1_page.records.len(),
        2,
        "both pre-close records must still export from the closed chain"
    );

    // --- the seal is idempotent ---------------------------------------------
    let again: SealReceipt = conductor
        .call(&z2, "seal_close", z1.cell_id().clone())
        .await;
    assert!(
        again.already_sealed,
        "a second seal must author NOTHING — a CloseChain cannot be taken back"
    );
    assert_eq!(again.close_hash, seal.close_hash);
    assert_eq!(again.open_hash, seal.open_hash);

    // --- POSITIVE control: a pre-close fact still carries after the seal -----
    let after_seal: ActionHash = conductor
        .call(
            &z2,
            "commit_witness",
            NotarizationWitness {
                lineage_dna_hash: v1_hash.clone(),
                proofs: vec![CarriedProof {
                    action: pre_b.action().clone(),
                    signature: pre_b.signature.clone(),
                    entry: None,
                }],
            },
        )
        .await;
    println!("[station 8] post-seal carry of a PRE-close fact ACCEPTED at {after_seal}");

    // --- the harness writes on v1 anyway ------------------------------------
    let post_close: Result<ActionHash, _> = conductor
        .call_fallible(
            &z1,
            "register_node",
            node_registration("s8-seal-post", &alice),
        )
        .await;
    assert!(
        post_close.is_ok(),
        "MEASURED CHANGE: the author's conductor now refuses a post-close create — \
         probes B and B2 measured that it does not; re-read Station 8"
    );
    let post = conductor
        .call::<_, Option<SignedActionHashed>>(
            &z1,
            "get_signed_action",
            post_close.unwrap(),
        )
        .await
        .expect("the post-close action is on v1's chain, accepted by its author");
    assert!(
        post.action().header.action_seq > signed_close.action().header.action_seq,
        "the post-close write must sit after the close"
    );
    println!(
        "[station 8] post-close v1 write ACCEPTED BY THE CONDUCTOR at seq {}",
        post.action().header.action_seq
    );

    // --- and v2 refuses to carry it, by name --------------------------------
    let err = conductor
        .call_fallible::<_, ActionHash>(
            &z2,
            "commit_witness",
            NotarizationWitness {
                lineage_dna_hash: v1_hash.clone(),
                proofs: vec![CarriedProof {
                    action: post.action().clone(),
                    signature: post.signature.clone(),
                    entry: None,
                }],
            },
        )
        .await
        .expect_err("after the seal, a post-close fact MUST be refused by v2");
    let msg = format!("{err:?}");
    println!("[station 8] NEGATIVE across-witness refusal:\n{msg}");
    assert!(
        msg.contains("after close"),
        "expected the after-close refusal BY NAME, got: {msg}"
    );

    // The same fact refused through the DRIVER, not just by hand: `carry_from`
    // pages v1's whole chain, so the post-close record is on the page and the
    // page's witness must be refused for the same reason.
    let driven = conductor
        .call_fallible::<_, CarryReceipt>(
            &z2,
            "carry_from",
            CarryInput {
                v1_cell: z1.cell_id().clone(),
                cursor: None,
                limit: 16,
            },
        )
        .await;
    match &driven {
        Ok(receipt) => panic!(
            "carry_from must not be able to land a post-close page — got {receipt:?}"
        ),
        Err(e) => {
            let msg = format!("{e:?}");
            println!("[station 8] NEGATIVE carry_from refusal:\n{msg}");
            assert!(
                msg.contains("after close"),
                "expected the after-close refusal BY NAME through the driver, got: {msg}"
            );
        }
    }

    Ok(())
}

/// The HALF-SEAL window: v1 already closed toward v2, nothing on v2 to key on.
/// A retry must RESUME at the open step, never close v1 a second time.
///
/// Why it matters, in Probe B2's own measurement: the remote agent-activity
/// authority rejects the action after a close and issues a WARRANT against its
/// author. A re-close would therefore warrant the very peer performing the
/// sunset — the sunset accusing itself.
#[tokio::test(flavor = "multi_thread")]
async fn station_8_seal_close_resumes_a_half_seal_instead_of_closing_v1_twice() -> Result<()> {
    let Some(_v2) = v2_bundle_or_skip() else { return Ok(()); };
    let Crossing {
        conductor,
        alice,
        v1_hash,
        v2_hash,
        z1,
        z2,
    } = install_crossing().await?;

    let (_pre_ah, pre) = author_and_read_back(&conductor, &z1, "s8-half-pre", &alice).await;

    // Simulate the half state: v1 is closed toward v2 by a call that then died
    // before v2 could open. `close_chain_for` alone is exactly that state.
    let close_hash: ActionHash = conductor
        .call_fallible(&z1, "close_chain_for", v2_hash.clone())
        .await
        .map_err(|e| anyhow::anyhow!("close_chain on v1 was refused: {e:?}"))?;
    println!("[station 8/half] v1 CloseChain = {close_hash}");

    let before = close_chain_actions(&conductor, &z1).await;
    assert_eq!(
        before.len(),
        1,
        "the half state has exactly one CloseChain on v1, got {before:?}"
    );

    // The retry.
    let seal: SealReceipt = conductor
        .call(&z2, "seal_close", z1.cell_id().clone())
        .await;
    println!("[station 8/half] seal = {seal:?}");

    assert!(
        seal.resumed,
        "the retry must RESUME the half-seal, not start a fresh one"
    );
    assert!(
        !seal.already_sealed,
        "the half state has no OpenChain, so this is not an already-sealed crossing"
    );
    assert_eq!(
        seal.close_hash,
        close_hash.to_string(),
        "the resumed seal must adopt v1's EXISTING close, not a new one"
    );

    // The measurement that matters: v1 was not closed twice.
    let after = close_chain_actions(&conductor, &z1).await;
    assert_eq!(
        after, before,
        "a resumed seal must author NO second CloseChain on v1 — got {after:?}"
    );

    // And the crossing is complete: the open names that close, and the witness
    // carries it, so the after-close rule is armed on every later witness.
    assert!(!seal.open_hash.is_empty() && !seal.witness_hash.is_empty());
    let open_hash = ActionHash::try_from(seal.open_hash.as_str())
        .map_err(|e| anyhow::anyhow!("open_hash is not an ActionHash: {e:?}"))?;
    let signed_open: SignedActionHashed = conductor
        .call::<_, Option<SignedActionHashed>>(&z2, "get_signed_action", open_hash)
        .await
        .expect("the OpenChain must be on v2's chain");
    match &signed_open.action().data {
        ActionData::OpenChain(d) => {
            assert_eq!(d.close_hash, close_hash);
            assert_eq!(d.prev_target, MigrationTarget::Dna(v1_hash.clone()));
        }
        other => panic!("open_hash does not name an OpenChain: {other:?}"),
    }

    // A third call now finds the completed seal and still authors nothing.
    let third: SealReceipt = conductor
        .call(&z2, "seal_close", z1.cell_id().clone())
        .await;
    assert!(third.already_sealed, "the completed seal is idempotent");
    assert_eq!(third.close_hash, seal.close_hash);
    assert_eq!(
        third.witness_hash, seal.witness_hash,
        "the reported seal witness must be the one carrying THIS close"
    );
    assert_eq!(
        close_chain_actions(&conductor, &z1).await,
        before,
        "no seal call may ever author a second CloseChain on v1"
    );

    // The fence is armed all the same: a post-close fact is still refused.
    let post_ah: ActionHash = conductor
        .call(&z1, "register_node", node_registration("s8-half-post", &alice))
        .await;
    let post: SignedActionHashed = conductor
        .call::<_, Option<SignedActionHashed>>(&z1, "get_signed_action", post_ah)
        .await
        .expect("the post-close action is on v1's chain");
    let err = conductor
        .call_fallible::<_, ActionHash>(
            &z2,
            "commit_witness",
            NotarizationWitness {
                lineage_dna_hash: v1_hash.clone(),
                proofs: vec![CarriedProof {
                    action: post.action().clone(),
                    signature: post.signature.clone(),
                    entry: None,
                }],
            },
        )
        .await
        .expect_err("a resumed seal must arm the after-close fence like any other");
    let msg = format!("{err:?}");
    println!("[station 8/half] NEGATIVE refusal after a RESUMED seal:\n{msg}");
    assert!(
        msg.contains("after close"),
        "expected the after-close refusal BY NAME, got: {msg}"
    );

    // And the pre-close fact still carries — the resume did not over-fence.
    let ok: ActionHash = conductor
        .call(
            &z2,
            "commit_witness",
            NotarizationWitness {
                lineage_dna_hash: v1_hash.clone(),
                proofs: vec![CarriedProof {
                    action: pre.action().clone(),
                    signature: pre.signature.clone(),
                    entry: None,
                }],
            },
        )
        .await;
    println!("[station 8/half] pre-close carry after a RESUMED seal ACCEPTED at {ok}");

    Ok(())
}

// ============================================================================
// STATION 7 — REVERT BEFORE SUNSET (Holochain Evolution Epic §4 step 4, Task
// 13b; the ZOME half — the storage vehicle's revert path is Task 13a)
//
// The crossing has happened: v1's fact was carried into v2 under a witness,
// and the agent then authored a WINDOW-TIME fact on v2 — one that exists
// nowhere else. Now the elohim revoke the migration commitment inside its
// horizon and the peer returns to authoring on v1.
//
// Re-adoption is a RE-AUTHORING, not a carry. v1 has no witness type, so the
// agent writes its own window-time fact again, natively, on the chain it is
// returning to; the v2 action and signature stay in the disabled-but-intact v2
// cell as the evidence §7 C14 keeps. What this probe pins:
//
//   1. the window-time v2 fact lands on v1 with the SAME entry hash and a NEW
//      action — CID continuity across the revert, which is what makes the
//      re-authored fact the same fact;
//   2. the carried re-creations are NOT authored twice — they are already on
//      v1 and report as `already_present`;
//   3. v2's `NotarizationWitness` is counted `foreign` and never re-created —
//      an entry type v1 does not know must not fail the page, or revert would
//      be impossible for exactly the chains that took the crossing;
//   4. a retry re-authors NOTHING (§7 C6b) — `readopted` falls to 0 and every
//      record reports `already_present`.
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn station_7_readopt_from_re_authors_window_time_v2_records_on_v1() -> Result<()> {
    let Some(_v2) = v2_bundle_or_skip() else { return Ok(()); };
    let Crossing {
        conductor,
        alice,
        v1_hash: _,
        v2_hash: _,
        z1,
        z2,
    } = install_crossing().await?;

    // --- 1. the pre-crossing v1 fact, then the crossing ---------------------
    let (_ah_pre, sah_pre) =
        author_and_read_back(&conductor, &z1, "readopt-baseline", &alice).await;
    let eh_pre = sah_pre
        .action()
        .entry_hash()
        .cloned()
        .expect("a Create action commits to an entry hash");

    let carry: CarryReceipt = conductor
        .call(
            &z2,
            "carry_from",
            CarryInput {
                v1_cell: z1.cell_id().clone(),
                cursor: None,
                limit: 16,
            },
        )
        .await;
    println!("[station 7] carry receipt = {carry:?}");
    assert_eq!(carry.carried, 1, "one v1 fact crossed");
    assert_eq!(carry.self_carried, 1, "it was re-created natively on v2");
    assert!(
        !carry.witness_hash.is_empty(),
        "the crossing committed a witness — the entry type v1 will report `foreign`"
    );

    // --- 2. the WINDOW-TIME fact: authored on v2, existing nowhere else -----
    let (ah_window_v2, sah_window) =
        author_and_read_back(&conductor, &z2, "readopt-window", &alice).await;
    let eh_window = sah_window
        .action()
        .entry_hash()
        .cloned()
        .expect("a Create action commits to an entry hash");
    println!("[station 7] window-time v2 action = {ah_window_v2}");
    println!("[station 7] window-time entry     = {eh_window}");

    let v1_before: ExportPage = conductor
        .call(&z1, "export_records", ExportInput { cursor: None, limit: 64 })
        .await;
    assert_eq!(
        v1_before.total,
        Some(1),
        "before the revert v1 holds ONLY its own pre-crossing fact"
    );
    assert!(
        !v1_before
            .records
            .iter()
            .any(|r| r.action().entry_hash() == Some(&eh_window)),
        "the window-time fact must not be on v1 before the re-adoption — otherwise this probe \
         proves nothing"
    );

    let v2_page: ExportPage = conductor
        .call(&z2, "export_records", ExportInput { cursor: None, limit: 64 })
        .await;
    println!("[station 7] v2 chain = {:?} app records", v2_page.total);
    assert_eq!(
        v2_page.total,
        Some(3),
        "v2 holds the carried re-creation, the witness, and the window-time fact"
    );

    // --- 3. the revert: re-adopt v2's window-time records onto v1 -----------
    let readopt_input = || ReadoptInput {
        v2_cell: z2.cell_id().clone(),
        cursor: None,
        limit: 16,
    };
    let first: ReadoptReceipt = conductor.call(&z1, "readopt_from", readopt_input()).await;
    println!("[station 7] first readopt receipt = {first:?}");

    assert_eq!(
        first.readopted, 1,
        "exactly the window-time fact comes home — the carried re-creation is already here \
         and the witness is not v1's to hold"
    );
    assert_eq!(
        first.already_present, 1,
        "the carried re-creation of v1's own pre-crossing fact is already on this chain"
    );
    assert_eq!(
        first.foreign, 1,
        "v2's NotarizationWitness is an entry type v1 does not know — counted, never an error"
    );
    assert_eq!(first.next_cursor, None, "the page covered v2's whole chain");
    assert_eq!(
        first.v2_total, v2_page.total,
        "v2_total is READ from v2's export page, never derived from `readopted`"
    );
    assert_eq!(
        first.v2_digest, v2_page.digest,
        "the receipt reports v2's own whole-chain digest verbatim"
    );

    // --- 4. SAME entry hash, NEW action -------------------------------------
    let v1_after: ExportPage = conductor
        .call(&z1, "export_records", ExportInput { cursor: None, limit: 64 })
        .await;
    assert_eq!(
        v1_after.total,
        Some(2),
        "v1 now holds its pre-crossing fact and the re-authored window-time fact — and nothing \
         else: the witness was NOT re-created"
    );
    let readopted_records: Vec<_> = v1_after
        .records
        .iter()
        .filter(|r| r.action().entry_hash() == Some(&eh_window))
        .collect();
    assert_eq!(
        readopted_records.len(),
        1,
        "exactly one v1 action now commits to the window-time entry hash"
    );
    let ah_window_v1 = readopted_records[0].action_address().clone();
    println!("[station 7] re-authored on v1 at action {ah_window_v1}");
    assert_ne!(
        ah_window_v1, ah_window_v2,
        "re-adoption is a RE-AUTHORING: a NEW action on v1, not the v2 action moved"
    );
    assert_eq!(
        readopted_records[0].action().author(),
        &alice,
        "the agent re-authors its OWN fact — same key, no courier"
    );
    // The pre-crossing fact was not authored a second time.
    assert_eq!(
        v1_after
            .records
            .iter()
            .filter(|r| r.action().entry_hash() == Some(&eh_pre))
            .count(),
        1,
        "the pre-crossing fact must not be re-authored — it never left v1"
    );

    // --- 5. the retry re-authors nothing (§7 C6b) ---------------------------
    let retry: ReadoptReceipt = conductor.call(&z1, "readopt_from", readopt_input()).await;
    println!("[station 7] retry readopt receipt = {retry:?}");
    assert_eq!(
        retry.readopted, 0,
        "a retried page re-authors NOTHING — every own record is already on this chain"
    );
    assert_eq!(
        retry.already_present,
        first.already_present + first.readopted,
        "on the retry, what the first run re-authored joins what was already present"
    );
    assert_eq!(
        retry.foreign, first.foreign,
        "the witness stays foreign on every pass — it is never re-created, never an error"
    );
    assert_eq!(
        retry.v2_digest, first.v2_digest,
        "both passes drew from the same v2 chain"
    );

    let v1_final: ExportPage = conductor
        .call(&z1, "export_records", ExportInput { cursor: None, limit: 64 })
        .await;
    assert_eq!(
        v1_final.total,
        Some(2),
        "a retried re-adoption must not mint a second action over an entry hash this chain \
         already holds"
    );

    Ok(())
}

// ============================================================================
// TASK 26 (G10) — ENTRY TYPES TRAVEL BY NAME ACROSS LINEAGE ENDS
//
// An `AppEntryDef` on a carried action is a pair of INDEXES scoped to the DNA
// that authored it, and the two ends of a lineage are two DNAs whose entry-type
// order differs by construction — v2 appends `NotarizationWitness` to the same
// enum v1 defines. Trusting the carried index across that boundary is how a
// crossing re-creates a record AS THE WRONG TYPE: silently, under a fresh entry
// hash wearing an old fact's name.
//
// So the type travels by NAME. What this probe pins:
//
//   1. every export page NAMES its records' types, positionally with `records`,
//      using the entry-def id the integrity zome registers;
//   2. a page naming a type this DNA does not host is refused BY NAME on the
//      self-carry arm — not carried as bytes, not re-created as whatever the
//      carried index happens to point at here;
//   3. a PERMUTED page — every name real, none of them this record's — is
//      refused too, and authors nothing;
//   4. a page whose names and records disagree in length is refused by name;
//   5. the honest page still carries, through the real `carry_from` driver, and
//      v2's own export names both its re-creations and its witness;
//   6. §7 C10 — a page carrying NO names (a coordinator predating this task)
//      still carries, by the carried index, with a logged warning;
//   7. re-adoption reads the successor's names the same way: v2's
//      `notarization_witness` is `foreign` because v1 hosts no such NAME.
//
// `carry_page_for_test` is the gated test entry point points 2–4 and 6 need:
// `carry_from` fetches its page from a predecessor CELL, so a test can only
// ever hand it an honest page, and a refusal nothing can exercise is a claim
// rather than a check. It runs exactly the carry `carry_from` runs — same
// declared predecessor, same `carry_page`, same witness.
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn task_26_entry_types_travel_by_name_across_lineage_ends() -> Result<()> {
    let Some(_v2) = v2_bundle_or_skip() else {
        return Ok(());
    };
    let Crossing {
        conductor,
        alice,
        v1_hash: _,
        v2_hash: _,
        z1,
        z2,
    } = install_crossing().await?;

    // --- 1. two v1 facts, and the page that NAMES their types ---------------
    for id in ["g10-a", "g10-b"] {
        let _ = author_and_read_back(&conductor, &z1, id, &alice).await;
    }
    let page: ExportPage = conductor
        .call(
            &z1,
            "export_records",
            ExportInput {
                cursor: None,
                limit: 16,
            },
        )
        .await;
    println!("[G10] v1 page type_names = {:?}", page.type_names);
    assert_eq!(page.records.len(), 2, "one app entry per register_node");
    assert_eq!(
        page.type_names.len(),
        page.records.len(),
        "`type_names` is paired POSITIONALLY with `records` — the export must name every record \
         or the pairing means nothing"
    );
    assert_eq!(
        page.type_names,
        vec!["node_registration".to_string(); 2],
        "the name is the entry-def id the integrity zome REGISTERS (snake_case), not the Rust \
         variant ident — that id is what both lineage ends publish for the same type"
    );

    // --- 2. a name this DNA does not host is a NAMED refusal ----------------
    let mut unknown = page.clone();
    unknown.type_names = vec![
        "not_a_type_on_this_dna".to_string(),
        "node_registration".to_string(),
    ];
    let err = conductor
        .call_fallible::<_, CarryReceipt>(&z2, "carry_page_for_test", unknown)
        .await
        .expect_err(
            "a page naming a type this DNA cannot host must be refused on the self-carry arm — \
             there is no honest way to re-create the record",
        );
    let msg = format!("{err:?}");
    println!("[G10] unknown-name refusal:\n{msg}");
    assert!(
        msg.contains("not_a_type_on_this_dna") && msg.contains("travel by NAME"),
        "the refusal must NAME the type the predecessor claimed — a driver reads it to tell \
         schema drift from a lying page, got: {msg}"
    );

    // --- 3. a PERMUTED page: every name real, none of them this record's ----
    let mut permuted = page.clone();
    permuted.type_names = vec!["string_anchor".to_string(); 2];
    let err = conductor
        .call_fallible::<_, CarryReceipt>(&z2, "carry_page_for_test", permuted)
        .await
        .expect_err(
            "a page that names the WRONG hosted type for a record must be refused — this is the \
             exact shape of an entry-def index reused across two DNAs",
        );
    let msg = format!("{err:?}");
    println!("[G10] permuted-name refusal:\n{msg}");
    assert!(
        msg.contains("string_anchor"),
        "the refusal must name the type the page claimed, got: {msg}"
    );

    // --- 4. names and records of different lengths cannot be paired ---------
    let mut short = page.clone();
    short.type_names = vec!["node_registration".to_string()];
    let err = conductor
        .call_fallible::<_, CarryReceipt>(&z2, "carry_page_for_test", short)
        .await
        .expect_err("a page whose names and records disagree in length must be refused");
    let msg = format!("{err:?}");
    println!("[G10] length-mismatch refusal:\n{msg}");
    assert!(
        msg.contains("1 type names for 2 records") && msg.contains("POSITIONALLY"),
        "expected the NAMED positional-pairing refusal, got: {msg}"
    );

    // Every refusal above authored NOTHING: a zome call that returns an error
    // discards its whole source-chain workspace, which is what makes refusing
    // the safe answer rather than a partial carry.
    let v2_after_refusals: ExportPage = conductor
        .call(
            &z2,
            "export_records",
            ExportInput {
                cursor: None,
                limit: 64,
            },
        )
        .await;
    assert_eq!(
        v2_after_refusals.total,
        Some(0),
        "three refused pages must leave v2's chain empty — a refusal that half-carried would be \
         worse than the mis-typing it prevented"
    );

    // --- 5. the HONEST page still carries, through the real driver ----------
    let carry: CarryReceipt = conductor
        .call(
            &z2,
            "carry_from",
            CarryInput {
                v1_cell: z1.cell_id().clone(),
                cursor: None,
                limit: 16,
            },
        )
        .await;
    println!("[G10] honest carry receipt = {carry:?}");
    assert_eq!(carry.carried, 2, "both v1 facts crossed");
    assert_eq!(
        carry.self_carried, 2,
        "both were re-created NATIVELY on v2 — resolved through the name v1 published"
    );

    let v2_page: ExportPage = conductor
        .call(
            &z2,
            "export_records",
            ExportInput {
                cursor: None,
                limit: 64,
            },
        )
        .await;
    println!("[G10] v2 page type_names = {:?}", v2_page.type_names);
    assert_eq!(
        v2_page
            .type_names
            .iter()
            .filter(|n| *n == "node_registration")
            .count(),
        2,
        "the re-creations carry v1's type under the SAME name on v2 — which is what makes the \
         name a translation rather than a coincidence of ordering"
    );
    assert_eq!(
        v2_page
            .type_names
            .iter()
            .filter(|n| *n == "notarization_witness")
            .count(),
        1,
        "the successor names its OWN extra type too — the one v1 will report `foreign` BY NAME"
    );

    // --- 6. §7 C10 — an UNNAMED page still carries, by the carried index ----
    let _ = author_and_read_back(&conductor, &z1, "g10-c", &alice).await;
    let mut legacy: ExportPage = conductor
        .call(
            &z1,
            "export_records",
            ExportInput {
                cursor: Some(2),
                limit: 16,
            },
        )
        .await;
    assert_eq!(legacy.records.len(), 1, "the third v1 fact, on its own page");
    // Exactly what a predecessor coordinator predating Task 26 puts on the
    // wire: no names at all.
    legacy.type_names = Vec::new();
    let old: CarryReceipt = conductor
        .call(&z2, "carry_page_for_test", legacy)
        .await;
    println!("[G10] unnamed-page carry receipt = {old:?}");
    assert_eq!(
        old.self_carried, 1,
        "a page with NO `type_names` falls back to the carried entry-def index and still carries \
         — §7 C10: old peers keep working, loudly, rather than being refused"
    );

    // --- 7. re-adoption reads the successor's names the same way ------------
    let readopt: ReadoptReceipt = conductor
        .call(
            &z1,
            "readopt_from",
            ReadoptInput {
                v2_cell: z2.cell_id().clone(),
                cursor: None,
                limit: 16,
            },
        )
        .await;
    println!("[G10] readopt receipt = {readopt:?}");
    assert_eq!(
        readopt.foreign, 2,
        "both of v2's witnesses are the NAME `notarization_witness`, which v1 hosts no entry type \
         for — counted, never an error, and recognised by name rather than by an index that means \
         something else here"
    );
    assert_eq!(
        readopt.readopted, 0,
        "nothing was authored only on v2 — every fact came from v1 in the first place"
    );
    assert_eq!(
        readopt.already_present, 3,
        "all three re-creations hash to entries v1 already holds — CID continuity across the \
         crossing, established through the name and not the index"
    );

    Ok(())
}
