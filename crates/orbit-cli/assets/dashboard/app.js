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

let searchQuery = "";
let activeStatuses = new Set(STATUS_ORDER);
let lastTasks = [];

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

function filterTasks(tasks) {
  const q = searchQuery;
  return tasks.filter((t) => {
    if (!activeStatuses.has(t.status)) return false;
    if (!q) return true;
    return (
      (t.id && t.id.toLowerCase().includes(q)) ||
      (t.title && t.title.toLowerCase().includes(q))
    );
  });
}

function renderTasks(tasks) {
  const body = $("tasks-body");
  body.innerHTML = "";
  const filtered = filterTasks(tasks);
  $("tasks-count").textContent =
    filtered.length === tasks.length
      ? `${tasks.length}`
      : `${filtered.length}/${tasks.length}`;
  if (filtered.length === 0) {
    body.appendChild(
      el("div", {
        class: "empty",
        text: tasks.length === 0 ? "no tasks." : "no tasks match.",
      }),
    );
    return;
  }
  const groups = new Map();
  for (const t of filtered) {
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

const SCOREBOARD_COLUMNS = [
  { key: "agent", label: "agent", num: false },
  { key: "tasks_completed", label: "tasks", num: true },
  { key: "tokens.total", label: "tokens", num: true },
  { key: "tokens.output", label: "out", num: true },
  { key: "tool_calls", label: "tools", num: true },
  { key: "duels.wins", label: "duel w", num: true },
  { key: "duels.losses", label: "duel l", num: true },
  { key: "friction.reported", label: "frict r", num: true },
  { key: "friction.accepted", label: "frict a", num: true },
  { key: "friction.rejected", label: "frict rej", num: true },
  { key: "pr.merged_clean", label: "pr clean", num: true },
  { key: "pr.merged_with_revision", label: "pr w/rev", num: true },
  { key: "pr.review_comments", label: "pr cmts", num: true },
];

function readPath(obj, path) {
  let cur = obj;
  for (const part of path.split(".")) {
    if (cur == null) return undefined;
    cur = cur[part];
  }
  return cur;
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

  const table = el("table", { class: "scoreboard-table" });
  const thead = el("thead");
  const headRow = el("tr");
  for (const col of SCOREBOARD_COLUMNS) {
    headRow.appendChild(
      el(col === SCOREBOARD_COLUMNS[0] ? "th" : "th", {
        class: col.num ? "num" : "",
        text: col.label,
      }),
    );
  }
  thead.appendChild(headRow);
  table.appendChild(thead);

  const tbody = el("tbody");
  for (const [name, agent] of entries) {
    const row = el("tr");
    for (const col of SCOREBOARD_COLUMNS) {
      let cellText;
      let extra = "";
      if (col.key === "agent") {
        cellText = name;
      } else {
        const v = readPath(agent, col.key);
        const num = typeof v === "number" ? v : 0;
        cellText = String(num);
        if (num === 0) extra = " zero";
      }
      const cellClass =
        (col.num ? "num" : col.key === "agent" ? "agent" : "") + extra;
      row.appendChild(
        el("td", {
          class: cellClass,
          text: cellText,
          title: col.key === "agent" ? name : undefined,
        }),
      );
    }
    tbody.appendChild(row);
  }
  table.appendChild(tbody);
  body.appendChild(table);
}

function showError(panelId, err) {
  const body = $(panelId);
  body.innerHTML = "";
  body.appendChild(el("div", { class: "err", text: String(err) }));
}

function refreshChips() {
  for (const chip of document.querySelectorAll("#task-filter .chip")) {
    const status = chip.dataset.status;
    const isAll = chip.dataset.role === "all";
    const allOn = activeStatuses.size === STATUS_ORDER.length;
    const on = isAll ? allOn : activeStatuses.has(status);
    chip.classList.toggle("active", on);
  }
}

function buildChips() {
  const container = $("task-filter");
  container.innerHTML = "";
  const allChip = el("button", { class: "chip", text: "all" });
  allChip.dataset.role = "all";
  allChip.addEventListener("click", () => {
    activeStatuses = new Set(STATUS_ORDER);
    refreshChips();
    renderTasks(lastTasks);
  });
  container.appendChild(allChip);
  for (const status of STATUS_ORDER) {
    const chip = el("button", { class: "chip", text: status });
    chip.dataset.status = status;
    chip.style.borderLeft = `2px solid var(--status-${status}, var(--border))`;
    chip.addEventListener("click", () => {
      if (activeStatuses.has(status)) {
        activeStatuses.delete(status);
      } else {
        activeStatuses.add(status);
      }
      refreshChips();
      renderTasks(lastTasks);
    });
    container.appendChild(chip);
  }
  refreshChips();
}

function wireSearch() {
  $("task-search").addEventListener("input", (e) => {
    searchQuery = e.target.value.trim().toLowerCase();
    renderTasks(lastTasks);
  });
}

const TABS = ["tasks", "scoreboard"];

function setActiveTab(name) {
  if (!TABS.includes(name)) name = "tasks";
  for (const tab of document.querySelectorAll(".tab")) {
    tab.classList.toggle("active", tab.dataset.tab === name);
  }
  for (const pane of document.querySelectorAll(".tab-pane")) {
    pane.classList.toggle("active", pane.dataset.tab === name);
  }
  if (window.location.hash !== `#${name}`) {
    window.location.hash = `#${name}`;
  }
}

function initTabs() {
  for (const tab of document.querySelectorAll(".tab")) {
    tab.addEventListener("click", () => setActiveTab(tab.dataset.tab));
  }
  window.addEventListener("hashchange", () => {
    setActiveTab(window.location.hash.replace(/^#/, ""));
  });
  setActiveTab(window.location.hash.replace(/^#/, "") || "tasks");
}

async function tick() {
  const now = new Date();
  $("meta").textContent = `polled ${now.toLocaleTimeString()} · ${POLL_MS}ms`;
  await Promise.all([
    fetchJson("/api/tasks")
      .then((tasks) => {
        lastTasks = tasks;
        renderTasks(tasks);
      })
      .catch((e) => showError("tasks-body", e)),
    fetchJson("/api/job-runs").then(renderRuns).catch((e) => showError("runs-body", e)),
    fetchJson("/api/scoreboard").then(renderScoreboard).catch((e) => showError("scoreboard-body", e)),
  ]);
  $("footer").textContent = `orbit dashboard · GET /api/{tasks,jobs,job-runs,audit,scoreboard}`;
}

initTabs();
buildChips();
wireSearch();
tick();
setInterval(tick, POLL_MS);
