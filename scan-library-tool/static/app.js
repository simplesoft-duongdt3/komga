/* Komga Smart Scanner — SPA logic */
const API = {
  version: '/api/version',
  libraries: '/api/libraries',
  scan: '/api/scan',
  curl: '/api/curl',
  execute: '/api/execute',
  listRequests: '/api/list-requests',
  exports: '/api/exports',
  hashCache: '/api/hash-cache',
};

const state = {
  step: 1,
  libraries: [],
  selectedLibrary: null,
  scanResult: null,
  curlResult: null,
  requestId: null,
  curlEnabled: {},
  hashPanel: null,
  hashCacheInfo: {},
  hashGenerating: false,
  hashLog: [],
  hashStats: {},
};

/* ── Helpers ───────────────────────────────────────────── */

function apiUrl(path) { return API.exports + '/' + path; }

async function fetchJSON(url, opts = {}) {
  const resp = await fetch(url, {
    headers: { 'Content-Type': 'application/json', ...opts.headers },
    ...opts,
  });
  if (!resp.ok) {
    const body = await resp.text();
    throw new Error(`HTTP ${resp.status}: ${body.slice(0, 200)}`);
  }
  if (opts.raw) return resp;
  return resp.json();
}

function $(sel) { return document.querySelector(sel); }

function esc(str) {
  const d = document.createElement('div');
  d.textContent = str;
  return d.innerHTML;
}

/* ── Theme ─────────────────────────────────────────────── */

function toggleTheme() {
  const html = document.documentElement;
  html.classList.toggle('dark');
  localStorage.setItem('theme', html.classList.contains('dark') ? 'dark' : 'light');
}

/* ── Render ────────────────────────────────────────────── */

function render() {
  renderStepIndicator();
  state.step === 1 && renderStep1();
  state.step === 2 && renderStep2();
  state.step === 3 && renderStep3();
  state.step === 4 && renderStep4();
}

function renderStepIndicator() {
  const labels = ['Choose Library', 'Scan', 'Curls', 'Execute'];
  const steps = [1, 2, 3, 4];
  const html = steps.map(i => {
    let cls = 'opacity-30', icon = '○', label = labels[i - 1];
    if (i === state.step) { cls = 'text-blue-500 font-semibold'; icon = '●'; }
    if (i < state.step) { cls = 'text-green-500 font-semibold'; icon = '✓'; }
    const arrow = i < 4 ? '<span class="mx-1 opacity-30">→</span>' : '';
    return `<span class="${cls}">${icon} ${label}</span>${arrow}`;
  }).join('');
  document.getElementById('stepIndicator').innerHTML = html;
}

function showError(msg) {
  const c = document.getElementById('stepContent');
  c.innerHTML = `<div class="bg-red-100 dark:bg-red-900 border border-red-400 text-red-700 dark:text-red-200 px-4 py-3 rounded">${esc(msg)}</div>`;
}

function btn(label, onClick, cls = '') {
  return `<button onclick="${onClick}" class="px-4 py-2 rounded font-semibold text-sm ${cls}">${label}</button>`;
}

function spinner() { return '<span class="inline-block animate-spin">⟳</span>'; }

/* ── Step 1: Choose Library ─────────────────────────────── */

async function renderStep1() {
  const c = document.getElementById('stepContent');
  c.innerHTML = '<h2 class="text-lg font-semibold mb-4">Step 1: Choose a Library</h2><p class="text-gray-500 mb-4">Loading libraries...</p>';

  try {
    state.libraries = await fetchJSON(API.libraries);
  } catch (e) {
    showError('Failed to load libraries: ' + e.message);
    return;
  }

  if (state.libraries.length === 0) {
    c.innerHTML = '<p class="text-gray-500">No libraries found.</p>';
    return;
  }

  const cards = state.libraries.map(l => `
    <div onclick="selectLibrary('${esc(l.id)}')"
         class="cursor-pointer border rounded-lg p-4 hover:border-blue-500 hover:shadow transition bg-white dark:bg-gray-800 border-gray-200 dark:border-gray-700">
      <div class="font-semibold">${esc(l.name)}</div>
      <div class="text-xs text-gray-400 mt-1">${esc(l.id)}</div>
      <div class="text-xs text-gray-400 truncate">${esc(l.root || '(no root)')}</div>
      <button onclick="event.stopPropagation(); openHashPanel('${esc(l.id)}')"
              class="mt-2 px-3 py-1 text-xs rounded bg-purple-100 dark:bg-purple-900 text-purple-700 dark:text-purple-300 hover:bg-purple-200 dark:hover:bg-purple-800">
        ⚡ Hash Cache
      </button>
    </div>
  `).join('');

  c.innerHTML = `
    <h2 class="text-lg font-semibold mb-4">Step 1: Choose a Library</h2>
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">${cards}</div>
  `;
}

function selectLibrary(id) {
  state.selectedLibrary = state.libraries.find(l => l.id === id);
  state.scanResult = null;
  state.curlResult = null;
  state.requestId = null;
  state.step = 2;
  render();
}

/* ── Hash Cache Panel ──────────────────────────────────── */

function openHashPanel(libraryId) {
  state.hashPanel = state.hashPanel === libraryId ? null : libraryId;
  if (state.hashPanel) {
    state.hashCacheInfo[libraryId] = null;
    state.hashLog = [];
    state.hashStats = {};
    renderHashPanel(libraryId);
    loadHashCacheStatus(libraryId);
  } else {
    closeHashPanel();
  }
}

function closeHashPanel() {
  const el = document.getElementById('hashPanel');
  if (el) el.remove();
  state.hashPanel = null;
}

async function loadHashCacheStatus(libraryId) {
  try {
    const info = await fetchJSON(`${API.hashCache}/${libraryId}`);
    state.hashCacheInfo[libraryId] = info;
    renderHashPanel(libraryId);
  } catch (e) {
    state.hashCacheInfo[libraryId] = { error: e.message };
    renderHashPanel(libraryId);
  }
}

function renderHashPanel(libraryId) {
  const lib = state.libraries.find(l => l.id === libraryId);
  if (!lib) return;

  let existing = document.getElementById('hashPanel');
  if (!existing) {
    existing = document.createElement('div');
    existing.id = 'hashPanel';
    document.getElementById('stepContent').appendChild(existing);
  }

  const info = state.hashCacheInfo[libraryId];
  const generating = state.hashGenerating;
  const logLines = state.hashLog;
  const stats = state.hashStats;

  let statusHtml = '';
  let actionsHtml = '';

  if (!info) {
    statusHtml = '<div class="text-gray-400 text-sm">Loading...</div>';
  } else if (info.error) {
    statusHtml = `<div class="text-red-400 text-sm">Error: ${esc(info.error)}</div>`;
  } else if (info.exists) {
    const age = info.generated_at ? new Date(info.generated_at).toLocaleString() : 'unknown';
    const size = info.file_size_bytes ? fmtBytes(info.file_size_bytes) : 'unknown';
    const files = info.total_files != null ? info.total_files.toLocaleString() : '?';
    statusHtml = `
      <div class="text-xs space-y-1">
        <div><span class="text-gray-400">Status:</span> <span class="text-green-400">✅ Cached</span></div>
        <div><span class="text-gray-400">Generated:</span> ${esc(age)}</div>
        <div><span class="text-gray-400">Files:</span> ${files}</div>
        <div><span class="text-gray-400">Cache file:</span> ${size}</div>
      </div>`;
    actionsHtml = `
      <div class="flex gap-2 mt-2">
        <button onclick="generateHashCache('${esc(libraryId)}')" ${generating ? 'disabled' : ''}
                class="px-3 py-1 text-xs rounded font-semibold ${generating ? 'bg-gray-400 cursor-not-allowed' : 'bg-orange-500 hover:bg-orange-600 text-white'}">
          ${generating ? spinner() + ' Generating...' : '🔄 Regenerate Cache'}
        </button>
        <a href="${API.hashCache}/${libraryId}/download" target="_blank"
           class="px-3 py-1 text-xs rounded font-semibold bg-blue-600 hover:bg-blue-700 text-white inline-block">📥 Download Cache</a>
      </div>`;
  } else {
    statusHtml = `
      <div class="text-xs space-y-1">
        <div><span class="text-gray-400">Status:</span> <span class="text-yellow-400">⚠️ No cache</span></div>
        <div class="text-gray-400">Generate one to speed up future scans.</div>
      </div>`;
    actionsHtml = `
      <div class="flex gap-2 mt-2">
        <button onclick="generateHashCache('${esc(libraryId)}')" ${generating ? 'disabled' : ''}
                class="px-3 py-1 text-xs rounded font-semibold ${generating ? 'bg-gray-400 cursor-not-allowed' : 'bg-purple-600 hover:bg-purple-700 text-white'}">
          ${generating ? spinner() + ' Generating...' : '⚡ Generate Cache'}
        </button>
      </div>`;
  }

  const progressHtml = `
    <div class="mt-2 bg-gray-900 text-green-400 text-xs font-mono p-2 rounded max-h-40 overflow-y-auto ${logLines.length === 0 && !generating ? 'hidden' : ''}" id="hashLog">
      ${logLines.map(l => esc(l)).join('\n')}
    </div>`;

  const statHtml = Object.keys(stats).length > 0 ? `
    <div class="mt-2 text-xs grid grid-cols-2 gap-1" id="hashStatGrid">
      ${Object.entries(stats).map(([k, v]) => `<div><span class="text-gray-400">${esc(k.replace(/_/g, ' '))}:</span> <span class="stat-value">${_renderStatValue(v)}</span></div>`).join('')}
    </div>` : '<div class="mt-2 text-xs grid grid-cols-2 gap-1 hidden" id="hashStatGrid"></div>';

  existing.innerHTML = `
    <div class="border border-purple-300 dark:border-purple-700 rounded-lg p-4 mt-3 bg-purple-50 dark:bg-purple-900/30">
      <div class="flex items-center justify-between mb-2">
        <div class="font-semibold text-sm">⚡ Hash Cache — ${esc(lib.name)}</div>
        <button onclick="closeHashPanel()" class="text-gray-400 hover:text-gray-200 text-sm">✕</button>
      </div>
      ${statusHtml}
      ${statHtml}
      ${actionsHtml}
      ${progressHtml}
    </div>
  `;

  if (logLines.length > 0) {
    const logEl = document.getElementById('hashLog');
    if (logEl) logEl.scrollTop = logEl.scrollHeight;
  }
}

function appendHashLog(line) {
  state.hashLog.push(line);
  if (state.hashLog.length > 50) {
    state.hashLog = state.hashLog.slice(-50);
  }
  const logEl = document.getElementById('hashLog');
  if (logEl) {
    logEl.classList.remove('hidden');
    logEl.insertAdjacentHTML('beforeend', esc(line) + '\n');
    logEl.scrollTop = logEl.scrollHeight;
  }
}

function _renderStatValue(v) {
  if (typeof v === 'object' && v !== null) {
    return Object.entries(v).map(([k, val]) => `${esc(k)}: ${esc(String(val))}`).join(', ');
  }
  return esc(String(v));
}

function updateHashStat(key, value) {
  state.hashStats[key] = value;
  const grid = document.getElementById('hashStatGrid');
  if (!grid) return;
  grid.classList.remove('hidden');
  const attrVal = 'stat-' + key.replace(/[^a-zA-Z0-9_-]/g, '_');
  let row = grid.querySelector(`[data-skey="${attrVal}"]`);
  if (row) {
    const valEl = row.querySelector('.sv');
    if (valEl) valEl.textContent = _renderStatValue(value);
  } else {
    const label = key.replace(/_/g, ' ');
    const display = _renderStatValue(value);
    grid.insertAdjacentHTML('beforeend',
      `<div><span class="text-gray-400">${esc(label)}:</span> <span class="sv" data-skey="${attrVal}">${display}</span></div>`);
  }
}

async function generateHashCache(libraryId) {
  if (state.hashGenerating) return;
  state.hashGenerating = true;
  state.hashLog = [];
  state.hashStats = {};
  renderHashPanel(libraryId);

  try {
    const resp = await fetch(`${API.hashCache}/${libraryId}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({}),
    });

    if (!resp.ok) {
      const body = await resp.text();
      throw new Error(`HTTP ${resp.status}: ${body.slice(0, 200)}`);
    }

    const reader = resp.body.getReader();
    const decoder = new TextDecoder();
    let buf = '';

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buf += decoder.decode(value, { stream: true });
      const lines = buf.split('\n');
      buf = lines.pop() || '';

      for (const line of lines) {
        if (line.startsWith(':') && line.includes('heartbeat')) continue;
        if (line.startsWith('data: ')) {
          try {
            const evt = JSON.parse(line.slice(6));
            handleHashEvent(evt, libraryId);
          } catch (_) {}
        }
      }
    }
  } catch (e) {
    appendHashLog(`ERROR: ${e.message}`);
  } finally {
    state.hashGenerating = false;
    renderHashPanel(libraryId);
  }
}

function handleHashEvent(evt, libraryId) {
  switch (evt.event) {
    case 'phase':
      appendHashLog(`── Phase: ${evt.phase} ──`);
      break;
    case 'progress':
      appendHashLog(evt.line);
      break;
    case 'stat':
      updateHashStat(evt.key, evt.value);
      break;
    case 'done':
      if (evt.success) {
        if (evt.file_size_bytes || evt.generated_at) {
          const cur = state.hashCacheInfo[libraryId] || {};
          state.hashCacheInfo[libraryId] = Object.assign(cur, {
            exists: true,
            file_size_bytes: evt.file_size_bytes || cur.file_size_bytes,
            generated_at: evt.generated_at || cur.generated_at,
          });
        }
        appendHashLog('✅ Hash cache generation complete.');
      } else {
        appendHashLog(`❌ Failed: ${evt.error || 'Unknown error'}`);
      }
      break;
  }
}

function fmtBytes(bytes) {
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1048576) return (bytes / 1024).toFixed(1) + ' KB';
  if (bytes < 1073741824) return (bytes / 1048576).toFixed(1) + ' MB';
  return (bytes / 1073741824).toFixed(2) + ' GB';
}

/* ── Step 2: Scan ──────────────────────────────────────── */

function renderStep2() {
  const lib = state.selectedLibrary;
  if (!lib) { state.step = 1; render(); return; }

  const c = document.getElementById('stepContent');
  c.innerHTML = `
    <h2 class="text-lg font-semibold mb-2">Step 2: Scan Library</h2>
    <div class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-4 mb-4">
      <div><span class="text-gray-400 text-sm">Library:</span> <span class="font-semibold">${esc(lib.name)}</span></div>
      <div><span class="text-gray-400 text-sm">ID:</span> <span class="text-xs">${esc(lib.id)}</span></div>
      <div><span class="text-gray-400 text-sm">Root:</span> <span class="text-xs">${esc(lib.root || '')}</span></div>
    </div>
    <label class="flex items-center gap-2 mb-4 cursor-pointer">
      <input type="checkbox" id="hashToggle" checked class="w-4 h-4">
      <span class="text-sm">🔐 Compute file hashes (SHA-256) — slower, more accurate</span>
    </label>
    <button onclick="runScan()" class="px-5 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded font-semibold text-sm">Run Scan</button>
    <div id="scanResults" class="mt-4"></div>
  `;
}

async function runScan() {
  const btn = document.querySelector('#stepContent button');
  const out = document.getElementById('scanResults');
  const hash = document.getElementById('hashToggle')?.checked ?? true;

  btn.disabled = true;
  btn.textContent = 'Scanning...';

  try {
    state.scanResult = await fetchJSON(API.scan, {
      method: 'POST',
      body: JSON.stringify({ library_id: state.selectedLibrary.id, hash_files: hash }),
    });
    state.requestId = state.scanResult.request_id;
    renderScanResults();
  } catch (e) {
    out.innerHTML = `<div class="bg-red-100 dark:bg-red-900 border border-red-400 text-red-700 dark:text-red-200 px-4 py-2 rounded mt-2">${esc(e.message)}</div>`;
  } finally {
    btn.disabled = false;
    btn.textContent = 'Run Scan';
  }
}

function renderScanResults() {
  const r = state.scanResult;
  const d = r.diff;
  const out = document.getElementById('scanResults');

  const hasChanges = d.new_series + d.deleted_series + d.new_books + d.deleted_books + d.changed_books + d.pending_hash + d.to_be_analyzed + d.no_metadata > 0;

  // Build timing table
  const perf = r.perf || {};
  const phases = perf.phases || [];
  const totalMs = perf.total_ms || 0;
  const timingRows = phases.map(p => {
    const ms = p.elapsed_ms;
    const pct = totalMs > 0 ? (ms / totalMs * 100) : 0;
    const icon = pct > 50 ? ' 🐌' : '';
    const cls = pct > 50 ? 'text-red-400' : (pct > 20 ? 'text-yellow-400' : '');
    return `<tr class="${cls}"><td class="py-1 text-xs">${esc(p.phase)}</td><td class="py-1 text-right text-xs">${timeFmt(ms)}</td><td class="py-1 text-right text-xs">${pct.toFixed(0)}%${icon}</td></tr>`;
  }).join('');

  out.innerHTML = `
    <div class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-4 mt-2">
      <div class="text-sm mb-2"><span class="text-gray-400">Request ID:</span> <code class="bg-gray-100 dark:bg-gray-700 px-1 rounded">${esc(r.request_id)}</code></div>
      <div class="grid grid-cols-2 md:grid-cols-4 gap-3 mb-3 text-sm">
        <div><span class="text-gray-400">DB series</span><br><span class="text-lg font-semibold">${r.db.series}</span></div>
        <div><span class="text-gray-400">DB books</span><br><span class="text-lg font-semibold">${r.db.books}</span></div>
        <div><span class="text-gray-400">FS series</span><br><span class="text-lg font-semibold">${r.fs.series}</span></div>
        <div><span class="text-gray-400">FS files</span><br><span class="text-lg font-semibold">${r.fs.files}</span></div>
      </div>
      <table class="w-full text-sm mb-3">
        <thead><tr class="border-b border-gray-200 dark:border-gray-700">
          <th class="text-left py-1">Category</th><th class="text-right py-1">Count</th>
        </tr></thead>
        <tbody>
          ${row('New series', d.new_series, d.new_series > 0 ? 'text-green-500' : '')}
          ${row('Deleted series', d.deleted_series, d.deleted_series > 0 ? 'text-red-500' : '')}
          ${row('New books', d.new_books, d.new_books > 0 ? 'text-green-500' : '')}
          ${row('Deleted books', d.deleted_books, d.deleted_books > 0 ? 'text-red-500' : '')}
          ${row('Changed books', d.changed_books, d.changed_books > 0 ? 'text-yellow-500' : '')}
          ${row('Hash book (needs hash)', d.pending_hash, d.pending_hash > 0 ? 'text-purple-500' : '')}
          ${row('To be analyzed', d.to_be_analyzed, d.to_be_analyzed > 0 ? 'text-orange-500' : '')}
          ${row('No metadata/thumbnail', d.no_metadata, d.no_metadata > 0 ? 'text-orange-500' : '')}
        </tbody>
        <tfoot><tr class="border-t border-gray-300 dark:border-gray-600 font-semibold">
          <td class="py-1">Total</td><td class="text-right py-1">${r.total_actions}</td>
        </tr></tfoot>
      </table>
      ${timingRows ? `
      <details class="mb-2">
        <summary class="text-xs text-gray-400 cursor-pointer hover:text-gray-200">⏱ Scan Timing (${timeFmt(totalMs)} total)</summary>
        <table class="w-full text-xs mt-1">
          <thead><tr class="border-b border-gray-200 dark:border-gray-700">
            <th class="text-left py-1">Phase</th><th class="text-right py-1">Time</th><th class="text-right py-1">%</th>
          </tr></thead>
          <tbody>${timingRows}</tbody>
          <tfoot><tr class="border-t border-gray-300 dark:border-gray-600 font-semibold">
            <td class="py-1">Total</td><td class="text-right py-1">${timeFmt(totalMs)}</td><td class="text-right py-1">100%</td>
          </tr></tfoot>
        </table>
      </details>` : ''}
      <div class="flex flex-wrap gap-2 items-center text-sm">
        <span class="text-gray-400">📁 Download:</span>
        <a href="${apiUrl(r.request_id + '/' + r.request_id + '_db.json')}" target="_blank" class="text-blue-500 hover:underline">DB JSON</a>
        <a href="${apiUrl(r.request_id + '/' + r.request_id + '_fs.json')}" target="_blank" class="text-blue-500 hover:underline">FS JSON</a>
        <a href="${apiUrl(r.request_id + '/' + r.request_id + '_diff.json')}" target="_blank" class="text-blue-500 hover:underline">Diff JSON</a>
        <a href="${apiUrl(r.request_id + '/' + r.request_id + '_perf.json')}" target="_blank" class="text-blue-500 hover:underline">Perf JSON</a>
      </div>
      <div class="mt-4">${btn('→ Generate Curls', 'state.step=3;render()', hasChanges ? 'bg-green-600 hover:bg-green-700 text-white' : 'bg-gray-400 text-white cursor-not-allowed')}</div>
    </div>
  `;
  function row(label, count, cls) {
    return `<tr><td class="py-1">${label}</td><td class="text-right py-1 ${cls}">${count}</td></tr>`;
  }
}

/* ── Time formatting helper ─────────────────────────── */

function timeFmt(ms) {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${Math.floor(ms / 60000)}m ${Math.round((ms % 60000) / 1000)}s`;
}

/* ── Step 3: Generate Curls ────────────────────────────── */

function renderStep3() {
  const r = state.scanResult;
  if (!r || !state.requestId) { state.step = 1; render(); return; }

  const c = document.getElementById('stepContent');

  const cats = [
    {key: 'new_series', label: 'New series', on: r.has_new_series},
    {key: 'deleted_series', label: 'Deleted series', on: r.has_deleted_series},
    {key: 'new_books', label: 'New books', on: r.has_new_books},
    {key: 'deleted_books', label: 'Deleted books', on: r.has_deleted_books},
    {key: 'changed_books', label: 'Changed books', on: r.has_changed_books},
    {key: 'pending_hash', label: 'Hash book (update DB hash via analyze)', on: r.has_pending_hash},
    {key: 'to_be_analyzed', label: 'Analyze books (MEDIA=UNKNOWN)', on: r.has_to_be_analyzed},
    {key: 'no_metadata', label: 'Refresh metadata (no thumbnail)', on: r.has_no_metadata},
  ];

  const checkboxes = cats.map(c => `
    <label class="flex items-center gap-2 cursor-pointer ${c.on ? '' : 'opacity-40'}">
      <input type="checkbox" class="catCheck w-4 h-4" value="${c.key}" ${c.on ? 'checked' : 'disabled'}>
      <span class="text-sm">${c.label}</span>
    </label>
  `).join('');

  c.innerHTML = `
    <h2 class="text-lg font-semibold mb-2">Step 3: Generate Curl Scripts</h2>
    <div class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-4 mb-4">
      <div class="text-sm mb-2"><span class="text-gray-400">Request:</span> <code class="bg-gray-100 dark:bg-gray-700 px-1 rounded">${esc(state.requestId)}</code></div>
      <div class="text-sm mb-3"><span class="text-gray-400">Categories to include:</span></div>
      <div class="grid grid-cols-1 md:grid-cols-2 gap-1 mb-3">${checkboxes}</div>
      <label class="flex items-center gap-2 mb-1 cursor-pointer">
        <input type="checkbox" id="curlAnalyze" checked class="w-4 h-4">
        <span class="text-sm">📖 Analyze books (post-create + changed)</span>
      </label>
      <label class="flex items-center gap-2 mb-3 cursor-pointer">
        <input type="checkbox" id="curlRefresh" checked class="w-4 h-4">
        <span class="text-sm">🔄 Refresh metadata (books + series)</span>
      </label>
      <button onclick="generateCurls()" class="px-5 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded font-semibold text-sm">Generate</button>
    </div>
    <div id="curlResults"></div>
    <div class="mt-4 flex gap-2">
      ${btn('← Back to Scan', 'state.step=2;render()', 'bg-gray-500 hover:bg-gray-600 text-white')}
    </div>
  `;
}

async function generateCurls() {
  const out = document.getElementById('curlResults');
  const checks = [...document.querySelectorAll('.catCheck:checked')].map(c => c.value);
  if (checks.length === 0) {
    out.innerHTML = '<div class="text-yellow-500 text-sm">Select at least one category.</div>';
    return;
  }

  const btnEl = document.querySelector('#stepContent button');
  btnEl.disabled = true; btnEl.textContent = 'Generating...';

  try {
    state.curlResult = await fetchJSON(API.curl, {
      method: 'POST',
      body: JSON.stringify({
        request_id: state.requestId,
        categories: checks,
        analyze: document.getElementById('curlAnalyze')?.checked ?? true,
        refresh: document.getElementById('curlRefresh')?.checked ?? true,
      }),
    });
    renderCurlResults();
  } catch (e) {
    out.innerHTML = `<div class="bg-red-100 dark:bg-red-900 border border-red-400 text-red-700 dark:text-red-200 px-4 py-2 rounded">${esc(e.message)}</div>`;
  } finally {
    btnEl.disabled = false; btnEl.textContent = 'Generate';
  }
}

function renderCurlResults() {
  const out = document.getElementById('curlResults');
  const scripts = state.curlResult.scripts;
  const entries = Object.entries(scripts);

  if (entries.length === 0) {
    out.innerHTML = '<div class="text-gray-500 text-sm">No scripts generated.</div>';
    return;
  }

  const rows = entries.map(([cat, path]) => {
    const scopedScriptName = cat;  // actually we need the filename
  
    // We have full path like /exports/20260527-131500/20260527-131500_new_series.sh
    // Extract relative: request_id/filename
    const parts = path.split('/');
    const filename = parts.pop();
    const folderName = parts.pop();
    const downloadUrl = apiUrl(folderName + '/' + filename);
    const scriptName = cat; // use category as key for execute

    return `
      <tr>
        <td class="py-2 text-sm font-medium">${esc(cat)}</td>
        <td class="py-2"><code class="text-xs bg-gray-100 dark:bg-gray-700 px-1 rounded">${esc(filename)}</code></td>
        <td class="py-2 flex gap-2">
          <a href="${downloadUrl}" target="_blank" class="text-blue-500 hover:underline text-sm">📥 Download</a>
          <button onclick="executeScript('${scriptName}')" class="text-green-500 hover:underline text-sm">▶ Execute</button>
        </td>
      </tr>
    `;
  }).join('');

  out.innerHTML = `
    <div class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-4">
      <table class="w-full text-sm">
        <thead><tr class="border-b border-gray-200 dark:border-gray-700">
          <th class="text-left py-1">Category</th><th class="text-left py-1">File</th><th class="text-left py-1">Actions</th>
        </tr></thead>
        <tbody>${rows}</tbody>
      </table>
      <div class="mt-4">${btn('→ Execute Step', 'state.step=4;render()', 'bg-green-600 hover:bg-green-700 text-white')}</div>
    </div>
  `;
}

/* ── Step 4: Execute ───────────────────────────────────── */

function renderStep4() {
  const r = state.curlResult;
  if (!r || !state.requestId) { state.step = 1; render(); return; }

  const c = document.getElementById('stepContent');
  const scripts = r.scripts || {};
  const entries = Object.entries(scripts);

  const rows = entries.map(([cat]) => {
    const filename = `${state.requestId}_${cat}.sh`;
    return `
      <tr id="row-${cat}">
        <td class="py-2 text-sm">${esc(cat)}</td>
        <td class="py-2"><code class="text-xs bg-gray-100 dark:bg-gray-700 px-1 rounded">${esc(filename)}</code></td>
        <td class="py-2">
          <button onclick="executeScript('${cat}')" id="btn-${cat}" class="px-3 py-1 bg-green-600 hover:bg-green-700 text-white rounded text-xs font-semibold">Execute</button>
        </td>
        <td class="py-2" id="status-${cat}"><span class="text-gray-400 text-xs">—</span></td>
      </tr>
    `;
  }).join('');

  c.innerHTML = `
    <h2 class="text-lg font-semibold mb-2">Step 4: Execute Curl Scripts</h2>
    <div class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-4 mb-4">
      <table class="w-full text-sm">
        <thead><tr class="border-b border-gray-200 dark:border-gray-700">
          <th class="text-left py-1">Category</th><th class="text-left py-1">Script</th><th class="text-left py-1">Action</th><th class="text-left py-1">Result</th>
        </tr></thead>
        <tbody>${rows}</tbody>
      </table>
    </div>
    <div id="logArea"></div>
    <div id="dashboard" class="hidden"></div>
    <div class="mt-4">${btn('← Back to Curls', 'state.step=3;render()', 'bg-gray-500 hover:bg-gray-600 text-white')}</div>
  `;
}

async function executeScript(scriptName) {
  const btnEl = document.getElementById(`btn-${scriptName}`);
  const statusEl = document.getElementById(`status-${scriptName}`);
  const logArea = document.getElementById('logArea');

  btnEl.disabled = true;
  btnEl.textContent = 'Running...';
  statusEl.innerHTML = spinner();

  const logDiv = document.createElement('div');
  logDiv.className = 'bg-gray-900 text-green-400 text-xs font-mono p-3 rounded max-h-80 overflow-y-auto mb-4';
  logArea.prepend(logDiv);

  let logText = `Executing: ${scriptName}\n`;
  logDiv.textContent = logText;

  try {
    const resp = await fetch('/api/execute', {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({ request_id: state.requestId, script_name: scriptName }),
    });

    if (!resp.ok) throw new Error(`HTTP ${resp.status}`);

    const reader = resp.body.getReader();
    const decoder = new TextDecoder();
    let buf = '';
    let totalCalls = 0, succeeded = 0, failed = 0;

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buf += decoder.decode(value, { stream: true });
      const lines = buf.split('\n');
      buf = lines.pop() || '';

      for (const line of lines) {
        if (line.startsWith('data: ')) {
          try {
            const evt = JSON.parse(line.slice(6));
            handleExecuteEvent(evt, logDiv, statusEl);
          } catch (_) {}
        }
      }
    }
  } catch (e) {
    logDiv.textContent += `\n\nFATAL: ${e.message}`;
    statusEl.innerHTML = '<span class="text-red-500 text-xs">Error</span>';
  } finally {
    btnEl.disabled = false;
    btnEl.textContent = 'Execute';
  }

  function handleExecuteEvent(evt, logDiv, statusEl) {
    if (evt.type === 'start') {
      logDiv.textContent += `Starting — ${evt.total} commands\n`;
    } else if (evt.type === 'progress') {
      const icon = evt.success ? '✓' : '✗';
      logDiv.textContent += `[${icon}] (${evt.index}/${evt.total}) ${evt.command.slice(0, 100)}\n`;
      if (evt.error) logDiv.textContent += `  ERROR: ${evt.error}\n`;
      logDiv.scrollTop = logDiv.scrollHeight;
      statusEl.innerHTML = `<span class="text-xs">${evt.index}/${evt.total}</span>`;
    } else if (evt.type === 'done') {
      const ok = evt.succeeded || 0, fail = evt.failed || 0;
      statusEl.innerHTML = `<span class="text-green-500 text-xs">✓ ${ok}</span> ${fail > 0 ? `<span class="text-red-500 text-xs">✗ ${fail}</span>` : ''}`;
      logDiv.textContent += `\nDone. ${ok} succeeded, ${fail} failed.\n`;
      updateDashboard();
    }
  }
}

function updateDashboard() {
  const scripts = state.curlResult?.scripts || {};
  const entries = Object.entries(scripts);
  let totalOk = 0, totalFail = 0;

  for (const [cat] of entries) {
    const el = document.getElementById(`status-${cat}`);
    if (!el) continue;
    const txt = el.textContent || '';
    const okMatch = txt.match(/✓\s*(\d+)/);
    const failMatch = txt.match(/✗\s*(\d+)/);
    if (okMatch) totalOk += parseInt(okMatch[1]);
    if (failMatch) totalFail += parseInt(failMatch[1]);
  }

  const dash = document.getElementById('dashboard');
  dash.classList.remove('hidden');
  dash.innerHTML = `
    <div class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-4">
      <h3 class="font-semibold mb-2">📊 Execution Summary</h3>
      <div class="flex gap-6 text-sm">
        <div>Total calls: <span class="font-semibold">${totalOk + totalFail}</span></div>
        <div class="text-green-500">Succeeded: ${totalOk}</div>
        <div class="text-red-500">Failed: ${totalFail}</div>
      </div>
    </div>
  `;
}

/* ── Init ────────────────────────────────────────────────── */

(function initVersion() {
  fetch(API.version).then(r => r.json()).then(d => {
    document.getElementById('statusBar').textContent = `v${d.version} — ${d.title}`;
  }).catch(() => {});
})();

render();
