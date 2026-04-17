// Orbit dashboard — terminal-dark, polling-based read-only SPA.
// Pure vanilla JS, no build step. Polls every POLL_MS or ?poll=<ms>.

const STATUS_ORDER = [
  "in-progress",
  "review",
  "blocked",
  "proposed",
  "backlog",
  "someday",
  "done",
  "rejected",
  "archived",
];

const params = new URLSearchParams(window.location.search);
const POLL_MS = Math.max(1000, parseInt(params.get("poll") || "5000", 10));

const $ = (id) => document.getElementById(id);

function el(tag, opts = {}, children = []) {
  const node = document.createElement(tag);
  if (opts.class) node.className = opts.class;
  if (opts.text != null) node.textContent = opts.text;
  if (opts.title != null) node.title = opts.title;
  if (opts.style) Object.assign(node.style, opts.style);
  for (const child of children) {
    if (child == null) continue;
    node.appendChild(typeof child === "string" ? document.createTextNode(child) : child);
  }
  return node;
}

function statusPill(status) {
  const color = `var(--status-${status}, var(--fg))`;
  const pill = el("span", { class: "pill mono", text: status });
  pill.style.color = color;
  pill.style.borderLeft = `2px solid ${color}`;
  return pill;
}

function priorityCell(p) {
  const node = el("span", { class: "priority mono", text: p });
  node.style.color = `var(--priority-${p}, var(--fg-dim))`;
  return node;
}

function stateCell(state) {
  const node = el("span", { class: "mono", text: state });
  node.style.color = `var(--state-${state}, var(--fg-dim))`;
  return node;
}

async function fetchJson(path) {
  const res = await fetch(path, { headers: { accept: "application/json" } });
  if (!res.ok) throw new Error(`${path}: HTTP ${res.status}`);
  return res.json();
}

function renderTasks(tasks) {
  const body = $("tasks-body");
  body.innerHTML = "";
  $("tasks-count").textContent = `${tasks.length}`;
  if (tasks.length === 0) {
    body.appendChild(el("div", { class: "empty", text: "no tasks." }));
    return;
  }
  const groups = new Map();
  for (const t of tasks) {
    if (!groups.has(t.status)) groups.set(t.status, []);
    groups.get(t.status).push(t);
  }
  const ordered = STATUS_ORDER.filter((s) => groups.has(s)).concat(
    [...groups.keys()].filter((s) => !STATUS_ORDER.includes(s)),
  );
  for (const status of ordered) {
    const group = groups.get(status);
    const header = el("div", { class: "group-header" }, [
      statusPill(status),
      el("span", { class: "group-count", text: `${group.length}` }),
    ]);
    body.appendChild(header);
    for (const t of group) {
      const row = el("div", { class: "row", title: t.title }, [
        el("span", { class: "id mono", text: t.id }),
        el("span", { class: "title", text: t.title }),
        priorityCell(t.priority),
        el("span", { class: "type mono", text: t.type }),
      ]);
      body.appendChild(row);
    }
  }
}

function fmtTimestamp(iso) {
  if (!iso) return "-";
  const d = new Date(iso);
  if (isNaN(d.getTime())) return iso;
  const now = Date.now();
  const diff = (now - d.getTime()) / 1000;
  if (diff < 60) return `${Math.floor(diff)}s`;
  if (diff < 3600) return `${Math.floor(diff / 60)}m`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h`;
  return `${Math.floor(diff / 86400)}d`;
}

function fmtDuration(ms) {
  if (ms == null) return "-";
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${Math.floor(ms / 60000)}m${Math.floor((ms % 60000) / 1000)}s`;
}

function renderRuns(runs) {
  const body = $("runs-body");
  body.innerHTML = "";
  const top = runs.slice(0, 20);
  $("runs-count").textContent = `${top.length}/${runs.length}`;
  if (top.length === 0) {
    body.appendChild(el("div", { class: "empty", text: "no job runs yet." }));
    return;
  }
  for (const r of top) {
    const ts = r.finished_at || r.started_at || r.scheduled_at || r.created_at;
    const row = el("div", { class: "runs-row", title: r.run_id }, [
      el("span", { class: "when", text: fmtTimestamp(ts) }),
      el("span", { class: "id", text: r.job_id }),
      el("span", { class: "duration", text: fmtDuration(r.duration_ms) }),
      el("span", { class: "state" }, [stateCell(r.state)]),
    ]);
    body.appendChild(row);
  }
}

function renderScoreboard(summary) {
  const body = $("scoreboard-body");
  body.innerHTML = "";
  const agentsMap = (summary && summary.agents) || {};
  const entries = Object.entries(agentsMap);
  $("scoreboard-count").textContent = `${entries.length}`;
  if (entries.length === 0) {
    body.appendChild(el("div", { class: "empty", text: "no scoreboard data yet." }));
    return;
  }
  entries.sort(([, a], [, b]) => (b.tasks_completed || 0) - (a.tasks_completed || 0));
  body.appendChild(
    el("div", { class: "scoreboard-row header" }, [
      el("span", { text: "agent" }),
      el("span", { class: "num", text: "tasks" }),
      el("span", { class: "num", text: "merged" }),
      el("span", { class: "num", text: "rev" }),
    ]),
  );
  for (const [name, a] of entries) {
    const tasks = a.tasks_completed ?? 0;
    const pr = a.pr || {};
    const merged = (pr.merged_clean || 0) + (pr.merged_with_revision || 0);
    const rev = pr.merged_with_revision || 0;
    body.appendChild(
      el("div", { class: "scoreboard-row" }, [
        el("span", { text: name, title: name }),
        el("span", { class: "num", text: String(tasks) }),
        el("span", { class: "num", text: String(merged) }),
        el("span", { class: "num", text: String(rev) }),
      ]),
    );
  }
}

function showError(panelId, err) {
  const body = $(panelId);
  body.innerHTML = "";
  body.appendChild(el("div", { class: "err", text: String(err) }));
}

async function tick() {
  const now = new Date();
  $("meta").textContent = `polled ${now.toLocaleTimeString()} · ${POLL_MS}ms`;
  await Promise.all([
    fetchJson("/api/tasks").then(renderTasks).catch((e) => showError("tasks-body", e)),
    fetchJson("/api/job-runs").then(renderRuns).catch((e) => showError("runs-body", e)),
    fetchJson("/api/scoreboard").then(renderScoreboard).catch((e) => showError("scoreboard-body", e)),
  ]);
  $("footer").textContent = `orbit dashboard · GET /api/{tasks,jobs,job-runs,audit,scoreboard}`;
}

tick();
setInterval(tick, POLL_MS);
