import { execFileSync } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const websiteRoot = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const repoRoot = path.dirname(websiteRoot);
const targetRoot = path.join(websiteRoot, 'src', 'content', 'docs', 'tasks');
const githubPullBaseUrl = 'https://github.com/danieljhkim/orbit/pull/';

const tasks = await loadTasks();

await rm(targetRoot, { recursive: true, force: true });
await mkdir(targetRoot, { recursive: true });

const sortedTasks = [...tasks].sort((a, b) => (b.updated_at ?? '').localeCompare(a.updated_at ?? ''));

for (const [index, task] of sortedTasks.entries()) {
  const file = path.join(targetRoot, `${task.id}.md`);
  await writeFile(file, renderTaskPage(task, index + 10), 'utf8');
}

await writeFile(path.join(targetRoot, 'index.md'), renderIndex(sortedTasks), 'utf8');

async function loadTasks() {
  const override = process.env.ORBIT_TASK_LIST_JSON;
  if (override !== undefined) {
    return JSON.parse(override);
  }
  const raw = execFileSync('orbit', ['task', 'list', '--status', 'done', '--json'], {
    cwd: repoRoot,
    encoding: 'utf8',
    maxBuffer: 256 * 1024 * 1024,
  });
  return JSON.parse(raw);
}

function renderTaskPage(task, sidebarOrder) {
  const summary = pickSummary(task.description, task.title);
  const frontmatter = [
    '---',
    `title: ${JSON.stringify(task.title)}`,
    `description: ${JSON.stringify(summary)}`,
    `slug: tasks/${task.id}`,
    'sidebar:',
    `  label: ${JSON.stringify(task.id)}`,
    `  order: ${sidebarOrder}`,
    '---',
    '',
  ].join('\n');

  const sections = [];
  pushSection(sections, 'Description', task.description);
  pushAcceptanceCriteria(sections, task.acceptance_criteria);
  pushSection(sections, 'Plan', task.plan);
  pushSection(sections, 'Execution Summary', task.execution_summary);
  pushHistory(sections, task.history);
  pushAttribution(sections, task);
  pushExternalRefs(sections, task.external_refs);

  return frontmatter + sections.join('\n\n') + '\n';
}

function renderIndex(tasks) {
  const frontmatter = [
    '---',
    'title: "Done Tasks"',
    'description: "Index of completed Orbit tasks synced to the website."',
    'tableOfContents: false',
    'sidebar:',
    '  order: 1',
    '  label: "All Done Tasks"',
    '---',
    '',
  ].join('\n');

  if (tasks.length === 0) {
    return `${frontmatter}# Done Tasks\n\n_No done tasks yet._\n`;
  }

  const sorted = [...tasks].sort((a, b) => (b.updated_at ?? '').localeCompare(a.updated_at ?? ''));
  const generated = new Date().toISOString().replace('T', ' ').replace(/\.\d+Z$/, ' UTC');
  const intro = `_${sorted.length} done task${sorted.length === 1 ? '' : 's'}. Generated ${generated}._`;
  const headers = ['ID', 'Title', 'Type', 'Implemented by', 'Updated'];
  const rows = sorted.map((task) => [
    `[${task.id}](/tasks/${task.id}/)`,
    escapeCell(task.title ?? ''),
    escapeCell(task.type ?? ''),
    escapeCell(task.implemented_by ?? '—'),
    formatDate(task.updated_at),
  ]);

  const table = markdownTable(headers, rows);
  return `${frontmatter}# Done Tasks\n\n${intro}\n\n${table}\n`;
}

function pushSection(sections, heading, value) {
  if (!hasContent(value)) return;
  sections.push(`## ${heading}\n\n${value.trim()}`);
}

function pushAcceptanceCriteria(sections, criteria) {
  if (!Array.isArray(criteria) || criteria.length === 0) return;
  const list = criteria.map((c, i) => `${i + 1}. ${c}`).join('\n');
  sections.push(`## Acceptance Criteria\n\n${list}`);
}

function pushHistory(sections, history) {
  if (!Array.isArray(history) || history.length === 0) return;
  const lines = history.map((entry) => {
    const when = formatTimestamp(entry.at);
    const by = entry.by ?? 'unknown';
    const event = entry.event ?? 'event';
    const transition = entry.from_status && entry.to_status
      ? ` (${entry.from_status} → ${entry.to_status})`
      : entry.to_status
      ? ` (→ ${entry.to_status})`
      : '';
    const note = entry.note ? ` — _${entry.note}_` : '';
    return `- ${when} — \`${by}\`: ${event}${transition}${note}`;
  });
  sections.push(`## History\n\n${lines.join('\n')}`);
}

function pushAttribution(sections, task) {
  const lines = [];
  if (hasContent(task.created_by)) lines.push(`- **Created by:** \`${task.created_by}\``);
  if (hasContent(task.planned_by)) lines.push(`- **Planned by:** \`${task.planned_by}\``);
  if (hasContent(task.implemented_by)) lines.push(`- **Implemented by:** \`${task.implemented_by}\``);
  if (lines.length === 0) return;
  sections.push(`## Model Attribution\n\n${lines.join('\n')}`);
}

function pushExternalRefs(sections, refs) {
  if (!Array.isArray(refs) || refs.length === 0) return;
  const lines = refs.map((ref) => {
    const system = ref.system ?? '';
    const id = ref.id ?? '';
    if (ref.url) {
      return `- [${system}: ${id}](${ref.url})`;
    }
    if (system === 'github-pr' && id) {
      return `- [github-pr #${id}](${githubPullBaseUrl}${id})`;
    }
    return `- ${system}: ${id}`;
  });
  sections.push(`## External Refs\n\n${lines.join('\n')}`);
}

function pickSummary(description, fallback) {
  if (!hasContent(description)) return fallback ?? '';
  for (const rawLine of description.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (line.length === 0) continue;
    if (line.startsWith('#')) continue;
    if (line.startsWith('>')) continue;
    return truncate(line, 200);
  }
  return fallback ?? '';
}

function truncate(value, max) {
  if (value.length <= max) return value;
  return `${value.slice(0, max - 1).trimEnd()}…`;
}

function escapeCell(value) {
  return String(value).replace(/\|/g, '\\|').replace(/\r?\n/g, ' ');
}

function formatDate(iso) {
  if (!iso) return '—';
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return date.toISOString().slice(0, 10);
}

function formatTimestamp(iso) {
  if (!iso) return 'unknown time';
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return date.toISOString().replace('T', ' ').replace(/\.\d+Z$/, ' UTC');
}

function markdownTable(headers, rows) {
  const headerLine = `| ${headers.join(' | ')} |`;
  const separator = `| ${headers.map(() => '---').join(' | ')} |`;
  const body = rows.map((row) => `| ${row.join(' | ')} |`).join('\n');
  return [headerLine, separator, body].join('\n');
}

function hasContent(value) {
  return typeof value === 'string' && value.trim().length > 0;
}
