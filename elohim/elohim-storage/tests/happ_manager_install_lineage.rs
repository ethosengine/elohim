//! Integration test (ignored by default) for `happ_manager::install_lineage` —
//! Holochain Evolution Epic Task 5.
//!
//! Installs a second app (`elohim@test-lineage`) beside the running mesh's
//! base `elohim` app, under the SAME agent key, and asserts via `list_apps`
//! that both apps exist for that one key. Never touches the base app's
//! cells; the side app it creates is torn down at the end of the test —
//! production code (`install_lineage` itself) never uninstalls anything.
//!
//! ## Running
//!
//! Requires a running conductor's admin websocket AND a packed one-role
//! `.happ` bundle to install as the lineage app:
//!
//! ```bash
//! MESH_ADMIN_URL=ws://localhost:4444 \
//! LINEAGE_TEST_HAPP=/path/to/node-registry-v2.happ \
//! cargo test -p elohim-storage --test happ_manager_install_lineage -- --ignored
//! ```
//!
//! `LINEAGE_TEST_HAPP` must name a bundle with exactly one role, `node_registry`
//! (any bundle works — the role and DNA content are opaque to `install_lineage`,
//! which only reads/writes conductor-level install metadata). Until Task 9's
//! `just build-witness-happ` produces a canonical `node-registry-v2.happ`, pack
//! one from an existing `.dna`:
//!
//! ```bash
//! /projects/.claude-config/tools/hc-0.7/hc app pack <dir-with-happ.yaml> -o node-registry-v2.happ
//! ```

use std::path::PathBuf;

use holochain_client::AdminWebsocket;
use holochain_types::prelude::DnaHash;

#[tokio::test]
#[ignore = "needs a conductor: MESH_ADMIN_URL=ws://localhost:4444 (admin port, not the app port 4445), LINEAGE_TEST_HAPP=<path to a one-role .happ>"]
async fn install_lineage_installs_beside_under_same_key() {
    let admin_url = std::env::var("MESH_ADMIN_URL").expect(
        "set MESH_ADMIN_URL to the mesh conductor's admin websocket, e.g. ws://localhost:4444 (HOLOCHAIN_ADMIN_URL — NOT the app-interface port 4445)",
    );
    let happ_path: PathBuf = std::env::var("LINEAGE_TEST_HAPP")
        .expect("set LINEAGE_TEST_HAPP to a packed one-role .happ bundle")
        .into();

    // AdminWebsocket::connect takes a `ToSocketAddrs`, not a `ws://` URL —
    // same stripping hc_client.rs::HcClient::to_socket_addr does.
    let admin_addr = admin_url
        .strip_prefix("ws://")
        .unwrap_or(admin_url.as_str());

    let admin = AdminWebsocket::connect(admin_addr, None)
        .await
        .expect("connect to mesh admin websocket");

    // The base app must already be installed on this conductor — this test
    // never installs it, only reads its agent key and network seed.
    let apps = admin.list_apps(None).await.expect("list_apps");
    let elohim = apps
        .iter()
        .find(|a| a.installed_app_id == elohim_storage::happ_manager::APP_ID)
        .expect("base 'elohim' app installed on the mesh conductor");
    let key = elohim.agent_pub_key.clone();

    // Opaque for this test — install_lineage records it as a `lineage`
    // property on the new role's modifiers; it need not resolve to any real
    // installed DNA to exercise the install/idempotency/key-sharing path.
    let v1_hash = DnaHash::try_from("uhC0kyvKwO2J5u3mf52tjASWe0ryhdpNYalrSeMGJODF3OpUxyeoH")
        .expect("valid DnaHash literal");

    let lineage_app_id = "elohim@test-lineage";

    // First call installs; second call must be a no-op (idempotent).
    elohim_storage::happ_manager::install_lineage(
        &admin,
        &happ_path,
        lineage_app_id,
        key.clone(),
        std::slice::from_ref(&v1_hash),
        "node_registry",
    )
    .await
    .expect("install_lineage (first call, fresh install)");

    elohim_storage::happ_manager::install_lineage(
        &admin,
        &happ_path,
        lineage_app_id,
        key.clone(),
        std::slice::from_ref(&v1_hash),
        "node_registry",
    )
    .await
    .expect("install_lineage (second call, must be idempotent no-op)");

    let apps = admin
        .list_apps(None)
        .await
        .expect("list_apps after install");
    eprintln!(
        "[evidence] list_apps after install_lineage: {:?}",
        apps.iter()
            .map(|a| (a.installed_app_id.clone(), a.agent_pub_key.to_string()))
            .collect::<Vec<_>>()
    );
    let base_still_present = apps
        .iter()
        .any(|a| a.installed_app_id == elohim_storage::happ_manager::APP_ID);
    assert!(
        base_still_present,
        "install_lineage must never touch the base app — 'elohim' vanished from list_apps"
    );

    let side = apps
        .iter()
        .find(|a| a.installed_app_id == lineage_app_id)
        .expect("side app 'elohim@test-lineage' present in list_apps");
    assert_eq!(
        side.agent_pub_key, key,
        "lineage app must be installed under the SAME agent key as the base app"
    );
    eprintln!(
        "[evidence] both '{}' and '{lineage_app_id}' present under the same agent key {}",
        elohim_storage::happ_manager::APP_ID,
        key
    );

    // Test-only cleanup. Production install_lineage never calls
    // uninstall_app / uninstall_for_reinstall on this path — see the module
    // doc on happ_manager.rs. Leaves the mesh exactly as this test found it.
    admin
        .disable_app(lineage_app_id.to_string())
        .await
        .expect("disable_app (test cleanup)");
    admin
        .uninstall_app(lineage_app_id.to_string(), false)
        .await
        .expect("uninstall_app (test cleanup)");

    let apps_after_cleanup = admin
        .list_apps(None)
        .await
        .expect("list_apps after cleanup");
    let side_gone = !apps_after_cleanup
        .iter()
        .any(|a| a.installed_app_id == lineage_app_id);
    assert!(
        side_gone,
        "'{lineage_app_id}' must be gone after test cleanup — mesh left dirty"
    );
    eprintln!(
        "[evidence] post-cleanup list_apps: {:?} — '{lineage_app_id}' removed, mesh left as found",
        apps_after_cleanup
            .iter()
            .map(|a| a.installed_app_id.clone())
            .collect::<Vec<_>>()
    );
}
