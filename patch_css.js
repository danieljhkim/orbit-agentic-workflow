const fs = require('fs');
const htmlPath = 'crates/orbit-dashboard/assets/dashboard/index.html';
let html = fs.readFileSync(htmlPath, 'utf8');

const missingCss = `
      .sb-leader-badge {
        color: var(--accent-hover);
        font-size: 10px;
        margin-left: 4px;
      }
      .sb-empty {
        color: var(--border);
        font-family: var(--font-mono);
      }
      .sb-pair-right {
        color: var(--fg-dim);
      }
`;

if (!html.includes('.sb-leader-badge {')) {
  html = html.replace('</style>', missingCss + '\n</style>');
  fs.writeFileSync(htmlPath, html);
}
