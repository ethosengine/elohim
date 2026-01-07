// Elohim Node Dashboard JavaScript

// === Multi-Node Dashboard Support ===

// Current viewing context
let currentNode = {
    isLocal: true,
    address: null,
    nodeId: null,
    hostname: null
};

// Store discovered nodes
let discoveredNodes = [];

// Fetch list of nodes on the network
async function fetchNodes() {
    try {
        const res = await fetch('/api/nodes');
        discoveredNodes = await res.json();
        renderNodeSelector();
    } catch (err) {
        console.error('Failed to fetch nodes:', err);
    }
}

// Render the node selector dropdown
function renderNodeSelector() {
    const container = document.getElementById('node-list');
    const currentName = document.getElementById('current-node-name');

    if (!discoveredNodes || discoveredNodes.length === 0) {
        container.innerHTML = '<div class="node-option"><span class="node-option-info">No nodes found</span></div>';
        return;
    }

    // Update current node name in button
    if (currentNode.isLocal) {
        const localNode = discoveredNodes.find(n => n.is_local);
        if (localNode) {
            currentName.textContent = localNode.hostname;
        }
    } else {
        currentName.textContent = currentNode.hostname || 'Remote Node';
    }

    container.innerHTML = discoveredNodes.map(node => {
        const isCurrent = currentNode.isLocal ? node.is_local :
            (node.addresses.includes(currentNode.address?.split(':')[0]));
        const address = node.addresses[0] || '127.0.0.1';

        return `
            <div class="node-option ${isCurrent ? 'current' : ''} ${node.is_local ? 'local' : ''}"
                 onclick="switchToNode('${node.node_id}', '${node.hostname}', '${address}:${node.port}', ${node.is_local})">
                <div class="node-indicator ${node.status === 'online' ? '' : 'offline'}"></div>
                <div class="node-option-info">
                    <div class="node-option-name">${node.hostname}</div>
                    <div class="node-option-details">${address}${node.version ? ' • v' + node.version : ''}</div>
                </div>
                ${node.is_local ? '<span class="node-option-badge">Local</span>' : ''}
            </div>
        `;
    }).join('');
}

// Switch to viewing a different node
function switchToNode(nodeId, hostname, address, isLocal) {
    // Close dropdown
    document.getElementById('node-dropdown').classList.remove('open');

    if (isLocal) {
        // Switch back to local node
        currentNode = { isLocal: true, address: null, nodeId: null, hostname: null };
        document.getElementById('viewing-remote').style.display = 'none';

        // Refresh all data from local
        refreshAllData();
    } else {
        // Switch to remote node
        currentNode = { isLocal: false, address, nodeId, hostname };
        document.getElementById('viewing-remote').style.display = 'flex';
        document.getElementById('remote-node-name').textContent = hostname;

        // Fetch data from remote node
        refreshAllData();
    }

    // Update selector
    renderNodeSelector();
}

// Return to local node
function returnToLocal() {
    switchToNode(null, null, null, true);
}

// Wrapper to fetch from current node (local or remote via proxy)
async function fetchFromCurrentNode(endpoint) {
    if (currentNode.isLocal) {
        return fetch(endpoint);
    } else {
        // Proxy through local node
        const res = await fetch('/api/proxy', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                node_address: currentNode.address,
                endpoint: endpoint
            })
        });
        return res;
    }
}

// Refresh all dashboard data
async function refreshAllData() {
    await Promise.all([
        fetchStatus(),
        fetchMetrics(),
        fetchNetworkStatus(),
        fetchConnectedApps()
    ]);
}

// Toggle node dropdown
function toggleNodeDropdown() {
    const dropdown = document.getElementById('node-dropdown');
    dropdown.classList.toggle('open');
}

// Close dropdown when clicking outside
document.addEventListener('click', (e) => {
    const selector = document.querySelector('.node-selector');
    const dropdown = document.getElementById('node-dropdown');
    if (selector && !selector.contains(e.target)) {
        dropdown.classList.remove('open');
    }
});

// Format bytes to human readable
function formatBytes(bytes) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

// Format uptime
function formatUptime(seconds) {
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    const mins = Math.floor((seconds % 3600) / 60);

    if (days > 0) return `${days}d ${hours}h ${mins}m`;
    if (hours > 0) return `${hours}h ${mins}m`;
    return `${mins}m`;
}

// Update status badge
function updateClusterStatus(status) {
    const badge = document.getElementById('cluster-status');
    badge.className = 'status-badge';

    if (status === 'active') {
        badge.textContent = 'Connected';
        badge.classList.add('status-active');
    } else if (status === 'connecting') {
        badge.textContent = 'Connecting';
        badge.classList.add('status-connecting');
    } else if (status === 'disconnected') {
        badge.textContent = 'Disconnected';
        badge.classList.add('status-disconnected');
    } else {
        badge.textContent = 'Unconfigured';
        badge.classList.add('status-unknown');
    }
}

// Fetch and display node status
async function fetchStatus() {
    try {
        const res = await fetchFromCurrentNode('/api/status');
        const data = await res.json();

        document.getElementById('node-id').textContent = data.node_id || '-';
        document.getElementById('hostname').textContent = data.hostname || '-';
        document.getElementById('cluster-name').textContent = data.cluster_name || '-';
        document.getElementById('cluster-role').textContent = data.cluster_role || '-';
        document.getElementById('uptime').textContent = formatUptime(data.uptime_secs);
        document.getElementById('version').textContent = data.version || '-';

        if (data.setup_complete) {
            updateClusterStatus('active');
        } else {
            updateClusterStatus('unconfigured');
        }
    } catch (err) {
        console.error('Failed to fetch status:', err);
    }
}

// Fetch and display metrics
async function fetchMetrics() {
    try {
        const res = await fetchFromCurrentNode('/api/metrics');
        const data = await res.json();

        // CPU
        const cpuUsage = data.cpu.usage_percent.toFixed(1);
        document.getElementById('cpu-usage').textContent = cpuUsage;
        document.getElementById('cpu-bar').style.width = cpuUsage + '%';
        document.getElementById('cpu-bar').className = 'progress-fill' +
            (cpuUsage > 90 ? ' critical' : cpuUsage > 70 ? ' high' : '');
        document.getElementById('cpu-cores').textContent = data.cpu.cores + ' cores';
        document.getElementById('cpu-model').textContent = data.cpu.model;
        document.getElementById('load-avg').textContent =
            data.cpu.load_average.map(v => v.toFixed(2)).join(', ');

        // Memory
        const memUsage = data.memory.usage_percent.toFixed(1);
        document.getElementById('mem-usage').textContent = memUsage;
        document.getElementById('mem-bar').style.width = memUsage + '%';
        document.getElementById('mem-bar').className = 'progress-fill' +
            (memUsage > 90 ? ' critical' : memUsage > 70 ? ' high' : '');
        document.getElementById('mem-used').textContent = formatBytes(data.memory.used_bytes);
        document.getElementById('mem-total').textContent = formatBytes(data.memory.total_bytes);
        document.getElementById('swap-used').textContent = formatBytes(data.memory.swap_used_bytes);
        document.getElementById('swap-total').textContent = formatBytes(data.memory.swap_total_bytes);

        // Disk
        const diskUsage = data.disk.usage_percent.toFixed(1);
        document.getElementById('disk-usage').textContent = diskUsage;
        document.getElementById('disk-bar').style.width = diskUsage + '%';
        document.getElementById('disk-bar').className = 'progress-fill' +
            (diskUsage > 90 ? ' critical' : diskUsage > 70 ? ' high' : '');
        document.getElementById('disk-used').textContent = formatBytes(data.disk.used_bytes);
        document.getElementById('disk-total').textContent = formatBytes(data.disk.total_bytes);
        document.getElementById('disk-mount').textContent = data.disk.mount_point;

        // Network
        document.getElementById('net-connections').textContent = data.network.connections;
        document.getElementById('net-rx').textContent = formatBytes(data.network.rx_bytes);
        document.getElementById('net-tx').textContent = formatBytes(data.network.tx_bytes);
        document.getElementById('net-interfaces').textContent =
            data.network.interfaces.length + ' interfaces';

        // Elohim
        document.getElementById('synced-docs').textContent = data.elohim.synced_documents;
        document.getElementById('stored-blobs').textContent = data.elohim.stored_blobs;
        document.getElementById('storage-used').textContent = formatBytes(data.elohim.storage_used_bytes);
        document.getElementById('connected-peers').textContent = data.elohim.connected_peers;
        document.getElementById('discovered-peers').textContent = data.elohim.discovered_peers;

        // System Info
        if (data.system_info) {
            document.getElementById('sys-os').textContent = data.system_info.os_version || '-';
            document.getElementById('sys-kernel').textContent = data.system_info.kernel_version || '-';
            document.getElementById('sys-arch').textContent = data.system_info.architecture || '-';
            document.getElementById('sys-boot').textContent = data.system_info.boot_time ?
                new Date(data.system_info.boot_time * 1000).toLocaleString() : '-';
            document.getElementById('sys-machine-id').textContent =
                data.system_info.machine_id ? data.system_info.machine_id.slice(0, 12) + '...' : '-';
        }

        // Primary IP from network
        if (data.network.primary_ip) {
            document.getElementById('sys-primary-ip').textContent = data.network.primary_ip;
        }

        // Health Conditions
        if (data.conditions) {
            renderConditions(data.conditions);
        }

        // Services
        if (data.services) {
            renderServices(data.services);
        }

        // Temperatures
        if (data.temperatures) {
            renderTemperatures(data.temperatures);
        }

        // Storage Volumes
        if (data.volumes) {
            renderVolumes(data.volumes);
        }

    } catch (err) {
        console.error('Failed to fetch metrics:', err);
    }
}

// Render health conditions
function renderConditions(conditions) {
    // Ready
    updateCondition('cond-ready', conditions.ready);
    document.getElementById('cond-ready-msg').textContent = conditions.ready.message;

    // Memory Pressure (inverted - status=true means NO pressure)
    updateCondition('cond-memory', conditions.memory_pressure);
    document.getElementById('cond-memory-msg').textContent = conditions.memory_pressure.message;

    // Disk Pressure
    updateCondition('cond-disk', conditions.disk_pressure);
    document.getElementById('cond-disk-msg').textContent = conditions.disk_pressure.message;

    // PID Pressure
    updateCondition('cond-pid', conditions.pid_pressure);
    document.getElementById('cond-pid-msg').textContent = conditions.pid_pressure.message;

    // Network Ready
    updateCondition('cond-network', conditions.network_ready);
    document.getElementById('cond-network-msg').textContent = conditions.network_ready.message;
}

function updateCondition(id, condition) {
    const el = document.getElementById(id);
    const icon = el.querySelector('.condition-icon');

    if (condition.status) {
        el.classList.remove('unhealthy');
        icon.innerHTML = '&#x2714;'; // checkmark
    } else {
        el.classList.add('unhealthy');
        icon.innerHTML = '&#x2718;'; // X mark
    }
}

// Render services
function renderServices(services) {
    const container = document.getElementById('services-list');
    const serviceList = [
        services.holochain,
        services.sync,
        services.storage,
        services.p2p,
        services.api
    ];

    container.innerHTML = serviceList.map(svc => {
        const indicatorClass = svc.running && svc.healthy ? '' :
            svc.running ? 'degraded' : 'stopped';
        const status = svc.running && svc.healthy ? 'Healthy' :
            svc.running ? 'Degraded' : 'Stopped';

        return `
            <div class="service-item">
                <div class="service-indicator ${indicatorClass}"></div>
                <div class="service-info">
                    <span class="service-name">${svc.name}</span>
                    <span class="service-status">${status}${svc.message ? ': ' + svc.message : ''}</span>
                </div>
            </div>
        `;
    }).join('');
}

// Render temperatures
function renderTemperatures(temps) {
    const container = document.getElementById('temps-list');

    if (!temps || temps.length === 0) {
        container.innerHTML = '<p class="empty-state">No temperature sensors detected</p>';
        return;
    }

    container.innerHTML = temps.map(t => {
        const tempClass = t.is_critical ? 'hot' :
            t.current_celsius > 70 ? 'warm' : 'normal';

        return `
            <div class="temp-item">
                <span class="temp-label">${t.label}</span>
                <span class="temp-value ${tempClass}">${t.current_celsius.toFixed(1)}°C</span>
            </div>
        `;
    }).join('');
}

// Render storage volumes
function renderVolumes(volumes) {
    const container = document.getElementById('volumes-list');

    if (!volumes || volumes.length === 0) {
        container.innerHTML = '<p class="empty-state">No volumes detected</p>';
        return;
    }

    container.innerHTML = volumes.map(v => {
        const fillClass = v.usage_percent > 90 ? 'critical' :
            v.usage_percent > 70 ? 'high' : '';

        return `
            <div class="volume-item">
                <div class="volume-header">
                    <span class="volume-name">${v.name || v.mount_point}</span>
                    <span class="volume-mount">${v.mount_point}</span>
                </div>
                <div class="volume-bar">
                    <div class="volume-fill ${fillClass}" style="width: ${v.usage_percent}%"></div>
                </div>
                <div class="volume-details">
                    <span>${formatBytes(v.used_bytes)} / ${formatBytes(v.total_bytes)}</span>
                    <span>${v.usage_percent.toFixed(1)}% used</span>
                </div>
            </div>
        `;
    }).join('');
}

// Fetch discovered peers
async function fetchPeers() {
    try {
        const res = await fetch('/api/discovery/peers');
        const peers = await res.json();

        const list = document.getElementById('peers-list');

        if (peers.length === 0) {
            list.innerHTML = '<p class="empty-state">No peers discovered on local network</p>';
            return;
        }

        list.innerHTML = peers.map(peer => `
            <div class="peer-item">
                <div class="peer-info">
                    <span class="peer-name">${peer.hostname || peer.peer_id}</span>
                    <span class="peer-details">
                        ${peer.addresses.join(', ')}
                        ${peer.mac_address ? ' | MAC: ' + peer.mac_address : ''}
                    </span>
                </div>
                <span class="peer-type ${peer.node_type.toLowerCase()}">${peer.node_type}</span>
            </div>
        `).join('');

    } catch (err) {
        console.error('Failed to fetch peers:', err);
    }
}

// Fetch pairing requests
async function fetchPairingRequests() {
    try {
        const res = await fetch('/api/pairing/requests');
        const requests = await res.json();

        const section = document.getElementById('pairing-section');
        const list = document.getElementById('pairing-list');

        const pending = requests.filter(r => r.status === 'Pending');

        if (pending.length === 0) {
            section.style.display = 'none';
            return;
        }

        section.style.display = 'block';
        list.innerHTML = pending.map(req => `
            <div class="peer-item">
                <div class="peer-info">
                    <span class="peer-name">${req.from_peer.hostname || req.from_peer.peer_id}</span>
                    <span class="peer-details">
                        MAC: ${req.from_peer.mac_address || 'Unknown'}
                        | Requested: ${new Date(req.requested_at * 1000).toLocaleString()}
                    </span>
                </div>
                <div>
                    <button class="btn btn-small btn-primary" onclick="approvePairing('${req.request_id}')">Approve</button>
                    <button class="btn btn-small btn-secondary" onclick="rejectPairing('${req.request_id}')">Reject</button>
                </div>
            </div>
        `).join('');

    } catch (err) {
        console.error('Failed to fetch pairing requests:', err);
    }
}

// Approve pairing
async function approvePairing(requestId) {
    try {
        const res = await fetch('/api/pairing/approve', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ request_id: requestId })
        });
        const data = await res.json();

        if (data.success) {
            fetchPairingRequests();
        } else {
            alert('Failed to approve: ' + data.message);
        }
    } catch (err) {
        console.error('Failed to approve pairing:', err);
    }
}

// Reject pairing
async function rejectPairing(requestId) {
    const reason = prompt('Reason for rejection (optional):');

    try {
        const res = await fetch('/api/pairing/reject', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ request_id: requestId, reason })
        });
        const data = await res.json();

        if (data.success) {
            fetchPairingRequests();
        } else {
            alert('Failed to reject: ' + data.message);
        }
    } catch (err) {
        console.error('Failed to reject pairing:', err);
    }
}

// Scan network
async function scanNetwork() {
    const btn = document.getElementById('scan-btn');
    btn.disabled = true;
    btn.textContent = 'Scanning...';

    try {
        await fetch('/api/discovery/scan', { method: 'POST' });
        await fetchPeers();
    } catch (err) {
        console.error('Failed to scan network:', err);
    } finally {
        btn.disabled = false;
        btn.textContent = 'Scan Network';
    }
}

// === Network Membership Functions ===

// Format timestamp to relative time
function formatRelativeTime(timestamp) {
    if (!timestamp) return '-';
    const now = Date.now() / 1000;
    const diff = now - timestamp;

    if (diff < 60) return 'Just now';
    if (diff < 3600) return Math.floor(diff / 60) + 'm ago';
    if (diff < 86400) return Math.floor(diff / 3600) + 'h ago';
    return new Date(timestamp * 1000).toLocaleDateString();
}

// Get status badge class
function getStatusClass(status) {
    const statusMap = {
        'Registered': 'status-active',
        'Active': 'status-active',
        'Registering': 'status-connecting',
        'Synced': 'status-active',
        'Syncing': 'status-connecting',
        'InitialSync': 'status-connecting',
        'Connected': 'status-active',
        'Disconnected': 'status-disconnected',
        'Failed': 'status-disconnected',
        'Suspended': 'status-disconnected',
        'Unregistered': 'status-unknown',
        'Paused': 'status-unknown',
        'Error': 'status-disconnected'
    };
    return statusMap[status] || 'status-unknown';
}

// Fetch network membership status
async function fetchNetworkStatus() {
    try {
        const res = await fetchFromCurrentNode('/api/network/status');
        const data = await res.json();

        // Network status badge
        const networkStatus = document.getElementById('network-status');
        const statusStr = typeof data.status === 'string' ? data.status : Object.keys(data.status)[0];
        networkStatus.textContent = statusStr;
        networkStatus.className = 'status-badge ' + getStatusClass(statusStr);

        // Operator info
        if (data.operator) {
            document.getElementById('operator-name').textContent =
                data.operator.display_name || data.operator.agent_pub_key.slice(0, 16) + '...';
        } else {
            document.getElementById('operator-name').textContent = 'Not assigned';
        }

        // Registration time
        document.getElementById('registered-at').textContent =
            data.registered_at ? new Date(data.registered_at * 1000).toLocaleDateString() : '-';

        // Last heartbeat
        document.getElementById('last-heartbeat').textContent =
            formatRelativeTime(data.last_heartbeat);

        // Doorways list
        renderDoorways(data.doorways);

        // Sync progress
        const syncState = data.sync_progress?.state;
        const syncStateStr = typeof syncState === 'string' ? syncState :
            (syncState ? Object.keys(syncState)[0] : '-');
        document.getElementById('sync-state').textContent = syncStateStr;
        document.getElementById('sync-position').textContent = data.sync_progress?.position || 0;
        document.getElementById('docs-synced').textContent = data.sync_progress?.documents_synced || 0;
        document.getElementById('blobs-synced').textContent = data.sync_progress?.blobs_synced || 0;

        // Connected apps summary
        document.getElementById('apps-total').textContent = data.connected_apps?.total || 0;
        document.getElementById('apps-syncing').textContent = data.connected_apps?.currently_syncing || 0;

        // Update main cluster status based on network registration
        if (data.is_registered) {
            updateClusterStatus('active');
        }

    } catch (err) {
        console.error('Failed to fetch network status:', err);
    }
}

// Render doorways list
function renderDoorways(doorways) {
    const container = document.getElementById('doorways-list');

    if (!doorways || doorways.length === 0) {
        container.innerHTML = '<p class="empty-state">Not connected to any doorways</p>';
        return;
    }

    container.innerHTML = doorways.map(d => `
        <div class="doorway-item">
            <div class="doorway-info">
                <span class="doorway-url">${d.url}</span>
                ${d.is_primary ? '<span class="badge primary">Primary</span>' : ''}
            </div>
            <div class="doorway-meta">
                <span class="status-badge ${getStatusClass(d.status)}">${d.status}</span>
                <span class="last-contact">Last contact: ${formatRelativeTime(d.last_contact)}</span>
            </div>
        </div>
    `).join('');
}

// Fetch connected apps
async function fetchConnectedApps() {
    try {
        const res = await fetchFromCurrentNode('/api/network/apps');
        const apps = await res.json();

        const container = document.getElementById('apps-list');

        if (!apps || apps.length === 0) {
            container.innerHTML = '<p class="empty-state">No apps connected</p>';
            return;
        }

        container.innerHTML = apps.map(app => {
            const syncInfo = app.sync_progress;
            let syncStatus = 'Idle';
            let syncClass = 'status-unknown';

            if (app.is_syncing && syncInfo) {
                const state = typeof syncInfo.state === 'string' ? syncInfo.state :
                    Object.keys(syncInfo.state)[0];
                syncStatus = state;
                syncClass = getStatusClass(state);
            } else if (app.is_syncing) {
                syncStatus = 'Syncing';
                syncClass = 'status-connecting';
            }

            return `
                <div class="app-item">
                    <div class="app-info">
                        <span class="app-icon">${getDeviceIcon(app.device_type)}</span>
                        <div class="app-details">
                            <span class="app-name">${app.display_name || app.app_id}</span>
                            <span class="app-meta">${app.device_type} | Connected ${formatRelativeTime(app.connected_at)}</span>
                        </div>
                    </div>
                    <div class="app-sync">
                        <span class="status-badge ${syncClass}">${syncStatus}</span>
                        ${syncInfo ? `<span class="sync-detail">${syncInfo.documents_synced || 0} docs</span>` : ''}
                    </div>
                </div>
            `;
        }).join('');

    } catch (err) {
        console.error('Failed to fetch connected apps:', err);
    }
}

// Get device icon
function getDeviceIcon(deviceType) {
    const icons = {
        'Desktop': '&#x1F4BB;',
        'Phone': '&#x1F4F1;',
        'Tablet': '&#x1F4F1;',
        'Web': '&#x1F310;',
        'Unknown': '&#x2753;'
    };
    return icons[deviceType] || icons['Unknown'];
}

// === Update Functions ===

let lastUpdateStatus = null;

// Fetch update status
async function fetchUpdateStatus() {
    try {
        const res = await fetch('/api/update/status');
        const data = await res.json();

        document.getElementById('current-version').textContent = data.current_version;
        document.getElementById('auto-update').textContent = data.auto_update_enabled ? 'Enabled' : 'Disabled';

        updateStatusDisplay(data.status);
        lastUpdateStatus = data.status;
    } catch (err) {
        console.error('Failed to fetch update status:', err);
    }
}

// Update the status display based on update status
function updateStatusDisplay(status) {
    const badge = document.getElementById('update-status');
    const actions = document.getElementById('update-actions');
    const info = document.getElementById('update-info');

    badge.className = 'status-badge';

    if (status === 'UpToDate') {
        badge.textContent = 'Up to Date';
        badge.classList.add('up-to-date');
        actions.style.display = 'none';
    } else if (status.UpdateAvailable) {
        badge.textContent = 'Update Available';
        badge.classList.add('update-available');
        actions.style.display = 'block';
        info.innerHTML = `
            <strong>New version available: ${status.UpdateAvailable.latest}</strong><br>
            Current: ${status.UpdateAvailable.current}<br>
            Size: ${formatBytes(status.UpdateAvailable.size_bytes)}
            ${status.UpdateAvailable.release_notes ? '<br><br>' + status.UpdateAvailable.release_notes : ''}
        `;
        document.getElementById('apply-update-btn').style.display = 'inline-block';
    } else if (status.Downloading) {
        badge.textContent = `Downloading ${status.Downloading.progress_percent}%`;
        badge.classList.add('downloading');
        actions.style.display = 'block';
        info.innerHTML = `<strong>Downloading update...</strong> ${status.Downloading.progress_percent}%`;
        document.getElementById('apply-update-btn').style.display = 'none';
    } else if (status.ReadyToApply) {
        badge.textContent = 'Ready to Apply';
        badge.classList.add('update-available');
        actions.style.display = 'block';
        info.innerHTML = `<strong>Update ${status.ReadyToApply.version} ready to apply</strong>`;
        document.getElementById('apply-update-btn').style.display = 'inline-block';
    } else if (status.PendingRestart) {
        badge.textContent = 'Restart Required';
        badge.classList.add('pending-restart');
        actions.style.display = 'block';
        info.innerHTML = `<strong>Update ${status.PendingRestart.version} applied!</strong><br>Please restart the node to complete the update.`;
        document.getElementById('apply-update-btn').style.display = 'none';
    } else if (status.Failed) {
        badge.textContent = 'Update Failed';
        badge.classList.add('failed');
        actions.style.display = 'block';
        info.innerHTML = `<strong>Update failed:</strong> ${status.Failed.error}`;
        document.getElementById('apply-update-btn').style.display = 'none';
    } else {
        badge.textContent = 'Unknown';
        actions.style.display = 'none';
    }
}

// Check for updates
async function checkForUpdates() {
    const btn = document.getElementById('check-updates-btn');
    btn.disabled = true;
    btn.textContent = 'Checking...';

    try {
        const res = await fetch('/api/update/check', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({})
        });
        const data = await res.json();
        updateStatusDisplay(data.status);
        lastUpdateStatus = data.status;
    } catch (err) {
        console.error('Failed to check for updates:', err);
        alert('Failed to check for updates: ' + err.message);
    } finally {
        btn.disabled = false;
        btn.textContent = 'Check for Updates';
    }
}

// Apply update
async function applyUpdate() {
    if (!confirm('Apply the update now? The node will need to restart to complete the update.')) {
        return;
    }

    const btn = document.getElementById('apply-update-btn');
    btn.disabled = true;
    btn.textContent = 'Applying...';

    try {
        const res = await fetch('/api/update/apply', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({})
        });
        const data = await res.json();
        updateStatusDisplay(data.status);
    } catch (err) {
        console.error('Failed to apply update:', err);
        alert('Failed to apply update: ' + err.message);
    } finally {
        btn.disabled = false;
        btn.textContent = 'Apply Update';
    }
}

// Rollback
async function rollbackUpdate() {
    if (!confirm('Rollback to the previous version? The node will need to restart.')) {
        return;
    }

    try {
        const res = await fetch('/api/update/rollback', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({})
        });
        const data = await res.json();
        updateStatusDisplay(data.status);
    } catch (err) {
        console.error('Failed to rollback:', err);
        alert('Failed to rollback: ' + err.message);
    }
}

// Initialize
document.addEventListener('DOMContentLoaded', () => {
    // Fetch nodes first to populate selector
    fetchNodes();

    // Then fetch all dashboard data
    fetchStatus();
    fetchMetrics();
    fetchPeers();
    fetchPairingRequests();
    fetchUpdateStatus();
    fetchNetworkStatus();
    fetchConnectedApps();

    // Refresh periodically
    setInterval(fetchStatus, 30000);
    setInterval(fetchMetrics, 5000);
    setInterval(fetchPeers, 30000);
    setInterval(fetchPairingRequests, 10000);
    setInterval(fetchUpdateStatus, 60000);
    setInterval(fetchNetworkStatus, 10000);
    setInterval(fetchConnectedApps, 15000);
    setInterval(fetchNodes, 30000); // Refresh node list every 30s

    // Node selector
    document.getElementById('node-selector-btn').addEventListener('click', toggleNodeDropdown);
    document.getElementById('return-local-btn').addEventListener('click', returnToLocal);

    // Scan button
    document.getElementById('scan-btn').addEventListener('click', scanNetwork);

    // Update buttons
    document.getElementById('check-updates-btn').addEventListener('click', checkForUpdates);
    document.getElementById('apply-update-btn').addEventListener('click', applyUpdate);
    document.getElementById('rollback-btn').addEventListener('click', rollbackUpdate);
});
