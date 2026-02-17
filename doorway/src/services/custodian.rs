//! Custodian Selection Service - Intelligent P2P Blob Distribution
//!
//! Manages custodian network for blob replication and retrieval:
//! - Queries projection layer for custodian commitments
//! - Tracks custodian health metrics (bandwidth, latency, uptime)
//! - Selects optimal custodians based on reach level and performance
//! - Provides fallback URL management for resilient blob delivery
//!
//! ## Custodian Selection Algorithm
//!
//! Custodians are scored based on:
//! - **Bandwidth (40%)**: Available bandwidth vs. blob bitrate requirements
//! - **Latency (30%)**: Network latency to custodian (lower is better)
//! - **Uptime (20%)**: Historical availability ratio (0.0-1.0)
//! - **Region (10%)**: Geographic proximity bonus for preferred regions
//!
//! ## Integration with Tiered Cache
//!
//! When a blob is not in the tiered cache:
//! 1. Query custodian service for available sources
//! 2. Select best custodian based on scoring algorithm
//! 3. Fetch blob from custodian's fallback URL
//! 4. Cache in appropriate tier

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{debug, info};

use crate::orchestrator::OrchestratorState;
use crate::projection::ProjectionStore;

// ============================================================================
// Helpers
// ============================================================================

/// Get current time as Unix milliseconds
fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ============================================================================
// Types
// ============================================================================

/// Reach level for content access control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ReachLevel {
    /// Only the creator can access
    Private,
    /// Invited agents only
    Invited,
    /// Local network only
    Local,
    /// Neighborhood (mDNS discovered)
    Neighborhood,
    /// Municipal/city level
    Municipal,
    /// Bioregional/state level
    Bioregional,
    /// Regional/country level
    Regional,
    /// Public/commons access
    #[default]
    Commons,
}

impl ReachLevel {
    /// Parse from string
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "private" => ReachLevel::Private,
            "invited" => ReachLevel::Invited,
            "local" => ReachLevel::Local,
            "neighborhood" => ReachLevel::Neighborhood,
            "municipal" => ReachLevel::Municipal,
            "bioregional" => ReachLevel::Bioregional,
            "regional" => ReachLevel::Regional,
            "commons" | "public" => ReachLevel::Commons,
            _ => ReachLevel::Commons,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            ReachLevel::Private => "private",
            ReachLevel::Invited => "invited",
            ReachLevel::Local => "local",
            ReachLevel::Neighborhood => "neighborhood",
            ReachLevel::Municipal => "municipal",
            ReachLevel::Bioregional => "bioregional",
            ReachLevel::Regional => "regional",
            ReachLevel::Commons => "commons",
        }
    }

    /// Check if content at this reach level is accessible to a requester
    ///
    /// The content's reach level indicates how public it is:
    /// - Commons (7) = most public, accessible to all
    /// - Private (0) = most restricted
    ///
    /// A requester with higher reach can access content at that level or below.
    /// - Commons content is accessible to everyone
    /// - Private content is only accessible to private-level requesters
    pub fn is_accessible_at(&self, requester_reach: &ReachLevel) -> bool {
        // Content reach must be >= requester reach for access
        // (higher value = more public = less restrictive)
        self.numeric_value() >= requester_reach.numeric_value()
    }

    /// Get numeric value (0=private, 7=commons)
    pub fn numeric_value(&self) -> u8 {
        match self {
            ReachLevel::Private => 0,
            ReachLevel::Invited => 1,
            ReachLevel::Local => 2,
            ReachLevel::Neighborhood => 3,
            ReachLevel::Municipal => 4,
            ReachLevel::Bioregional => 5,
            ReachLevel::Regional => 6,
            ReachLevel::Commons => 7,
        }
    }
}

/// Custodian capability metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodianCapability {
    /// Agent public key or ID
    pub agent_id: String,
    /// Display name (optional)
    pub display_name: Option<String>,
    /// Available bandwidth in Mbps
    pub bandwidth_mbps: f32,
    /// Network latency in milliseconds
    pub latency_ms: u32,
    /// Uptime ratio (0.0 - 1.0)
    pub uptime_ratio: f32,
    /// Geographic region (e.g., "us-west", "eu-central")
    pub region: Option<String>,
    /// Maximum blob size this custodian can handle (in GB)
    pub max_blob_size_gb: f32,
    /// Current number of blobs being custodied
    pub current_blob_count: u32,
    /// Reach level this custodian serves
    pub reach_level: ReachLevel,
    /// Base URL for blob retrieval
    pub base_url: String,
    /// Last health check timestamp (Unix millis)
    #[serde(default)]
    pub last_health_check_ms: Option<u64>,
    /// Health check success count
    #[serde(default)]
    pub health_check_successes: u32,
    /// Health check failure count
    #[serde(default)]
    pub health_check_failures: u32,
}

impl CustodianCapability {
    /// Calculate health score (0.0 - 1.0)
    pub fn health_score(&self) -> f32 {
        let total = self.health_check_successes + self.health_check_failures;
        if total == 0 {
            return 0.5; // Unknown health
        }
        self.health_check_successes as f32 / total as f32
    }

    /// Generate fallback URL for a content hash
    pub fn blob_url(&self, blob_hash: &str) -> String {
        format!(
            "{}/store/{}",
            self.base_url.trim_end_matches('/'),
            blob_hash
        )
    }

    /// Generate chunk URL for a content hash and chunk index
    pub fn chunk_url(&self, blob_hash: &str, chunk_index: usize) -> String {
        format!(
            "{}/store/{}/chunk/{}",
            self.base_url.trim_end_matches('/'),
            blob_hash,
            chunk_index
        )
    }
}

/// Criteria for selecting custodians
#[derive(Debug, Clone, Default)]
pub struct CustodianSelectionCriteria {
    /// Minimum required reach level
    pub min_reach: ReachLevel,
    /// Minimum bandwidth in Mbps (optional)
    pub min_bandwidth_mbps: Option<f32>,
    /// Maximum acceptable latency in ms (optional)
    pub max_latency_ms: Option<u32>,
    /// Minimum uptime ratio (optional)
    pub min_uptime_ratio: Option<f32>,
    /// Preferred regions for geographic affinity
    pub preferred_regions: Vec<String>,
    /// Maximum number of custodians to return
    pub max_custodians: Option<usize>,
    /// Blob size requirement (for capacity checking)
    pub blob_size_bytes: Option<u64>,
    /// Required bitrate for streaming (for bandwidth checking)
    pub required_bitrate_mbps: Option<f32>,
}

/// Blob commitment from a custodian
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodianBlobCommitment {
    /// Content ID this blob belongs to
    pub content_id: String,
    /// SHA256 hash of the blob
    pub blob_hash: String,
    /// Custodian agent ID
    pub custodian_id: String,
    /// Commitment status
    pub status: CommitmentStatus,
    /// When commitment started
    pub started_at: u64,
    /// When commitment expires
    pub expires_at: u64,
    /// Measured bandwidth for this blob
    pub bandwidth_mbps: f32,
    /// Replication progress (0-100)
    pub replication_progress: u8,
    /// Fallback URL for this blob
    pub fallback_url: String,
    /// Last verification timestamp
    pub last_verified_at: Option<u64>,
}

/// Commitment status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommitmentStatus {
    /// Commitment is active and blob is being served
    Active,
    /// Blob is being replicated
    Pending,
    /// Commitment failed or blob unavailable
    Failed,
    /// Commitment has expired
    Expired,
}

/// Result of custodian health probe
#[derive(Debug, Clone)]
pub struct HealthProbeResult {
    /// Whether the custodian is online
    pub online: bool,
    /// Whether the custodian is accepting new blobs
    pub accepting_blobs: bool,
    /// Measured bandwidth in Mbps
    pub bandwidth_mbps: f32,
    /// Measured latency in ms
    pub latency_ms: u32,
    /// Probe duration
    pub probe_duration: Duration,
}

// ============================================================================
// Custodian Service
// ============================================================================

/// Configuration for custodian service
#[derive(Debug, Clone)]
pub struct CustodianServiceConfig {
    /// Health probe timeout
    pub probe_timeout: Duration,
    /// Health probe interval
    pub probe_interval: Duration,
    /// Minimum custodians for "healthy" status
    pub min_healthy_custodians: usize,
    /// Default maximum custodians to return
    pub default_max_custodians: usize,
}

impl Default for CustodianServiceConfig {
    fn default() -> Self {
        Self {
            probe_timeout: Duration::from_secs(5),
            probe_interval: Duration::from_secs(60),
            min_healthy_custodians: 3,
            default_max_custodians: 5,
        }
    }
}

/// Custodian selection and management service
pub struct CustodianService {
    /// Known custodian capabilities
    capabilities: DashMap<String, CustodianCapability>,
    /// Blob -> Custodian ID mappings
    blob_custodians: DashMap<String, Vec<String>>,
    /// Active commitments by "content_id:blob_hash"
    commitments: DashMap<String, Vec<CustodianBlobCommitment>>,
    /// Service configuration
    config: CustodianServiceConfig,
    /// Statistics
    stats: CustodianServiceStats,
    /// Optional projection store for querying commitments
    projection: Option<Arc<ProjectionStore>>,
    /// Optional orchestrator state for human-scale metrics
    orchestrator: Option<Arc<OrchestratorState>>,
    /// HTTP client for health probing
    http_client: reqwest::Client,
}

/// Service statistics
struct CustodianServiceStats {
    total_probes: AtomicU64,
    successful_probes: AtomicU64,
    failed_probes: AtomicU64,
    total_selections: AtomicU64,
}

impl CustodianService {
    /// Create a new custodian service
    pub fn new(config: CustodianServiceConfig) -> Self {
        // Build HTTP client with timeout matching probe timeout
        let http_client = reqwest::Client::builder()
            .timeout(config.probe_timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            capabilities: DashMap::new(),
            blob_custodians: DashMap::new(),
            commitments: DashMap::new(),
            config,
            stats: CustodianServiceStats {
                total_probes: AtomicU64::new(0),
                successful_probes: AtomicU64::new(0),
                failed_probes: AtomicU64::new(0),
                total_selections: AtomicU64::new(0),
            },
            projection: None,
            orchestrator: None,
            http_client,
        }
    }

    /// Create with projection store for querying commitments
    pub fn with_projection(
        config: CustodianServiceConfig,
        projection: Arc<ProjectionStore>,
    ) -> Self {
        let mut service = Self::new(config);
        service.projection = Some(projection);
        service
    }

    /// Create with orchestrator state for human-scale metrics integration
    pub fn with_orchestrator(
        config: CustodianServiceConfig,
        orchestrator: Arc<OrchestratorState>,
    ) -> Self {
        let mut service = Self::new(config);
        service.orchestrator = Some(orchestrator);
        service
    }

    /// Set orchestrator state (for adding after construction)
    pub fn set_orchestrator(&mut self, orchestrator: Arc<OrchestratorState>) {
        self.orchestrator = Some(orchestrator);
    }

    /// Register a custodian's capabilities
    pub fn register_custodian(&self, capability: CustodianCapability) {
        info!(
            agent_id = %capability.agent_id,
            bandwidth = capability.bandwidth_mbps,
            region = ?capability.region,
            "Registered custodian"
        );
        self.capabilities
            .insert(capability.agent_id.clone(), capability);
    }

    /// Unregister a custodian
    pub fn unregister_custodian(&self, agent_id: &str) -> bool {
        self.capabilities.remove(agent_id).is_some()
    }

    /// Get custodian capability by agent ID
    pub fn get_capability(&self, agent_id: &str) -> Option<CustodianCapability> {
        self.capabilities.get(agent_id).map(|c| c.clone())
    }

    // ========================================================================
    // Custodian Selection
    // ========================================================================

    /// Get custodian URLs for a blob based on selection criteria
    pub fn get_custodian_urls(
        &self,
        blob_hash: &str,
        criteria: &CustodianSelectionCriteria,
    ) -> Vec<String> {
        let custodians = self.select_custodians(blob_hash, criteria);
        custodians.iter().map(|c| c.blob_url(blob_hash)).collect()
    }

    /// Select best custodians for a blob
    pub fn select_custodians(
        &self,
        blob_hash: &str,
        criteria: &CustodianSelectionCriteria,
    ) -> Vec<CustodianCapability> {
        self.stats.total_selections.fetch_add(1, Ordering::Relaxed);

        // First, check if we have specific custodians for this blob
        let known_custodians = self.blob_custodians.get(blob_hash);

        // Get all custodians that match criteria
        let mut candidates: Vec<(CustodianCapability, f32)> = self
            .capabilities
            .iter()
            .filter(|entry| {
                let c = entry.value();
                self.matches_criteria(c, criteria, known_custodians.as_ref())
            })
            .map(|entry| {
                let c = entry.value().clone();
                let score = self.score_custodian(&c, criteria);
                (c, score)
            })
            .collect();

        // Sort by score (highest first)
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top N
        let max = criteria
            .max_custodians
            .unwrap_or(self.config.default_max_custodians);

        let selected: Vec<CustodianCapability> =
            candidates.into_iter().take(max).map(|(c, _)| c).collect();

        debug!(
            blob_hash = %blob_hash,
            count = selected.len(),
            "Selected custodians"
        );

        selected
    }

    /// Select the single best URL for a blob
    pub fn select_best_url(
        &self,
        blob_hash: &str,
        criteria: &CustodianSelectionCriteria,
    ) -> Option<String> {
        let custodians = self.select_custodians(blob_hash, criteria);
        custodians.first().map(|c| c.blob_url(blob_hash))
    }

    /// Check if a custodian matches selection criteria
    fn matches_criteria(
        &self,
        custodian: &CustodianCapability,
        criteria: &CustodianSelectionCriteria,
        known_custodians: Option<&dashmap::mapref::one::Ref<'_, String, Vec<String>>>,
    ) -> bool {
        // Check reach level
        if !criteria.min_reach.is_accessible_at(&custodian.reach_level) {
            return false;
        }

        // Check bandwidth
        if let Some(min_bw) = criteria.min_bandwidth_mbps {
            if custodian.bandwidth_mbps < min_bw {
                return false;
            }
        }

        // Check required bitrate (with 2x overhead)
        if let Some(required) = criteria.required_bitrate_mbps {
            if custodian.bandwidth_mbps < required * 2.0 {
                return false;
            }
        }

        // Check latency
        if let Some(max_lat) = criteria.max_latency_ms {
            if custodian.latency_ms > max_lat {
                return false;
            }
        }

        // Check uptime
        if let Some(min_uptime) = criteria.min_uptime_ratio {
            if custodian.uptime_ratio < min_uptime {
                return false;
            }
        }

        // Check blob size capacity
        if let Some(size) = criteria.blob_size_bytes {
            let size_gb = size as f32 / (1024.0 * 1024.0 * 1024.0);
            if size_gb > custodian.max_blob_size_gb {
                return false;
            }
        }

        // If we have known custodians for this blob, prefer those
        if let Some(known) = known_custodians {
            if !known.contains(&custodian.agent_id) {
                // Still allow, but will have lower score
            }
        }

        true
    }

    /// Score a custodian for selection (0-100)
    pub fn score_custodian(
        &self,
        custodian: &CustodianCapability,
        criteria: &CustodianSelectionCriteria,
    ) -> f32 {
        let mut score = 0.0;

        // Bandwidth score (40% weight)
        // More bandwidth = better, up to 10x what's needed
        let bandwidth_needed = criteria.required_bitrate_mbps.unwrap_or(5.0) * 2.0;
        let bandwidth_score = (custodian.bandwidth_mbps / bandwidth_needed).min(1.0);
        score += bandwidth_score * 40.0;

        // Latency score (30% weight)
        // Lower latency = better
        let max_latency = criteria.max_latency_ms.unwrap_or(500) as f32;
        let latency_score = 1.0 - (custodian.latency_ms as f32 / max_latency).min(1.0);
        score += latency_score * 30.0;

        // Uptime score (20% weight)
        score += custodian.uptime_ratio * 20.0;

        // Region score (10% weight)
        if !criteria.preferred_regions.is_empty() {
            if let Some(ref region) = custodian.region {
                if criteria.preferred_regions.contains(region) {
                    score += 10.0;
                }
            }
        } else {
            // No preference, give partial credit
            score += 5.0;
        }

        // Health bonus (up to 5 extra points)
        score += custodian.health_score() * 5.0;

        score
    }

    // ========================================================================
    // Human-Scale Selection (async, integrates with orchestrator)
    // ========================================================================

    /// Select custodians with human-scale metrics from orchestrator
    ///
    /// This async method integrates with the orchestrator to factor in:
    /// - Trust score from peer attestations
    /// - Steward tier commitment level
    /// - Impact score combining multiple human-scale factors
    ///
    /// When orchestrator isn't available, falls back to standard selection.
    pub async fn select_custodians_with_metrics(
        &self,
        blob_hash: &str,
        criteria: &CustodianSelectionCriteria,
    ) -> Vec<CustodianCapability> {
        // If no orchestrator, use standard selection
        let Some(ref orchestrator) = self.orchestrator else {
            return self.select_custodians(blob_hash, criteria);
        };

        self.stats.total_selections.fetch_add(1, Ordering::Relaxed);

        // Get reach level as u8 for orchestrator query
        let reach_level = criteria.min_reach.numeric_value();

        // Get nodes from orchestrator that serve this reach level
        let ranked_nodes = orchestrator.get_nodes_for_reach(reach_level).await;

        // Build a map of node_id -> social metrics
        let social_scores: std::collections::HashMap<String, f64> = ranked_nodes
            .iter()
            .map(|n| (n.node_id.clone(), n.combined_score))
            .collect();

        // First, check if we have specific custodians for this blob
        let known_custodians = self.blob_custodians.get(blob_hash);

        // Score candidates including human-scale metrics
        let mut candidates: Vec<(CustodianCapability, f32)> = self
            .capabilities
            .iter()
            .filter(|entry| {
                let c = entry.value();
                self.matches_criteria(c, criteria, known_custodians.as_ref())
            })
            .map(|entry| {
                let c = entry.value().clone();
                let base_score = self.score_custodian(&c, criteria);

                // Add human-scale bonus (up to 20 extra points)
                // This effectively reweights: tech (80%) + human (20%)
                let social_bonus = social_scores
                    .get(&c.agent_id)
                    .map(|&s| (s * 20.0) as f32)
                    .unwrap_or(0.0);

                let total_score = base_score + social_bonus;
                (c, total_score)
            })
            .collect();

        // Sort by total score (highest first)
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top N
        let max = criteria
            .max_custodians
            .unwrap_or(self.config.default_max_custodians);

        let selected: Vec<CustodianCapability> =
            candidates.into_iter().take(max).map(|(c, _)| c).collect();

        debug!(
            blob_hash = %blob_hash,
            count = selected.len(),
            reach_level = reach_level,
            "Selected custodians with human-scale metrics"
        );

        selected
    }

    /// Select single best URL with human-scale metrics
    pub async fn select_best_url_with_metrics(
        &self,
        blob_hash: &str,
        criteria: &CustodianSelectionCriteria,
    ) -> Option<String> {
        let custodians = self
            .select_custodians_with_metrics(blob_hash, criteria)
            .await;
        custodians.first().map(|c| c.blob_url(blob_hash))
    }

    /// Get ranked custodians from orchestrator for a specific reach level
    ///
    /// This is useful when you want to find new custodians to commit
    /// content to based on their trustworthiness and capability.
    pub async fn get_trusted_custodians_for_reach(
        &self,
        reach_level: u8,
        max_count: usize,
    ) -> Vec<CustodianCapability> {
        let Some(ref orchestrator) = self.orchestrator else {
            // No orchestrator, return capabilities filtered by reach
            return self
                .capabilities
                .iter()
                .filter(|e| e.reach_level.numeric_value() >= reach_level)
                .take(max_count)
                .map(|e| e.value().clone())
                .collect();
        };

        // Get ranked nodes from orchestrator
        let ranked_nodes = orchestrator.get_nodes_for_reach(reach_level).await;

        // Match to registered capabilities
        ranked_nodes
            .iter()
            .filter_map(|node| self.capabilities.get(&node.node_id).map(|c| c.clone()))
            .take(max_count)
            .collect()
    }

    // ========================================================================
    // Blob Commitment Management
    // ========================================================================

    /// Register a blob commitment
    pub fn register_commitment(&self, commitment: CustodianBlobCommitment) {
        let key = format!("{}:{}", commitment.content_id, commitment.blob_hash);

        // Update blob_custodians mapping
        self.blob_custodians
            .entry(commitment.blob_hash.clone())
            .or_default()
            .push(commitment.custodian_id.clone());

        // Store commitment
        self.commitments.entry(key).or_default().push(commitment);
    }

    /// Get commitments for a blob
    pub fn get_commitments(
        &self,
        content_id: &str,
        blob_hash: &str,
    ) -> Vec<CustodianBlobCommitment> {
        let key = format!("{content_id}:{blob_hash}");
        self.commitments
            .get(&key)
            .map(|c| c.clone())
            .unwrap_or_default()
    }

    /// Get active fallback URLs for a blob
    pub fn get_fallback_urls(&self, content_id: &str, blob_hash: &str) -> Vec<String> {
        self.get_commitments(content_id, blob_hash)
            .into_iter()
            .filter(|c| c.status == CommitmentStatus::Active)
            .map(|c| c.fallback_url)
            .collect()
    }

    /// Update commitment status
    pub fn update_commitment_status(
        &self,
        content_id: &str,
        blob_hash: &str,
        custodian_id: &str,
        status: CommitmentStatus,
    ) -> bool {
        let key = format!("{content_id}:{blob_hash}");

        if let Some(mut commitments) = self.commitments.get_mut(&key) {
            if let Some(commitment) = commitments
                .iter_mut()
                .find(|c| c.custodian_id == custodian_id)
            {
                commitment.status = status;
                return true;
            }
        }

        false
    }

    /// Update replication progress
    pub fn update_replication_progress(
        &self,
        content_id: &str,
        blob_hash: &str,
        custodian_id: &str,
        progress: u8,
        bandwidth: f32,
    ) -> bool {
        let key = format!("{content_id}:{blob_hash}");

        if let Some(mut commitments) = self.commitments.get_mut(&key) {
            if let Some(commitment) = commitments
                .iter_mut()
                .find(|c| c.custodian_id == custodian_id)
            {
                commitment.replication_progress = progress.min(100);
                commitment.bandwidth_mbps = bandwidth;
                commitment.last_verified_at = Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                );

                // Mark as active when complete
                if progress >= 100 {
                    commitment.status = CommitmentStatus::Active;
                }

                return true;
            }
        }

        false
    }

    // ========================================================================
    // Health Probing
    // ========================================================================

    /// Probe a custodian's health by making an HTTP request to their health endpoint
    ///
    /// Performs actual HTTP GET to `{base_url}/health` and measures latency.
    /// Updates custodian capability with probe results.
    pub async fn probe_custodian_health(&self, agent_id: &str) -> Option<HealthProbeResult> {
        let capability = self.capabilities.get(agent_id)?;
        let base_url = capability.base_url.clone();
        let bandwidth_mbps = capability.bandwidth_mbps;
        drop(capability); // Release the read lock before async work

        self.stats.total_probes.fetch_add(1, Ordering::Relaxed);
        let start = Instant::now();

        // Construct health endpoint URL
        let health_url = format!("{}/health", base_url.trim_end_matches('/'));

        // Make actual HTTP request
        let probe_result = match self.http_client.get(&health_url).send().await {
            Ok(response) => {
                let latency = start.elapsed();
                let latency_ms = latency.as_millis() as u32;

                if response.status().is_success() {
                    // Try to parse health response for additional info
                    let accepting_blobs = response
                        .json::<serde_json::Value>()
                        .await
                        .ok()
                        .and_then(|v| v.get("accepting_blobs")?.as_bool())
                        .unwrap_or(true);

                    self.stats.successful_probes.fetch_add(1, Ordering::Relaxed);

                    // Update capability with probe results
                    if let Some(mut cap) = self.capabilities.get_mut(agent_id) {
                        cap.last_health_check_ms = Some(current_time_ms());
                        cap.health_check_successes += 1;
                        cap.latency_ms = latency_ms; // Update with actual measured latency
                    }

                    HealthProbeResult {
                        online: true,
                        accepting_blobs,
                        bandwidth_mbps,
                        latency_ms,
                        probe_duration: latency,
                    }
                } else {
                    debug!(
                        "Health probe to {} returned status {}",
                        health_url,
                        response.status()
                    );
                    self.stats.failed_probes.fetch_add(1, Ordering::Relaxed);

                    if let Some(mut cap) = self.capabilities.get_mut(agent_id) {
                        cap.health_check_failures += 1;
                    }

                    HealthProbeResult {
                        online: false,
                        accepting_blobs: false,
                        bandwidth_mbps: 0.0,
                        latency_ms,
                        probe_duration: latency,
                    }
                }
            }
            Err(e) => {
                debug!("Health probe to {} failed: {}", health_url, e);
                self.stats.failed_probes.fetch_add(1, Ordering::Relaxed);

                if let Some(mut cap) = self.capabilities.get_mut(agent_id) {
                    cap.health_check_failures += 1;
                }

                HealthProbeResult {
                    online: false,
                    accepting_blobs: false,
                    bandwidth_mbps: 0.0,
                    latency_ms: 0,
                    probe_duration: start.elapsed(),
                }
            }
        };

        Some(probe_result)
    }

    /// Probe health of a specific URL and return the latency
    ///
    /// Makes an HTTP HEAD request to measure round-trip time.
    /// Returns None if the request fails or times out.
    pub async fn probe_url_health(&self, url: &str) -> Option<Duration> {
        let start = Instant::now();

        match self.http_client.head(url).send().await {
            Ok(response) if response.status().is_success() => Some(start.elapsed()),
            Ok(response) => {
                debug!("URL probe to {} returned status {}", url, response.status());
                None
            }
            Err(e) => {
                debug!("URL probe to {} failed: {}", url, e);
                None
            }
        }
    }

    /// Probe all registered custodians
    pub async fn probe_all_custodians(&self) -> (usize, usize) {
        let agent_ids: Vec<String> = self.capabilities.iter().map(|e| e.key().clone()).collect();

        let mut success = 0;
        let mut failure = 0;

        for agent_id in agent_ids {
            if let Some(result) = self.probe_custodian_health(&agent_id).await {
                if result.online {
                    success += 1;
                } else {
                    failure += 1;
                }
            } else {
                failure += 1;
            }
        }

        info!(
            success = success,
            failure = failure,
            "Completed health probe cycle"
        );
        (success, failure)
    }

    // ========================================================================
    // Statistics
    // ========================================================================

    /// Get service statistics
    pub fn stats(&self) -> CustodianStats {
        CustodianStats {
            registered_custodians: self.capabilities.len(),
            tracked_blobs: self.blob_custodians.len(),
            total_commitments: self.commitments.iter().map(|e| e.len()).sum(),
            total_probes: self.stats.total_probes.load(Ordering::Relaxed),
            successful_probes: self.stats.successful_probes.load(Ordering::Relaxed),
            failed_probes: self.stats.failed_probes.load(Ordering::Relaxed),
            total_selections: self.stats.total_selections.load(Ordering::Relaxed),
        }
    }

    /// Get count of healthy custodians
    pub fn healthy_custodian_count(&self) -> usize {
        self.capabilities
            .iter()
            .filter(|e| e.health_score() >= 0.8)
            .count()
    }
}

/// Statistics returned by the custodian service
#[derive(Debug, Clone, Serialize)]
pub struct CustodianStats {
    pub registered_custodians: usize,
    pub tracked_blobs: usize,
    pub total_commitments: usize,
    pub total_probes: u64,
    pub successful_probes: u64,
    pub failed_probes: u64,
    pub total_selections: u64,
}

impl CustodianStats {
    /// Calculate probe success rate
    pub fn probe_success_rate(&self) -> f64 {
        if self.total_probes == 0 {
            return 0.0;
        }
        self.successful_probes as f64 / self.total_probes as f64
    }
}

// ============================================================================
// Background Health Task
// ============================================================================

/// Spawn background health probe task
pub fn spawn_health_probe_task(service: Arc<CustodianService>, interval: Duration) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;
            let (_success, _failure) = service.probe_all_custodians().await;
        }
    });

    info!(
        interval_secs = interval.as_secs(),
        "Custodian health probe task started"
    );
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_custodian(agent_id: &str) -> CustodianCapability {
        CustodianCapability {
            agent_id: agent_id.to_string(),
            display_name: Some(format!("Custodian {agent_id}")),
            bandwidth_mbps: 100.0,
            latency_ms: 50,
            uptime_ratio: 0.99,
            region: Some("us-west".to_string()),
            max_blob_size_gb: 10.0,
            current_blob_count: 5,
            reach_level: ReachLevel::Commons,
            base_url: format!("https://custodian-{agent_id}.example.com"),
            last_health_check_ms: None,
            health_check_successes: 10,
            health_check_failures: 0,
        }
    }

    #[test]
    fn test_register_custodian() {
        let service = CustodianService::new(CustodianServiceConfig::default());
        let custodian = test_custodian("agent1");

        service.register_custodian(custodian.clone());

        assert!(service.get_capability("agent1").is_some());
        assert_eq!(service.stats().registered_custodians, 1);
    }

    #[test]
    fn test_select_custodians() {
        let service = CustodianService::new(CustodianServiceConfig::default());

        // Register multiple custodians with different scores
        let mut c1 = test_custodian("agent1");
        c1.bandwidth_mbps = 100.0;
        c1.latency_ms = 20;

        let mut c2 = test_custodian("agent2");
        c2.bandwidth_mbps = 50.0;
        c2.latency_ms = 100;

        let mut c3 = test_custodian("agent3");
        c3.bandwidth_mbps = 200.0;
        c3.latency_ms = 10;

        service.register_custodian(c1);
        service.register_custodian(c2);
        service.register_custodian(c3);

        let criteria = CustodianSelectionCriteria {
            max_custodians: Some(2),
            ..Default::default()
        };

        let selected = service.select_custodians("test_hash", &criteria);
        assert_eq!(selected.len(), 2);

        // Agent3 should be first (highest bandwidth, lowest latency)
        assert_eq!(selected[0].agent_id, "agent3");
    }

    #[test]
    fn test_score_custodian() {
        let service = CustodianService::new(CustodianServiceConfig::default());
        let custodian = test_custodian("agent1");

        let criteria = CustodianSelectionCriteria {
            required_bitrate_mbps: Some(10.0),
            max_latency_ms: Some(100),
            preferred_regions: vec!["us-west".to_string()],
            ..Default::default()
        };

        let score = service.score_custodian(&custodian, &criteria);

        // Should be high score:
        // - Bandwidth: 100 Mbps for 20 Mbps needed = 1.0 * 40 = 40
        // - Latency: 50ms / 100ms = 0.5 inverted = 0.5 * 30 = 15
        // - Uptime: 0.99 * 20 = 19.8
        // - Region: matched = 10
        // - Health: 1.0 * 5 = 5
        // Total ~90
        assert!(score > 80.0, "Score should be high: {score}");
    }

    #[test]
    fn test_reach_level() {
        assert!(ReachLevel::Commons.is_accessible_at(&ReachLevel::Commons));
        assert!(ReachLevel::Commons.is_accessible_at(&ReachLevel::Private));
        assert!(!ReachLevel::Private.is_accessible_at(&ReachLevel::Commons));
    }

    #[test]
    fn test_commitment_management() {
        let service = CustodianService::new(CustodianServiceConfig::default());

        let commitment = CustodianBlobCommitment {
            content_id: "content1".to_string(),
            blob_hash: "hash123".to_string(),
            custodian_id: "agent1".to_string(),
            status: CommitmentStatus::Pending,
            started_at: 1000,
            expires_at: 2000,
            bandwidth_mbps: 50.0,
            replication_progress: 0,
            fallback_url: "https://example.com/blob/hash123".to_string(),
            last_verified_at: None,
        };

        service.register_commitment(commitment);

        let commitments = service.get_commitments("content1", "hash123");
        assert_eq!(commitments.len(), 1);
        assert_eq!(commitments[0].status, CommitmentStatus::Pending);

        // Update progress
        service.update_replication_progress("content1", "hash123", "agent1", 100, 75.0);

        let updated = service.get_commitments("content1", "hash123");
        assert_eq!(updated[0].status, CommitmentStatus::Active);
        assert_eq!(updated[0].replication_progress, 100);
    }

    #[test]
    fn test_fallback_urls() {
        let service = CustodianService::new(CustodianServiceConfig::default());

        // Add two commitments, one active, one pending
        let c1 = CustodianBlobCommitment {
            content_id: "content1".to_string(),
            blob_hash: "hash123".to_string(),
            custodian_id: "agent1".to_string(),
            status: CommitmentStatus::Active,
            started_at: 1000,
            expires_at: 2000,
            bandwidth_mbps: 50.0,
            replication_progress: 100,
            fallback_url: "https://a.example.com/blob/hash123".to_string(),
            last_verified_at: None,
        };

        let c2 = CustodianBlobCommitment {
            custodian_id: "agent2".to_string(),
            status: CommitmentStatus::Pending,
            fallback_url: "https://b.example.com/blob/hash123".to_string(),
            ..c1.clone()
        };

        service.register_commitment(c1);
        service.register_commitment(c2);

        let urls = service.get_fallback_urls("content1", "hash123");
        assert_eq!(urls.len(), 1); // Only active commitment
        assert!(urls[0].contains("a.example.com"));
    }

    #[tokio::test]
    async fn test_select_custodians_with_metrics_fallback() {
        // Without orchestrator, should fall back to standard selection
        let service = CustodianService::new(CustodianServiceConfig::default());

        let mut c1 = test_custodian("agent1");
        c1.bandwidth_mbps = 100.0;
        c1.latency_ms = 20;

        let mut c2 = test_custodian("agent2");
        c2.bandwidth_mbps = 50.0;
        c2.latency_ms = 100;

        service.register_custodian(c1);
        service.register_custodian(c2);

        let criteria = CustodianSelectionCriteria {
            max_custodians: Some(2),
            ..Default::default()
        };

        let selected = service
            .select_custodians_with_metrics("test_hash", &criteria)
            .await;

        assert_eq!(selected.len(), 2);
        // Agent1 should be first (higher bandwidth, lower latency)
        assert_eq!(selected[0].agent_id, "agent1");
    }

    #[test]
    fn test_reach_level_numeric_values() {
        assert_eq!(ReachLevel::Private.numeric_value(), 0);
        assert_eq!(ReachLevel::Invited.numeric_value(), 1);
        assert_eq!(ReachLevel::Local.numeric_value(), 2);
        assert_eq!(ReachLevel::Neighborhood.numeric_value(), 3);
        assert_eq!(ReachLevel::Municipal.numeric_value(), 4);
        assert_eq!(ReachLevel::Bioregional.numeric_value(), 5);
        assert_eq!(ReachLevel::Regional.numeric_value(), 6);
        assert_eq!(ReachLevel::Commons.numeric_value(), 7);
    }

    #[tokio::test]
    async fn test_get_trusted_custodians_for_reach() {
        // Without orchestrator, should return filtered capabilities
        let service = CustodianService::new(CustodianServiceConfig::default());

        let mut c1 = test_custodian("agent1");
        c1.reach_level = ReachLevel::Commons;

        let mut c2 = test_custodian("agent2");
        c2.reach_level = ReachLevel::Regional;

        let mut c3 = test_custodian("agent3");
        c3.reach_level = ReachLevel::Neighborhood;

        service.register_custodian(c1);
        service.register_custodian(c2);
        service.register_custodian(c3);

        // Request Regional level (6) - should get Commons (7) and Regional (6)
        let custodians = service.get_trusted_custodians_for_reach(6, 10).await;
        assert!(custodians.len() >= 2);

        // All returned should have reach >= 6
        for c in &custodians {
            assert!(c.reach_level.numeric_value() >= 6);
        }
    }
}
