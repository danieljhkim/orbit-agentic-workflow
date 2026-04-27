# User Interface — Vision

**Status:** Draft
**Owner:** gemini
**Last updated:** 2026-04-26

This document scopes the forward-looking evolution of the Orbit UI. While the current implementation establishes the "Canon Refined" aesthetic, the long-term vision aims to enhance dynamic interactivity, elevate the branding through motion, and build a unified design system that bridges the local CLI tooling with the public project presence.

## 1. Open Questions

1. **Framework Adoption**: As the dashboard grows to handle complex state (e.g., interactive graph visualization, live agent duel debugging), do we migrate from vanilla HTML/JS to a lightweight framework (e.g., Preact or Svelte), and at what cost to bundle size and build complexity?
2. **Logo Animation**: How do we evolve the static "two orbiting circles" logo into a dynamic, interactive element that responds to actual system load or telemetry while adhering to the monochromatic aesthetic?
3. **Component Unification**: How do we share UI components between the Rust-served local dashboard and the static website without introducing heavy Node.js build steps?

## 2. Prior Work

### Terminal and TUI Tools
- **k9s & htop**: Excellent examples of data-dense, keyboard-driven terminal interfaces. They validate the utility of high-contrast text and tabular density but lack the graphical fidelity we can achieve in a web view.
- **Bloomberg Terminal**: The industry standard for high-density, high-stakes data presentation. It proves that users tolerate steep learning curves if the interface maximizes raw data throughput.

### Modern Pro-Tools
- **Linear & Vercel**: High watermarks for modern, keyboard-centric web design. The Canon Refined aesthetic draws heavily from their principles of layered dark modes, subtle borders, and precise typography, while dialing up the data density required for our specific use case.

## 3. What May Be Distinctive

Orbit's UI approach is distinct because it deliberately pairs cutting-edge autonomous agent technology with a high-density, modern "pro-tool" visual language. Instead of abstracting away the complexity behind friendly chatbots, Orbit leans into the complexity, surfacing raw graphs, execution traces, and telemetry in a stark, uncompromising dashboard. The "Canon Refined" aesthetic signals to users that Orbit is a serious tool for engineers, not a toy.

## 4. References

- [Canon Refined Theme Spec](./specs/theme.md) (Internal)

## Task References

- [T20260427-29]

> Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
