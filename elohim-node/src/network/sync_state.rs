//! Sync state and connected apps tracking
//!
//! Tracks:
//! - Connected elohim-app instances
//! - Sync progress for each app
//! - Overall sync state
//! - Data flow direction

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Connected elohim-app instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedApp {
    /// App instance ID
    pub app_id: String,

    /// Device type
    pub device_type: DeviceType,

    /// Device name (e.g., "John's iPhone")
    pub device_name: Option<String>,

    /// Owner's agent key (links to imagodei identity)
    pub owner_agent_key: String,

    /// Owner display name
    pub owner_name: Option<String>,

    /// Connection status
    pub connection_status: ConnectionStatus,

    /// Sync direction
    pub sync_direction: SyncDirection,

    /// Last sync timestamp
    pub last_sync: u64,

    /// Current sync position
    pub sync_position: u64,

    /// Whether actively syncing right now
    pub is_syncing: bool,

    /// Sync progress if currently syncing
    pub sync_progress: Option<SyncProgressInfo>,

    /// When this app first connected
    pub first_seen: u64,

    /// Platform info
    pub platform: Option<String>,

    /// App version
    pub app_version: Option<String>,
}

/// Type of device running elohim-app
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeviceType {
    /// Desktop (Windows, macOS, Linux)
    Desktop,
    /// Mobile phone (iOS, Android)
    Phone,
    /// Tablet (iPad, Android tablet)
    Tablet,
    /// Web browser
    Web,
    /// Unknown device type
    Unknown,
}

/// Connection status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConnectionStatus {
    /// Currently connected
    Connected,
    /// Recently disconnected (within last hour)
    RecentlyDisconnected,
    /// Offline for extended period
    Offline,
    /// Never connected (pending first sync)
    Pending,
}

/// Direction of sync
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncDirection {
    /// App is receiving data from node (download)
    Download,
    /// App is sending data to node (upload)
    Upload,
    /// Bidirectional sync
    Bidirectional,
    /// No active sync
    Idle,
}

/// Progress info for active sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncProgressInfo {
    /// Percentage complete (0-100)
    pub percent: u8,
    /// Items synced so far
    pub items_synced: u64,
    /// Total items to sync
    pub items_total: u64,
    /// Bytes transferred
    pub bytes_transferred: u64,
    /// Estimated time remaining (seconds)
    pub eta_secs: Option<u32>,
    /// Current item being synced
    pub current_item: Option<String>,
}

/// Overall sync progress for the node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncProgress {
    /// Overall sync state
    pub state: SyncState,

    /// Total documents synced
    pub documents_synced: u64,

    /// Total blobs synced
    pub blobs_synced: u64,

    /// Total bytes stored
    pub total_bytes: u64,

    /// Sync position (monotonic, comparable across peers)
    pub position: u64,

    /// Last sync timestamp
    pub last_sync: u64,

    /// Sync lag (seconds behind leader)
    pub lag_secs: u32,

    /// Per-collection sync status
    pub collections: HashMap<String, CollectionSync>,
}

/// Sync state for the node
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncState {
    /// Initial sync in progress
    InitialSync { progress_percent: u8 },
    /// Fully synced and live
    Synced,
    /// Syncing new content
    Syncing { items_pending: u64 },
    /// Paused (user requested)
    Paused,
    /// Error during sync
    Error { message: String },
    /// Not connected to network
    Disconnected,
}

/// Sync status for a specific collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionSync {
    /// Collection name (e.g., "lamad-content", "presence")
    pub name: String,
    /// Items in this collection
    pub item_count: u64,
    /// Sync position for this collection
    pub position: u64,
    /// Whether synced
    pub synced: bool,
}

impl Default for SyncProgress {
    fn default() -> Self {
        Self {
            state: SyncState::Disconnected,
            documents_synced: 0,
            blobs_synced: 0,
            total_bytes: 0,
            position: 0,
            last_sync: 0,
            lag_secs: 0,
            collections: HashMap::new(),
        }
    }
}

#[allow(dead_code)]
impl SyncProgress {
    /// Check if node is fully synced
    pub fn is_synced(&self) -> bool {
        matches!(self.state, SyncState::Synced)
    }

    /// Get sync percentage (for initial sync)
    pub fn sync_percent(&self) -> Option<u8> {
        match &self.state {
            SyncState::InitialSync { progress_percent } => Some(*progress_percent),
            SyncState::Synced => Some(100),
            _ => None,
        }
    }

    /// Update collection sync status
    pub fn update_collection(&mut self, name: &str, item_count: u64, position: u64, synced: bool) {
        self.collections.insert(
            name.to_string(),
            CollectionSync {
                name: name.to_string(),
                item_count,
                position,
                synced,
            },
        );
    }
}

#[allow(dead_code)]
impl ConnectedApp {
    /// Create a new connected app entry
    pub fn new(app_id: String, device_type: DeviceType, owner_agent_key: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            app_id,
            device_type,
            device_name: None,
            owner_agent_key,
            owner_name: None,
            connection_status: ConnectionStatus::Connected,
            sync_direction: SyncDirection::Idle,
            last_sync: 0,
            sync_position: 0,
            is_syncing: false,
            sync_progress: None,
            first_seen: now,
            platform: None,
            app_version: None,
        }
    }

    /// Mark app as disconnected
    pub fn disconnect(&mut self) {
        self.connection_status = ConnectionStatus::RecentlyDisconnected;
        self.is_syncing = false;
        self.sync_progress = None;
        self.sync_direction = SyncDirection::Idle;
    }

    /// Start syncing
    pub fn start_sync(&mut self, direction: SyncDirection, total_items: u64) {
        self.is_syncing = true;
        self.sync_direction = direction;
        self.sync_progress = Some(SyncProgressInfo {
            percent: 0,
            items_synced: 0,
            items_total: total_items,
            bytes_transferred: 0,
            eta_secs: None,
            current_item: None,
        });
    }

    /// Update sync progress
    pub fn update_progress(&mut self, items_synced: u64, bytes: u64, current_item: Option<String>) {
        if let Some(ref mut progress) = self.sync_progress {
            progress.items_synced = items_synced;
            progress.bytes_transferred = bytes;
            progress.current_item = current_item;
            if progress.items_total > 0 {
                progress.percent = ((items_synced * 100) / progress.items_total) as u8;
            }
        }
    }

    /// Complete sync
    pub fn complete_sync(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.is_syncing = false;
        self.sync_direction = SyncDirection::Idle;
        self.last_sync = now;
        self.sync_progress = None;
    }
}

/// Summary of connected apps for dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedAppsSummary {
    /// Total connected apps
    pub total: usize,
    /// Currently syncing
    pub syncing: usize,
    /// By device type
    pub by_device: HashMap<String, usize>,
    /// By owner
    pub by_owner: HashMap<String, usize>,
}

impl ConnectedAppsSummary {
    pub fn from_apps(apps: &[ConnectedApp]) -> Self {
        let mut by_device: HashMap<String, usize> = HashMap::new();
        let mut by_owner: HashMap<String, usize> = HashMap::new();

        for app in apps {
            if app.connection_status == ConnectionStatus::Connected {
                let device = format!("{:?}", app.device_type);
                *by_device.entry(device).or_insert(0) += 1;

                let owner = app
                    .owner_name
                    .clone()
                    .unwrap_or_else(|| app.owner_agent_key.clone());
                *by_owner.entry(owner).or_insert(0) += 1;
            }
        }

        Self {
            total: apps
                .iter()
                .filter(|a| a.connection_status == ConnectionStatus::Connected)
                .count(),
            syncing: apps.iter().filter(|a| a.is_syncing).count(),
            by_device,
            by_owner,
        }
    }
}
