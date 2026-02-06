//! System metrics collection
//!
//! Collects comprehensive system metrics for node management,
//! similar to `kubectl describe node`

use serde::{Deserialize, Serialize};
use sysinfo::{System, Disks, Networks, Components, CpuRefreshKind, RefreshKind};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Global system info cache (refreshed periodically)
static SYSTEM: OnceLock<std::sync::Mutex<CachedSystem>> = OnceLock::new();

struct CachedSystem {
    system: System,
    disks: Disks,
    networks: Networks,
    components: Components,
    last_refresh: Instant,
}

impl CachedSystem {
    fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        Self {
            system,
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            components: Components::new_with_refreshed_list(),
            last_refresh: Instant::now(),
        }
    }

    fn refresh_if_needed(&mut self) {
        if self.last_refresh.elapsed() > Duration::from_secs(1) {
            self.system.refresh_all();
            self.disks.refresh_list();
            self.networks.refresh_list();
            self.components.refresh();
            self.last_refresh = Instant::now();
        }
    }
}

fn get_system() -> std::sync::MutexGuard<'static, CachedSystem> {
    SYSTEM
        .get_or_init(|| std::sync::Mutex::new(CachedSystem::new()))
        .lock()
        .unwrap()
}

/// Complete node metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetrics {
    pub timestamp: u64,
    pub node_id: String,
    pub hostname: String,
    pub uptime_secs: u64,

    // Core resources
    pub cpu: CpuMetrics,
    pub memory: MemoryMetrics,
    pub disk: DiskMetrics,
    pub network: NetworkMetrics,

    // Health conditions (like k8s node conditions)
    pub conditions: NodeConditions,

    // System information
    pub system_info: SystemInfo,

    // Temperature sensors
    pub temperatures: Vec<TemperatureSensor>,

    // Service health
    pub services: ServiceHealth,

    // Storage volumes
    pub volumes: Vec<VolumeInfo>,

    // Elohim-specific
    pub elohim: ElohimMetrics,
}

/// Health conditions similar to Kubernetes node conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConditions {
    /// Memory pressure - true if available memory is low
    pub memory_pressure: ConditionStatus,
    /// Disk pressure - true if available disk space is low
    pub disk_pressure: ConditionStatus,
    /// PID pressure - true if too many processes running
    pub pid_pressure: ConditionStatus,
    /// Network available - true if network is working
    pub network_ready: ConditionStatus,
    /// Node ready - overall health status
    pub ready: ConditionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionStatus {
    pub status: bool,
    pub reason: String,
    pub message: String,
    pub last_transition: u64,
}

/// System information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Machine ID (unique hardware identifier)
    pub machine_id: String,
    /// Kernel version
    pub kernel_version: String,
    /// OS name and version
    pub os_version: String,
    /// System architecture
    pub architecture: String,
    /// Boot time (Unix timestamp)
    pub boot_time: u64,
    /// Distribution name (e.g., "Ubuntu 24.04")
    pub distribution: String,
}

/// Temperature sensor reading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureSensor {
    pub label: String,
    pub current_celsius: f32,
    pub max_celsius: Option<f32>,
    pub critical_celsius: Option<f32>,
    pub is_critical: bool,
}

/// Health status of internal services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    /// Holochain conductor status
    pub holochain: ServiceStatus,
    /// Sync service status
    pub sync: ServiceStatus,
    /// Storage service status
    pub storage: ServiceStatus,
    /// P2P networking status
    pub p2p: ServiceStatus,
    /// Dashboard API status
    pub api: ServiceStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub name: String,
    pub running: bool,
    pub healthy: bool,
    pub message: Option<String>,
    pub uptime_secs: Option<u64>,
    pub restart_count: u32,
}

/// Storage volume information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeInfo {
    pub name: String,
    pub mount_point: String,
    pub filesystem: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub usage_percent: f32,
    pub is_removable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuMetrics {
    /// Number of CPU cores
    pub cores: usize,
    /// CPU model name
    pub model: String,
    /// Current usage percentage (0-100) per core
    pub per_core_usage: Vec<f32>,
    /// Average usage percentage
    pub usage_percent: f32,
    /// Load average (1, 5, 15 minutes)
    pub load_average: [f64; 3],
    /// CPU frequency in MHz
    pub frequency_mhz: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetrics {
    /// Total memory in bytes
    pub total_bytes: u64,
    /// Used memory in bytes
    pub used_bytes: u64,
    /// Available memory in bytes
    pub available_bytes: u64,
    /// Memory usage percentage
    pub usage_percent: f32,
    /// Swap total
    pub swap_total_bytes: u64,
    /// Swap used
    pub swap_used_bytes: u64,
    /// Cached memory
    pub cached_bytes: u64,
    /// Buffer memory
    pub buffer_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskMetrics {
    /// Total disk space in bytes (primary volume)
    pub total_bytes: u64,
    /// Used disk space in bytes
    pub used_bytes: u64,
    /// Available disk space in bytes
    pub available_bytes: u64,
    /// Disk usage percentage
    pub usage_percent: f32,
    /// Mount point (e.g., /var/lib/elohim)
    pub mount_point: String,
    /// Filesystem type
    pub filesystem: String,
    /// Disk I/O reads since boot
    pub read_bytes: u64,
    /// Disk I/O writes since boot
    pub write_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    /// Network interfaces
    pub interfaces: Vec<NetworkInterface>,
    /// Total bytes received (all interfaces)
    pub rx_bytes: u64,
    /// Total bytes transmitted (all interfaces)
    pub tx_bytes: u64,
    /// Packets received
    pub rx_packets: u64,
    /// Packets transmitted
    pub tx_packets: u64,
    /// Active connections count
    pub connections: usize,
    /// Primary IP address
    pub primary_ip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub mac_address: String,
    pub ip_addresses: Vec<String>,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub is_up: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElohimMetrics {
    /// Node setup status
    pub setup_complete: bool,
    /// Cluster membership status
    pub cluster_status: ClusterStatus,
    /// Number of synced documents
    pub synced_documents: usize,
    /// Number of stored blobs
    pub stored_blobs: usize,
    /// Storage used by elohim data
    pub storage_used_bytes: u64,
    /// Connected peers
    pub connected_peers: usize,
    /// Discovered peers on local network
    pub discovered_peers: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClusterStatus {
    /// Not yet configured
    Unconfigured,
    /// Looking for cluster
    Discovering,
    /// Joining cluster
    Joining,
    /// Active cluster member
    Active { cluster_name: String, role: String },
    /// Disconnected from cluster
    Disconnected,
}

/// Collect current system metrics
pub fn collect_metrics(node_id: &str, setup_complete: bool) -> NodeMetrics {
    let mut cached = get_system();
    cached.refresh_if_needed();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let cpu = collect_cpu_metrics(&cached.system);
    let memory = collect_memory_metrics(&cached.system);
    let disk = collect_disk_metrics(&cached.disks);
    let network = collect_network_metrics(&cached.networks);
    let temperatures = collect_temperatures(&cached.components);
    let volumes = collect_volumes(&cached.disks);
    let conditions = evaluate_conditions(&memory, &disk, &cached.system);

    NodeMetrics {
        timestamp: now,
        node_id: node_id.to_string(),
        hostname: get_hostname(),
        uptime_secs: get_uptime(&cached.system),

        cpu,
        memory,
        disk,
        network,

        conditions,
        system_info: collect_system_info(&cached.system),
        temperatures,
        services: collect_service_health(setup_complete),
        volumes,

        elohim: ElohimMetrics {
            setup_complete,
            cluster_status: if setup_complete {
                ClusterStatus::Active {
                    cluster_name: "dev-cluster".to_string(),
                    role: "replica".to_string(),
                }
            } else {
                ClusterStatus::Unconfigured
            },
            synced_documents: 0,
            stored_blobs: 0,
            storage_used_bytes: 0,
            connected_peers: 0,
            discovered_peers: 0,
        },
    }
}

fn get_hostname() -> String {
    hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

fn get_uptime(sys: &System) -> u64 {
    System::uptime()
}

fn collect_cpu_metrics(sys: &System) -> CpuMetrics {
    let cpus = sys.cpus();
    let per_core_usage: Vec<f32> = cpus.iter().map(|cpu| cpu.cpu_usage()).collect();
    let avg_usage = if per_core_usage.is_empty() {
        0.0
    } else {
        per_core_usage.iter().sum::<f32>() / per_core_usage.len() as f32
    };

    let load_avg = System::load_average();
    let frequency = cpus.first().map(|c| c.frequency()).unwrap_or(0);
    let model = cpus.first()
        .map(|c| c.brand().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    CpuMetrics {
        cores: cpus.len(),
        model,
        per_core_usage,
        usage_percent: avg_usage,
        load_average: [load_avg.one, load_avg.five, load_avg.fifteen],
        frequency_mhz: frequency,
    }
}

fn collect_memory_metrics(sys: &System) -> MemoryMetrics {
    let total = sys.total_memory();
    let used = sys.used_memory();
    let available = sys.available_memory();
    let swap_total = sys.total_swap();
    let swap_used = sys.used_swap();

    let usage_percent = if total > 0 {
        (used as f32 / total as f32) * 100.0
    } else {
        0.0
    };

    MemoryMetrics {
        total_bytes: total,
        used_bytes: used,
        available_bytes: available,
        usage_percent,
        swap_total_bytes: swap_total,
        swap_used_bytes: swap_used,
        cached_bytes: 0, // sysinfo doesn't expose this directly
        buffer_bytes: 0,
    }
}

fn collect_disk_metrics(disks: &Disks) -> DiskMetrics {
    // Find the primary data volume (prefer /var/lib/elohim, fallback to root)
    let primary = disks.list().iter()
        .find(|d| d.mount_point().to_string_lossy().contains("elohim"))
        .or_else(|| disks.list().iter().find(|d| d.mount_point().to_string_lossy() == "/"))
        .or_else(|| disks.list().first());

    if let Some(disk) = primary {
        let total = disk.total_space();
        let available = disk.available_space();
        let used = total.saturating_sub(available);
        let usage_percent = if total > 0 {
            (used as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        DiskMetrics {
            total_bytes: total,
            used_bytes: used,
            available_bytes: available,
            usage_percent,
            mount_point: disk.mount_point().to_string_lossy().to_string(),
            filesystem: disk.file_system().to_string_lossy().to_string(),
            read_bytes: 0, // Would need /proc/diskstats parsing
            write_bytes: 0,
        }
    } else {
        DiskMetrics {
            total_bytes: 0,
            used_bytes: 0,
            available_bytes: 0,
            usage_percent: 0.0,
            mount_point: "/".to_string(),
            filesystem: "unknown".to_string(),
            read_bytes: 0,
            write_bytes: 0,
        }
    }
}

fn collect_network_metrics(networks: &Networks) -> NetworkMetrics {
    let mut interfaces = Vec::new();
    let mut total_rx = 0u64;
    let mut total_tx = 0u64;
    let mut total_rx_packets = 0u64;
    let mut total_tx_packets = 0u64;
    let mut primary_ip = None;

    for (name, data) in networks.list() {
        // Skip loopback and virtual interfaces for primary IP detection
        let is_physical = !name.starts_with("lo") &&
                          !name.starts_with("veth") &&
                          !name.starts_with("docker") &&
                          !name.starts_with("br-");

        // sysinfo v0.30 doesn't expose IP addresses directly on NetworkData
        // We'll leave this empty for now (would need netlink or /proc/net parsing)
        let ip_addresses: Vec<String> = Vec::new();

        // Use first non-loopback IPv4 as primary (when we can get IPs)
        if is_physical && primary_ip.is_none() && !ip_addresses.is_empty() {
            primary_ip = ip_addresses.iter()
                .find(|ip: &&String| !ip.starts_with("127.") && !ip.contains(':'))
                .cloned();
        }

        let rx = data.total_received();
        let tx = data.total_transmitted();
        let rx_packets = data.total_packets_received();
        let tx_packets = data.total_packets_transmitted();

        total_rx += rx;
        total_tx += tx;
        total_rx_packets += rx_packets;
        total_tx_packets += tx_packets;

        interfaces.push(NetworkInterface {
            name: name.to_string(),
            mac_address: data.mac_address().to_string(),
            ip_addresses,
            rx_bytes: rx,
            tx_bytes: tx,
            rx_packets,
            tx_packets,
            is_up: true, // sysinfo doesn't expose this
        });
    }

    NetworkMetrics {
        interfaces,
        rx_bytes: total_rx,
        tx_bytes: total_tx,
        rx_packets: total_rx_packets,
        tx_packets: total_tx_packets,
        connections: 0, // Would need netstat/ss parsing
        primary_ip,
    }
}

fn collect_temperatures(components: &Components) -> Vec<TemperatureSensor> {
    components.list().iter().map(|c| {
        let current = c.temperature();
        let max = c.max();
        let critical = c.critical();

        TemperatureSensor {
            label: c.label().to_string(),
            current_celsius: current,
            max_celsius: Some(max),
            critical_celsius: critical,
            is_critical: critical.map(|crit| current >= crit).unwrap_or(false),
        }
    }).collect()
}

fn collect_volumes(disks: &Disks) -> Vec<VolumeInfo> {
    disks.list().iter().map(|d| {
        let total = d.total_space();
        let available = d.available_space();
        let used = total.saturating_sub(available);
        let usage_percent = if total > 0 {
            (used as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        VolumeInfo {
            name: d.name().to_string_lossy().to_string(),
            mount_point: d.mount_point().to_string_lossy().to_string(),
            filesystem: d.file_system().to_string_lossy().to_string(),
            total_bytes: total,
            used_bytes: used,
            available_bytes: available,
            usage_percent,
            is_removable: d.is_removable(),
        }
    }).collect()
}

fn collect_system_info(sys: &System) -> SystemInfo {
    SystemInfo {
        machine_id: get_machine_id(),
        kernel_version: System::kernel_version().unwrap_or_else(|| "unknown".to_string()),
        os_version: System::long_os_version().unwrap_or_else(|| "unknown".to_string()),
        architecture: std::env::consts::ARCH.to_string(),
        boot_time: System::boot_time(),
        distribution: System::distribution_id(),
    }
}

fn get_machine_id() -> String {
    // Try to read machine-id from standard Linux location
    std::fs::read_to_string("/etc/machine-id")
        .map(|s| s.trim().to_string())
        .or_else(|_| std::fs::read_to_string("/var/lib/dbus/machine-id").map(|s| s.trim().to_string()))
        .unwrap_or_else(|_| "unknown".to_string())
}

fn evaluate_conditions(memory: &MemoryMetrics, disk: &DiskMetrics, sys: &System) -> NodeConditions {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Memory pressure: > 90% used or available < 256MB
    let memory_pressure = memory.usage_percent > 90.0 || memory.available_bytes < 256 * 1024 * 1024;

    // Disk pressure: > 90% used or available < 1GB
    let disk_pressure = disk.usage_percent > 90.0 || disk.available_bytes < 1024 * 1024 * 1024;

    // PID pressure: check if we can still create processes (simplified check)
    let pid_count = sys.processes().len();
    let pid_pressure = pid_count > 30000; // Most systems have ~32k PIDs max

    NodeConditions {
        memory_pressure: ConditionStatus {
            status: !memory_pressure,
            reason: if memory_pressure { "MemoryPressure" } else { "MemorySufficient" }.to_string(),
            message: if memory_pressure {
                format!("Memory usage at {:.1}%, available: {} MB",
                    memory.usage_percent,
                    memory.available_bytes / 1024 / 1024)
            } else {
                "kubelet has sufficient memory available".to_string()
            },
            last_transition: now,
        },
        disk_pressure: ConditionStatus {
            status: !disk_pressure,
            reason: if disk_pressure { "DiskPressure" } else { "DiskSufficient" }.to_string(),
            message: if disk_pressure {
                format!("Disk usage at {:.1}%, available: {} GB",
                    disk.usage_percent,
                    disk.available_bytes / 1024 / 1024 / 1024)
            } else {
                "kubelet has no disk pressure".to_string()
            },
            last_transition: now,
        },
        pid_pressure: ConditionStatus {
            status: !pid_pressure,
            reason: if pid_pressure { "PIDPressure" } else { "PIDSufficient" }.to_string(),
            message: if pid_pressure {
                format!("Process count at {}", pid_count)
            } else {
                "kubelet has sufficient PID available".to_string()
            },
            last_transition: now,
        },
        network_ready: ConditionStatus {
            status: true, // TODO: Actually check network connectivity
            reason: "NetworkReady".to_string(),
            message: "Network is available".to_string(),
            last_transition: now,
        },
        ready: ConditionStatus {
            status: !memory_pressure && !disk_pressure && !pid_pressure,
            reason: if !memory_pressure && !disk_pressure && !pid_pressure {
                "Ready"
            } else {
                "NotReady"
            }.to_string(),
            message: if !memory_pressure && !disk_pressure && !pid_pressure {
                "Node is healthy and ready".to_string()
            } else {
                "Node has resource pressure".to_string()
            },
            last_transition: now,
        },
    }
}

fn collect_service_health(setup_complete: bool) -> ServiceHealth {
    // TODO: Actually check service status via IPC/health endpoints
    ServiceHealth {
        holochain: ServiceStatus {
            name: "holochain".to_string(),
            running: setup_complete,
            healthy: setup_complete,
            message: if setup_complete { None } else { Some("Not configured".to_string()) },
            uptime_secs: None,
            restart_count: 0,
        },
        sync: ServiceStatus {
            name: "sync".to_string(),
            running: setup_complete,
            healthy: setup_complete,
            message: None,
            uptime_secs: None,
            restart_count: 0,
        },
        storage: ServiceStatus {
            name: "storage".to_string(),
            running: true,
            healthy: true,
            message: None,
            uptime_secs: None,
            restart_count: 0,
        },
        p2p: ServiceStatus {
            name: "p2p".to_string(),
            running: setup_complete,
            healthy: setup_complete,
            message: None,
            uptime_secs: None,
            restart_count: 0,
        },
        api: ServiceStatus {
            name: "api".to_string(),
            running: true,
            healthy: true,
            message: None,
            uptime_secs: None,
            restart_count: 0,
        },
    }
}
