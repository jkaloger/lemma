In: approval by transition, human-checked receipts, merged `complete`/`archived`, README type list. Removing `waived:` and `verified-by` needs no code; README only.

**24. Approval is the transition.** `approved-by` is removed. A human moving a delta `in-progress`→`complete` approves it; lazyspec's `reviewed: <sha>` stamp records where. Stop reports an error when a delta whose status at HEAD is not `complete` is `complete` in the working tree. A commit made inside the session escapes this, as it escaped the `approved-by` check. Contract mutation no longer invalidates anything; it stays a validate finding.

**25. Human-checked receipt.** `@checked <name>@<sha>` under a claim is a receipt. Fresh when nothing under the governing spec's `governs` changed since `<sha>`. `lemma receipt` does not write it; humans do. `re-pin` leaves it alone; only a human re-dates it. Replaces `verified-by`.

**26. No waivers.** `waived:` is dropped. A claim that should not be machine-checked is `[intent]`.

**27. `complete` folds.** Delta lifecycle is `draft`→`in-progress`→`complete`. The `in-progress`→`complete` gate checks receipts and, when clean, returns the fold updates in the same response. `archived` and the `complete`→`archived` dispatch are removed. By-hand dry-run at `in-progress` reports gate findings plus the `would fold` preview, no write. Amends decisions 6, 7, 14 (DELTA-001).

**28. Five types.** README drops `bug` and `idea`. A bug is a draft delta whose first claim reproduces the defect. An idea is a draft spec with `Intent` only. Release 2 types are `convention` and `dictum`.

Later: DELTA-005 changed the gate (clean tree, base check) and re-pin. DELTA-007 removed receipt pins and `@checked`; DELTA-009 removed the fold and the gate's fold updates, so `complete` is only a transition and the fold preview is gone. Stop's in-session completion check changed so a completion committed in the session is reported once per session (stop tests), not escaped.
