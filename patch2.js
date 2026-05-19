const fs = require('fs');
const jsPath = 'crates/orbit-dashboard/assets/dashboard/app.js';
let js = fs.readFileSync(jsPath, 'utf8');

const missingFuncs = `
function rowMaxValue(rows, col) {
  return rows.reduce((max, [, agent]) => Math.max(max, scoreboardColumnValue(agent, col)), 0);
}

function scoreboardColumnValue(agent, col) {
  if (col.format === "pair") {
    return formatScoreboardPair(agent, col).left;
  }
  const value = col.compute ? col.compute(agent) : readPath(agent, col.key);
  return Math.max(0, asScoreboardNumber(value));
}
`;

if (!js.includes('function rowMaxValue')) {
  js = js.replace('function renderUnifiedScoreboardGrid', missingFuncs + '\nfunction renderUnifiedScoreboardGrid');
  fs.writeFileSync(jsPath, js);
}
