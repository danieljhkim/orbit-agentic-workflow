# User Interface — Design

**Status:** Draft
**Owner:** gemini
**Last updated:** 2026-04-26

This document details the active implementation of the Orbit UI, covering the visual design constraints and structural layout of both the local dashboard and the website. It focuses on the mechanisms employed to enforce the "Canon Refined" aesthetic.

## 1. High-Density Layout Mechanism

The dashboard prioritizes maximizing data density while maintaining structural clarity. We employ a modular grid system that packs tabular data, telemetry feeds, and state indicators into a single view without scrolling where possible. 
- Elements use standardized, subtle border radii (`4px` for small elements, `6px` for panels).
- Expandable rows use sunken backgrounds (`--bg-sunk`) to nest detailed hierarchies without sacrificing the root tabular view.

## 2. Layered Dark Palette

The entire UI is anchored on a layered dark mode (`#0a0a0a` base, `#17171a` elevated cards) rather than flat pitch black.
- This mitigates eye strain while providing clear visual depth.
- Semantic coloring is muted but distinct: `--status-done` (`#4cc38a`), `--status-in-progress` (`#5ec8d4`), and `--status-blocked` (`#ef6b6b`).
- Accents use a soft blue (`#6e9fff`) rather than harsh neons.

## 3. Typography Rules

The UI employs a dual-typography approach:
- `Inter` (sans-serif) is used for prose, headers, and UI elements to maximize readability.
- `JetBrains Mono` is strictly reserved for data points, IDs, metrics, timestamps, and logs to ensure tabular alignment.
- Base size is `13px` to maintain high density without compromising legibility.

## 4. Real-Time Status Indication

To provide a "live telemetry" feel, status markers rely on visual motion and high contrast. Spinners, blinking dots, and updating ticker formats are used to communicate active processing without requiring the user to read underlying text logs.

## 5. Concerns & Honest Limitations

- **Accessibility**: The extreme contrast, small font sizes, and dense layouts may pose readability challenges for some users. We lean on the assumption that the primary audience is technical professionals, but true WCAG compliance is currently a secondary priority.
- **Responsive Design**: True high-density "terminal" layouts are difficult to adapt gracefully to mobile or narrow viewports. The current implementation heavily favors wide desktop displays.
- **Component Reusability**: Relying strictly on raw CSS variables and HTML currently means component logic is duplicated across the dashboard and website, lacking a formalized UI framework like React or Vue.

## Task References

- [T20260427-29]

> Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
