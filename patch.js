const fs = require('fs');

const htmlPath = 'crates/orbit-dashboard/assets/dashboard/index.html';
let html = fs.readFileSync(htmlPath, 'utf8');

// Replace CSS
const newCss = `
      .sb-unified-grid {
        width: 100%;
        border-collapse: collapse;
        font-size: 11px;
        table-layout: fixed;
      }
      .sb-unified-grid th, .sb-unified-grid td {
        padding: 4px 6px;
        border-bottom: 1px solid var(--border);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
      }
      .sb-unified-grid th {
        color: var(--fg-dim);
        font-weight: 600;
        text-transform: uppercase;
        position: sticky;
        top: 0;
        z-index: 1;
        background: var(--bg-elev);
      }
      .sb-unified-grid th.sb-category-head {
        border-bottom: 1px solid rgba(255, 255, 255, 0.05);
        font-size: 10px;
        padding-bottom: 2px;
        box-shadow: none;
      }
      .sb-unified-grid th.sb-metric-head {
        padding-top: 2px;
        box-shadow: 0 1px 0 var(--border);
      }
      .sb-unified-grid th.sb-agent-head {
        box-shadow: 0 1px 0 var(--border);
      }
      .sb-unified-grid tbody tr:hover {
        background: rgba(255, 255, 255, 0.03);
      }
      .sb-unified-grid td.sb-metric-cell {
        position: relative;
        text-align: right;
        font-family: var(--font-mono);
      }
      .sb-unified-grid td.sb-agent-cell {
        font-weight: 500;
        color: var(--accent-hover);
        text-align: left;
      }
      .sb-unified-grid th .sb-th-content {
        display: flex;
        justify-content: flex-end;
        align-items: center;
        gap: 2px;
      }
      .sb-unified-grid th.clickable {
        cursor: pointer;
      }
      .sb-unified-grid th.clickable:hover {
        color: var(--fg);
      }
      .perf-bar {
        position: absolute;
        bottom: 1px;
        left: 0;
        height: 2px;
        background: var(--accent);
        opacity: 0.5;
        border-radius: 1px;
      }
      .sb-leader .perf-bar {
        background: var(--accent-hover);
        opacity: 1;
      }
      .zero-dot {
        color: transparent;
      }
`;

html = html.replace(/table\.sb-leaderboard \{[\s\S]*?\}\s*\.sb-section-divider td \{[\s\S]*?\}/, newCss);
fs.writeFileSync(htmlPath, html);


const jsPath = 'crates/orbit-dashboard/assets/dashboard/app.js';
let js = fs.readFileSync(jsPath, 'utf8');

const jsReplacement = `let currentScoreboardSort = { columnKey: null, dir: -1 };

function setScoreboardSort(key) {
  if (currentScoreboardSort.columnKey === key) {
    currentScoreboardSort.dir = -currentScoreboardSort.dir;
  } else {
    currentScoreboardSort = { columnKey: key, dir: -1 };
  }
  renderScoreboard(lastSummary);
}

function renderScoreboard(summary) {
  const body = $("scoreboard-body");
  lastSummary = summary;

  const agentsMap = (summary && summary.agents) || {};
  const entries = Object.entries(agentsMap);
  $("scoreboard-count").textContent = \`\${entries.length} Agents\`;

  if (entries.length === 0) {
    syncNodes(body, [el("div", { class: "empty-state" }, [
      el("div", { class: "icon", text: "✧" }),
      el("div", { class: "text", text: "No scoreboard data yet." })
    ])]);
    return;
  }

  const canonicalRows = canonicalScoreboardRows(agentsMap);
  const otherRows = entries
    .filter(([name]) => !CANONICAL_SCOREBOARD_SET.has(name))
    .filter(([, agent]) => scoreboardSignalForColumns(agent, OTHER_SCOREBOARD_COLUMNS) > 0)
    .sort(([a], [b]) => a.localeCompare(b));

  let sectionsToRender = ALL_SCOREBOARD_SECTIONS;
  const showAttribution = otherRows.length > 0;
  if (showAttribution) {
    sectionsToRender = ALL_SCOREBOARD_SECTIONS.concat([
      { title: "Attribution Cleanup", columns: OTHER_SCOREBOARD_COLUMNS }
    ]);
  }

  const allRows = canonicalRows.concat(otherRows);

  const sections = [
    renderUnifiedScoreboardGrid(allRows, sectionsToRender, showAttribution),
  ];

  syncNodes(body, [el("div", { class: "scoreboard-sections" }, sections)]);
}

function renderUnifiedScoreboardGrid(rows, sections, showAttribution) {
  if (!rows.length) {
    return el("div", { class: "empty-state compact", text: "No rows." });
  }

  const table = el("table", { class: "scoreboard-table sb-unified-grid" });
  const thead = el("thead");
  
  const categoryRow = el("tr");
  categoryRow.appendChild(el("th", { class: "sb-agent-head", text: "AGENT", rowSpan: 2 }));
  
  for (const section of sections) {
    const validCols = section.columns.filter(c => c.key !== "agent");
    const th = el("th", { 
      class: "sb-category-head", 
      text: section.title, 
      colSpan: validCols.length 
    });
    th.style.textAlign = "center";
    categoryRow.appendChild(th);
  }
  thead.appendChild(categoryRow);
  
  const headerRow = el("tr");
  for (const section of sections) {
    for (const col of section.columns.filter(c => c.key !== "agent")) {
      const th = el("th", { class: "sb-metric-head clickable num", title: col.title || col.label });
      th.addEventListener("click", () => setScoreboardSort(col.key));
      
      const content = [el("span", { text: col.label })];
      if (currentScoreboardSort.columnKey === col.key) {
        content.push(el("span", { class: "sort-arrow", text: currentScoreboardSort.dir === 1 ? " ▲" : " ▼" }));
      }
      th.appendChild(el("div", { class: "sb-th-content" }, content));
      headerRow.appendChild(th);
    }
  }
  thead.appendChild(headerRow);
  table.appendChild(thead);
  
  let sortedRows = rows.slice();
  if (currentScoreboardSort.columnKey) {
    let sortCol = null;
    for (const sec of sections) {
      sortCol = sec.columns.find(c => c.key === currentScoreboardSort.columnKey);
      if (sortCol) break;
    }
    if (sortCol) {
      sortedRows.sort((a, b) => {
        const valA = scoreboardColumnValue(a[1], sortCol);
        const valB = scoreboardColumnValue(b[1], sortCol);
        if (valA !== valB) {
          return (valA - valB) * currentScoreboardSort.dir;
        }
        return a[0].localeCompare(b[0]);
      });
    }
  }
  
  const tbody = el("tbody");
  
  const colMaxes = new Map();
  for (const section of sections) {
    for (const col of section.columns.filter(c => c.key !== "agent")) {
      colMaxes.set(col.key, rowMaxValue(sortedRows, col));
    }
  }

  for (const [name, agent] of sortedRows) {
    const tr = el("tr");
    const tdAgent = el("td", { class: "sb-agent-cell agent clickable", text: name });
    tdAgent.addEventListener("click", () => navigateToRole(name));
    tr.appendChild(tdAgent);
    
    for (const section of sections) {
      for (const col of section.columns.filter(c => c.key !== "agent")) {
        const rowMax = colMaxes.get(col.key);
        const value = scoreboardColumnValue(agent, col);
        const isLeader = rowMax > 0 && value === rowMax;
        
        const td = el("td", { class: \`sb-metric-cell num\${value === 0 ? " zero" : ""}\${isLeader ? " sb-leader" : ""}\` });
        td.title = \`\${col.title || col.label}: \${value}\`;
        
        const pct = rowMax > 0 ? Math.min(100, Math.max(2, (value / rowMax) * 100)) : 0;

        if (col.format === "pair") {
          const pair = formatScoreboardPair(agent, col);
          const bar = el("div", { class: "perf-bar" });
          bar.style.width = \`\${pct}%\`;
          
          if (pair.zero) {
            td.appendChild(el("span", { class: "zero-dot", innerHTML: "&middot;" }));
          } else {
            if (value > 0) td.appendChild(bar);
            const valSpan = el("span", { class: "sb-value" });
            valSpan.appendChild(pairTextNodesSpan(pair.left, pair.right, pair.zero));
            if (isLeader) valSpan.appendChild(leaderBadge());
            td.appendChild(valSpan);
          }
        } else {
          const bar = el("div", { class: "perf-bar" });
          bar.style.width = \`\${pct}%\`;
          
          if (value === 0) {
            td.appendChild(el("span", { class: "zero-dot", innerHTML: "&middot;" }));
          } else {
            if (value > 0) td.appendChild(bar);
            const valSpan = el("span", { class: "sb-value" }, [
              document.createTextNode(fmtScoreboardCount(value)),
              ...(isLeader ? [leaderBadge()] : [])
            ]);
            td.appendChild(valSpan);
          }
        }
        tr.appendChild(td);
      }
    }
    tbody.appendChild(tr);
  }
  table.appendChild(tbody);
  return table;
}

function pairTextNodesSpan(left, right, zero) {
  const frag = document.createDocumentFragment();
  if (zero) {
    frag.appendChild(el("span", { class: "sb-empty", text: "—" }));
    return frag;
  }
  frag.appendChild(left === 0 ? el("span", { class: "sb-empty", text: "—" }) : el("span", { class: "sb-value", text: fmtScoreboardCount(left) }));
  frag.appendChild(document.createTextNode("/"));
  frag.appendChild(right === 0 ? el("span", { class: "sb-empty", text: "—" }) : el("span", { class: "sb-pair-right", text: fmtScoreboardCount(right) }));
  return frag;
}
`;

const replaceRegex = /function renderScoreboard\(summary\) \{[\s\S]*?function renderDuelMatrixSection\(summary\) \{[\s\S]*?return section;\n\}/;
js = js.replace(replaceRegex, jsReplacement);

fs.writeFileSync(jsPath, js);
