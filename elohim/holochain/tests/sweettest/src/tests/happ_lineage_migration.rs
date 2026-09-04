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
//! just build-witness                                        # v2 (node-registry-v2.dna)
//! ```
//!
//! v1 is the pristine artifact (predecessor); v2 is the `lineage-witness`
//! artifact (successor). Their paths come from `NODE_REGISTRY_V1_DNA` /
//! `NODE_REGISTRY_V2_DNA` when set, else the in-repo defaults below
//! (`node-registry-v1.dna` / `node-registry-v2.dna`). The test is
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

/// Mirror of `node_registry_coordinator::ExportPage`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ExportPage {
    records: Vec<SignedActionHashed>,
    entries: Vec<Option<Entry>>,
    next_cursor: Option<u32>,
    digest: String,
    /// Task 9 (additive): the WHOLE-chain app-record count the export walks.
    /// `#[serde(default)]` because a v1 bundle packed before Task 9 does not
    /// emit the field — the carry receipt then reports `v1_total: None` rather
    /// than a fabricated number.
    #[serde(default)]
    total: Option<u32>,
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
    /// is READ from v1's export page, never derived from `carried`.
    v1_total: Option<u32>,
    /// Additive: how many of `carried` were re-created NATIVELY here
    /// (held-carries excluded).
    #[serde(default)]
    self_carried: u32,
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
        .unwrap_or_else(|_| dna_dir().join("node-registry-v2.dna"))
}

/// The v2 bundle is a FEATURE build (`just build-witness`) that CI's DNA pipeline does not
/// produce yet. Until it does, a missing bundle is a loud SKIP, never a silent pass and never
/// a red for a bundle nobody built: the probe's evidence is its own log line.
fn v2_bundle_or_skip() -> Option<PathBuf> {
    let p = v2_path();
    if p.exists() {
        Some(p)
    } else {
        eprintln!(
            "SKIPPED @concern:happ-lineage-migration — v2 bundle absent at {} \
             (build it: cd elohim/holochain/dna/node-registry && just build-witness)",
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
        "[export/held] total={:?} digest={} records={}",
        held.total,
        held.digest,
        held.records.len()
    );
    assert_eq!(
        held.total,
        Some(5),
        "the held view of alice's own chain must see the same 5 app-entry records"
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
        "[task18] alice's held view of bob: total={:?} digest={} records={}",
        held.total,
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
        "LIKE-FOR-LIKE across peers: alice's held digest of bob's chain must equal bob's own"
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
        bob_page.total.expect("bob reported a total"),
        "everything bob had was carried"
    );
    assert_eq!(receipt.carried, 1);
    assert_eq!(
        receipt.self_carried, 0,
        "bob's record is NOT alice's to re-create — a held-carry is never a self-carry"
    );
    assert_eq!(
        receipt.v1_digest, bob_page.digest,
        "the receipt reports the digest of the chain it actually carried — BOB's"
    );
    assert_eq!(receipt.v1_total, Some(1));
    assert_eq!(receipt.next_cursor, None, "a partial page offers no further cursor");
    assert!(!receipt.witness_hash.is_empty(), "a non-empty page commits exactly one witness");

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
