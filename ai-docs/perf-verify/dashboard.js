// Komga DB Live — dashboard logic
var PK = window.KOMGA_REFRESH_SEC || 5;
var timers = [];
var history = [];
var MAX_HIST = 72;

function fmtDur(ms) {
  if (ms == null) return '—';
  if (ms < 1000) return ms + 'ms';
  if (ms < 60000) return (ms / 1000).toFixed(1) + 's';
  return Math.floor(ms / 60000) + 'm ' + Math.round((ms % 60000) / 1000) + 's';
}

function fmtNum(n) {
  return n != null ? n.toLocaleString() : '—';
}

function fmtPct(n, total) {
  if (!total) return '—';
  return (n / total * 100).toFixed(1) + '%';
}

function fmtAgo(ts) {
  if (!ts || ts === 'None') return '';
  var s = (Date.now() - Date.parse(ts + 'Z')) / 1000;
  return s < 60 ? Math.floor(s) + 's ago'
       : s < 3600 ? Math.floor(s / 60) + 'm ago'
       : Math.floor(s / 3600) + 'h ago';
}

function tag(text, cls) {
  return '<span class="tag ' + cls + '">' + text + '</span>';
}

function switchTab(name) {
  document.querySelectorAll('.tab-btn').forEach(function (b) {
    b.classList.toggle('active', b.dataset.tab === name);
  });
  document.querySelectorAll('.tab-panel').forEach(function (p) {
    p.classList.toggle('active', p.id === 'tab-' + name);
  });
  document.querySelectorAll('nav a[data-tab]').forEach(function (a) {
    a.classList.toggle('active', a.dataset.tab === name);
  });
}

function updateHistory(n) {
  history.push(n);
  if (history.length > MAX_HIST) history.shift();
  var max = Math.max.apply(null, history) || 1;
  var el = document.getElementById('sparkline');
  if (!el) return;
  el.innerHTML = history.map(function (v) {
    var h = Math.round(v / max * 100);
    return '<div class="spark-bar" style="height:' + h + '%" title="' + v + ' tasks/h"></div>';
  }).join('');
}

function render(d) {
  if (d.error) {
    document.getElementById('nav-status').textContent = 'ERROR: ' + d.error;
    return;
  }
  document.getElementById('nav-status').textContent = 'Ping: ' + d.took_ms + 'ms';
  document.getElementById('stamp').textContent = (d.now || '').replace('T', ' ').slice(0, 19);
  document.getElementById('latency').textContent = 'refresh ' + PK + 's · ' + d.took_ms + 'ms query';

  // Overall health
  var warnings = 0;
  if (!d.idxs.idx_task_queue) warnings++;
  if (!d.idxs.idx_task_owner_group) warnings++;
  if (d.blocked > 0) warnings += 2;
  if (d.lock_waiters > 0) warnings++;
  var dot = document.getElementById('status-dot');
  dot.className = 'status-dot ' + (d.error ? 'red' : warnings > 2 ? 'yellow' : warnings > 0 ? 'yellow' : 'green');

  // ── KPI row ──
  var kpis = '';
  kpis += '<div class="kpi"><div class="kpi-label">Queue Pending</div><div class="kpi-value">' + fmtNum(d.queue_pending) + '</div></div>';
  kpis += '<div class="kpi"><div class="kpi-label">Queue Running</div><div class="kpi-value" style="color:var(--blue)">' + fmtNum(d.queue_running) + '</div></div>';
  kpis += '<div class="kpi"><div class="kpi-label">Total Executed</div><div class="kpi-value">' + fmtNum(d.task_exec_rows) + '</div></div>';
  var actCount = d.activity ? d.activity.length : 0;
  kpis += '<div class="kpi"><div class="kpi-label">Active Queries</div><div class="kpi-value" style="color:' + (actCount > 0 ? 'var(--yellow)' : 'var(--green)') + '">' + actCount + '</div></div>';
  kpis += '<div class="kpi"><div class="kpi-label">Lock Blocked</div><div class="kpi-value" style="color:' + (d.blocked > 0 || d.lock_waiters > 0 ? 'var(--red)' : 'var(--green)') + '">' + (d.blocked + d.lock_waiters) + '</div></div>';
  kpis += '<div class="kpi"><div class="kpi-label">Index Scan</div><div class="kpi-value" style="color:' + (d.uses_index ? 'var(--green)' : 'var(--red)') + '">' + (d.uses_index ? 'ON' : 'OFF') + '</div></div>';
  var k = document.getElementById('kpis');
  if (k) k.innerHTML = kpis;

  // ── Throughput sparkline ──
  if (d.throughput && d.throughput.length > 0 && !history.length) {
    d.throughput.forEach(function (h) { history.push(h.cnt); });
  }
  var lastHr = d.throughput && d.throughput.length ? d.throughput[d.throughput.length - 1].cnt : 0;
  updateHistory(lastHr);

  // ── Duration bars ──
  var dbhtml = '';
  var maxP99 = 1;
  (d.type_stats || []).forEach(function (t) { if (t.p99 > maxP99) maxP99 = t.p99; });
  (d.type_stats || []).forEach(function (t, i) {
    var w50 = Math.round(t.p50 / maxP99 * 100);
    var w95 = Math.round(t.p95 / maxP99 * 100);
    var w99 = Math.round(t.p99 / maxP99 * 100);
    dbhtml += '<div class="dur-row"><div class="dur-label tc' + i + '">' + t.SIMPLE_TYPE + '</div><div class="dur-bars">';
    dbhtml += '<div class="dur-bar-row"><span class="pct">p50</span><div class="bar-track" style="flex:1"><div class="bar-fill bar-blue" style="width:' + w50 + '%"></div></div><span class="mono">' + fmtDur(t.p50) + '</span></div>';
    dbhtml += '<div class="dur-bar-row"><span class="pct">p95</span><div class="bar-track" style="flex:1"><div class="bar-fill bar-yellow" style="width:' + w95 + '%"></div></div><span class="mono">' + fmtDur(t.p95) + '</span></div>';
    dbhtml += '<div class="dur-bar-row"><span class="pct">p99</span><div class="bar-track" style="flex:1"><div class="bar-fill bar-red" style="width:' + w99 + '%"></div></div><span class="mono">' + fmtDur(t.p99) + '</span></div>';
    dbhtml += '</div></div>';
  });
  var dbEl = document.getElementById('dur-bars');
  if (dbEl) dbEl.innerHTML = dbhtml || '<div class="empty-state">No data</div>';

  // ── Queue delay ──
  var qhtml = '';
  (d.queue_delay || []).forEach(function (q, i) {
    qhtml += '<div class="dur-row"><div class="dur-label tc' + i + '">' + q.SIMPLE_TYPE + '</div><div class="dur-bars">';
    qhtml += '<div class="dur-bar-row"><span class="pct">work</span><div class="bar-track" style="flex:1"><div class="bar-fill bar-green" style="width:100%"></div></div><span class="mono">' + fmtDur(q.avg_work_ms) + '</span></div>';
    qhtml += '<div class="dur-bar-row"><span class="pct">wall</span><div class="bar-track" style="flex:1"><div class="bar-fill bar-purple" style="width:' + Math.min(100, q.avg_wall_ms / Math.max(1, q.avg_work_ms) * 100) + '%"></div></div><span class="mono">' + fmtDur(q.avg_wall_ms) + '</span></div>';
    qhtml += '</div></div>';
  });
  var qEl = document.getElementById('queue-delay');
  if (qEl) qEl.innerHTML = qhtml || '<div class="empty-state">No recent task completions</div>';

  // ── Recent executions ──
  var rhtml = '<table class="tbl"><tr><th>Type</th><th>When</th><th>Duration</th><th>Status</th></tr>';
  (d.recent || []).forEach(function (t) {
    rhtml += '<tr><td>' + t.SIMPLE_TYPE + '</td><td class="muted">' + fmtAgo(t.START_DATE) + '</td><td class="num">' + fmtDur(t.DURATION_MILLIS) + '</td><td>' + tag(t.SUCCESS ? 'OK' : 'FAIL', t.SUCCESS ? 't-ok' : 't-bad') + '</td></tr>';
  });
  rhtml += '</table>';
  var rEl = document.getElementById('recent-exec');
  if (rEl) rEl.innerHTML = rhtml || '<div class="empty-state">No executions</div>';

  // ── Group queue breakdown ──
  var ghtml = '';
  if (d.group_queue && d.group_queue.length) {
    ghtml = '<table class="tbl"><tr><th>Group ID</th><th>Type</th><th class="num">Pending</th><th class="num">Running</th><th class="num">Total</th></tr>';
    var totalPending = 0;
    (d.group_queue || []).forEach(function (g, i) {
      totalPending += g.pending;
      var pendingColor = g.running > 0 ? 'var(--yellow)' : 'var(--green)';
      ghtml += '<tr><td class="mono">' + (g.group_id || '') + '</td><td class="tc' + (i % 6) + '">' + g.SIMPLE_TYPE + '</td><td class="num" style="color:' + pendingColor + '">' + fmtNum(g.pending) + '</td><td class="num" style="color:var(--blue)">' + fmtNum(g.running) + '</td><td class="num">' + fmtNum(g.total) + '</td></tr>';
    });
    ghtml += '</table>';
    // concurrency ceiling
    if (d.concurrency_ceiling && d.concurrency_ceiling.length) {
      var ceiling = d.concurrency_ceiling[0].groups || 0;
      ghtml += '<div class="muted" style="margin-top:6px">Concurrency ceiling: <b style="color:var(--blue)">' + ceiling + '</b> distinct GROUP_IDs pending</div>';
    }
  } else {
    ghtml = '<div class="empty-state">Queue empty</div>';
  }
  var gEl = document.getElementById('group-queue');
  if (gEl) gEl.innerHTML = ghtml;

  // ── Running tasks detail ──
  var rtdhtml = '';
  if (d.running_detail && d.running_detail.length) {
    rtdhtml = '<table class="tbl"><tr><th>Type</th><th>Group ID</th><th>Age</th><th>Stale (since update)</th></tr>';
    d.running_detail.forEach(function (rt, i) {
      var cls = rt.age_sec > 300 ? 't-bad' : rt.age_sec > 60 ? 't-warn' : 't-ok';
      rtdhtml += '<tr><td class="tc' + (i % 6) + '">' + rt.SIMPLE_TYPE + '</td><td class="mono">' + (rt.group_id || '') + '</td><td>' + tag(fmtDur(rt.age_sec * 1000), cls) + '</td><td class="muted">' + fmtAgo(new Date(Date.now() - rt.stale_sec * 1000).toISOString()) + '</td></tr>';
    });
    rtdhtml += '</table>';
  } else {
    rtdhtml = '<div class="empty-state">No running tasks</div>';
  }
  var rtdEl = document.getElementById('running-tasks');
  if (rtdEl) rtdEl.innerHTML = rtdhtml;

  // ── Type breakdown table (target is already <table>) ──
  var texec = d.task_exec_rows || 1;
  var trows = '<thead><tr><th>Type</th><th class="num">Count</th><th class="num">Share</th><th class="num">Avg</th><th class="num">p50</th><th class="num">p95</th><th class="num">p99</th><th class="num">Max</th><th class="num">Success</th></tr></thead><tbody>';
  (d.type_stats || []).forEach(function (t, i) {
    trows += '<tr><td class="tc' + i + '">' + t.SIMPLE_TYPE + '</td><td class="num">' + fmtNum(t.n) + '</td><td class="num">' + fmtPct(t.n, texec) + '</td><td class="num">' + fmtDur(t.avg_ms) + '</td><td class="num">' + fmtDur(t.p50) + '</td><td class="num">' + fmtDur(t.p95) + '</td><td class="num">' + fmtDur(t.p99) + '</td><td class="num">' + fmtDur(t.max_ms) + '</td><td class="num">' + tag('OK', t.fail ? t.fail / t.n > 0.01 ? 't-warn' : 't-ok' : 't-ok') + '</td></tr>';
  });
  trows += '</tbody>';
  var tEl = document.getElementById('type-table');
  if (tEl) tEl.innerHTML = trows || '<div class="empty-state">No data</div>';

  // ── Failures ──
  var fhtml = '';
  if (d.recent_failures && d.recent_failures.length) {
    fhtml = '<table class="tbl"><tr><th>Type</th><th>When</th><th>Error</th></tr>';
    d.recent_failures.forEach(function (f) {
      fhtml += '<tr><td>' + f.SIMPLE_TYPE + '</td><td class="muted">' + fmtAgo(f.START_DATE) + '</td><td class="mono" style="max-width:400px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">' + (f.ERROR_MESSAGE || '') + '</td></tr>';
    });
    fhtml += '</table>';
  } else {
    fhtml = '<div class="empty-state">No failures</div>';
  }
  var fEl = document.getElementById('failures');
  if (fEl) fEl.innerHTML = fhtml;

  // ── Indexes ──
  var ihtml = '';
  var idxs = [
    ['idx_task_queue', d.idxs.idx_task_queue, 'Partial (OWNER,PRIORITY DESC,LAST_MODIFIED_DATE) WHERE OWNER IS NULL', 'Primary claim query'],
    ['idx_task_owner_group', d.idxs.idx_task_owner_group, 'Partial (OWNER,GROUP_ID) WHERE OWNER IS NOT NULL', 'NOT EXISTS subquery']
  ];
  idxs.forEach(function (idx) {
    ihtml += '<div class="dur-row"><div class="dur-label mono">' + idx[0] + '</div><div>' + tag(idx[1] ? 'EXISTS' : 'MISSING', idx[1] ? 't-ok' : 't-bad') + '</div></div>';
    ihtml += '<div class="muted" style="font-size:10px;margin-bottom:6px">' + idx[2] + ' — ' + idx[3] + '</div>';
  });
  var iEl = document.getElementById('idx-list');
  if (iEl) iEl.innerHTML = ihtml;

  // ── Explain ──
  var xEl = document.getElementById('explain');
  if (xEl) xEl.textContent = d.explain || 'N/A';

  // ── Activity ──
  var ahtml = '';
  if (d.activity && d.activity.length) {
    ahtml = '<table class="tbl"><tr><th>PID</th><th>State</th><th>Wait</th><th>Elapsed</th><th>Query</th></tr>';
    d.activity.forEach(function (a) {
      ahtml += '<tr><td class="mono">' + a.pid + '</td><td>' + tag(a.state, a.state === 'active' ? 't-ok' : a.state === 'idle' ? 't-info' : 't-warn') + '</td><td class="muted">' + (a.wait_event || '') + '</td><td class="num">' + fmtDur(a.elapsed_sec * 1000) + '</td><td class="mono" style="max-width:300px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">' + (a.q || '') + '</td></tr>';
    });
    ahtml += '</table>';
  } else {
    ahtml = '<div class="empty-state">Idle — no active queries</div>';
  }
  var aEl = document.getElementById('activity');
  if (aEl) aEl.innerHTML = ahtml;

  // ── Table health ──
  var thealth = '<table class="tbl"><tr><th>Table</th><th class="num">Live</th><th class="num">Dead</th><th>Last Vacuum</th><th>Last Analyze</th></tr>';
  (d.table_health || []).forEach(function (t) {
    var dc = t.n_dead_tup > 1000 ? 't-bad' : t.n_dead_tup > 100 ? 't-warn' : 't-ok';
    thealth += '<tr><td>' + t.relname + '</td><td class="num">' + fmtNum(t.n_live_tup) + '</td><td class="num">' + tag(fmtNum(t.n_dead_tup), dc) + '</td><td class="muted">' + fmtAgo(t.last_vc) + '</td><td class="muted">' + fmtAgo(t.last_an) + '</td></tr>';
  });
  thealth += '</table>';
  var thEl = document.getElementById('table-health');
  if (thEl) thEl.innerHTML = thealth;

  // ── PG config ──
  var rec = {
    shared_buffers: '512MB',
    effective_cache_size: '1GB',
    work_mem: '16MB',
    maintenance_work_mem: '128MB',
    random_page_cost: '1.1',
    max_wal_size: '2GB'
  };
  var chtml = '<table class="tbl"><tr><th>Parameter</th><th class="num">Current</th><th class="num">Recommended</th><th>Status</th></tr>';
  (d.pg_config || []).forEach(function (c) {
    var v = (c.setting || '') + (c.unit || '');
    var rv = rec[c.name] || '?';
    var ok = v === rv;
    chtml += '<tr><td class="mono">' + c.name + '</td><td class="num" style="color:' + (ok ? 'var(--green)' : 'var(--yellow)') + '">' + v + '</td><td class="num">' + rv + '</td><td>' + tag(ok ? 'OK' : 'MISMATCH', ok ? 't-ok' : 't-warn') + '</td></tr>';
  });
  chtml += '</table>';
  var cEl = document.getElementById('pgcfg');
  if (cEl) cEl.innerHTML = chtml;
}

function load() {
  fetch('/api/stats')
    .then(function (r) { return r.json(); })
    .then(function (d) { render(d); })
    .catch(function (e) { render({ error: e.message }); });
  timers.push(setTimeout(load, PK * 1000));
}

// Tab switching
document.addEventListener('click', function (e) {
  var t = e.target.closest('[data-tab]');
  if (t) { e.preventDefault(); switchTab(t.dataset.tab); }
});

load();
