---
title: "Ledger"
type: spec
status: active
governs: ["src/ledger/**", "tests/ledger.rs"]
---

## Intent

Append-only record of money moving between accounts. Every read is derivable from entries; nothing is stored that an entry does not imply.

## Constraints

- Amounts are integer minor units. No floats anywhere in this area.
- An entry is never updated or deleted. Corrections are new entries.

## Non-goals

- Multi-currency. One currency per ledger, set at creation.
- Querying an account that does not exist is an error, not an empty result.

## Open questions

- Whether running balances need a materialised column once entries pass ~1M. DELTA-004 chose derive-on-read; revisit with numbers.
