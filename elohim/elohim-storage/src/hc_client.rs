//! Holochain Client Wrapper
//!
//! Uses the official holochain_client crate for proper signed zome calls.
//! Holochain 0.6+ requires all zome calls to be signed with nonce, expires_at,
//! and ed25519 signature. This module handles:
//!
//! 1. Connecting to admin and app websockets
//! 2. Authorizing signing credentials via admin API
//! 3. Making signed zome calls that the conductor will accept
//!
//! ## Usage
//!
//! ```ignore
//! let client = HcClient::connect(HcClientConfig {
//!     admin_url: "localhost:4444".to_string(),
//!     app_url: "localhost:4445".to_string(),
//!     app_id: "elohim".to_string(),
//!     role: Some("lamad".to_string()),
//! }).await?;
//! let result = client.call_zome("content_store", "process_import_chunk", payload).await?;
//! ```

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

use holochain_client::{
    AdminWebsocket, AllowedOrigins, AppWebsocket, AuthorizeSigningCredentialsPayload, CellId,
    ClientAgentSigner, ExternIO, ZomeCallTarget,
};

use crate::error::StorageError;

/// Conductor health information for backpressure decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConductorHealth {
    /// Storage information
    pub storage: Option<StorageHealth>,
    /// Network statistics
    pub network: Option<NetworkHealth>,
    /// Raw responses for debugging (JSON strings)
    pub raw_storage: Option<String>,
    pub raw_network_stats: Option<String>,
    pub raw_network_metrics: Option<String>,
}

/// Storage health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageHealth {
    /// Total bytes used by entries
    pub bytes_used: u64,
    /// Number of entries
    pub entry_count: u64,
}

/// Network health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkHealth {
    /// Number of connected peers
    pub peer_count: Option<u64>,
    /// Any additional stats we can extract
    pub details: String,
}

/// Configuration for HcClient
#[derive(Debug, Clone)]
pub struct HcClientConfig {
    /// Admin websocket URL (e.g., "ws://localhost:4444")
    pub admin_url: String,
    /// App websocket URL (e.g., "ws://localhost:4445")
    pub app_url: String,
    /// Installed app ID
    pub app_id: String,
    /// Role to use for the cell (e.g., "lamad")
    pub role: Option<String>,
}

/// Holochain client with proper signing support
#[allow(dead_code)]
pub struct HcClient {
    config: HcClientConfig,
    /// Admin websocket connection
    admin_ws: AdminWebsocket,
    /// App websocket connection with signing
    app_ws: AppWebsocket,
    /// The cell ID for zome calls
    cell_id: CellId,
    /// Signing credentials
    signer: Arc<ClientAgentSigner>,
}

impl HcClient {
    /// Strip ws:// or wss:// prefix from URL to get socket address
    /// holochain_client expects "host:port" format, not "ws://host:port"
    fn to_socket_addr(url: &str) -> String {
        let url = url.trim();
        if let Some(rest) = url.strip_prefix("wss://") {
            rest.to_string()
        } else if let Some(rest) = url.strip_prefix("ws://") {
            rest.to_string()
        } else {
            url.to_string()
        }
    }

    /// Connect to Holochain conductor and set up signing credentials
    pub async fn connect(config: HcClientConfig) -> Result<Self, StorageError> {
        // Convert URLs to socket addresses (strip ws:// prefix)
        let admin_addr = Self::to_socket_addr(&config.admin_url);
        let app_addr = Self::to_socket_addr(&config.app_url);

        info!(
            admin_addr = %admin_addr,
            app_addr = %app_addr,
            app_id = %config.app_id,
            "Connecting to Holochain conductor with signing support"
        );

        // Connect to admin interface (socket_addr, origin)
        let admin_ws = AdminWebsocket::connect(&admin_addr, None)
            .await
            .map_err(|e| StorageError::Connection(format!("Admin connect failed: {}", e)))?;

        info!("Connected to admin interface");

        // Ensure an app interface exists on the expected port.
        // The embedded conductor (tauri-plugin-holochain) uses random ports,
        // so we attach one on our expected port via admin API.
        let app_port: u16 = app_addr
            .rsplit(':')
            .next()
            .and_then(|p| p.parse().ok())
            .unwrap_or(4445);
        match admin_ws
            .attach_app_interface(app_port, None, AllowedOrigins::Any, None)
            .await
        {
            Ok(port) => info!(port, "Attached app interface"),
            Err(e) => {
                // May already be attached from a previous call — continue
                warn!(
                    "attach_app_interface on port {}: {} (may already exist)",
                    app_port, e
                );
            }
        }

        // Get app info to find cell
        let apps = admin_ws
            .list_apps(None)
            .await
            .map_err(|e| StorageError::Connection(format!("list_apps failed: {}", e)))?;

        let app_info = apps
            .iter()
            .find(|a| a.installed_app_id == config.app_id)
            .ok_or_else(|| StorageError::NotFound(format!("App '{}' not found", config.app_id)))?;

        // Find the cell for the specified role
        let cell_id = if let Some(role) = &config.role {
            app_info
                .cell_info
                .get(role)
                .and_then(|cells| cells.first())
                .and_then(|cell| match cell {
                    holochain_client::CellInfo::Provisioned(p) => Some(p.cell_id.clone()),
                    _ => None,
                })
                .ok_or_else(|| StorageError::NotFound(format!("Role '{}' not found", role)))?
        } else {
            // Use first available cell
            app_info
                .cell_info
                .values()
                .next()
                .and_then(|cells| cells.first())
                .and_then(|cell| match cell {
                    holochain_client::CellInfo::Provisioned(p) => Some(p.cell_id.clone()),
                    _ => None,
                })
                .ok_or_else(|| StorageError::NotFound("No cells found".to_string()))?
        };

        info!(
            dna_hash = %hex::encode(&cell_id.dna_hash().get_raw_39()[..8]),
            "Found cell"
        );

        // Create signing credentials
        let signer = ClientAgentSigner::default();

        // Authorize signing credentials for this cell
        let credentials = admin_ws
            .authorize_signing_credentials(AuthorizeSigningCredentialsPayload {
                cell_id: cell_id.clone(),
                functions: None, // All functions
            })
            .await
            .map_err(|e| {
                StorageError::Connection(format!("authorize_signing_credentials failed: {}", e))
            })?;

        // Add credentials to signer
        signer.add_credentials(cell_id.clone(), credentials);
        info!("Signing credentials authorized");

        // Get app auth token
        let token = admin_ws
            .issue_app_auth_token(holochain_client::IssueAppAuthenticationTokenPayload {
                installed_app_id: config.app_id.clone(),
                expiry_seconds: 3600, // 1 hour
                single_use: false,
            })
            .await
            .map_err(|e| StorageError::Connection(format!("issue_app_auth_token failed: {}", e)))?;

        // Connect to app interface with signer (socket_addr, token, signer, origin)
        let signer_arc: Arc<ClientAgentSigner> = Arc::new(signer);
        let app_ws = AppWebsocket::connect(&app_addr, token.token, signer_arc.clone(), None)
            .await
            .map_err(|e| StorageError::Connection(format!("App connect failed: {}", e)))?;

        info!("Connected to app interface with signing");

        Ok(Self {
            config,
            admin_ws,
            app_ws,
            cell_id,
            signer: signer_arc,
        })
    }

    /// Make a signed zome call
    pub async fn call_zome(
        &self,
        zome_name: &str,
        fn_name: &str,
        payload: Vec<u8>,
    ) -> Result<Vec<u8>, StorageError> {
        debug!(
            zome = %zome_name,
            fn_name = %fn_name,
            payload_len = payload.len(),
            "Making signed zome call"
        );

        // The holochain_client handles signing automatically
        // Use ExternIO::from() for raw bytes - payload is already MessagePack encoded
        let result = self
            .app_ws
            .call_zome(
                ZomeCallTarget::CellId(self.cell_id.clone()),
                zome_name.into(),
                fn_name.into(),
                ExternIO::from(payload),
            )
            .await
            .map_err(|e| StorageError::Conductor(format!("Zome call failed: {}", e)))?;

        // Return raw bytes - caller will deserialize as needed
        Ok(result.into_vec())
    }

    /// Get the cell ID
    pub fn cell_id(&self) -> &CellId {
        &self.cell_id
    }

    /// Get DNA hash bytes
    pub fn dna_hash(&self) -> Vec<u8> {
        self.cell_id.dna_hash().get_raw_39().to_vec()
    }

    /// Get agent pubkey bytes
    pub fn agent_pub_key(&self) -> Vec<u8> {
        self.cell_id.agent_pubkey().get_raw_39().to_vec()
    }

    /// Get conductor health metrics for backpressure decisions
    ///
    /// Fetches storage info, network stats, and network metrics from the conductor.
    /// Returns raw JSON responses for evaluation - we can parse specific fields later
    /// once we understand the response structure.
    pub async fn get_health(&self) -> ConductorHealth {
        let mut health = ConductorHealth {
            storage: None,
            network: None,
            raw_storage: None,
            raw_network_stats: None,
            raw_network_metrics: None,
        };

        // Fetch storage info
        match self.admin_ws.storage_info().await {
            Ok(storage_info) => {
                // Convert to JSON for inspection
                let json = serde_json::to_string_pretty(&storage_info)
                    .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e));
                health.raw_storage = Some(json.clone());

                // Try to extract key metrics
                // StorageInfo has blobs.used_by_entries field
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json) {
                    let bytes_used = parsed["blobs"]["used_by_entries"].as_u64().unwrap_or(0);
                    let entry_count = parsed["blobs"]["used_by_entries_count"]
                        .as_u64()
                        .or_else(|| parsed["entries_count"].as_u64())
                        .unwrap_or(0);

                    health.storage = Some(StorageHealth {
                        bytes_used,
                        entry_count,
                    });
                }

                info!(
                    bytes_used = health.storage.as_ref().map(|s| s.bytes_used).unwrap_or(0),
                    "📊 STORAGE_INFO: Fetched conductor storage metrics"
                );
            }
            Err(e) => {
                warn!(error = %e, "Failed to fetch storage_info");
                health.raw_storage = Some(format!("{{\"error\": \"{}\"}}", e));
            }
        }

        // Fetch network stats
        match self.admin_ws.dump_network_stats().await {
            Ok(network_stats) => {
                let json = serde_json::to_string_pretty(&network_stats)
                    .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e));
                health.raw_network_stats = Some(json.clone());

                // Extract what we can
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json) {
                    let peer_count = parsed["peer_count"]
                        .as_u64()
                        .or_else(|| parsed["peers"].as_array().map(|a| a.len() as u64));

                    health.network = Some(NetworkHealth {
                        peer_count,
                        details: format!(
                            "Keys: {:?}",
                            parsed.as_object().map(|o| o.keys().collect::<Vec<_>>())
                        ),
                    });
                }

                info!(
                    peer_count = health.network.as_ref().and_then(|n| n.peer_count),
                    "📊 NETWORK_STATS: Fetched conductor network statistics"
                );
            }
            Err(e) => {
                warn!(error = %e, "Failed to fetch dump_network_stats");
                health.raw_network_stats = Some(format!("{{\"error\": \"{}\"}}", e));
            }
        }

        // Fetch network metrics (more detailed DHT info)
        // dump_network_metrics takes optional DNA hash filter and DHT summary flag
        match self.admin_ws.dump_network_metrics(None, true).await {
            Ok(network_metrics) => {
                let json = serde_json::to_string_pretty(&network_metrics)
                    .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e));
                health.raw_network_metrics = Some(json);

                info!("📊 NETWORK_METRICS: Fetched conductor DHT metrics");
            }
            Err(e) => {
                warn!(error = %e, "Failed to fetch dump_network_metrics");
                health.raw_network_metrics = Some(format!("{{\"error\": \"{}\"}}", e));
            }
        }

        health
    }

    /// Subscribe to InfrastructureSignals emitted by the conductor's app interface.
    ///
    /// Wraps `AppWebsocket::on_signal` — the official client filters signals to
    /// this app's cells. For each signal, we msgpack-decode the inner `AppSignal`
    /// payload into a `serde_json::Value`, then try to parse it as an
    /// `InfrastructureSignal`. Non-infrastructure signals (e.g. lamad DNA signals
    /// that share the same conductor) are logged at debug and ignored.
    ///
    /// Returns a subscription handle (string id) from the underlying client.
    /// Dropping the returned value does NOT unsubscribe — `AppWebsocket::on_signal`
    /// registers the handler for the lifetime of the websocket connection. When the
    /// HcClient is dropped, the subscription ends with it.
    pub async fn subscribe_infrastructure_signals<F>(&self, handler: F) -> String
    where
        F: Fn(crate::signals::InfrastructureSignal) + Send + Sync + 'static,
    {
        use holochain_types::signal::Signal;

        self.app_ws
            .on_signal(move |signal| {
                if let Signal::App { signal, .. } = signal {
                    let bytes: Vec<u8> = signal.into_inner().into();
                    // Typed msgpack decode — the old `rmp → serde_json::Value`
                    // pre-pass failed on EVERY HoloHash-bearing signal
                    // (serde_json::Value cannot represent msgpack byte arrays)
                    // and dropped it at debug level: the 2026-06-12
                    // peer-statuses dark-surface root cause. Real misses are
                    // now LOUD; foreign signals on the shared app interface
                    // stay quiet.
                    match crate::signals::decode_infrastructure_signal(&bytes) {
                        Ok(infra) => handler(infra),
                        Err(crate::signals::SignalDecodeMiss::InfraShapeMismatch {
                            type_tag,
                            error,
                        }) => {
                            warn!(
                                type_tag = %type_tag,
                                error = %error,
                                misses_total = crate::signals::infrastructure_decode_miss_count(),
                                "InfrastructureSignal DECODE MISS — signal dropped, projection will go dark for this variant"
                            );
                        }
                        Err(crate::signals::SignalDecodeMiss::ForeignSignal { type_tag }) => {
                            debug!(
                                type_tag = %type_tag,
                                "App signal from another family — ignoring"
                            );
                        }
                        Err(crate::signals::SignalDecodeMiss::Undecodable { error }) => {
                            debug!(error = %error, "App signal payload undecodable — ignoring");
                        }
                        // Mishpat / REA / ElohimContent shape-mismatches cannot
                        // arise from the infra decoder (different mirror-variant
                        // sets), but the shared enum requires the arms.
                        Err(crate::signals::SignalDecodeMiss::MishpatShapeMismatch {
                            type_tag, ..
                        })
                        | Err(crate::signals::SignalDecodeMiss::ReaShapeMismatch {
                            type_tag, ..
                        })
                        | Err(crate::signals::SignalDecodeMiss::ElohimContentShapeMismatch {
                            type_tag,
                            ..
                        }) => {
                            debug!(
                                type_tag = %type_tag,
                                "Unexpected non-infra shape classification from infra decoder — ignoring"
                            );
                        }
                    }
                }
            })
            .await
    }

    /// Subscribe to ReaProjectionSignals emitted by the content_store coordinator.
    /// Routes ReaCommitmentCommitted (incl. updates), AgreementCommitted, and
    /// ReaEconomicEventCommitted to the local SQL projection via
    /// `rea_projection::handle_rea_signal`.
    ///
    /// Wired by main.rs alongside the infrastructure + elohim-content
    /// subscribers. Required for the substrate-correct write path landed
    /// per 2026-05-26-substrate-rea-replication-fix.md — without this,
    /// HTTP POST /api/v1/commitments (project-epr) would create the DHT
    /// entry but the SQL projection would never land, causing the service
    /// layer's bounded poll to time out at 1s.
    ///
    /// Non-REA signals (Content, Attestation, etc.) are logged at debug and
    /// dropped — they have their own dedicated subscribers.
    pub async fn subscribe_rea_projection_signals<F>(&self, handler: F) -> String
    where
        F: Fn(crate::rea_projection::ReaProjectionSignal) + Send + Sync + 'static,
    {
        use holochain_types::signal::Signal;

        self.app_ws
            .on_signal(move |signal| {
                if let Signal::App { signal, .. } = signal {
                    let bytes: Vec<u8> = signal.into_inner().into();
                    // Typed msgpack decode — the old `rmp → serde_json::Value`
                    // pre-pass failed on EVERY HoloHash-bearing ReaProjectionSignal
                    // (action_hash/entry_hash/author are ActionHash/EntryHash/
                    // AgentPubKey on the DNA side → raw 39-byte arrays on the
                    // MessagePack wire, which serde_json::Value cannot represent)
                    // and dropped it at debug level: the same dark-drop class as
                    // d33b0e1f5. Without this, the substrate-correct write path
                    // (POST /commitments → project-epr → SQL projection) silently
                    // never lands and the service-layer bounded poll times out.
                    // Real misses are now LOUD; foreign signals on the shared app
                    // interface stay quiet.
                    match crate::rea_projection::decode_rea_projection_signal(&bytes) {
                        Ok(rea) => handler(rea),
                        Err(crate::signals::SignalDecodeMiss::ReaShapeMismatch {
                            type_tag,
                            error,
                        }) => {
                            warn!(
                                type_tag = %type_tag,
                                error = %error,
                                misses_total = crate::rea_projection::rea_decode_miss_count(),
                                "ReaProjectionSignal DECODE MISS — signal dropped, projection will go dark for this variant"
                            );
                        }
                        Err(crate::signals::SignalDecodeMiss::ForeignSignal { type_tag }) => {
                            debug!(
                                type_tag = %type_tag,
                                "App signal from another family — ignoring"
                            );
                        }
                        Err(crate::signals::SignalDecodeMiss::Undecodable { error }) => {
                            debug!(error = %error, "App signal payload undecodable — ignoring");
                        }
                        // Infra/Mishpat/ElohimContent shape mismatches cannot
                        // arise from the REA decoder (different mirror-variant
                        // set), but the shared enum requires the arms.
                        Err(crate::signals::SignalDecodeMiss::InfraShapeMismatch {
                            type_tag,
                            ..
                        })
                        | Err(crate::signals::SignalDecodeMiss::MishpatShapeMismatch {
                            type_tag,
                            ..
                        })
                        | Err(crate::signals::SignalDecodeMiss::ElohimContentShapeMismatch {
                            type_tag,
                            ..
                        }) => {
                            debug!(
                                type_tag = %type_tag,
                                "Unexpected non-REA shape classification from REA decoder — ignoring"
                            );
                        }
                    }
                }
            })
            .await
    }

    /// Subscribe to ElohimContentSignals emitted by the elohim DNA's content_store
    /// coordinator. Each signal carries an `attestation:*` or `governance-action:*`
    /// Content entry's projection-relevant fields.
    ///
    /// Non-content signals (lamad/imagodei/mishpat) are logged at debug and ignored.
    pub async fn subscribe_elohim_content_signals<F>(&self, handler: F) -> String
    where
        F: Fn(crate::signals::ElohimContentSignal) + Send + Sync + 'static,
    {
        use holochain_types::signal::Signal;

        self.app_ws
            .on_signal(move |signal| {
                if let Signal::App { signal, .. } = signal {
                    let bytes: Vec<u8> = signal.into_inner().into();
                    // Typed msgpack decode of the REAL `ProjectionSignal::
                    // ContentCommitted` wire envelope, translated to the flat
                    // `ElohimContentSignal` the dispatcher consumes. The old
                    // `rmp → serde_json::Value → from_value::<ElohimContentSignal>`
                    // path failed on every real signal TWICE: serde_json::Value
                    // cannot hold the holo_hash byte arrays, AND the flat field
                    // names never matched the `{type, payload}` envelope — the
                    // attestation + recovery projections were dark since wiring.
                    // `Ok(None)` is a non-attestation/governance ContentCommitted
                    // (owned by the REA subscriber) — a quiet, expected skip.
                    match crate::signals::decode_elohim_content_signal(&bytes) {
                        Ok(Some(content)) => handler(content),
                        Ok(None) => {
                            debug!(
                                "ContentCommitted for a non-attestation/governance content_type — handled by REA subscriber, skipping"
                            );
                        }
                        Err(crate::signals::SignalDecodeMiss::ElohimContentShapeMismatch {
                            type_tag,
                            error,
                        }) => {
                            warn!(
                                type_tag = %type_tag,
                                error = %error,
                                misses_total = crate::signals::elohim_content_decode_miss_count(),
                                "ElohimContentSignal DECODE MISS — signal dropped, attestation/recovery projection will go dark"
                            );
                        }
                        Err(crate::signals::SignalDecodeMiss::ForeignSignal { type_tag }) => {
                            debug!(
                                type_tag = %type_tag,
                                "App signal from another family — ignoring"
                            );
                        }
                        Err(crate::signals::SignalDecodeMiss::Undecodable { error }) => {
                            debug!(error = %error, "App signal payload undecodable — ignoring");
                        }
                        // Infra/Mishpat/Rea shape mismatches cannot arise from
                        // the elohim-content decoder (different mirror-variant
                        // set), but the shared enum requires the arms.
                        Err(crate::signals::SignalDecodeMiss::InfraShapeMismatch {
                            type_tag,
                            ..
                        })
                        | Err(crate::signals::SignalDecodeMiss::MishpatShapeMismatch {
                            type_tag,
                            ..
                        })
                        | Err(crate::signals::SignalDecodeMiss::ReaShapeMismatch {
                            type_tag,
                            ..
                        }) => {
                            debug!(
                                type_tag = %type_tag,
                                "Unexpected non-content shape classification from content decoder — ignoring"
                            );
                        }
                    }
                }
            })
            .await
    }

    /// Subscribe to MishpatSignals emitted by the mishpat DNA's coordinator
    /// (`mishpat` zome `post_commit` hook). Routes `CommitmentCommitted` (and the
    /// gate-decision / challenge variants) to `signals::handle_mishpat_signal`,
    /// which projects the commitment into `mishpat_commitments` with
    /// `dht_anchor_hash = action_hash`.
    ///
    /// `AppWebsocket::on_signal` is registered per app-websocket connection and
    /// receives EVERY `Signal::App` emitted by ANY cell in the app — it does NOT
    /// filter by cell or role. So although the mishpat zome lives in the mishpat
    /// role cell (a different DNA than this client's connected role), its
    /// post-commit signal still arrives here, as long as the mishpat cell is part
    /// of the same installed app. Non-mishpat signals (Infrastructure, REA,
    /// content) fail to decode as `MishpatSignal` and are logged at debug and
    /// dropped — each has its own dedicated subscriber.
    ///
    /// Without this subscriber the `mishpat_commitments` projection is never
    /// populated by live authoring: the provide reconciler's
    /// `live_commons_provides_for_provider` dedup query stays empty (re-authoring
    /// every tick → unbounded commitment proliferation) and rea graduation never
    /// fires. Slice-2b code-review fix.
    pub async fn subscribe_mishpat_signals<F>(&self, handler: F) -> String
    where
        F: Fn(crate::signals::MishpatSignal) + Send + Sync + 'static,
    {
        use holochain_types::signal::Signal;

        self.app_ws
            .on_signal(move |signal| {
                if let Signal::App { signal, .. } = signal {
                    let bytes: Vec<u8> = signal.into_inner().into();
                    // Typed msgpack decode — the old `rmp → serde_json::Value`
                    // pre-pass failed on EVERY HoloHash-bearing MishpatSignal
                    // (serde_json::Value cannot represent msgpack byte arrays)
                    // and dropped it at debug level: the 2026-06-12
                    // dark-`CommitmentCommitted` root cause that left the Epic B
                    // provide projection (4626f820b) dead in production. Real
                    // misses are now LOUD; foreign signals on the shared app
                    // interface stay quiet.
                    match crate::signals::decode_mishpat_signal(&bytes) {
                        Ok(mishpat) => handler(mishpat),
                        Err(crate::signals::SignalDecodeMiss::MishpatShapeMismatch {
                            type_tag,
                            error,
                        }) => {
                            warn!(
                                type_tag = %type_tag,
                                error = %error,
                                misses_total = crate::signals::mishpat_decode_miss_count(),
                                "MishpatSignal DECODE MISS — signal dropped, projection will go dark for this variant"
                            );
                        }
                        Err(crate::signals::SignalDecodeMiss::ForeignSignal { type_tag }) => {
                            debug!(
                                type_tag = %type_tag,
                                "App signal from another family — ignoring"
                            );
                        }
                        Err(crate::signals::SignalDecodeMiss::Undecodable { error }) => {
                            debug!(error = %error, "App signal payload undecodable — ignoring");
                        }
                        // Infra/Rea/ElohimContent shape mismatches cannot arise
                        // from the mishpat decoder (different mirror-variant
                        // set), but the shared enum requires the arms.
                        Err(crate::signals::SignalDecodeMiss::InfraShapeMismatch {
                            type_tag,
                            ..
                        })
                        | Err(crate::signals::SignalDecodeMiss::ReaShapeMismatch {
                            type_tag,
                            ..
                        })
                        | Err(crate::signals::SignalDecodeMiss::ElohimContentShapeMismatch {
                            type_tag,
                            ..
                        }) => {
                            debug!(
                                type_tag = %type_tag,
                                "Unexpected non-mishpat shape classification from mishpat decoder — ignoring"
                            );
                        }
                    }
                }
            })
            .await
    }

    /// Quick health check - just verify conductor is responsive
    pub async fn ping(&self) -> Result<(), StorageError> {
        // Use list_apps as a simple ping
        self.admin_ws
            .list_apps(None)
            .await
            .map_err(|e| StorageError::Connection(format!("Conductor ping failed: {}", e)))?;
        Ok(())
    }
}

/// A `CellOwner` exposes the agent key of the cell a client is connected to.
///
/// The mode gate in `api/account.rs` uses this to assert that the caller's
/// resolved human key matches the connected cell's owner — a Tauri-direct
/// invariant. Trait-ifying it lets unit tests exercise the gate without
/// touching the holochain_client websocket layer.
pub trait CellOwner: Send + Sync {
    /// Returns the connected cell's owner agent key as a hex string,
    /// matching the encoding used by `api/identity::extract_agent_key`
    /// (X-Agent-Id header / local session row).
    fn agent_key_hex(&self) -> String;
}

impl CellOwner for HcClient {
    fn agent_key_hex(&self) -> String {
        hex::encode(self.cell_id.agent_pubkey().get_raw_39())
    }
}

#[cfg(test)]
mod cell_owner_tests {
    use super::*;

    /// A stub CellOwner used by `account.rs` mode-gate tests. Verifies the
    /// trait dispatch lands on the stub's `agent_key_hex()` return.
    struct StubOwner(String);
    impl CellOwner for StubOwner {
        fn agent_key_hex(&self) -> String {
            self.0.clone()
        }
    }

    #[test]
    fn stub_cell_owner_returns_configured_hex() {
        let stub = StubOwner("uhCAkSTUB".to_string());
        let dyn_owner: &dyn CellOwner = &stub;
        assert_eq!(dyn_owner.agent_key_hex(), "uhCAkSTUB");
    }
}
