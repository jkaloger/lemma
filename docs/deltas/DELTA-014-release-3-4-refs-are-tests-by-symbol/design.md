Check 3 stays in `evidence::resolve`. `names_a_test` (path glob) becomes one of two accepted answers.

**Resolver.** After a ref resolves to a symbol, `evidence::resolve` asks the extractor whether the symbol is a test. `RustSymbolExtractor` reports test attributes on the symbol and whether an enclosing module is `#[cfg(test)]`. `TypeScriptSymbolExtractor` reports `it`/`test` blocks. A ref passes if the symbol is a test or its file matches the governing spec's `tests`. A language with no extractor passes on the glob only, as today. The diagnostic for a failing ref names both ways to fix it.

**Build.** `build.rs` resolves each ref in a runner or fix reply with the same function lint calls, before writing `claims.md`. A failing ref feeds the existing fix round with the diagnostic text; when rounds run out the task stops as blocked. Nothing unresolved is written.

**Prompts.** `.lazyspec/prompts/build.md` and `fix.md` state that tests may live in a `#[cfg(test)]` module or as `#[test]` fns in the same file, in `tests/`, or in any file the spec lists under `tests`, and that a ref must point at the test fn, not the code it tests.

**Chore.** Remove `crates/lemma/src/activity.rs` from SPEC-002 `tests`. `governs` keeps it.
