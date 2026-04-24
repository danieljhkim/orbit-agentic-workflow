---
title: Policies
description: "How Orbit uses filesystem profiles and global deny rules to scope execution."
sidebar:
  order: 4
---

## Definition

Policy is a filesystem-scoping surface. It controls what an activity can read or modify, then applies global deny rules on top.

An activity can select a named profile with `fsProfile`. If it omits the field, Orbit resolves an implicit unrestricted profile before global denies are applied.

## Shape

```yaml
schemaVersion: 2
kind: Policy
metadata:
  name: default
spec:
  denyRead:
    - "**/*.env"
  denyModify:
    - .orbit/**
    - "**/*.env"
  fsProfiles:
    reviewer:
      read: [./**]
      modify: []
```

## Use

Use narrow profiles for review, summarization, and read-only graph operations. Use broader profiles only when an agent is expected to edit code.
