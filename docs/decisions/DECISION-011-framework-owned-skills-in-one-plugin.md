---
title: "Framework-owned skills in one plugin"
type: decision
status: superseded
author: "Jack Kaloger"
date: 2026-10-02
tags: [rfc-001]
related: []
---

## Context

From RFC-001 §9 (2026-10-02). The lazyspec verbs are generic over any DAG, the one property lemma does not need. An opinion needs one voice.

## Decision

Framework-owned skills in one plugin, written against lemma's fixed DAG and adapted from the lazyspec verbs where the shape matches. The lazyspec plugin is not a dependency.

Six skills: an entry router plus five group skills (shape, delta, verify, maintain, operate). The twenty-four workflows are reference files inside their group, loaded on demand. The group is a structural question an agent answers reliably. The workflow within a group is a judgement call, so it belongs in a reference file, not a trigger description.

Rejected: one skill per workflow (twenty-four competing trigger descriptions, every amendment a plugin change) and a single router taking the workflow as an argument (cannot trigger from symptoms).

## Consequences

Superseded by DECISION-003: the binary runs the workflow (`lemma review`, `plan`, `build`) and the plugin ships hooks only. Releases 1.0 to 2.1 shipped `entry`, `delta` and reference files; release 3.0 (DELTA-009) deleted them.
