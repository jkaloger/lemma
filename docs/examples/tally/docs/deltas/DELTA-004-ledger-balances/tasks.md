- [x] T1 add `Balance` type and the `SUM` query in `src/ledger/balance.rs` (settles AC-1)
- [x] T2 balance endpoint with account lookup first (settles AC-2, AC-3)
- [x] T3 wire route into the router (chore)

## Follow-ups
- [ ] T4 `balance()` loads every entry; DICTUM-003 [DATA-4] caps unbounded reads. Page or aggregate in SQL. (settles AC-2)
- [ ] T5 `balance_sums_entries` asserts on a float; DICTUM-001 [TEST-5]. Use integer cents. (settles AC-1)
