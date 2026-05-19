const fs = require('fs');
const jsPath = 'crates/orbit-dashboard/assets/dashboard/app.js';
let js = fs.readFileSync(jsPath, 'utf8');

const missingFuncs = `
function scaledMetricWidth(value, rowMax) {
  const max = Math.max(0, asScoreboardNumber(rowMax));
  if (max < 3) return Math.min(value * 14, 56);
  return Math.max(2, Math.round((value / max) * 56));
}

function leaderBadge() {
  return el("span", { class: "sb-leader-badge", text: "▲", title: "row leader" });
}
`;

if (!js.includes('function leaderBadge')) {
  js = js.replace('function renderUnifiedScoreboardGrid', missingFuncs + '\nfunction renderUnifiedScoreboardGrid');
  fs.writeFileSync(jsPath, js);
}
