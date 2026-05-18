---
title: Agent Observation Conventions
owner: daniel
last_updated: 2026-05-18
---

# Agent Observation Conventions

Rules for writing and maintaining files under `docs/agent-observations/`. The goal is a directory that reads as a single longitudinal log — entries comparable across windows, claims grounded in data, and a clear graduation path to lessons and ADRs.

This doc is itself the source of truth for the conventions. When a convention changes, update this doc and then update existing observation files to match — do not silently diverge.

---

## 1. Purpose

`docs/agent-observations/` is a running log of how agents (codex, claude, grok, gemini, gpt-5.x, …) actually behave when given Orbit tooling — tool-surface choice, tool reach, blind spots, failure modes — grounded in audit data, benchmark runs, or session transcripts.

This is **not** the place for:

- Distilled principles → [`../LESSONS.md`](../LESSONS.md)
- Decisions about how Orbit will change in response → ADRs in `docs/design/<feature>/4_decisions.md`
- Speculation without data

---

## 2. File Layout

```
docs/agent-observations/
├── CONVENTION.md                            this file
├── YYYY-MM-DD-<short-slug>.md               one observation per file
└── YYYY-MM-DD-<short-slug>.md
```

- Filename date is when the observation was **recorded**, not when the underlying behavior happened. The data window is declared in frontmatter.
- Slug is lowercase, hyphenated, ≤ 6 words.
- One observation per file. If you find a second pattern in the same data, write a second file.
- No subdirectories. No `README.md`.

---

## 3. Required Frontmatter

Every observation starts with YAML frontmatter:

```yaml
---
title: <one-line headline>
owner: <agent family — codex, claude, grok, gemini>
last_updated: YYYY-MM-DD
recorded: YYYY-MM-DD
tags: [tag1, tag2, ...]
---
```

- `title` mirrors the H1 verbatim.
- `owner` is the agent family, not a full model string.
- `last_updated` is the calendar date the file last had a meaningful edit. Distinct from `recorded`, which is fixed at creation.
- `recorded` is the date the observation was made. Never changes after the file is created.
- `tags` use lowercase-hyphenated form. Reuse existing tags before inventing new ones — grep the directory first.

The data window and source belong in the body (typically a metadata block under the H1, before the TL;DR) — not in frontmatter. They're prose context, not machine-readable indexing.

---

## 4. Required Sections

| Section | Required | Purpose |
|---|---|---|
| `# <title>` | yes | Matches frontmatter `title`. |
| `## TL;DR` | yes | Two or three sentences. The headline finding, stated as a claim. |
| Data tables | yes | At least one. Raw counts plus a derived ratio. Markdown tables only — no images, no charts. |
| `## Caveats` | yes | What this data does not prove. Sample size, attribution risks, selection bias. |
| `## Open questions` | recommended | Things that would sharpen or refute the finding if we had more data. |
| `## Reproducing this` | recommended | The exact command(s) and jq pipeline used. Future readers should be able to re-run on a fresh window. |

Other sections (mechanism, hypotheses, knock-on effects) are encouraged where they earn their place. Skip them if there's nothing to say.

---

## 5. Claim Standard

- **Ratios are load-bearing; absolute counts are suggestive.** Cross-model comparisons must normalize. If you cite a raw number, state what's in the denominator.
- **Same-window comparisons only.** Don't compare a 3-day codex slice to a 30-day claude slice.
- **Distinguish behavior from wiring.** Before claiming a model prefers X, verify the alternative was actually available and equally easy to reach.
- **Attribute carefully.** The `role` field in audit events comes from the caller's self-identification. A row tagged `claude` may be a human invocation in a claude session.

---

## 6. Graduation

If a behavior holds across multiple windows and starts shaping product decisions:

- Lift the **principle** into [`../LESSONS.md`](../LESSONS.md).
- Capture the **response** as an ADR in the relevant `docs/design/<feature>/4_decisions.md`.
- Leave the raw observation file in place. It's the receipt.

Do not edit the original observation to backfill what we later did about it. Write a follow-up observation that cites the original.

---

## 7. What Not to Do

- Don't edit an observation's `recorded` date after the fact. If the data changes, write a new file.
- Don't author observations on data you didn't pull yourself or can't reproduce.
- Don't bury the finding under prose. The TL;DR and the first table should carry the claim; everything else is supporting evidence.
- Don't speculate about model internals beyond what the data warrants. "OpenAI models picked CLI 93% of the time" is a finding; "OpenAI models were trained to prefer shell" is a hypothesis — label it as one.
