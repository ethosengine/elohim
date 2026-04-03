// genesis/landing/src/main.ts
import './style.css';
import { manifestoSummary, pillars } from './generated/content';

// --- Populate manifesto summary ---
const manifestoBody = document.getElementById('manifesto-body');
if (manifestoBody) {
  manifestoBody.innerHTML = manifestoSummary;
}

// --- Populate pillars grid ---
const pillarsGrid = document.getElementById('pillars-grid');
if (pillarsGrid) {
  pillarsGrid.innerHTML = pillars
    .map(
      (p) => `
    <div class="pillar-card" data-epr-ref="${p.id}">
      <span class="pillar-icon">${p.icon}</span>
      <h3 class="pillar-name">${p.name}</h3>
      <p class="pillar-desc">${p.description}</p>
    </div>
  `,
    )
    .join('');
}

// --- Fetch live stats from doorway ---
async function loadStats(): Promise<void> {
  try {
    const response = await fetch('/health/startup');
    if (!response.ok) return;

    const data = await response.json();

    const contentEl = document.getElementById('stat-content');
    const humansEl = document.getElementById('stat-humans');
    const peersEl = document.getElementById('stat-peers');

    if (contentEl && data.projection?.content != null) {
      contentEl.textContent = String(data.projection.content);
    }
    if (humansEl && data.projection?.humans != null) {
      humansEl.textContent = String(data.projection.humans);
    }
    if (peersEl && data.projection?.relationships != null) {
      peersEl.textContent = String(data.projection.relationships);
    }
  } catch {
    // Stats are progressive enhancement — page works without them
  }
}

loadStats();

// --- Footer delivery source ---
const footerSource = document.getElementById('footer-source');
if (footerSource) {
  footerSource.textContent = `Served by ${window.location.hostname}`;
}
