// Elohim Node Setup Wizard JavaScript

let selectedMode = null;
let discoveredPeers = [];

// Check for discovered peers on load
async function checkForPeers() {
    try {
        const res = await fetch('/api/discovery/peers');
        discoveredPeers = await res.json();

        if (discoveredPeers.length > 0) {
            const alert = document.getElementById('peers-alert');
            const text = document.getElementById('peers-alert-text');

            const nodeCount = discoveredPeers.filter(p => p.node_type === 'Node').length;
            const appCount = discoveredPeers.filter(p => p.node_type === 'App').length;
            const doorwayCount = discoveredPeers.filter(p => p.node_type === 'Doorway').length;

            let parts = [];
            if (nodeCount > 0) parts.push(`${nodeCount} node(s)`);
            if (appCount > 0) parts.push(`${appCount} app(s)`);
            if (doorwayCount > 0) parts.push(`${doorwayCount} doorway(s)`);

            text.textContent = `Found ${parts.join(', ')} on your local network.`;
            alert.style.display = 'block';
        }
    } catch (err) {
        console.error('Failed to check for peers:', err);
    }
}

// Show peers modal
function showPeersModal() {
    const modal = document.getElementById('peers-modal');
    const list = document.getElementById('modal-peers-list');

    if (discoveredPeers.length === 0) {
        list.innerHTML = '<p class="empty-state">No peers discovered</p>';
    } else {
        list.innerHTML = discoveredPeers.map(peer => `
            <div class="peer-item">
                <div class="peer-info">
                    <span class="peer-name">${peer.hostname || peer.peer_id}</span>
                    <span class="peer-details">
                        ${peer.addresses.join(', ')}
                        ${peer.mac_address ? '<br>MAC: ' + peer.mac_address : ''}
                    </span>
                </div>
                <span class="peer-type ${peer.node_type.toLowerCase()}">${peer.node_type}</span>
            </div>
        `).join('');
    }

    modal.style.display = 'flex';
}

// Select mode
function selectMode(mode) {
    selectedMode = mode;

    document.querySelectorAll('.mode-card').forEach(card => {
        card.classList.remove('selected');
    });
    document.querySelector(`[data-mode="${mode}"]`).classList.add('selected');

    document.getElementById('mode-selection').style.display = 'none';

    if (mode === 'join') {
        document.getElementById('join-form').style.display = 'block';
        document.getElementById('doorway-form').style.display = 'none';
    } else {
        document.getElementById('join-form').style.display = 'none';
        document.getElementById('doorway-form').style.display = 'block';
    }
}

// Go back to mode selection
function goBack() {
    selectedMode = null;
    document.getElementById('mode-selection').style.display = 'grid';
    document.getElementById('join-form').style.display = 'none';
    document.getElementById('doorway-form').style.display = 'none';
    document.querySelectorAll('.mode-card').forEach(card => {
        card.classList.remove('selected');
    });
}

// Handle DNS provider change
function onDnsProviderChange(provider) {
    // Hide all DNS fields
    document.querySelectorAll('.dns-fields').forEach(el => {
        el.style.display = 'none';
    });

    // Show relevant fields
    const fieldMap = {
        'cloudflare': 'cloudflare-fields',
        'duckdns': 'duckdns-fields',
        'noip': 'noip-fields',
        'ddclient': 'ddclient-fields'
    };

    if (fieldMap[provider]) {
        document.getElementById(fieldMap[provider]).style.display = 'block';
    }
}

// Handle HTTPS toggle
function onHttpsToggle(enabled) {
    document.getElementById('email-group').style.display = enabled ? 'block' : 'none';
}

// Show progress
function showProgress(title, message) {
    document.getElementById('join-form').style.display = 'none';
    document.getElementById('doorway-form').style.display = 'none';
    document.getElementById('setup-progress').style.display = 'block';
    document.getElementById('progress-title').textContent = title;
    document.getElementById('progress-message').textContent = message;
}

// Show result
function showResult(success, title, message, details) {
    document.getElementById('setup-progress').style.display = 'none';
    document.getElementById('setup-result').style.display = 'block';

    const icon = document.getElementById('result-icon');
    icon.className = 'result-icon ' + (success ? 'success' : 'error');
    icon.textContent = success ? '✓' : '✗';

    document.getElementById('result-title').textContent = title;
    document.getElementById('result-message').textContent = message;

    const detailsEl = document.getElementById('result-details');
    if (details) {
        detailsEl.style.display = 'block';
        detailsEl.innerHTML = Object.entries(details)
            .map(([k, v]) => `<div><strong>${k}:</strong> ${v}</div>`)
            .join('');
    } else {
        detailsEl.style.display = 'none';
    }

    const btn = document.getElementById('result-action');
    if (success) {
        btn.textContent = 'Go to Dashboard';
        btn.onclick = () => window.location.href = '/';
    } else {
        btn.textContent = 'Try Again';
        btn.onclick = () => {
            document.getElementById('setup-result').style.display = 'none';
            goBack();
        };
    }
}

// Submit join form
async function submitJoin(e) {
    e.preventDefault();

    const formData = new FormData(e.target);
    const data = {
        join_key: formData.get('join_key'),
        doorway_url: formData.get('doorway_url'),
        cluster_name: formData.get('cluster_name') || null
    };

    showProgress('Joining Network', 'Connecting to doorway and joining cluster...');

    try {
        const res = await fetch('/api/setup/join', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(data)
        });

        const result = await res.json();

        if (result.success) {
            showResult(true, 'Successfully Joined!', result.message, result.details ? {
                'Node ID': result.details.node_id,
                'Cluster': result.details.cluster_name,
                'Role': result.details.cluster_role,
                'Doorway': result.details.doorway_url
            } : null);
        } else {
            showResult(false, 'Setup Failed', result.message, null);
        }
    } catch (err) {
        showResult(false, 'Connection Error', 'Failed to connect to server: ' + err.message, null);
    }
}

// Submit doorway form
async function submitDoorway(e) {
    e.preventDefault();

    const formData = new FormData(e.target);
    const provider = formData.get('dns_provider');

    let dns_provider;
    switch (provider) {
        case 'cloudflare':
            dns_provider = {
                provider: 'Cloudflare',
                api_token: formData.get('cf_api_token'),
                zone_id: formData.get('cf_zone_id')
            };
            break;
        case 'duckdns':
            dns_provider = {
                provider: 'DuckDns',
                token: formData.get('duck_token'),
                domain: formData.get('duck_domain')
            };
            break;
        case 'noip':
            dns_provider = {
                provider: 'NoIp',
                username: formData.get('noip_username'),
                password: formData.get('noip_password'),
                hostname: formData.get('noip_hostname')
            };
            break;
        case 'ddclient':
            dns_provider = {
                provider: 'Ddclient',
                config: formData.get('ddclient_config')
            };
            break;
        default:
            dns_provider = { provider: 'None' };
    }

    const data = {
        hostname: formData.get('hostname'),
        dns_provider,
        enable_https: formData.get('enable_https') === 'on',
        admin_email: formData.get('admin_email') || null
    };

    showProgress('Configuring Doorway', 'Setting up DNS, certificates, and services...');

    try {
        const res = await fetch('/api/setup/doorway', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(data)
        });

        const result = await res.json();

        if (result.success) {
            showResult(true, 'Doorway Configured!', result.message, result.details ? {
                'Node ID': result.details.node_id,
                'Cluster': result.details.cluster_name,
                'Role': result.details.cluster_role,
                'Doorway URL': result.details.doorway_url
            } : null);
        } else {
            showResult(false, 'Setup Failed', result.message, null);
        }
    } catch (err) {
        showResult(false, 'Connection Error', 'Failed to connect to server: ' + err.message, null);
    }
}

// Initialize
document.addEventListener('DOMContentLoaded', () => {
    checkForPeers();

    // Mode selection
    document.querySelectorAll('.mode-card').forEach(card => {
        card.addEventListener('click', () => {
            selectMode(card.dataset.mode);
        });
    });

    // Back buttons
    document.getElementById('join-back').addEventListener('click', goBack);
    document.getElementById('doorway-back').addEventListener('click', goBack);

    // View peers button
    document.getElementById('view-peers-btn').addEventListener('click', showPeersModal);

    // Close modal
    document.getElementById('close-modal').addEventListener('click', () => {
        document.getElementById('peers-modal').style.display = 'none';
    });

    // DNS provider change
    document.getElementById('dns-provider').addEventListener('change', (e) => {
        onDnsProviderChange(e.target.value);
    });

    // HTTPS toggle
    document.getElementById('enable-https').addEventListener('change', (e) => {
        onHttpsToggle(e.target.checked);
    });

    // Form submissions
    document.getElementById('join-form').addEventListener('submit', submitJoin);
    document.getElementById('doorway-form').addEventListener('submit', submitDoorway);

    // Click outside modal to close
    document.getElementById('peers-modal').addEventListener('click', (e) => {
        if (e.target.id === 'peers-modal') {
            e.target.style.display = 'none';
        }
    });
});
