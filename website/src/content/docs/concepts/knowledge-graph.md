---
title: Knowledge Graph
description: "The parsed codebase structure Orbit builds and exposes to agents."
sidebar:
  order: 5
---

## Definition

The knowledge graph is Orbit's parsed, content-addressed model of a repository. It contains directories, files, extracted symbols, import edges, trait implementors, call sites, and source references.

Agents query the graph when they need code context. The graph gives structured selectors and bounded packs instead of large grep output.

## Commands

```bash
orbit graph build
orbit graph update
orbit graph search task
orbit graph show file:crates/orbit-cli/src/main.rs
```

## Branch Scope

Graph data is branch-scoped. Two worktrees on two branches can rebuild concurrently without corrupting each other. Reads can fall back to the default branch until a new branch has graph data.

## Selectors

Common selectors include:

```text
dir:crates/orbit-cli
file:crates/orbit-cli/src/main.rs
symbol:crates/orbit-cli/src/main.rs#main:function
```
