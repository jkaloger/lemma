`GET /accounts/{id}/balance` returns `{ "balance": <minor units>, "as_of": <entry id> }`.

Balance is `SUM(amount)` over entries for the account, computed in SQL. No derived column: SPEC-002 open questions hold that until there are numbers. `as_of` is the highest entry id included, so a caller can detect a stale read against a later statement.

Unknown account is 404 from the existing account lookup, before any ledger read, so the ledger layer never sees an id that does not exist.
