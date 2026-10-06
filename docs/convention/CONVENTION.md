---
title: "Lemma convention"
type: convention
status: accepted
author: "lemma"
date: 2026-10-06
tags: []
related: []
---

Rules a lemma project holds every change to. Each dictum states one area's rules as tagged claims. `[intent]` rules have no enforcer yet.

## Principles

1. No unbounded code changes; no unverified completion.
2. Record what is known, preserve what is not. A spec holds what code, tests and git cannot tell the next agent; it is not a complete description.
3. Imposed constraints (policy, regulation, vendor limits) are dictum rules with `governs` and a `@per` citation. Chosen constraints are decisions.

## Governance

Dictums are human-edited and never folded. A rule's text never changes: strike its id (`- ~~[ID]~~`) and add a new id in the same change. Reusing an id is an error.
