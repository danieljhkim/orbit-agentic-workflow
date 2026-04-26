// Orbit dashboard — terminal-dark, manually refreshed SPA.
// Pure vanilla JS, no build step.

const STATUS_ORDER = [
  "in-progress",
  "review",
  "blocked",
  "proposed",
  "backlog",
  "someday",
  "rejected",
];

const DEFAULT_INACTIVE_STATUSES = new Set(["someday"]);

const params = new URLSearchParams(window.location.search);
function positiveIntParam(name, fallback) {
  const parsed = parseInt(params.get(name) || String(fallback), 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}
const JOB_RUN_LIMIT = positiveIntParam("runs", 25);
const DIAG_LIMIT = positiveIntParam("diag", 50);
const AUDIT_LIMIT = positiveIntParam("audit", 50);
const RUN_EVENTS_LIMIT = positiveIntParam("events", 100);

const AUDIT_STATUSES = ["success", "failure", "denied"];

const $ = (id) => document.getElementById(id);

let searchQuery = "";
let activeStatuses = new Set(
  STATUS_ORDER.filter((s) => !DEFAULT_INACTIVE_STATUSES.has(s)),
);
let lastTasks = [];
let lastRuns = [];
let lastDiagnostics = { metrics: [], friction: [] };
let activeTab = "tasks";
let activeDiagSubtab = "runs";
let expandedTaskIds = new Set();
let isRefreshing = false;

// Audit tab state
let lastAudit = [];
let auditFilter = { status: null, q: "", tool: null, role: null, run_id: null };
let expandedAuditIds = new Set();

// Run detail state
let activeRunId = null;
let activeRunDetail = null;
let activeRunEvents = [];
let activeRunSubtab = "steps";
let expandedStepIndices = new Set();

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

function fetchJson(path) {
  return fetch(path, { headers: { accept: "application/json" } })
    .then(res => {
      if (!res.ok) throw new Error(`${path}: HTTP ${res.status}`);
      return res.json();
    });
}

function syncNodes(container, newNodesArr) {
  const oldNodes = Array.from(container.children);
  const oldMap = new Map();
  for (const node of oldNodes) {
    if (node.dataset.key) oldMap.set(node.dataset.key, node);
  }

  for (let i = 0; i < newNodesArr.length; i++) {
    const newNode = newNodesArr[i];
    const key = newNode.dataset.key;
    let nodeToPlace = newNode;

    if (key && oldMap.has(key)) {
      const oldNode = oldMap.get(key);
      if (oldNode.dataset.hash === newNode.dataset.hash) {
        nodeToPlace = oldNode;
      } else {
        nodeToPlace.classList.add("data-changed");
      }
    } else if (key) {
      nodeToPlace.classList.add("data-new");
    }

    if (container.children[i] !== nodeToPlace) {
      if (container.children[i]) {
        container.insertBefore(nodeToPlace, container.children[i]);
      } else {
        container.appendChild(nodeToPlace);
      }
    }
  }

  while (container.children.length > newNodesArr.length) {
    container.removeChild(container.lastElementChild);
  }
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

const TASK_META_FIELDS = [
  ["implemented_by", "implemented_by"],
  ["planned_by", "planned_by"],
  ["created_by", "created_by"],
  ["pr_number", "pr"],
  ["pr_status", "pr_status"],
  ["created_at", "created"],
  ["updated_at", "updated"],
];

function buildTaskDetail(task) {
  const detail = el("div", { class: "row-detail" });
  detail.addEventListener("click", (e) => e.stopPropagation());

  const meta = el("div", { class: "meta-line" });
  let metaCount = 0;
  for (const [key, label] of TASK_META_FIELDS) {
    const v = task[key];
    if (v == null || v === "") continue;
    const display = key.endsWith("_at") ? fmtAbsTime(v) : String(v);
    const span = el("span", {}, [
      el("span", { class: "label", text: `${label}:` }),
      el("span", { class: "value", text: display }),
    ]);
    meta.appendChild(span);
    metaCount++;
  }
  if (metaCount > 0) detail.appendChild(meta);

  const addField = (title, child) => {
    const block = el("div", { class: "field-block" });
    block.appendChild(el("h4", { text: title }));
    block.appendChild(child);
    detail.appendChild(block);
  };

  if (task.description && task.description.trim()) {
    const view = el("div", { class: "markdown-body" });
    if (typeof marked !== "undefined") {
      view.innerHTML = marked.parse(task.description);
    } else {
      view.textContent = task.description;
    }
    addField("description", view);
  }

  if (Array.isArray(task.acceptance_criteria) && task.acceptance_criteria.length > 0) {
    const ul = el("ul", { class: "ac-list" });
    for (const ac of task.acceptance_criteria) {
      if (typeof marked !== "undefined") {
        const li = el("li");
        li.innerHTML = marked.parseInline(ac);
        ul.appendChild(li);
      } else {
        ul.appendChild(el("li", { text: ac }));
      }
    }
    addField("acceptance criteria", ul);
  }

  if (task.plan && task.plan.trim()) {
    const view = el("div", { class: "markdown-body" });
    if (typeof marked !== "undefined") {
      view.innerHTML = marked.parse(task.plan);
    } else {
      view.textContent = task.plan;
    }
    addField("plan", view);
  }

  if (task.execution_summary && task.execution_summary.trim()) {
    const view = el("div", { class: "markdown-body" });
    if (typeof marked !== "undefined") {
      view.innerHTML = marked.parse(task.execution_summary);
    } else {
      view.textContent = task.execution_summary;
    }
    addField("execution summary", view);
  }

  if (Array.isArray(task.comments) && task.comments.length > 0) {
    const wrap = el("div");
    for (const c of task.comments) {
      const line = el("div", { class: "comment-line" }, [
        document.createTextNode(`[${fmtAbsTime(c.at)}] `),
        el("span", { class: "author", text: c.by || "?" }),
        document.createTextNode(`: ${c.message || ""}`),
      ]);
      wrap.appendChild(line);
    }
    addField("comments", wrap);
  }

  if (Array.isArray(task.context_files) && task.context_files.length > 0) {
    const ul = el("ul", { class: "file-list" });
    for (const path of task.context_files) {
      ul.appendChild(el("li", { text: path }));
    }
    addField("context", ul);
  }

  if (Array.isArray(task.history) && task.history.length > 0) {
    const wrap = el("div");
    const recent = task.history.slice(-5).reverse();
    for (const h of recent) {
      const note = h.note ? ` (${h.note})` : "";
      const line = el("div", { class: "history-line" }, [
        document.createTextNode(`[${fmtAbsTime(h.at)}] `),
        el("span", { class: "actor", text: h.by || "?" }),
        document.createTextNode(`: ${h.event}${note}`),
      ]);
      wrap.appendChild(line);
    }
    addField("recent history", wrap);
  }

  detail.appendChild(buildActionsRow(task, detail));

  return detail;
}

const APPROVE_STATUSES = new Set(["proposed", "review"]);
const REJECT_STATUSES = new Set(["proposed", "review", "backlog"]);

function buildActionsRow(task, detail) {
  const actions = el("div", { class: "actions" });
  if (APPROVE_STATUSES.has(task.status)) {
    const btn = el("button", { class: "action approve", text: "approve" });
    btn.addEventListener("click", (e) => {
      e.stopPropagation();
      runAction(task, "approve", detail, null, btn);
    });
    actions.appendChild(btn);
  }
  if (REJECT_STATUSES.has(task.status)) {
    const btn = el("button", { class: "action reject", text: "reject" });
    btn.addEventListener("click", (e) => {
      e.stopPropagation();
      showRejectForm(task, detail, actions);
    });
    actions.appendChild(btn);
  }
  if (task.status !== "archived") {
    const btn = el("button", { class: "action archive", text: "archive" });
    btn.addEventListener("click", (e) => {
      e.stopPropagation();
      if (window.confirm(`Archive task ${task.id}?`)) {
        runAction(task, "archive", detail, null, btn);
      }
    });
    actions.appendChild(btn);
  }
  return actions;
}

function showRejectForm(task, detail, actions) {
  const form = el("div", { class: "reject-form" });
  form.addEventListener("click", (e) => e.stopPropagation());
  const ta = el("textarea");
  ta.placeholder = "reason for rejection";
  const buttons = el("div", { class: "actions" });
  const submit = el("button", { class: "action reject", text: "submit" });
  const cancel = el("button", { class: "action cancel", text: "cancel" });
  submit.addEventListener("click", (e) => {
    e.stopPropagation();
    const note = ta.value.trim();
    if (!note) {
      ta.focus();
      return;
    }
    runAction(task, "reject", detail, { note }, submit);
  });
  cancel.addEventListener("click", (e) => {
    e.stopPropagation();
    form.replaceWith(actions);
  });
  buttons.appendChild(submit);
  buttons.appendChild(cancel);
  form.appendChild(ta);
  form.appendChild(buttons);
  actions.replaceWith(form);
  ta.focus();
}

async function runAction(task, kind, detail, body, btnNode) {
  // Disable buttons while in flight to prevent double-clicks
  for (const b of detail.querySelectorAll("button.action")) b.disabled = true;
  let oldText = "";
  if (btnNode) {
    oldText = btnNode.textContent;
    btnNode.innerHTML = `<span class="spinner"></span>wait`;
  }
  // Clear any prior error
  const prior = detail.querySelector(".action-error");
  if (prior) prior.remove();
  try {
    const res = await fetch(`/api/tasks/${encodeURIComponent(task.id)}/${kind}`, {
      method: "POST",
      headers: body ? { "content-type": "application/json" } : undefined,
      body: body ? JSON.stringify(body) : undefined,
    });
    if (!res.ok) {
      let msg = `${kind} failed: HTTP ${res.status}`;
      try {
        const errBody = await res.json();
        if (errBody && errBody.error) msg = `${kind} failed: ${errBody.error}`;
      } catch (_) {
        /* keep generic msg */
      }
      throw new Error(msg);
    }
    expandedTaskIds.delete(task.id);
    await refreshDashboard();
  } catch (err) {
    for (const b of detail.querySelectorAll("button.action")) b.disabled = false;
    if (btnNode) btnNode.textContent = oldText;
    const errEl = el("div", { class: "action-error", text: String(err.message || err) });
    detail.prepend(errEl);
  }
}

function renderTasks(tasks) {
  const body = $("tasks-body");
  const frag = document.createDocumentFragment();
  
  const filtered = filterTasks(tasks);
  $("tasks-count").textContent =
    filtered.length === tasks.length
      ? `${tasks.length}`
      : `${filtered.length}/${tasks.length}`;
  if (filtered.length === 0) {
    const defaultText = tasks.length === 0 ? "No tasks available." : "No tasks match filter.";
    syncNodes(body, [el("div", { class: "empty-state" }, [
      el("div", { class: "icon", text: "✧" }),
      el("div", { class: "text", text: defaultText })
    ])]);
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
    header.dataset.key = `header-${status}`;
    header.dataset.hash = `${status}-${group.length}`;
    frag.appendChild(header);
    for (const t of group) {
      const idSpan = el("span", { class: "id mono", text: t.id, title: "Click to copy ID" });
      idSpan.addEventListener("click", (e) => {
        e.stopPropagation();
        navigator.clipboard.writeText(t.id).catch(() => {});
        const oldText = idSpan.textContent;
        idSpan.textContent = "copied!";
        idSpan.style.color = "var(--state-success)";
        setTimeout(() => {
          idSpan.textContent = oldText;
          idSpan.style.color = "";
        }, 1000);
      });
      const row = el("div", { class: "row", title: t.title }, [
        idSpan,
        el("span", { class: "title", text: t.title }),
        priorityCell(t.priority),
        el("span", { class: "type mono", text: t.type }),
      ]);
      row.dataset.key = `task-${t.id}`;
      // Basic hash based on row presentation parameters + expanded state
      row.dataset.hash = `${t.id}-${t.title}-${t.priority}-${t.type}-${expandedTaskIds.has(t.id)}`;
      row.addEventListener("click", () => {
        const toggle = () => {
          if (expandedTaskIds.has(t.id)) expandedTaskIds.delete(t.id);
          else expandedTaskIds.add(t.id);
          renderTasks(lastTasks);
        };
        if (document.startViewTransition) {
          row.style.viewTransitionName = `task-row-${t.id}`;
          document.startViewTransition(toggle).finished.then(() => {
            row.style.viewTransitionName = "";
          });
        } else {
          toggle();
        }
      });
      if (expandedTaskIds.has(t.id)) row.classList.add("expanded");
      frag.appendChild(row);
      if (expandedTaskIds.has(t.id)) {
        const detail = buildTaskDetail(t);
        detail.dataset.key = `detail-${t.id}`;
        // Diff by full task object stringified
        detail.dataset.hash = JSON.stringify(t);
        frag.appendChild(detail);
      }
    }
  }
  syncNodes(body, Array.from(frag.children));
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

function fmtAbsTime(iso) {
  if (!iso) return "-";
  const d = new Date(iso);
  if (isNaN(d.getTime())) return iso;
  const pad = (n) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function fmtDuration(ms) {
  if (ms == null) return "-";
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${Math.floor(ms / 60000)}m${Math.floor((ms % 60000) / 1000)}s`;
}

function renderRuns(runs) {
  const body = $("runs-body");
  const frag = document.createDocumentFragment();
  
  const top = runs.slice(0, 20);
  if ($("diag-count") && activeDiagSubtab === "runs") {
    $("diag-count").textContent = `${top.length}/${runs.length}`;
  }
  if (top.length === 0) {
    syncNodes(body, [el("div", { class: "empty-state" }, [
      el("div", { class: "icon", text: "✧" }),
      el("div", { class: "text", text: "No job runs yet." })
    ])]);
    return;
  }
  for (const r of top) {
    const ts = r.finished_at || r.started_at || r.scheduled_at || r.created_at;
    const row = el("div", { class: "runs-row", title: `${r.run_id} (click to inspect)` }, [
      el("span", { class: "when", text: fmtTimestamp(ts) }),
      el("span", { class: "id", text: r.job_id }),
      el("span", { class: "duration", text: fmtDuration(r.duration_ms) }),
      el("span", { class: "state" }, [stateCell(r.state)]),
    ]);
    row.dataset.key = `run-${r.run_id}`;
    row.dataset.hash = `${r.run_id}-${ts}-${r.duration_ms}-${r.state}`;
    row.style.cursor = "pointer";
    row.addEventListener("click", () => navigateToRun(r.run_id));
    frag.appendChild(row);
  }
  syncNodes(body, Array.from(frag.children));
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
  
  const agentsMap = (summary && summary.agents) || {};
  const entries = Object.entries(agentsMap);
  $("scoreboard-count").textContent = `${entries.length}`;
  
  if (entries.length === 0) {
    syncNodes(body, [el("div", { class: "empty-state" }, [
      el("div", { class: "icon", text: "✧" }),
      el("div", { class: "text", text: "No scoreboard data yet." })
    ])]);
    return;
  }
  
  entries.sort(([, a], [, b]) => (b.tasks_completed || 0) - (a.tasks_completed || 0));

  let table = body.querySelector("table.scoreboard-table");
  let tbody;
  if (!table) {
    table = el("table", { class: "scoreboard-table" });
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
    tbody = el("tbody");
    table.appendChild(tbody);
    syncNodes(body, [table]);
  } else {
    tbody = table.querySelector("tbody");
  }

  const frag = document.createDocumentFragment();
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
    row.dataset.key = `agent-${name}`;
    row.dataset.hash = JSON.stringify(agent);
    frag.appendChild(row);
  }
  
  syncNodes(tbody, Array.from(frag.children));
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

const TABS = ["tasks", "scoreboard", "audit", "diagnostics", "run-detail"];
const DIAG_SUBTABS = ["runs", "metrics", "friction"];
const RUN_DETAIL_SUBTABS = ["steps", "events"];

function parseHashRoute(raw) {
  const trimmed = String(raw || "").replace(/^#/, "");
  const queryIdx = trimmed.indexOf("?");
  const path = queryIdx >= 0 ? trimmed.slice(0, queryIdx) : trimmed;
  const queryStr = queryIdx >= 0 ? trimmed.slice(queryIdx + 1) : "";
  const segments = path.split("/").filter(Boolean);
  const query = new URLSearchParams(queryStr);
  return { segments, query };
}

function setActiveTab(raw, opts = {}) {
  const { segments, query } = parseHashRoute(raw);
  const head = segments[0] || "tasks";
  let top;
  if (head === "runs" && segments[1]) {
    top = "run-detail";
    activeRunId = decodeURIComponent(segments[1]);
    const sub = RUN_DETAIL_SUBTABS.includes(segments[2]) ? segments[2] : activeRunSubtab;
    activeRunSubtab = sub;
  } else if (TABS.includes(head)) {
    top = head;
  } else {
    top = "tasks";
  }
  activeTab = top;
  for (const tab of document.querySelectorAll(".tab")) {
    tab.classList.toggle("active", tab.dataset.tab === top);
  }
  for (const pane of document.querySelectorAll(".tab-pane")) {
    pane.classList.toggle("active", pane.dataset.tab === top);
  }

  const indicator = $("tab-indicator") || el("div", {id: "tab-indicator", class: "tab-indicator"});
  if (!indicator.parentNode) document.querySelector(".tabs").appendChild(indicator);
  // For run-detail (no top tab button), hide the indicator
  const activeTabEl = document.querySelector(`.tab[data-tab="${top}"]`);
  if (activeTabEl) {
    indicator.style.display = "";
    indicator.style.width = `${activeTabEl.offsetWidth}px`;
    indicator.style.left = `${activeTabEl.offsetLeft}px`;
  } else {
    indicator.style.display = "none";
  }

  let hash;
  if (top === "diagnostics") {
    const sub = DIAG_SUBTABS.includes(segments[1]) ? segments[1] : activeDiagSubtab;
    setDiagSubtab(sub);
    hash = `#diagnostics/${sub}`;
  } else if (top === "audit") {
    auditFilter.status = query.get("status") || null;
    auditFilter.tool = query.get("tool") || null;
    auditFilter.role = query.get("role") || null;
    auditFilter.run_id = query.get("run_id") || null;
    auditFilter.q = query.get("q") || "";
    hash = buildAuditHash();
    syncAuditControls();
  } else if (top === "run-detail") {
    setRunDetailSubtab(activeRunSubtab);
    hash = `#runs/${encodeURIComponent(activeRunId || "")}` +
      (activeRunSubtab !== "steps" ? `/${activeRunSubtab}` : "");
  } else {
    hash = `#${top}`;
  }
  const hashChanged = window.location.hash !== hash;
  const shouldUpdateHash = opts.updateHash !== false;
  if (hashChanged && shouldUpdateHash) {
    window.location.hash = hash;
  }
  if (opts.refresh !== false && (!hashChanged || !shouldUpdateHash)) refreshDashboard();
}

function buildAuditHash() {
  const sp = new URLSearchParams();
  if (auditFilter.status) sp.set("status", auditFilter.status);
  if (auditFilter.tool) sp.set("tool", auditFilter.tool);
  if (auditFilter.role) sp.set("role", auditFilter.role);
  if (auditFilter.run_id) sp.set("run_id", auditFilter.run_id);
  if (auditFilter.q) sp.set("q", auditFilter.q);
  const qs = sp.toString();
  return qs ? `#audit?${qs}` : `#audit`;
}

function syncAuditControls() {
  const search = $("audit-search");
  if (search && search.value !== (auditFilter.q || "")) {
    search.value = auditFilter.q || "";
  }
  for (const chip of document.querySelectorAll("#audit-filter .chip")) {
    const status = chip.dataset.status;
    chip.classList.toggle("active", auditFilter.status === status);
  }
}

function setRunDetailSubtab(name) {
  if (!RUN_DETAIL_SUBTABS.includes(name)) name = "steps";
  activeRunSubtab = name;
  for (const btn of document.querySelectorAll("#run-detail-subtabs .subtab")) {
    btn.classList.toggle("active", btn.dataset.subtab === name);
  }
  $("run-steps-body").style.display = name === "steps" ? "block" : "none";
  $("run-events-body").style.display = name === "events" ? "block" : "none";
}

function setDiagSubtab(name) {
  if (!DIAG_SUBTABS.includes(name)) name = "runs";
  activeDiagSubtab = name;
  for (const btn of document.querySelectorAll("#diag-subtabs .subtab")) {
    btn.classList.toggle("active", btn.dataset.subtab === name);
  }

  const subIndicator = $("subtab-indicator") || el("div", {id: "subtab-indicator", class: "tab-indicator"});
  if (!subIndicator.parentNode) document.querySelector("#diag-subtabs").appendChild(subIndicator);
  const activeBtn = document.querySelector(`.subtab[data-subtab="${name}"]`);
  if (activeBtn) {
    subIndicator.style.width = `${activeBtn.offsetWidth}px`;
    subIndicator.style.left = `${activeBtn.offsetLeft}px`;
  }

  if (name === "runs") {
    $("diag-body").style.display = "none";
    $("runs-body").style.display = "block";
    renderRuns(lastRuns);
  } else {
    $("diag-body").style.display = "block";
    $("runs-body").style.display = "none";
    renderDiagnostics();
  }
}

function initTabs() {
  for (const tab of document.querySelectorAll(".tab")) {
    tab.addEventListener("click", () => setActiveTab(tab.dataset.tab, { refresh: false }));
  }
  for (const btn of document.querySelectorAll("#diag-subtabs .subtab")) {
    btn.addEventListener("click", () =>
      setActiveTab(`diagnostics/${btn.dataset.subtab}`, { refresh: false }),
    );
  }
  for (const btn of document.querySelectorAll("#run-detail-subtabs .subtab")) {
    btn.addEventListener("click", () => {
      activeRunSubtab = btn.dataset.subtab;
      const path = `runs/${encodeURIComponent(activeRunId || "")}` +
        (activeRunSubtab !== "steps" ? `/${activeRunSubtab}` : "");
      setActiveTab(path, { refresh: false });
      refreshDashboard();
    });
  }
  window.addEventListener("hashchange", () => {
    setActiveTab(window.location.hash);
  });
  setActiveTab(window.location.hash || "tasks", {
    refresh: false,
    updateHash: false,
  });
  refreshDashboard();
  setInterval(refreshDashboard, 30000);
}

function fmtRelative(iso) {
  return fmtTimestamp(iso);
}

function truncate(text, max) {
  if (text == null) return "";
  if (text.length <= max) return text;
  return text.slice(0, max) + "\u2026";
}

const DIAG_METRICS_COLUMNS = [
  { key: "ts", label: "time", num: false, render: (v) => fmtRelative(v) },
  { key: "step", label: "step", num: false },
  { key: "actor_identity", label: "actor", num: false, render: (v) => v || "-" },
  {
    key: "token_usage",
    label: "tokens",
    num: true,
    render: (v) => (v == null ? "-" : String(v)),
  },
  { key: "tool_invocations", label: "tools", num: true },
  {
    key: "step_duration_ms",
    label: "duration",
    num: true,
    render: (v) => (v == null ? "-" : fmtDuration(v)),
  },
  { key: "retry_count", label: "retries", num: true },
];

const DIAG_FRICTION_COLUMNS = [
  { key: "ts", label: "time", num: false, render: (v) => fmtRelative(v) },
  { key: "step", label: "step", num: false },
  { key: "command", label: "command", num: false },
  {
    key: "exit_code",
    label: "exit",
    num: true,
    render: (v, _row, td) => {
      if (v == null) return "-";
      if (v !== 0) td.classList.add("exit-fail");
      return String(v);
    },
  },
  {
    key: "stderr",
    label: "stderr",
    num: false,
    cellClass: "stderr",
    render: (v, _row, td) => {
      const full = v || "";
      td.title = full;
      return truncate(full, 160);
    },
  },
];

function renderDiagnosticsTable(rows, columns) {
  const body = $("diag-body");
  
  if (!rows || rows.length === 0) {
    syncNodes(body, [el("div", { class: "empty-state" }, [
      el("div", { class: "icon", text: "✧" }),
      el("div", { class: "text", text: "No entries this month." })
    ])]);
    return;
  }
  
  let table = body.querySelector("table.scoreboard-table");
  let tbody;
  const tableSig = columns.map(c => c.key).join("-");
  if (!table || table.dataset.sig !== tableSig) {
    table = el("table", { class: "scoreboard-table" });
    table.dataset.sig = tableSig;
    const thead = el("thead");
    const headRow = el("tr");
    for (const col of columns) {
      headRow.appendChild(el("th", { class: col.num ? "num" : "", text: col.label }));
    }
    thead.appendChild(headRow);
    table.appendChild(thead);
    tbody = el("tbody");
    table.appendChild(tbody);
    syncNodes(body, [table]);
  } else {
    tbody = table.querySelector("tbody");
  }

  const frag = document.createDocumentFragment();
  for (let i = 0; i < rows.length; i++) {
    const row = rows[i];
    const tr = el("tr");
    for (const col of columns) {
      const baseClass =
        (col.num ? "num" : "") + (col.cellClass ? ` ${col.cellClass}` : "");
      const td = el("td", { class: baseClass });
      const v = row[col.key];
      const text = col.render ? col.render(v, row, td) : v == null ? "" : String(v);
      td.textContent = text;
      tr.appendChild(td);
    }
    tr.dataset.key = `diag-${row.ts || ''}-${row.step || i}-${row.command || row.actor_identity || ''}`;
    tr.dataset.hash = JSON.stringify(row);
    frag.appendChild(tr);
  }
  
  syncNodes(tbody, Array.from(frag.children));
}

function renderDiagnostics() {
  const sub = activeDiagSubtab;
  const rows = lastDiagnostics[sub] || [];
  $("diag-count").textContent = `${rows.length}`;
  renderDiagnosticsTable(
    rows,
    sub === "metrics" ? DIAG_METRICS_COLUMNS : DIAG_FRICTION_COLUMNS,
  );
}

function activeRefreshJobs() {
  if (activeTab === "tasks") {
    return [
      fetchJson("/api/tasks").then((tasks) => {
        lastTasks = tasks;
        renderTasks(tasks);
      }),
    ];
  }

  if (activeTab === "scoreboard") {
    return [fetchJson("/api/scoreboard").then(renderScoreboard)];
  }

  if (activeTab === "audit") {
    return [fetchAndRenderAudit()];
  }

  if (activeTab === "run-detail") {
    if (!activeRunId) {
      renderRunDetailEmpty("No run selected.");
      return [];
    }
    const jobs = [
      fetchJson(`/api/runs/${encodeURIComponent(activeRunId)}`).then((data) => {
        activeRunDetail = data;
        renderRunDetailMeta();
        renderRunSteps();
      }).catch((e) => {
        renderRunDetailEmpty(`Run not found: ${activeRunId}`);
        throw e;
      }),
    ];
    if (activeRunSubtab === "events") {
      jobs.push(
        fetchJson(`/api/runs/${encodeURIComponent(activeRunId)}/events?limit=${RUN_EVENTS_LIMIT}`).then((events) => {
          activeRunEvents = events;
          renderRunEvents();
        }),
      );
    }
    return jobs;
  }

  if (activeDiagSubtab === "runs") {
    return [
      fetchJson(`/api/job-runs?limit=${JOB_RUN_LIMIT}`).then((runs) => {
        lastRuns = runs;
        renderRuns(runs);
      }),
    ];
  }

  if (activeDiagSubtab === "metrics") {
    return [
      fetchJson(`/api/diagnostics/metrics?limit=${DIAG_LIMIT}`).then((rows) => {
        lastDiagnostics.metrics = rows;
        renderDiagnostics();
      }),
    ];
  }

  return [
    fetchJson(`/api/diagnostics/friction?limit=${DIAG_LIMIT}`).then((rows) => {
      lastDiagnostics.friction = rows;
      renderDiagnostics();
    }),
  ];
}

function fetchAndRenderAudit() {
  const sp = new URLSearchParams();
  sp.set("limit", String(AUDIT_LIMIT));
  if (auditFilter.status) sp.set("status", auditFilter.status);
  if (auditFilter.tool) sp.set("tool", auditFilter.tool);
  if (auditFilter.role) sp.set("role", auditFilter.role);
  if (auditFilter.run_id) sp.set("run_id", auditFilter.run_id);
  if (auditFilter.q) sp.set("q", auditFilter.q);
  return fetchJson(`/api/audit?${sp.toString()}`).then((events) => {
    lastAudit = events;
    renderAudit(events);
  });
}

function refreshLabel() {
  if (activeTab === "diagnostics") return `diagnostics/${activeDiagSubtab}`;
  if (activeTab === "run-detail") return `run/${activeRunId || "?"}`;
  return activeTab;
}

function navigateToRun(runId) {
  activeRunId = runId;
  expandedStepIndices = new Set();
  activeRunDetail = null;
  activeRunEvents = [];
  setActiveTab(`runs/${encodeURIComponent(runId)}`);
}

function buildAuditChips() {
  const container = $("audit-filter");
  if (!container) return;
  container.innerHTML = "";
  const allChip = el("button", { class: "chip", text: "all" });
  allChip.addEventListener("click", () => {
    auditFilter.status = null;
    syncAuditControls();
    setActiveTab("audit" + buildAuditHash().slice(6), { refresh: true });
  });
  container.appendChild(allChip);
  for (const status of AUDIT_STATUSES) {
    const chip = el("button", { class: "chip", text: status });
    chip.dataset.status = status;
    chip.style.borderLeft = `2px solid var(--audit-status-${status}, var(--border))`;
    chip.addEventListener("click", () => {
      auditFilter.status = auditFilter.status === status ? null : status;
      syncAuditControls();
      const hash = buildAuditHash();
      window.location.hash = hash;
    });
    container.appendChild(chip);
  }
  syncAuditControls();
}

function wireAuditSearch() {
  const input = $("audit-search");
  if (!input) return;
  let debounce = null;
  input.addEventListener("input", (e) => {
    auditFilter.q = e.target.value.trim();
    if (debounce) clearTimeout(debounce);
    debounce = setTimeout(() => {
      const hash = buildAuditHash();
      if (window.location.hash !== hash) {
        window.location.hash = hash;
      } else {
        refreshDashboard();
      }
    }, 250);
  });
}

const AUDIT_COLUMNS = [
  { key: "time", label: "time" },
  { key: "role", label: "role" },
  { key: "tool", label: "tool" },
  { key: "command", label: "command" },
  { key: "target", label: "target" },
  { key: "status", label: "status" },
  { key: "exit", label: "exit", num: true },
  { key: "duration", label: "duration", num: true },
  { key: "run_id", label: "run id" },
];

function renderAudit(events) {
  const body = $("audit-body");
  if (!body) return;
  $("audit-count").textContent = `${events.length}`;

  if (events.length === 0) {
    syncNodes(body, [el("div", { class: "empty-state" }, [
      el("div", { class: "icon", text: "✧" }),
      el("div", { class: "text", text: "No audit events match the current filter." }),
    ])]);
    return;
  }

  let table = body.querySelector("table.scoreboard-table");
  let tbody;
  if (!table) {
    table = el("table", { class: "scoreboard-table" });
    const thead = el("thead");
    const headRow = el("tr");
    for (const col of AUDIT_COLUMNS) {
      headRow.appendChild(el("th", { class: col.num ? "num" : "", text: col.label }));
    }
    thead.appendChild(headRow);
    table.appendChild(thead);
    tbody = el("tbody");
    table.appendChild(tbody);
    syncNodes(body, [table]);
  } else {
    tbody = table.querySelector("tbody");
  }

  const frag = document.createDocumentFragment();
  for (const ev of events) {
    const exit = ev.exit_code;
    const exitClass = exit != null && exit !== 0 ? "num exit-fail" : "num";
    const tool = ev.tool_name || "-";
    const target = ev.target_id || ev.target_type || "-";
    const cmd = ev.subcommand ? `${ev.command} ${ev.subcommand}` : ev.command;
    const tr = el("tr", { class: "audit-row", title: `event ${ev.id}` });
    tr.dataset.key = `audit-${ev.id}`;
    tr.dataset.hash = `${ev.id}-${ev.status}-${exit}`;
    tr.appendChild(el("td", { text: fmtTimestamp(ev.timestamp) }));
    tr.appendChild(el("td", { text: ev.role || "-" }));
    tr.appendChild(el("td", { text: tool }));
    tr.appendChild(el("td", { text: cmd }));
    tr.appendChild(el("td", { text: target, title: target }));
    const statusTd = el("td");
    statusTd.appendChild(el("span", { class: `audit-status ${ev.status}`, text: ev.status }));
    tr.appendChild(statusTd);
    tr.appendChild(el("td", { class: exitClass, text: exit == null ? "-" : String(exit) }));
    tr.appendChild(el("td", { class: "num", text: fmtDuration(ev.duration_ms) }));
    tr.appendChild(el("td", { text: ev.execution_id || "-", title: ev.execution_id || "" }));
    if (expandedAuditIds.has(ev.id)) tr.classList.add("expanded");
    tr.addEventListener("click", () => {
      if (expandedAuditIds.has(ev.id)) expandedAuditIds.delete(ev.id);
      else expandedAuditIds.add(ev.id);
      renderAudit(lastAudit);
    });
    frag.appendChild(tr);

    if (expandedAuditIds.has(ev.id)) {
      frag.appendChild(buildAuditDetailRow(ev));
    }
  }
  syncNodes(tbody, Array.from(frag.children));
}

function buildAuditDetailRow(ev) {
  const tr = el("tr", { class: "audit-detail-row" });
  tr.dataset.key = `audit-detail-${ev.id}`;
  tr.dataset.hash = JSON.stringify(ev);
  const td = el("td");
  td.colSpan = AUDIT_COLUMNS.length;
  td.addEventListener("click", (e) => e.stopPropagation());

  const meta = el("div", { class: "audit-detail-meta" });
  const addMeta = (label, value) => {
    if (value == null || value === "") return;
    meta.appendChild(el("span", {}, [
      el("span", { class: "label", text: `${label}:` }),
      el("span", { class: "value", text: String(value) }),
    ]));
  };
  addMeta("execution_id", ev.execution_id);
  addMeta("session_id", ev.session_id);
  addMeta("host", ev.host);
  addMeta("pid", ev.pid);
  addMeta("cwd", ev.working_directory);
  addMeta("timestamp", fmtAbsTime(ev.timestamp));
  td.appendChild(meta);

  if (ev.arguments_json) {
    const block = el("div", { class: "audit-detail-block" });
    block.appendChild(el("div", { class: "label", text: "arguments" }));
    let pretty = ev.arguments_json;
    try {
      pretty = JSON.stringify(JSON.parse(ev.arguments_json), null, 2);
    } catch (_) {
      /* leave raw */
    }
    block.appendChild(el("pre", { text: pretty }));
    td.appendChild(block);
  }
  if (ev.stderr_truncated) {
    const block = el("div", { class: "audit-detail-block" });
    block.appendChild(el("div", { class: "label", text: "stderr (truncated)" }));
    block.appendChild(el("pre", { text: ev.stderr_truncated }));
    td.appendChild(block);
  }
  if (ev.stdout_truncated) {
    const block = el("div", { class: "audit-detail-block" });
    block.appendChild(el("div", { class: "label", text: "stdout (truncated)" }));
    block.appendChild(el("pre", { text: ev.stdout_truncated }));
    td.appendChild(block);
  }
  if (ev.error_message) {
    const block = el("div", { class: "audit-detail-block" });
    block.appendChild(el("div", { class: "label", text: "error" }));
    block.appendChild(el("pre", { text: ev.error_message }));
    td.appendChild(block);
  }

  const actions = el("div", { class: "audit-detail-actions" });
  if (ev.execution_id) {
    const btn = el("button", { class: "link-action", text: "View run" });
    btn.addEventListener("click", (e) => {
      e.stopPropagation();
      navigateToRun(ev.execution_id);
    });
    actions.appendChild(btn);
  }
  td.appendChild(actions);

  tr.appendChild(td);
  return tr;
}

function renderRunDetailEmpty(message) {
  const meta = $("run-detail-meta");
  if (meta) syncNodes(meta, [el("div", { class: "empty-state" }, [
    el("div", { class: "icon", text: "✧" }),
    el("div", { class: "text", text: message }),
  ])]);
  $("run-detail-title").textContent = "Run Detail";
  $("run-detail-count").textContent = "-";
  $("run-steps-body").innerHTML = "";
  $("run-events-body").innerHTML = "";
}

function renderRunDetailMeta() {
  const meta = $("run-detail-meta");
  if (!meta) return;
  const detail = activeRunDetail || {};
  const run = detail.run || {};
  $("run-detail-title").textContent = `Run ${run.run_id || activeRunId || "?"}`;
  const stepCount = Array.isArray(detail.steps) ? detail.steps.length : 0;
  $("run-detail-count").textContent = `${stepCount} steps`;

  const grid = el("div", { class: "run-meta-grid" });
  const addCell = (label, value) => {
    const cell = el("div");
    cell.appendChild(el("div", { class: "label", text: label }));
    cell.appendChild(el("div", { class: "value", text: value == null ? "-" : String(value) }));
    grid.appendChild(cell);
  };
  addCell("job", run.job_id);
  addCell("state", run.state);
  addCell("attempt", run.attempt);
  addCell("started", run.started_at ? fmtAbsTime(run.started_at) : "-");
  addCell("finished", run.finished_at ? fmtAbsTime(run.finished_at) : "-");
  addCell("duration", run.duration_ms != null ? fmtDuration(run.duration_ms) : "-");

  const wrap = el("div");
  const back = el("button", { class: "back-action", text: "← back to runs" });
  back.addEventListener("click", () => setActiveTab("diagnostics/runs"));
  wrap.appendChild(el("div", { style: { padding: "8px 16px 0 16px" } }, [back]));
  wrap.appendChild(grid);
  syncNodes(meta, [wrap]);
}

function renderRunSteps() {
  const body = $("run-steps-body");
  if (!body) return;
  const steps = (activeRunDetail && activeRunDetail.steps) || [];
  if (steps.length === 0) {
    syncNodes(body, [el("div", { class: "empty-state" }, [
      el("div", { class: "icon", text: "✧" }),
      el("div", { class: "text", text: "No steps recorded for this run." }),
    ])]);
    return;
  }
  const frag = document.createDocumentFragment();
  for (const step of steps) {
    const exit = step.exit_code;
    const exitClass = exit != null && exit !== 0 ? "exit fail" : "exit";
    const row = el("div", { class: "step-row" }, [
      el("span", { class: "idx", text: `#${step.step_index}` }),
      el("span", { class: "target", text: `${step.target_type}:${step.target_id}` }),
      el("span", {}, [stateCell(step.state)]),
      el("span", { class: "duration", text: fmtDuration(step.duration_ms) }),
      el("span", { class: exitClass, text: exit == null ? "-" : String(exit) }),
    ]);
    row.dataset.key = `step-${step.step_index}`;
    row.dataset.hash = `${step.step_index}-${step.state}-${exit}`;
    if (expandedStepIndices.has(step.step_index)) row.classList.add("expanded");
    row.addEventListener("click", () => {
      if (expandedStepIndices.has(step.step_index)) expandedStepIndices.delete(step.step_index);
      else expandedStepIndices.add(step.step_index);
      renderRunSteps();
    });
    frag.appendChild(row);
    if (expandedStepIndices.has(step.step_index)) {
      frag.appendChild(buildStepDetail(step));
    }
  }
  syncNodes(body, Array.from(frag.children));
}

function buildStepDetail(step) {
  const wrap = el("div", { class: "step-detail" });
  wrap.dataset.key = `step-detail-${step.step_index}`;
  wrap.dataset.hash = JSON.stringify(step);
  wrap.addEventListener("click", (e) => e.stopPropagation());

  const addBlock = (label, raw) => {
    const v = raw == null ? "" : (typeof raw === "string" ? raw : JSON.stringify(raw, null, 2));
    if (!v) return;
    const block = el("div", { class: "audit-detail-block" });
    block.appendChild(el("div", { class: "label", text: label }));
    block.appendChild(el("pre", { text: v }));
    wrap.appendChild(block);
  };

  if (step.error_message) addBlock("error", `${step.error_code || ""} ${step.error_message}`);
  addBlock("agent_response", step.agent_response_json);
  const km = activeRunDetail && activeRunDetail.run && activeRunDetail.run.knowledge_metrics;
  if (km && step.step_index === 0) addBlock("knowledge_metrics (run)", km);
  return wrap;
}

const RUN_EVENT_COLUMNS = [
  { key: "ts", label: "time" },
  { key: "body_kind", label: "kind" },
  { key: "event_type", label: "scope" },
  { key: "agent_identity", label: "agent" },
  { key: "summary", label: "detail" },
];

function renderRunEvents() {
  const body = $("run-events-body");
  if (!body) return;
  const events = activeRunEvents || [];
  if (events.length === 0) {
    syncNodes(body, [el("div", { class: "empty-state" }, [
      el("div", { class: "icon", text: "✧" }),
      el("div", { class: "text", text: "No v2 envelope events for this run." }),
    ])]);
    return;
  }
  let table = body.querySelector("table.scoreboard-table");
  let tbody;
  if (!table) {
    table = el("table", { class: "scoreboard-table" });
    const thead = el("thead");
    const headRow = el("tr");
    for (const col of RUN_EVENT_COLUMNS) {
      headRow.appendChild(el("th", { text: col.label }));
    }
    thead.appendChild(headRow);
    table.appendChild(thead);
    tbody = el("tbody");
    table.appendChild(tbody);
    syncNodes(body, [table]);
  } else {
    tbody = table.querySelector("tbody");
  }
  const frag = document.createDocumentFragment();
  for (let i = 0; i < events.length; i++) {
    const ev = events[i];
    const summary = summarizeEvent(ev);
    const tr = el("tr");
    tr.appendChild(el("td", { text: fmtTimestamp(ev.ts) }));
    tr.appendChild(el("td", { text: ev.body_kind || "-" }));
    tr.appendChild(el("td", { text: ev.event_type || "-" }));
    tr.appendChild(el("td", { text: ev.agent_identity || "-" }));
    const td = el("td", { class: "stderr" });
    td.title = summary.title;
    td.textContent = summary.text;
    tr.appendChild(td);
    tr.dataset.key = `runev-${ev.event_id || i}`;
    tr.dataset.hash = `${ev.event_id || i}-${ev.body_kind}`;
    frag.appendChild(tr);
  }
  syncNodes(tbody, Array.from(frag.children));
}

function summarizeEvent(ev) {
  const ignoreKeys = new Set([
    "schemaVersion", "event_type", "event_id", "ts", "run_id",
    "agent_identity", "parent_event_id", "workspace_path", "body_kind",
  ]);
  const parts = [];
  for (const [k, v] of Object.entries(ev)) {
    if (ignoreKeys.has(k)) continue;
    if (v == null) continue;
    if (typeof v === "object") {
      parts.push(`${k}=${JSON.stringify(v)}`);
    } else {
      parts.push(`${k}=${v}`);
    }
  }
  const text = parts.join(" ");
  return { text: truncate(text, 200), title: text };
}

async function refreshDashboard() {
  if (isRefreshing) return;
  isRefreshing = true;
  const now = new Date();
  $("meta-text").textContent = `fetching...`;
  $("conn-status").className = "status-dot orange";
  const btn = $("refresh-btn");
  if (btn) btn.disabled = true;
  
  let hasErrors = false;
  
  await Promise.all(
    activeRefreshJobs().map((job) =>
      job.catch((e) => {
        hasErrors = true;
        console.error(e);
      }),
    ),
  );
  
  if (hasErrors) {
    $("conn-status").className = "status-dot red";
    $("meta-text").textContent = `offline · ${now.toLocaleTimeString()}`;
  } else {
    $("conn-status").className = "status-dot green";
    $("meta-text").textContent = `refreshed ${refreshLabel()} · ${now.toLocaleTimeString()}`;
  }
  if (btn) btn.disabled = false;
  isRefreshing = false;
  
  $("footer").textContent = `orbit dashboard · auto-refresh 30s · GET /api/{tasks,jobs,job-runs,audit?since|tool|status|role|run_id|q|limit|offset,runs/:id,runs/:id/events?kind|limit|offset,scoreboard,diagnostics/{metrics,friction}}`;
}

buildChips();
wireSearch();
buildAuditChips();
wireAuditSearch();
$("refresh-btn").addEventListener("click", refreshDashboard);
initTabs();
