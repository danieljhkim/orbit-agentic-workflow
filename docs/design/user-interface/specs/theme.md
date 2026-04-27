# Spec: Canon Refined Theme

This document defines the formal design tokens and visual language for the Orbit User Interface (Canon Refined aesthetic), superseding the deprecated Trading Terminal theme.

## Why This Exists

As Orbit matures, the extreme constraints of the "Trading Terminal" aesthetic (pitch black, pure monospace, sharp 0px corners) proved too rigid for complex, hierarchical data presentation like nested task plans, conversational review threads, and rich telemetry. The "Canon Refined" theme provides a balanced, modern, high-density dashboard language that maintains a "pro-tool" feel while adopting established UI affordances (subtle rounding, sans-serif readability, softer semantic colors).

## Design Tokens

### Background & Elevation
The theme uses a layered dark mode, relying on subtle lightness shifts rather than shadows.
- `bg`: `#0a0a0a` (Base canvas)
- `bg-elev`: `#17171a` (Cards, panels, buttons)
- `bg-sunk`: `#060607` (Deep wells, expanded detail rows)

### Borders
Borders delineate structure without heavy contrast.
- `border`: `#26262b` (Standard dividers)
- `border-strong`: `#3b3b42` (Active/focused inputs)

### Typography
- **Sans-serif (Primary):** `Inter`, used for prose, titles, and general UI text. Ensures high readability at small sizes.
- **Monospace (Secondary/Data):** `JetBrains Mono`, strictly reserved for IDs, metrics, timestamps, and code snippets.
- **Base Size:** `13px` with `1.5` line height.

### Semantic Colors
Colors are muted but distinct, avoiding harsh neon tones while maintaining semantic meaning.
- **Text:** `--text` (`#ededf0`), `--text-muted` (`#9b9ba3`), `--text-faint` (`#686872`)
- **Accent (Blue):** `--accent` (`#6e9fff`)
- **Success/Done (Green):** `--status-done` (`#4cc38a`)
- **In-Progress (Teal):** `--status-in-progress` (`#5ec8d4`)
- **Review (Purple):** `--status-review` (`#c97cf0`)
- **Warning/Proposed (Amber):** `--status-proposed` (`#f5b14a`)
- **Error/Blocked (Red):** `--status-blocked` (`#ef6b6b`)

### Structural Rules
- **Radii:** Standardized on `4px` for small elements (buttons, chips) and `6px` for large containers (panels).
- **Density:** Padding remains tight (e.g., `12px 16px` for headers, `8px` gaps), but text is allowed to breathe more than in the legacy terminal theme.
- **Animation:** Minimal, purposeful motion. Used primarily for "live" indicators (e.g., a pulsing dot `animation: pulse 2s infinite`).

## Mechanism-specific sections

### Expandable Rows
Data tables use expandable rows (`.row.open`). When expanded:
- The row background shifts to an accent wash (`--accent-low`).
- The expanded detail view drops into a sunken background (`--bg-sunk`) with a 2-column layout (main content + side metadata).
- Caret icons rotate `90deg` for clear state indication.

## Agent Signature
gemini
