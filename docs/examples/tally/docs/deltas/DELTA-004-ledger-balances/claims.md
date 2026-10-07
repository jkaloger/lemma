- [AC-1] The balance of an account is the sum of its entries in minor units.
  @ref tests/ledger.rs::balance_sums_entries
- [AC-2] An account that exists and has no entries has balance zero.
  @ref tests/ledger.rs::balance_of_empty_account_is_zero
- [AC-3] A balance request for an unknown account is a 404 and performs no ledger read.
  @ref tests/ledger.rs::balance_rejects_unknown_account
- [AC-4] [intent] A balance read of one account costs one query regardless of entry count.
