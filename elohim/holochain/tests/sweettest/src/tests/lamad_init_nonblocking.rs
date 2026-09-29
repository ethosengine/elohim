//! @dna-scope: lamad
//! Sweettest — lamad `init()` never waits on another cell.
//!
//! Backlog: `genesis/data/timeline/backlog/lamad-init-blocks-on-v1-bridge-probe.md`.
//!
//! On the fleet, adam's lamad cell answered every zome call for ~11 h with
//! "Another zome function has triggered the `init()` callback, which has been
//! blocking this zome call for longer than 30 seconds". `content_store::init`
//! ran a synchronous cross-cell probe (`hc_rna` `check_v1_on_startup` →
//! `call(OtherRole("lamad-v1"), "coordinator", "is_data_present")`) whose
//! result nothing read. A cross-cell call inside `init()` holds the cell's init
//! lock for as long as the other cell takes to answer.
//!
//! The cure keeps init local: no cross-cell and no network call. These tests
//! pin that from the outside, as a latency budget on the FIRST zome call of a
//! fresh cell (the call that runs init):
//!
//! 1. `first_call_is_prompt_without_v1_role` — the shipped hApp shape (no
//!    `lamad-v1` role at all).
//! 2. `first_call_is_prompt_beside_unanswering_v1_role` — a `lamad-v1` role is
//!    present and does not answer in time. The stand-in is an inline-zome DNA
//!    whose `coordinator::is_data_present` (the exact probe target) sleeps
//!    [`V1_HANG`] — longer than the conductor's 30 s init-lock give-up. Before
//!    the cure, init waited out that sleep, so the first lamad call took
//!    ≥ [`V1_HANG`] and a concurrent second call got the fleet's error. After
//!    the cure, neither call touches the v1 cell.
//!
//! Both tests first WARM the conductor's wasm module cache with a throwaway
//! lamad app (its own cells, its own network seed). A debug-built conductor
//! spends tens of seconds compiling the ~13 MB coordinator on first use; that
//! cost is not init's and would otherwise swamp the budget. The measured cells
//! are fresh (init not yet run) and share only the compiled module.
//!
//! What this can NOT simulate: a v1 cell on another conductor behind a slow
//! network, or a v1 cell disabled mid-call. Both reduce to "the other cell does
//! not answer inside init", which case 2 covers from the caller's side.

use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use holo_hash::AgentPubKey;
use holochain::sweettest::{SweetConductor, SweetDnaFile, SweetZome};
use holochain_types::inline_zome::InlineZomeSet;
use holochain_types::prelude::DnaFile;

use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor},
    fixtures::network_seed,
};

const DNA: &str = "lamad";

/// Well under the conductor's 30 s init-lock give-up. A local init (entry-type
/// registry setup, no calls) finishes in about a second on a debug conductor;
/// the headroom is for a loaded CI host.
const INIT_BUDGET: Duration = Duration::from_secs(15);

/// How long the stand-in v1 cell takes to answer the probe — past the
/// conductor's 30 s init-lock give-up, so a pre-cure concurrent call fails the
/// way adam's did.
const V1_HANG: Duration = Duration::from_secs(45);

/// A `lamad-v1` stand-in: an inline-zome DNA carrying the one function the
/// pre-cure init probed (`coordinator::is_data_present`), which blocks for
/// [`V1_HANG`] before answering.
async fn unanswering_v1_dna() -> DnaFile {
    let zomes = InlineZomeSet::new_unique_single("v1_integrity", "coordinator", vec![], 0)
        .function("coordinator", "is_data_present", |_api, ()| {
            std::thread::sleep(V1_HANG);
            Ok(true)
        });
    let (dna, _, _) = SweetDnaFile::unique_from_inline_zomes(zomes).await;
    dna
}

/// Time one `is_bootstrap_steward` call — cheap, no DHT, no arguments — so the
/// measured latency is init plus a trivial local read.
async fn timed_first_call(
    conductor: &SweetConductor,
    zome: &SweetZome,
) -> Result<(Duration, bool)> {
    let handle = conductor.sweet_handle();
    let started = Instant::now();
    let out = tokio::time::timeout(
        Duration::from_secs(60),
        handle.call_fallible::<_, bool>(zome, "is_bootstrap_steward", ()),
    )
    .await
    .map_err(|_| anyhow!("first zome call did not return within 60 s"))?
    .map_err(|e| anyhow!("first zome call failed: {e:?}"))?;
    let elapsed = started.elapsed();
    eprintln!("[lamad-init] first call on the fresh cell took {elapsed:?}");
    Ok((elapsed, out))
}

/// Install a throwaway lamad app and call it once so the coordinator wasm is
/// compiled and cached before any measured call. Its own app has no
/// `lamad-v1` role, so even pre-cure its init probe failed fast.
async fn warm_wasm_cache(conductor: &mut SweetConductor, agent: &AgentPubKey) -> Result<()> {
    let dna = load_dna(DNA, "elohim_lamad_init_warmup", Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent(
            "lamad-init-warmup",
            agent.clone(),
            &[("lamad".to_string(), dna)],
        )
        .await?;
    let cell = app.cells().first().expect("warm-up cell installed").clone();
    let started = Instant::now();
    let _: bool = tokio::time::timeout(
        Duration::from_secs(900),
        conductor.sweet_handle().call_fallible(
            &cell.zome("content_store"),
            "is_bootstrap_steward",
            (),
        ),
    )
    .await
    .map_err(|_| anyhow!("warm-up call did not return within 900 s"))?
    .map_err(|e| anyhow!("warm-up call failed: {e:?}"))?;
    eprintln!(
        "[lamad-init] warm-up (wasm compile + init) took {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn first_call_is_prompt_without_v1_role() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    warm_wasm_cache(&mut conductor, &agent).await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent(
            "lamad-init-no-v1",
            agent.clone(),
            &[("lamad".to_string(), dna)],
        )
        .await?;
    let cell = app.cells().first().expect("lamad cell installed").clone();

    let (elapsed, is_steward) = timed_first_call(&conductor, &cell.zome("content_store")).await?;
    assert!(is_steward, "the installing agent is the bootstrap steward");
    assert!(
        elapsed < INIT_BUDGET,
        "first lamad zome call (runs init) took {elapsed:?}; budget {INIT_BUDGET:?}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn first_call_is_prompt_beside_unanswering_v1_role() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    warm_wasm_cache(&mut conductor, &agent).await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let v1_stub = unanswering_v1_dna().await;
    let app = conductor
        .setup_app_for_agent(
            "lamad-init-beside-v1",
            agent.clone(),
            &[
                ("lamad".to_string(), dna),
                ("lamad-v1".to_string(), v1_stub),
            ],
        )
        .await?;
    // Cells come back in role order: [lamad, lamad-v1].
    let [lamad, v1]: [_; 2] = app
        .into_cells()
        .try_into()
        .map_err(|_| anyhow!("expected exactly two cells"))?;
    assert_ne!(lamad.cell_id().dna_hash(), v1.cell_id().dna_hash());
    let zome = lamad.zome("content_store");

    // The first call runs init; a second call lands while init may still be
    // running — the shape that returned the fleet's 30 s init-lock error.
    let second_handle = conductor.sweet_handle();
    let second_zome = zome.clone();
    let second = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(250)).await;
        let started = Instant::now();
        let out = tokio::time::timeout(
            Duration::from_secs(60),
            second_handle.call_fallible::<_, bool>(&second_zome, "is_bootstrap_steward", ()),
        )
        .await;
        (started.elapsed(), out)
    });

    let first = timed_first_call(&conductor, &zome).await;
    // Collect the concurrent call before judging either, so a red run reports
    // both halves (the second is where the fleet's init-lock error shows).
    let (second_elapsed, second_out) = second.await?;
    eprintln!("[lamad-init] concurrent call took {second_elapsed:?}: {second_out:?}");

    let (elapsed, is_steward) = first?;
    assert!(is_steward, "the installing agent is the bootstrap steward");
    assert!(
        elapsed < INIT_BUDGET,
        "first lamad zome call (runs init) took {elapsed:?} beside a lamad-v1 role \
         that cannot answer; budget {INIT_BUDGET:?} — init must not wait on another cell"
    );
    let second_out = second_out
        .map_err(|_| anyhow!("concurrent call did not return within 60 s"))?
        .map_err(|e| anyhow!("concurrent call failed while init ran: {e:?}"))?;
    assert!(second_out);
    assert!(
        second_elapsed < INIT_BUDGET,
        "concurrent lamad call took {second_elapsed:?}; budget {INIT_BUDGET:?}"
    );
    Ok(())
}
