---
title: Local Development
description: "Commands and expectations for working on Orbit locally."
sidebar:
  order: 2
---

## Setup

Run targeted tests while iterating, then run the workspace checks before landing a change.

```bash
cargo test --workspace
make build
make fmt
```

## Website

The website is independent of the Rust workspace.

```bash
cd website
npm install
npm run dev
npm run build
```

`npm run build` syncs the architecture mirror before building.

## Orbit State

Review `.orbit/` changes carefully before committing. Tracked asset changes are product changes. Mutable runtime artifacts are operational data unless the change is intentional.
